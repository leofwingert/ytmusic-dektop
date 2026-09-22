use crate::action::Action;
use crate::api::models::SearchResult;
use crate::backend::Backend;
use crate::state::{ActiveView, AppState, AppTheme, PlayerStatus, RepeatMode, TrackMeta};
use crate::thumbnail::ThumbnailManager;
use crate::tray::TrayManager;
use egui::{
    Align, CentralPanel, Color32, CornerRadius, FontId, Layout, RichText, ScrollArea, Vec2,
};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// Main application struct — holds shared state, background workers, and UI cache.
pub struct App {
    state: Arc<RwLock<AppState>>,
    action_tx: mpsc::UnboundedSender<Action>,
    url_input: String,
    search_input: String,
    active_view: ActiveView,
    thumbnails: ThumbnailManager,
    tray: TrayManager,
    last_theme: Option<AppTheme>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(RwLock::new(AppState::default()));
        let action_tx = Backend::spawn(Arc::clone(&state), cc.egui_ctx.clone());
        let thumbnails = ThumbnailManager::new();
        let tray = TrayManager::new();

        Self {
            state,
            action_tx,
            url_input: String::new(),
            search_input: String::new(),
            active_view: ActiveView::Home,
            thumbnails,
            tray,
            last_theme: None,
        }
    }

    fn apply_theme(ctx: &egui::Context, theme: AppTheme) {
        let mut style = (*ctx.style()).clone();

        match theme {
            AppTheme::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.override_text_color = Some(Color32::from_rgb(230, 230, 230));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(18, 18, 18);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 38, 38);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 55);
                visuals.widgets.active.bg_fill = Color32::from_rgb(255, 0, 0);
                visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
                visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
                visuals.widgets.active.corner_radius = CornerRadius::same(8);
                visuals.panel_fill = Color32::from_rgb(18, 18, 18);
                style.visuals = visuals;
            }
            AppTheme::Amoled => {
                let mut visuals = egui::Visuals::dark();
                visuals.override_text_color = Some(Color32::from_rgb(240, 240, 240));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0, 0, 0);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(20, 20, 20);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(35, 35, 35);
                visuals.widgets.active.bg_fill = Color32::from_rgb(255, 0, 0);
                visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
                visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
                visuals.widgets.active.corner_radius = CornerRadius::same(8);
                visuals.panel_fill = Color32::from_rgb(0, 0, 0);
                style.visuals = visuals;
            }
            AppTheme::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.override_text_color = Some(Color32::from_rgb(25, 25, 25));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(245, 245, 247);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(230, 230, 235);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(215, 215, 220);
                visuals.widgets.active.bg_fill = Color32::from_rgb(255, 0, 0);
                visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
                visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
                visuals.widgets.active.corner_radius = CornerRadius::same(8);
                visuals.panel_fill = Color32::from_rgb(245, 245, 247);
                style.visuals = visuals;
            }
        }

        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        ctx.set_style(style);
    }

    fn send(&self, action: Action) {
        let _ = self.action_tx.send(action);
    }

    fn theme_colors(&self, theme: AppTheme) -> (Color32, Color32, Color32) {
        match theme {
            AppTheme::Dark => (
                Color32::from_rgb(14, 14, 14), // Sidebar
                Color32::from_rgb(18, 18, 18), // Central
                Color32::from_rgb(26, 26, 26), // Player bar
            ),
            AppTheme::Amoled => (
                Color32::from_rgb(4, 4, 4),
                Color32::from_rgb(0, 0, 0),
                Color32::from_rgb(10, 10, 10),
            ),
            AppTheme::Light => (
                Color32::from_rgb(238, 238, 242),
                Color32::from_rgb(250, 250, 252),
                Color32::from_rgb(240, 240, 244),
            ),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll system tray events
        self.tray.poll_events(&self.action_tx);

        let state = self.state.read().unwrap().clone_snapshot();

        // Apply theme changes dynamically
        if self.last_theme != Some(state.theme) {
            Self::apply_theme(ctx, state.theme);
            self.last_theme = Some(state.theme);
        }

        let (sidebar_bg, central_bg, player_bar_bg) = self.theme_colors(state.theme);

        // Sidebar Panel
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(210.0)
            .frame(
                egui::Frame::new()
                    .fill(sidebar_bg)
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| {
                self.render_sidebar(ui, &state);
            });

        // Bottom Player Bar
        egui::TopBottomPanel::bottom("player_bar")
            .min_height(84.0)
            .frame(
                egui::Frame::new()
                    .fill(player_bar_bg)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                self.render_player_bar(ui, ctx, &state);
            });

        // Central Content Area
        CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(central_bg)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| match self.active_view {
                ActiveView::Home => self.render_home(ui, ctx, &state),
                ActiveView::Search => self.render_search(ui, ctx, &state),
                ActiveView::Library => self.render_library(ui, ctx, &state),
                ActiveView::Queue => self.render_queue(ui, ctx, &state),
                ActiveView::Settings => self.render_settings(ui, &state),
            });

        if state.status == PlayerStatus::Playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

/// State snapshot for safe rendering without holding RwLock.
pub struct StateSnapshot {
    pub status: PlayerStatus,
    pub current_track: Option<TrackMeta>,
    pub progress_secs: f32,
    pub duration_secs: f32,
    pub volume: f32,
    pub queue: VecDeque<TrackMeta>,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,
    pub theme: AppTheme,
    pub search_results: Vec<SearchResult>,
    pub is_searching: bool,
    pub library_songs: Vec<crate::api::models::LibrarySong>,
    pub is_loading_library: bool,
    pub is_authenticated: bool,
    pub auth_error: Option<String>,
}

impl AppState {
    pub fn clone_snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            status: self.status.clone(),
            current_track: self.current_track.clone(),
            progress_secs: self.progress_secs,
            duration_secs: self.duration_secs,
            volume: self.volume,
            queue: self.queue.clone(),
            repeat_mode: self.repeat_mode,
            shuffle: self.shuffle,
            theme: self.theme,
            search_results: self.search_results.clone(),
            is_searching: self.is_searching,
            library_songs: self.library_songs.clone(),
            is_loading_library: self.is_loading_library,
            is_authenticated: self.is_authenticated,
            auth_error: self.auth_error.clone(),
        }
    }
}

impl App {
    // ── Sidebar ───────────────────────────────────────────────────────────

    fn render_sidebar(&mut self, ui: &mut egui::Ui, state: &StateSnapshot) {
        ui.add_space(8.0);

        // Logo
        ui.horizontal(|ui| {
            ui.label(RichText::new("▶").font(FontId::proportional(26.0)).color(Color32::from_rgb(255, 0, 0)));
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("YouTube Music")
                        .font(FontId::proportional(16.0))
                        .strong(),
                );
                ui.label(
                    RichText::new("Desktop Client")
                        .font(FontId::proportional(11.0))
                        .color(Color32::from_rgb(140, 140, 140)),
                );
            });
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Navigation links
        let nav_items = [
            ("🏠", "Home", ActiveView::Home, 0),
            ("🔍", "Search", ActiveView::Search, 0),
            ("📚", "Library", ActiveView::Library, 0),
            ("📋", "Queue", ActiveView::Queue, state.queue.len()),
            ("⚙", "Settings", ActiveView::Settings, 0),
        ];

        for (icon, label, view, badge) in nav_items {
            let is_active = self.active_view == view;
            let text_color = if is_active {
                Color32::WHITE
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            let bg_color = if is_active {
                Color32::from_rgb(255, 0, 0)
            } else {
                Color32::TRANSPARENT
            };

            let title = if badge > 0 {
                format!("{icon}  {label} ({badge})")
            } else {
                format!("{icon}  {label}")
            };

            let btn = egui::Button::new(
                RichText::new(title)
                    .font(FontId::proportional(14.5))
                    .color(text_color),
            )
            .fill(bg_color)
            .corner_radius(CornerRadius::same(8))
            .min_size(Vec2::new(ui.available_width(), 38.0));

            if ui.add(btn).clicked() {
                self.active_view = view;
            }
            ui.add_space(2.0);
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // Auth status
        if state.is_authenticated {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(Color32::from_rgb(0, 200, 83)));
                ui.label(
                    RichText::new("Authenticated")
                        .font(FontId::proportional(12.0))
                        .color(Color32::from_rgb(0, 200, 83)),
                );
            });
        } else {
            ui.horizontal(|ui| {
                ui.label(RichText::new("○").color(Color32::from_rgb(180, 180, 180)));
                ui.label(
                    RichText::new("Anonymous Mode")
                        .font(FontId::proportional(12.0))
                        .color(Color32::from_rgb(180, 180, 180)),
                );
            });
            if let Some(err) = &state.auth_error {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(err)
                        .font(FontId::proportional(10.5))
                        .color(Color32::from_rgb(255, 170, 0)),
                );
            }
        }
    }

    // ── Home View ─────────────────────────────────────────────────────────

    fn render_home(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, state: &StateSnapshot) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);

                ui.label(
                    RichText::new("YouTube Music Desktop")
                        .font(FontId::proportional(30.0))
                        .color(Color32::from_rgb(255, 0, 0))
                        .strong(),
                );

                ui.add_space(6.0);
                ui.label(
                    RichText::new("Native, lightweight Rust music player")
                        .font(FontId::proportional(15.0))
                        .color(Color32::from_rgb(160, 160, 160)),
                );

                ui.add_space(30.0);

                // Quick Play Input Card
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    let input = egui::TextEdit::singleline(&mut self.url_input)
                        .hint_text("Paste YouTube video URL or ID (e.g. dQw4w9WgXcQ)...")
                        .desired_width(450.0)
                        .font(FontId::proportional(15.0))
                        .margin(Vec2::new(14.0, 10.0));
                    let response = ui.add(input);

                    let play_btn = ui.add_sized(
                        Vec2::new(90.0, 40.0),
                        egui::Button::new(
                            RichText::new("▶  Play")
                                .font(FontId::proportional(15.0))
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(255, 0, 0))
                        .corner_radius(CornerRadius::same(8)),
                    );

                    if (play_btn.clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                        && !self.url_input.trim().is_empty()
                    {
                        self.send(Action::Play(self.url_input.trim().to_string()));
                    }
                });

                ui.add_space(30.0);

                // Player status banner
                match &state.status {
                    PlayerStatus::Idle => {
                        ui.label(
                            RichText::new("Ready. Play a track above or search songs.")
                                .font(FontId::proportional(14.0))
                                .color(Color32::from_rgb(130, 130, 130)),
                        );
                    }
                    PlayerStatus::Loading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new("Loading and streaming track...")
                                    .font(FontId::proportional(14.0))
                                    .color(Color32::from_rgb(255, 200, 0)),
                            );
                        });
                    }
                    PlayerStatus::Error(msg) => {
                        ui.label(
                            RichText::new(format!("⚠ {msg}"))
                                .font(FontId::proportional(14.0))
                                .color(Color32::from_rgb(255, 80, 80)),
                        );
                    }
                    _ => {}
                }

                // Now Playing Hero Card if track active
                if let Some(track) = &state.current_track {
                    ui.add_space(20.0);
                    ui.group(|ui| {
                        ui.set_min_width(500.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;

                            // Thumbnail
                            let thumb_tex = self.thumbnails.get_or_load(ctx, &track.thumbnail_url);
                            if let Some(tex) = thumb_tex {
                                ui.image((tex.id(), Vec2::new(80.0, 80.0)));
                            } else {
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(80.0, 80.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(8),
                                    Color32::from_rgb(45, 45, 45),
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "🎵",
                                    FontId::proportional(28.0),
                                    Color32::from_rgb(180, 180, 180),
                                );
                            }

                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&track.title)
                                        .font(FontId::proportional(18.0))
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(&track.artist)
                                        .font(FontId::proportional(14.0))
                                        .color(Color32::from_rgb(180, 180, 180)),
                                );
                                if track.duration_secs > 0.0 {
                                    ui.label(
                                        RichText::new(format!(
                                            "Duration: {}",
                                            format_time(track.duration_secs)
                                        ))
                                        .font(FontId::proportional(12.0))
                                        .color(Color32::from_rgb(140, 140, 140)),
                                    );
                                }
                            });
                        });
                    });
                }
            });
        });
    }

    // ── Search View ───────────────────────────────────────────────────────

    fn render_search(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, state: &StateSnapshot) {
        ui.add_space(4.0);

        ui.label(
            RichText::new("Search Music")
                .font(FontId::proportional(24.0))
                .strong(),
        );

        ui.add_space(14.0);

        // Search Bar
        ui.horizontal(|ui| {
            let input = egui::TextEdit::singleline(&mut self.search_input)
                .hint_text("Search songs, artists, albums...")
                .desired_width(ui.available_width() - 110.0)
                .font(FontId::proportional(15.0))
                .margin(Vec2::new(12.0, 9.0));
            let response = ui.add(input);

            let search_btn = ui.add_sized(
                Vec2::new(90.0, 36.0),
                egui::Button::new(
                    RichText::new("🔍 Search")
                        .font(FontId::proportional(14.0))
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(255, 0, 0))
                .corner_radius(CornerRadius::same(8)),
            );

            if (search_btn.clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                && !self.search_input.trim().is_empty()
            {
                self.send(Action::Search(self.search_input.trim().to_string()));
            }
        });

        ui.add_space(16.0);

        if state.is_searching {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new("Searching YouTube Music...")
                        .color(Color32::from_rgb(180, 180, 180)),
                );
            });
            return;
        }

        if state.search_results.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    RichText::new("Type an artist, song title, or album to search")
                        .font(FontId::proportional(14.0))
                        .color(Color32::from_rgb(120, 120, 120)),
                );
            });
            return;
        }

        ui.label(
            RichText::new(format!("{} songs found", state.search_results.len()))
                .font(FontId::proportional(12.0))
                .color(Color32::from_rgb(140, 140, 140)),
        );
        ui.add_space(8.0);

        let results = state.search_results.clone();

        ScrollArea::vertical().show(ui, |ui| {
            for result in &results {
                self.render_search_item(ui, ctx, result);
            }
        });
    }

    fn render_search_item(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        result: &SearchResult,
    ) {
        let response = ui.horizontal(|ui| {
            ui.set_min_height(56.0);

            // Thumbnail
            let thumb = self.thumbnails.get_or_load(ctx, &result.thumbnail_url);
            if let Some(tex) = thumb {
                ui.image((tex.id(), Vec2::new(48.0, 48.0)));
            } else {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(4),
                    Color32::from_rgb(50, 50, 50),
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "♪",
                    FontId::proportional(20.0),
                    Color32::from_rgb(180, 180, 180),
                );
            }

            ui.add_space(10.0);

            // Title & Artist
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(&result.title)
                        .font(FontId::proportional(14.5))
                        .strong(),
                );
                let meta_str = if !result.album.is_empty() {
                    format!("{} • {} • {}", result.artist, result.album, result.duration)
                } else {
                    format!("{} • {}", result.artist, result.duration)
                };
                ui.label(
                    RichText::new(meta_str)
                        .font(FontId::proportional(12.0))
                        .color(Color32::from_rgb(160, 160, 160)),
                );
            });

            // Action buttons (Play & Queue)
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // Add to Queue button
                let track_meta = TrackMeta {
                    title: result.title.clone(),
                    artist: result.artist.clone(),
                    thumbnail_url: result.thumbnail_url.clone(),
                    duration_secs: crate::state::parse_time_str(&result.duration),
                    video_id: result.video_id.clone(),
                };

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("+ Queue")
                                .font(FontId::proportional(12.0))
                                .color(Color32::from_rgb(220, 220, 220)),
                        )
                        .fill(Color32::from_rgb(45, 45, 45))
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    self.send(Action::QueueAdd(track_meta));
                }

                // Play Now button
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("▶ Play")
                                .font(FontId::proportional(12.5))
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(255, 0, 0))
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    self.send(Action::PlaySearchResult(result.clone()));
                }
            });
        });

        if response.response.hovered() {
            ui.painter().rect_filled(
                response.response.rect,
                CornerRadius::same(4),
                Color32::from_rgba_premultiplied(255, 255, 255, 8),
            );
        }

        ui.separator();
    }

    // ── Library View ──────────────────────────────────────────────────────

    fn render_library(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, state: &StateSnapshot) {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Your Library")
                    .font(FontId::proportional(24.0))
                    .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("↻ Refresh")
                                .font(FontId::proportional(13.0))
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(55, 55, 55))
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    self.send(Action::LoadLibrary);
                }
            });
        });

        ui.add_space(16.0);

        if !state.is_authenticated {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    RichText::new("🔒 Authentication Required")
                        .font(FontId::proportional(18.0))
                        .color(Color32::from_rgb(255, 190, 0)),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new(
                        "To browse your saved library, export your YouTube cookies and save them to:",
                    )
                    .font(FontId::proportional(13.5))
                    .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.add_space(6.0);
                let path = crate::api::auth::Auth::cookie_file_path();
                ui.label(
                    RichText::new(path.display().to_string())
                        .font(FontId::monospace(13.0))
                        .color(Color32::from_rgb(100, 200, 255)),
                );
            });
            return;
        }

        if state.is_loading_library {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new("Loading songs from your library...")
                        .color(Color32::from_rgb(180, 180, 180)),
                );
            });
            return;
        }

        if state.library_songs.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    RichText::new("No songs loaded. Click '↻ Refresh' to load your saved songs.")
                        .font(FontId::proportional(14.0))
                        .color(Color32::from_rgb(140, 140, 140)),
                );
            });
            return;
        }

        ui.label(
            RichText::new(format!("{} library songs", state.library_songs.len()))
                .font(FontId::proportional(12.0))
                .color(Color32::from_rgb(140, 140, 140)),
        );
        ui.add_space(8.0);

        let songs = state.library_songs.clone();

        ScrollArea::vertical().show(ui, |ui| {
            for song in &songs {
                let response = ui.horizontal(|ui| {
                    ui.set_min_height(54.0);

                    // Thumbnail
                    let thumb = self.thumbnails.get_or_load(ctx, &song.thumbnail_url);
                    if let Some(tex) = thumb {
                        ui.image((tex.id(), Vec2::new(44.0, 44.0)));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(44.0, 44.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(4),
                            Color32::from_rgb(45, 45, 45),
                        );
                    }

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&song.title)
                                .font(FontId::proportional(14.0))
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("{} • {}", song.artist, song.duration))
                                .font(FontId::proportional(12.0))
                                .color(Color32::from_rgb(160, 160, 160)),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let track_meta = TrackMeta {
                            title: song.title.clone(),
                            artist: song.artist.clone(),
                            thumbnail_url: song.thumbnail_url.clone(),
                            duration_secs: crate::state::parse_time_str(&song.duration),
                            video_id: song.video_id.clone(),
                        };

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("+ Queue")
                                        .font(FontId::proportional(12.0))
                                        .color(Color32::from_rgb(220, 220, 220)),
                                )
                                .fill(Color32::from_rgb(45, 45, 45))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            self.send(Action::QueueAdd(track_meta.clone()));
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("▶ Play")
                                        .font(FontId::proportional(12.0))
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(255, 0, 0))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            self.send(Action::PlayTrack(track_meta));
                        }
                    });
                });

                if response.response.hovered() {
                    ui.painter().rect_filled(
                        response.response.rect,
                        CornerRadius::same(4),
                        Color32::from_rgba_premultiplied(255, 255, 255, 8),
                    );
                }

                ui.separator();
            }
        });
    }

    // ── Queue View ────────────────────────────────────────────────────────

    fn render_queue(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, state: &StateSnapshot) {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Playback Queue")
                    .font(FontId::proportional(24.0))
                    .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !state.queue.is_empty(),
                        egui::Button::new(
                            RichText::new("Clear Queue")
                                .font(FontId::proportional(12.5))
                                .color(Color32::from_rgb(255, 100, 100)),
                        )
                        .fill(Color32::from_rgb(45, 25, 25))
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    self.send(Action::QueueClear);
                }
            });
        });

        ui.add_space(16.0);

        // Now Playing Section
        ui.label(
            RichText::new("NOW PLAYING")
                .font(FontId::proportional(11.0))
                .color(Color32::from_rgb(255, 0, 0))
                .strong(),
        );
        ui.add_space(4.0);

        if let Some(track) = &state.current_track {
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let thumb = self.thumbnails.get_or_load(ctx, &track.thumbnail_url);
                    if let Some(tex) = thumb {
                        ui.image((tex.id(), Vec2::new(44.0, 44.0)));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(50, 50, 50));
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&track.title).font(FontId::proportional(14.5)).strong());
                        ui.label(RichText::new(&track.artist).font(FontId::proportional(12.0)).color(Color32::from_rgb(160, 160, 160)));
                    });
                });
            });
        } else {
            ui.label(RichText::new("Nothing playing right now").color(Color32::from_rgb(140, 140, 140)));
        }

        ui.add_space(20.0);

        // Next Up Section
        ui.label(
            RichText::new(format!("NEXT UP ({} TRACKS)", state.queue.len()))
                .font(FontId::proportional(11.0))
                .color(Color32::from_rgb(160, 160, 160))
                .strong(),
        );
        ui.add_space(6.0);

        if state.queue.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(
                    RichText::new("Queue is empty. Search songs and click '+ Queue' to queue up music.")
                        .font(FontId::proportional(13.5))
                        .color(Color32::from_rgb(130, 130, 130)),
                );
            });
            return;
        }

        let queue = state.queue.clone();

        ScrollArea::vertical().show(ui, |ui| {
            for (idx, track) in queue.iter().enumerate() {
                let response = ui.horizontal(|ui| {
                    ui.set_min_height(50.0);

                    ui.label(
                        RichText::new(format!("{}.", idx + 1))
                            .font(FontId::monospace(12.0))
                            .color(Color32::from_rgb(130, 130, 130)),
                    );
                    ui.add_space(6.0);

                    let thumb = self.thumbnails.get_or_load(ctx, &track.thumbnail_url);
                    if let Some(tex) = thumb {
                        ui.image((tex.id(), Vec2::new(40.0, 40.0)));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(40.0, 40.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(45, 45, 45));
                    }

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new(&track.title).font(FontId::proportional(14.0)).strong());
                        ui.label(RichText::new(&track.artist).font(FontId::proportional(12.0)).color(Color32::from_rgb(160, 160, 160)));
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("✕")
                                        .font(FontId::proportional(12.0))
                                        .color(Color32::from_rgb(180, 180, 180)),
                                )
                                .fill(Color32::TRANSPARENT)
                                .corner_radius(CornerRadius::same(4)),
                            )
                            .clicked()
                        {
                            self.send(Action::QueueRemove(idx));
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("▶ Play")
                                        .font(FontId::proportional(11.5))
                                        .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(255, 0, 0))
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            self.send(Action::PlayTrack(track.clone()));
                            self.send(Action::QueueRemove(idx));
                        }
                    });
                });

                if response.response.hovered() {
                    ui.painter().rect_filled(
                        response.response.rect,
                        CornerRadius::same(4),
                        Color32::from_rgba_premultiplied(255, 255, 255, 8),
                    );
                }

                ui.separator();
            }
        });
    }

    // ── Settings View ─────────────────────────────────────────────────────

    fn render_settings(&mut self, ui: &mut egui::Ui, state: &StateSnapshot) {
        ui.add_space(4.0);

        ui.label(
            RichText::new("Settings & Preferences")
                .font(FontId::proportional(24.0))
                .strong(),
        );

        ui.add_space(20.0);

        // Theme Selector
        ui.label(RichText::new("Appearance & Theme").font(FontId::proportional(16.0)).strong());
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let themes = [
                ("Dark (Default)", AppTheme::Dark),
                ("AMOLED Pure Black", AppTheme::Amoled),
                ("Light Theme", AppTheme::Light),
            ];

            for (name, theme) in themes {
                let is_selected = state.theme == theme;
                let bg = if is_selected {
                    Color32::from_rgb(255, 0, 0)
                } else {
                    Color32::from_rgb(45, 45, 45)
                };

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(name)
                                .font(FontId::proportional(13.5))
                                .color(Color32::WHITE),
                        )
                        .fill(bg)
                        .corner_radius(CornerRadius::same(8)),
                    )
                    .clicked()
                {
                    self.send(Action::SetTheme(theme));
                }
            }
        });

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(16.0);

        // Authentication & Cookies
        ui.label(RichText::new("Authentication & Account").font(FontId::proportional(16.0)).strong());
        ui.add_space(8.0);

        let cookie_path = crate::api::auth::Auth::cookie_file_path();
        ui.label(
            RichText::new(format!("Cookie file location: {}", cookie_path.display()))
                .font(FontId::monospace(12.5))
                .color(Color32::from_rgb(120, 180, 255)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Export your cookies using a browser extension (like 'Get cookies.txt LOCALLY')\n\
                 and save the JSON or Netscape format file to the path above.",
            )
            .font(FontId::proportional(13.0))
            .color(Color32::from_rgb(160, 160, 160)),
        );

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(16.0);

        // Subsystems status
        ui.label(RichText::new("System Integrations").font(FontId::proportional(16.0)).strong());
        ui.add_space(8.0);
        ui.label("• Audio Engine: rodio 0.20 (symphonia decoders)");
        ui.label("• Media Keys / MPRIS: souvlaki 0.8 (active)");
        ui.label("• Stream Resolver: yt-dlp native subprocess");
        ui.label("• Cache: ~/.cache/ytmusic-rs/thumbnails");
        ui.label("• Config: ~/.config/ytmusic-rs/settings.json");
    }

    // ── Player Bar ────────────────────────────────────────────────────────

    fn render_player_bar(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        state: &StateSnapshot,
    ) {
        ui.horizontal(|ui| {
            // ── Left: Track info & Thumbnail ──
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.set_width(260.0);

                if let Some(track) = &state.current_track {
                    // Small thumbnail in player bar
                    let thumb = self.thumbnails.get_or_load(ctx, &track.thumbnail_url);
                    if let Some(tex) = thumb {
                        ui.image((tex.id(), Vec2::new(48.0, 48.0)));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(48.0, 48.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(6),
                            Color32::from_rgb(45, 45, 45),
                        );
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "♪",
                            FontId::proportional(18.0),
                            Color32::from_rgb(180, 180, 180),
                        );
                    }

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&track.title)
                                .font(FontId::proportional(13.5))
                                .strong(),
                        );
                        ui.label(
                            RichText::new(&track.artist)
                                .font(FontId::proportional(11.5))
                                .color(Color32::from_rgb(160, 160, 160)),
                        );
                    });
                } else {
                    ui.label(
                        RichText::new("No track selected")
                            .font(FontId::proportional(13.0))
                            .color(Color32::from_rgb(130, 130, 130)),
                    );
                }
            });

            // ── Center: Transport Controls & Progress ──
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // Shuffle Button
                let shuffle_color = if state.shuffle {
                    Color32::from_rgb(255, 0, 0)
                } else {
                    Color32::from_rgb(140, 140, 140)
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("🔀").font(FontId::proportional(16.0)).color(shuffle_color),
                        )
                        .min_size(Vec2::new(32.0, 32.0))
                        .corner_radius(CornerRadius::same(16)),
                    )
                    .clicked()
                {
                    self.send(Action::ToggleShuffle);
                }

                // Previous Button
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("⏮").font(FontId::proportional(16.0)),
                        )
                        .min_size(Vec2::new(34.0, 34.0))
                        .corner_radius(CornerRadius::same(17)),
                    )
                    .clicked()
                {
                    self.send(Action::PrevTrack);
                }

                // Play / Pause toggle
                let can_pause = state.status == PlayerStatus::Playing;
                let can_resume = state.status == PlayerStatus::Paused;

                if can_pause {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("⏸")
                                    .font(FontId::proportional(20.0))
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(42.0, 42.0))
                            .fill(Color32::from_rgb(255, 0, 0))
                            .corner_radius(CornerRadius::same(21)),
                        )
                        .clicked()
                    {
                        self.send(Action::Pause);
                    }
                } else if can_resume {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("▶")
                                    .font(FontId::proportional(20.0))
                                    .color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(42.0, 42.0))
                            .fill(Color32::from_rgb(255, 0, 0))
                            .corner_radius(CornerRadius::same(21)),
                        )
                        .clicked()
                    {
                        self.send(Action::Resume);
                    }
                } else {
                    let has_queue = !state.queue.is_empty();
                    if ui
                        .add_enabled(
                            has_queue,
                            egui::Button::new(
                                RichText::new("▶").font(FontId::proportional(20.0)),
                            )
                            .min_size(Vec2::new(42.0, 42.0))
                            .corner_radius(CornerRadius::same(21)),
                        )
                        .clicked()
                    {
                        self.send(Action::TogglePlayPause);
                    }
                }

                // Next Button
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("⏭").font(FontId::proportional(16.0)),
                        )
                        .min_size(Vec2::new(34.0, 34.0))
                        .corner_radius(CornerRadius::same(17)),
                    )
                    .clicked()
                {
                    self.send(Action::NextTrack);
                }

                // Repeat Mode Button
                let (repeat_icon, repeat_color) = match state.repeat_mode {
                    RepeatMode::Off => ("🔁", Color32::from_rgb(140, 140, 140)),
                    RepeatMode::All => ("🔁", Color32::from_rgb(255, 0, 0)),
                    RepeatMode::One => ("🔂", Color32::from_rgb(255, 0, 0)),
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(repeat_icon)
                                .font(FontId::proportional(16.0))
                                .color(repeat_color),
                        )
                        .min_size(Vec2::new(32.0, 32.0))
                        .corner_radius(CornerRadius::same(16)),
                    )
                    .clicked()
                {
                    self.send(Action::CycleRepeat);
                }

                ui.add_space(8.0);

                // Progress Time (current)
                ui.label(
                    RichText::new(format_time(state.progress_secs))
                        .font(FontId::monospace(11.0))
                        .color(Color32::from_rgb(160, 160, 160)),
                );

                // Slider seek bar
                let mut progress = if state.duration_secs > 0.0 {
                    state.progress_secs / state.duration_secs
                } else {
                    0.0
                };

                let avail = (ui.available_width() - 170.0).max(80.0);
                let slider = egui::Slider::new(&mut progress, 0.0..=1.0)
                    .show_value(false)
                    .trailing_fill(true);

                if ui.add_sized(Vec2::new(avail, 18.0), slider).changed() {
                    let seek_pos = progress * state.duration_secs;
                    self.send(Action::Seek(seek_pos));
                }

                // Progress Time (total duration)
                ui.label(
                    RichText::new(format_time(state.duration_secs))
                        .font(FontId::monospace(11.0))
                        .color(Color32::from_rgb(160, 160, 160)),
                );
            });

            // ── Right: Volume Slider ──
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut volume = state.volume;
                let vol_icon = if volume > 0.5 {
                    "🔊"
                } else if volume > 0.0 {
                    "🔉"
                } else {
                    "🔇"
                };

                ui.label(RichText::new(vol_icon).font(FontId::proportional(14.0)));

                let vol_slider = egui::Slider::new(&mut volume, 0.0..=1.0)
                    .show_value(false)
                    .trailing_fill(true);

                if ui.add_sized(Vec2::new(85.0, 16.0), vol_slider).changed() {
                    self.send(Action::SetVolume(volume));
                }
            });
        });
    }
}

fn format_time(secs: f32) -> String {
    let total = secs.max(0.0) as u32;
    let mins = total / 60;
    let secs = total % 60;
    format!("{mins:02}:{secs:02}")
}
