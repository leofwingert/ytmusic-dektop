use crate::api::models::SearchResult;
use crate::state::{AppTheme, TrackMeta};

/// Actions the UI or background services emit to the backend.
#[derive(Debug, Clone)]
pub enum Action {
    /// Start playing a YouTube video by URL or video ID.
    Play(String),
    /// Play a specific track with known metadata.
    PlayTrack(TrackMeta),
    /// Pause the current track.
    Pause,
    /// Resume the paused track.
    Resume,
    /// Toggle between play and pause.
    TogglePlayPause,
    /// Stop playback completely.
    Stop,
    /// Seek to a position in seconds.
    Seek(f32),
    /// Set volume (0.0 to 1.0).
    SetVolume(f32),

    // Phase 2 — Search & Library
    /// Search YouTube Music for a query string.
    Search(String),
    /// Load the user's library songs.
    LoadLibrary,
    /// Play a specific track from search results.
    PlaySearchResult(SearchResult),

    // Phase 3 — Queue & Playback controls
    /// Add a track to the playback queue.
    QueueAdd(TrackMeta),
    /// Remove a track from the queue by index.
    QueueRemove(usize),
    /// Clear all upcoming tracks in the queue.
    QueueClear,
    /// Skip to the next track.
    NextTrack,
    /// Return to previous track (or restart current track if >3s).
    PrevTrack,
    /// Toggle shuffle mode.
    ToggleShuffle,
    /// Cycle repeat mode (Off -> All -> One -> Off).
    CycleRepeat,

    // Phase 4 — Theme & Settings
    /// Change the visual theme.
    SetTheme(AppTheme),
}
