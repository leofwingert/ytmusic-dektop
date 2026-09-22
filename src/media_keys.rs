use crate::action::Action;
use crate::state::{PlayerStatus, TrackMeta};
use log::{info, warn};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

pub struct MediaControlsManager {
    controls: Option<MediaControls>,
}

impl MediaControlsManager {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        #[cfg(target_os = "linux")]
        let hwnd = None;
        #[cfg(target_os = "windows")]
        let hwnd = None;

        let config = PlatformConfig {
            dbus_name: "ytmusic_rs",
            display_name: "YouTube Music Desktop",
            hwnd,
        };

        let mut controls = match MediaControls::new(config) {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("Failed to initialize media controls / MPRIS: {e:?}");
                None
            }
        };

        if let Some(c) = controls.as_mut() {
            let tx = action_tx.clone();
            let attach_res = c.attach(move |event: MediaControlEvent| {
                match event {
                    MediaControlEvent::Play => {
                        let _ = tx.send(Action::Resume);
                    }
                    MediaControlEvent::Pause => {
                        let _ = tx.send(Action::Pause);
                    }
                    MediaControlEvent::Toggle => {
                        let _ = tx.send(Action::TogglePlayPause);
                    }
                    MediaControlEvent::Next => {
                        let _ = tx.send(Action::NextTrack);
                    }
                    MediaControlEvent::Previous => {
                        let _ = tx.send(Action::PrevTrack);
                    }
                    MediaControlEvent::Stop => {
                        let _ = tx.send(Action::Stop);
                    }
                    MediaControlEvent::SetVolume(vol) => {
                        let _ = tx.send(Action::SetVolume(vol as f32));
                    }
                    MediaControlEvent::SetPosition(pos) => {
                        let _ = tx.send(Action::Seek(pos.0.as_secs_f32()));
                    }
                    _ => {}
                }
            });

            if let Err(e) = attach_res {
                warn!("Failed to attach media controls handler: {e:?}");
            } else {
                info!("Global media keys and MPRIS integration initialized");
            }
        }

        Self { controls }
    }

    /// Update media controls metadata and playback status.
    pub fn update(
        &mut self,
        status: &PlayerStatus,
        track: Option<&TrackMeta>,
        progress_secs: f32,
        duration_secs: f32,
    ) {
        let controls = match self.controls.as_mut() {
            Some(c) => c,
            None => return,
        };

        // Update metadata
        if let Some(t) = track {
            let metadata = MediaMetadata {
                title: Some(&t.title),
                album: None,
                artist: Some(&t.artist),
                duration: if duration_secs > 0.0 {
                    Some(Duration::from_secs_f32(duration_secs))
                } else {
                    None
                },
                cover_url: if !t.thumbnail_url.is_empty() {
                    Some(&t.thumbnail_url)
                } else {
                    None
                },
            };
            let _ = controls.set_metadata(metadata);
        }

        // Update playback status
        let progress = Some(MediaPosition(Duration::from_secs_f32(progress_secs)));
        let playback = match status {
            PlayerStatus::Playing => MediaPlayback::Playing { progress },
            PlayerStatus::Paused => MediaPlayback::Paused { progress },
            _ => MediaPlayback::Stopped,
        };
        let _ = controls.set_playback(playback);
    }
}
