use crate::action::Action;
use log::{error, info, warn};
use tokio::sync::mpsc::UnboundedSender;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    _tray: Option<TrayIcon>,
    play_pause_id: String,
    next_id: String,
    prev_id: String,
    quit_id: String,
}

impl TrayManager {
    pub fn new() -> Self {
        let menu = Menu::new();

        let play_pause_item = MenuItem::new("Play / Pause", true, None);
        let next_item = MenuItem::new("Next Track", true, None);
        let prev_item = MenuItem::new("Previous Track", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let play_pause_id = play_pause_item.id().0.clone();
        let next_id = next_item.id().0.clone();
        let prev_id = prev_item.id().0.clone();
        let quit_id = quit_item.id().0.clone();

        let _ = menu.append(&play_pause_item);
        let _ = menu.append(&next_item);
        let _ = menu.append(&prev_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit_item);

        let icon = Self::create_default_icon();

        let tray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("YouTube Music Desktop")
                .with_icon(icon)
                .build()
        }));

        let tray = match tray_result {
            Ok(Ok(t)) => {
                info!("System tray icon initialized successfully");
                Some(t)
            }
            Ok(Err(e)) => {
                error!("Failed to initialize system tray icon: {e}");
                None
            }
            Err(_) => {
                warn!("System tray icon unavailable (libappindicator3 runtime library not present). Continuing without tray icon.");
                None
            }
        };

        Self {
            _tray: tray,
            play_pause_id,
            next_id,
            prev_id,
            quit_id,
        }
    }

    /// Process tray menu events and dispatch actions.
    pub fn poll_events(&self, action_tx: &UnboundedSender<Action>) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id.0 == self.play_pause_id {
                let _ = action_tx.send(Action::TogglePlayPause);
            } else if event.id.0 == self.next_id {
                let _ = action_tx.send(Action::NextTrack);
            } else if event.id.0 == self.prev_id {
                let _ = action_tx.send(Action::PrevTrack);
            } else if event.id.0 == self.quit_id {
                std::process::exit(0);
            }
        }
    }

    /// Generate a 32x32 RGBA icon (red background with white triangle).
    fn create_default_icon() -> Icon {
        let width = 32u32;
        let height = 32u32;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                // Circle or rounded square in red
                let dx = x as f32 - 16.0;
                let dy = y as f32 - 16.0;
                let dist = (dx * dx + dy * dy).sqrt();

                // White play triangle in center: x from 12 to 22, y from 10 to 22
                let is_triangle = x >= 12 && x <= 22 && {
                    let top = 10.0 + (x as f32 - 12.0) * 0.6;
                    let bottom = 22.0 - (x as f32 - 12.0) * 0.6;
                    (y as f32) >= top && (y as f32) <= bottom
                };

                if is_triangle {
                    rgba.extend_from_slice(&[255, 255, 255, 255]); // White
                } else if dist <= 14.0 {
                    rgba.extend_from_slice(&[255, 0, 0, 255]); // YouTube Red
                } else {
                    rgba.extend_from_slice(&[0, 0, 0, 0]); // Transparent
                }
            }
        }

        Icon::from_rgba(rgba, width, height).expect("Failed to create tray icon")
    }
}
