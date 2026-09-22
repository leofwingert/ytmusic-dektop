use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub volume: f32,
    pub repeat_mode: String,
    pub shuffle: bool,
    pub theme: String,
    pub last_track_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 0.75,
            repeat_mode: "off".to_string(),
            shuffle: false,
            theme: "dark".to_string(),
            last_track_id: None,
        }
    }
}

impl Settings {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("ytmusic-rs").join("settings.json"))
            .unwrap_or_else(|| PathBuf::from("settings.json"))
    }

    /// Load settings from disk, or return default if missing or invalid.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(settings) => {
                        info!("Loaded settings from {:?}", path);
                        return settings;
                    }
                    Err(e) => error!("Failed to parse settings at {:?}: {e}", path),
                },
                Err(e) => error!("Failed to read settings file at {:?}: {e}", path),
            }
        }
        info!("Using default settings");
        Self::default()
    }

    /// Save current settings to disk.
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    error!("Failed to write settings to {:?}: {e}", path);
                } else {
                    info!("Saved settings to {:?}", path);
                }
            }
            Err(e) => error!("Failed to serialize settings: {e}"),
        }
    }
}
