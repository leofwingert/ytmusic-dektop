use crate::action::Action;
use crate::api::auth::Auth;
use crate::api::YtMusicClient;
use crate::media_keys::MediaControlsManager;
use crate::player::Player;
use crate::resolver::Resolver;
use crate::settings::Settings;
use crate::state::{AppState, AppTheme, PlayerStatus, RepeatMode, TrackMeta};
use log::{error, info, warn};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// The backend runs on a dedicated tokio runtime thread.
/// It receives Actions from the UI, processes them, and updates AppState.
pub struct Backend;

impl Backend {
    /// Spawn the backend on a new thread with its own tokio runtime.
    pub fn spawn(
        state: Arc<RwLock<AppState>>,
        ctx: egui::Context,
    ) -> mpsc::UnboundedSender<Action> {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(Self::run(rx, tx_clone, state, ctx));
        });

        tx
    }

    async fn run(
        mut rx: mpsc::UnboundedReceiver<Action>,
        self_tx: mpsc::UnboundedSender<Action>,
        state: Arc<RwLock<AppState>>,
        ctx: egui::Context,
    ) {
        // Load persistent settings
        let settings = Settings::load();
        {
            let mut s = state.write().unwrap();
            s.volume = settings.volume;
            s.shuffle = settings.shuffle;
            s.repeat_mode = match settings.repeat_mode.as_str() {
                "all" => RepeatMode::All,
                "one" => RepeatMode::One,
                _ => RepeatMode::Off,
            };
            s.theme = match settings.theme.as_str() {
                "light" => AppTheme::Light,
                "amoled" => AppTheme::Amoled,
                _ => AppTheme::Dark,
            };
        }

        // Check yt-dlp availability
        match Resolver::check_ytdlp().await {
            Ok(version) => info!("yt-dlp ready: {version}"),
            Err(e) => {
                error!("{e}");
                if let Ok(mut s) = state.write() {
                    s.status = PlayerStatus::Error(e);
                }
                ctx.request_repaint();
                return;
            }
        }

        // Initialize audio player
        let mut player = match Player::new() {
            Ok(mut p) => {
                p.set_volume(settings.volume);
                p
            }
            Err(e) => {
                error!("Player init failed: {e}");
                if let Ok(mut s) = state.write() {
                    s.status = PlayerStatus::Error(e);
                }
                ctx.request_repaint();
                return;
            }
        };

        // Initialize YouTube Music API client
        let api_client = match Auth::try_load_cookies() {
            Some(cookies) => {
                info!("Authenticated with cookies");
                if let Ok(mut s) = state.write() {
                    s.is_authenticated = true;
                }
                YtMusicClient::new(cookies)
            }
            None => {
                info!("Running in anonymous mode (no cookies found)");
                if let Ok(mut s) = state.write() {
                    s.is_authenticated = false;
                    s.auth_error = Some("No cookies found. Export your browser cookies to authenticate.".to_string());
                }
                YtMusicClient::anonymous()
            }
        };

        // Initialize Media keys and MPRIS
        let mut media_controls = MediaControlsManager::new(self_tx.clone());

        ctx.request_repaint();

        // Progress ticker
        let ctx_tick = ctx.clone();
        let _tick_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                ctx_tick.request_repaint();
            }
        });

        info!("Backend ready, waiting for actions...");

        while let Some(action) = rx.recv().await {
            match action {
                Action::Play(input) => {
                    Self::handle_play_input(&mut player, &state, &ctx, &input).await;
                }

                Action::PlayTrack(track) => {
                    Self::handle_play_meta(&mut player, &state, &ctx, track).await;
                }

                Action::PlaySearchResult(result) => {
                    let track = TrackMeta {
                        title: result.title,
                        artist: result.artist,
                        thumbnail_url: result.thumbnail_url,
                        duration_secs: 0.0,
                        video_id: result.video_id,
                    };
                    Self::handle_play_meta(&mut player, &state, &ctx, track).await;
                }

                Action::Pause => {
                    player.pause();
                    let mut s = state.write().unwrap();
                    s.status = PlayerStatus::Paused;
                    s.progress_secs = player.progress_secs();
                    ctx.request_repaint();
                }

                Action::Resume => {
                    player.resume();
                    let mut s = state.write().unwrap();
                    s.status = PlayerStatus::Playing;
                    ctx.request_repaint();
                }

                Action::TogglePlayPause => {
                    let current_status = { state.read().unwrap().status.clone() };
                    match current_status {
                        PlayerStatus::Playing => {
                            player.pause();
                            let mut s = state.write().unwrap();
                            s.status = PlayerStatus::Paused;
                            s.progress_secs = player.progress_secs();
                        }
                        PlayerStatus::Paused => {
                            player.resume();
                            let mut s = state.write().unwrap();
                            s.status = PlayerStatus::Playing;
                        }
                        _ => {
                            let next = { state.write().unwrap().queue.pop_front() };
                            if let Some(t) = next {
                                Self::handle_play_meta(&mut player, &state, &ctx, t).await;
                            }
                        }
                    }
                    ctx.request_repaint();
                }

                Action::Stop => {
                    player.stop();
                    let mut s = state.write().unwrap();
                    s.status = PlayerStatus::Idle;
                    s.current_track = None;
                    s.progress_secs = 0.0;
                    s.duration_secs = 0.0;
                    ctx.request_repaint();
                }

                Action::Seek(pos) => {
                    player.seek(pos);
                    let mut s = state.write().unwrap();
                    s.progress_secs = player.progress_secs();
                    ctx.request_repaint();
                }

                Action::SetVolume(vol) => {
                    let vol = vol.clamp(0.0, 1.0);
                    player.set_volume(vol);
                    let mut s = state.write().unwrap();
                    s.volume = vol;
                    Self::save_settings(&s);
                    ctx.request_repaint();
                }

                Action::QueueAdd(track) => {
                    let mut s = state.write().unwrap();
                    info!("Added to queue: {} - {}", track.artist, track.title);
                    s.queue.push_back(track);
                    ctx.request_repaint();
                }

                Action::QueueRemove(idx) => {
                    let mut s = state.write().unwrap();
                    if idx < s.queue.len() {
                        s.queue.remove(idx);
                    }
                    ctx.request_repaint();
                }

                Action::QueueClear => {
                    let mut s = state.write().unwrap();
                    s.queue.clear();
                    ctx.request_repaint();
                }

                Action::NextTrack => {
                    Self::advance_track(&mut player, &state, &ctx, false).await;
                }

                Action::PrevTrack => {
                    let progress = player.progress_secs();
                    if progress > 3.0 {
                        // Restart track
                        player.seek(0.0);
                        let mut s = state.write().unwrap();
                        s.progress_secs = 0.0;
                    } else {
                        // Play previous from history
                        let prev_track = {
                            let mut s = state.write().unwrap();
                            s.queue_history.pop()
                        };

                        if let Some(prev) = prev_track {
                            if let Some(curr) = { state.read().unwrap().current_track.clone() } {
                                state.write().unwrap().queue.push_front(curr);
                            }
                            Self::handle_play_meta(&mut player, &state, &ctx, prev).await;
                        } else {
                            player.seek(0.0);
                        }
                    }
                    ctx.request_repaint();
                }

                Action::ToggleShuffle => {
                    let mut s = state.write().unwrap();
                    s.shuffle = !s.shuffle;
                    Self::save_settings(&s);
                    ctx.request_repaint();
                }

                Action::CycleRepeat => {
                    let mut s = state.write().unwrap();
                    s.repeat_mode = s.repeat_mode.cycle();
                    Self::save_settings(&s);
                    ctx.request_repaint();
                }

                Action::SetTheme(theme) => {
                    let mut s = state.write().unwrap();
                    s.theme = theme;
                    Self::save_settings(&s);
                    ctx.request_repaint();
                }

                Action::Search(query) => {
                    {
                        let mut s = state.write().unwrap();
                        s.is_searching = true;
                        s.search_results.clear();
                    }
                    ctx.request_repaint();

                    match api_client.search(&query).await {
                        Ok(results) => {
                            let mut s = state.write().unwrap();
                            s.search_results = results;
                            s.is_searching = false;
                        }
                        Err(e) => {
                            error!("Search failed: {e}");
                            let mut s = state.write().unwrap();
                            s.is_searching = false;
                        }
                    }
                    ctx.request_repaint();
                }

                Action::LoadLibrary => {
                    {
                        let mut s = state.write().unwrap();
                        s.is_loading_library = true;
                    }
                    ctx.request_repaint();

                    match api_client.get_library_songs().await {
                        Ok(songs) => {
                            let mut s = state.write().unwrap();
                            s.library_songs = songs;
                            s.is_loading_library = false;
                        }
                        Err(e) => {
                            warn!("Library load failed: {e}");
                            let mut s = state.write().unwrap();
                            s.is_loading_library = false;
                        }
                    }
                    ctx.request_repaint();
                }
            }

            // Check if track finished automatically
            if player.is_finished() {
                let status = { state.read().unwrap().status.clone() };
                if status == PlayerStatus::Playing {
                    info!("Track finished, advancing...");
                    Self::advance_track(&mut player, &state, &ctx, true).await;
                }
            }

            // Sync progress and media controls
            {
                let s = state.read().unwrap();
                let progress = player.progress_secs();
                media_controls.update(
                    &s.status,
                    s.current_track.as_ref(),
                    progress,
                    s.duration_secs,
                );
            }
        }
    }

    /// Advance to the next track according to repeat and shuffle rules.
    async fn advance_track(
        player: &mut Player,
        state: &Arc<RwLock<AppState>>,
        ctx: &egui::Context,
        natural_end: bool,
    ) {
        let (repeat_mode, shuffle, current_track, queue_len) = {
            let s = state.read().unwrap();
            (s.repeat_mode, s.shuffle, s.current_track.clone(), s.queue.len())
        };

        // If repeat one and natural end, replay
        if repeat_mode == RepeatMode::One && natural_end {
            if let Some(t) = current_track {
                Self::handle_play_meta(player, state, ctx, t).await;
                return;
            }
        }

        // Add finished track to history
        if let Some(curr) = current_track {
            let mut s = state.write().unwrap();
            s.queue_history.push(curr);
        }

        if queue_len > 0 {
            let next_track = {
                let mut s = state.write().unwrap();
                if shuffle && s.queue.len() > 1 {
                    use std::time::SystemTime;
                    let rand_idx = (SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as usize)
                        % s.queue.len();
                    s.queue.remove(rand_idx)
                } else {
                    s.queue.pop_front()
                }
            };

            if let Some(track) = next_track {
                Self::handle_play_meta(player, state, ctx, track).await;
                return;
            }
        }

        // Repeat all: re-queue history
        if repeat_mode == RepeatMode::All {
            let mut history = {
                let mut s = state.write().unwrap();
                std::mem::take(&mut s.queue_history)
            };
            if !history.is_empty() {
                let first = history.remove(0);
                {
                    let mut s = state.write().unwrap();
                    for t in history {
                        s.queue.push_back(t);
                    }
                }
                Self::handle_play_meta(player, state, ctx, first).await;
                return;
            }
        }

        // Nothing left to play
        player.stop();
        {
            let mut s = state.write().unwrap();
            s.status = PlayerStatus::Idle;
            s.progress_secs = 0.0;
        }
        ctx.request_repaint();
    }

    async fn handle_play_input(
        player: &mut Player,
        state: &Arc<RwLock<AppState>>,
        ctx: &egui::Context,
        input: &str,
    ) {
        {
            let mut s = state.write().unwrap();
            s.status = PlayerStatus::Loading;
            s.progress_secs = 0.0;
        }
        ctx.request_repaint();

        let (meta_result, url_result) = tokio::join!(
            Resolver::resolve_metadata(input),
            Resolver::resolve_stream_url(input),
        );

        let meta = match meta_result {
            Ok(m) => Some(m),
            Err(e) => {
                warn!("Could not resolve metadata: {e}");
                None
            }
        };

        let stream_url = match url_result {
            Ok(url) => url,
            Err(e) => {
                let mut s = state.write().unwrap();
                s.status = PlayerStatus::Error(e);
                ctx.request_repaint();
                return;
            }
        };

        let track = meta.unwrap_or_else(|| TrackMeta {
            title: input.to_string(),
            artist: "YouTube".to_string(),
            thumbnail_url: String::new(),
            duration_secs: 0.0,
            video_id: input.to_string(),
        });

        Self::download_and_play(player, state, ctx, stream_url, track).await;
    }

    async fn handle_play_meta(
        player: &mut Player,
        state: &Arc<RwLock<AppState>>,
        ctx: &egui::Context,
        mut track: TrackMeta,
    ) {
        {
            let mut s = state.write().unwrap();
            s.status = PlayerStatus::Loading;
            s.current_track = Some(track.clone());
            s.progress_secs = 0.0;
        }
        ctx.request_repaint();

        // Resolve stream URL and update metadata if duration missing
        let (url_result, meta_result) = tokio::join!(
            Resolver::resolve_stream_url(&track.video_id),
            Resolver::resolve_metadata(&track.video_id)
        );

        let stream_url = match url_result {
            Ok(url) => url,
            Err(e) => {
                let mut s = state.write().unwrap();
                s.status = PlayerStatus::Error(e);
                ctx.request_repaint();
                return;
            }
        };

        if let Ok(m) = meta_result {
            if track.duration_secs == 0.0 {
                track.duration_secs = m.duration_secs;
            }
            if track.thumbnail_url.is_empty() {
                track.thumbnail_url = m.thumbnail_url;
            }
        }

        Self::download_and_play(player, state, ctx, stream_url, track).await;
    }

    async fn download_and_play(
        player: &mut Player,
        state: &Arc<RwLock<AppState>>,
        ctx: &egui::Context,
        stream_url: String,
        track: TrackMeta,
    ) {
        info!("Downloading audio stream for '{}'...", track.title);
        let download_result = reqwest::get(&stream_url).await;
        let audio_data = match download_result {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    info!("Downloaded {} bytes", bytes.len());
                    bytes.to_vec()
                }
                Err(e) => {
                    let mut s = state.write().unwrap();
                    s.status = PlayerStatus::Error(format!("Download error: {e}"));
                    ctx.request_repaint();
                    return;
                }
            },
            Err(e) => {
                let mut s = state.write().unwrap();
                s.status = PlayerStatus::Error(format!("HTTP error: {e}"));
                ctx.request_repaint();
                return;
            }
        };

        let duration = track.duration_secs;

        match player.play_bytes(audio_data, duration) {
            Ok(()) => {
                let mut s = state.write().unwrap();
                s.status = PlayerStatus::Playing;
                s.current_track = Some(track.clone());
                s.duration_secs = duration;
                s.progress_secs = 0.0;
                Self::save_settings(&s);
            }
            Err(e) => {
                let mut s = state.write().unwrap();
                s.status = PlayerStatus::Error(e);
            }
        }
        ctx.request_repaint();
    }

    fn save_settings(state: &AppState) {
        let theme_str = match state.theme {
            AppTheme::Light => "light",
            AppTheme::Amoled => "amoled",
            AppTheme::Dark => "dark",
        };
        let repeat_str = match state.repeat_mode {
            RepeatMode::All => "all",
            RepeatMode::One => "one",
            RepeatMode::Off => "off",
        };
        let settings = Settings {
            volume: state.volume,
            repeat_mode: repeat_str.to_string(),
            shuffle: state.shuffle,
            theme: theme_str.to_string(),
            last_track_id: state.current_track.as_ref().map(|t| t.video_id.clone()),
        };
        settings.save();
    }
}
