use serde::{Deserialize, Serialize};

/// A search result from YouTube Music.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub thumbnail_url: String,
    pub result_type: SearchResultType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchResultType {
    Song,
    Video,
    Album,
    Artist,
    Playlist,
    Unknown,
}

/// A library song entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrarySong {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub thumbnail_url: String,
    pub duration: String,
}

/// A playlist entry.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub playlist_id: String,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub count: String,
}

/// Album info.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub browse_id: String,
    pub title: String,
    pub artist: String,
    pub year: String,
    pub thumbnail_url: String,
}
