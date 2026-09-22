use crate::api::models::SearchResult;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Current player status — what the UI shows.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerStatus {
    Idle,
    Loading,
    Playing,
    Paused,
    Error(String),
}

impl fmt::Display for PlayerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Loading => write!(f, "Loading…"),
            Self::Playing => write!(f, "Playing"),
            Self::Paused => write!(f, "Paused"),
            Self::Error(e) => write!(f, "Error: {e}"),
        }
    }
}

/// Repeat mode for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Off => "🔁",
            Self::All => "🔁",
            Self::One => "🔂",
        }
    }
}

/// Visual theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    Dark,
    Amoled,
    Light,
}

/// Metadata about a track.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub thumbnail_url: String,
    pub duration_secs: f32,
    pub video_id: String,
}

/// Which view/page is currently active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveView {
    Home,
    Search,
    Library,
    Queue,
    Settings,
}

/// Shared application state — the UI reads this via Arc<RwLock<AppState>>.
/// The backend is the primary writer.
pub struct AppState {
    // Player state
    pub status: PlayerStatus,
    pub current_track: Option<TrackMeta>,
    pub progress_secs: f32,
    pub duration_secs: f32,
    pub volume: f32,

    // Queue & Playback modes
    pub queue: VecDeque<TrackMeta>,
    pub queue_history: Vec<TrackMeta>,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,

    // Search state
    pub search_results: Vec<SearchResult>,
    pub is_searching: bool,

    // Library state
    pub library_songs: Vec<crate::api::models::LibrarySong>,
    pub is_loading_library: bool,

    // Auth state
    pub is_authenticated: bool,
    pub auth_error: Option<String>,

    // App theme
    pub theme: AppTheme,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            status: PlayerStatus::Idle,
            current_track: None,
            progress_secs: 0.0,
            duration_secs: 0.0,
            volume: 0.75,
            queue: VecDeque::new(),
            queue_history: Vec::new(),
            repeat_mode: RepeatMode::Off,
            shuffle: false,
            search_results: Vec::new(),
            is_searching: false,
            library_songs: Vec::new(),
            is_loading_library: false,
            is_authenticated: false,
            auth_error: None,
            theme: AppTheme::Dark,
        }
    }
}

/// Parse time string in "M:SS" or "H:MM:SS" format into total seconds.
pub fn parse_time_str(s: &str) -> f32 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: f32 = parts[0].trim().parse().unwrap_or(0.0);
            let s: f32 = parts[1].trim().parse().unwrap_or(0.0);
            m * 60.0 + s
        }
        3 => {
            let h: f32 = parts[0].trim().parse().unwrap_or(0.0);
            let m: f32 = parts[1].trim().parse().unwrap_or(0.0);
            let s: f32 = parts[2].trim().parse().unwrap_or(0.0);
            h * 3600.0 + m * 60.0 + s
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_str() {
        assert_eq!(parse_time_str("4:41"), 281.0);
        assert_eq!(parse_time_str("0:30"), 30.0);
        assert_eq!(parse_time_str("1:02:15"), 3735.0);
        assert_eq!(parse_time_str(""), 0.0);
    }
}

