use super::models::LibrarySong;
use super::YtMusicClient;
use log::{info, warn};
use serde_json::json;

impl YtMusicClient {
    /// Fetch the user's library songs.
    /// Requires authentication (cookies).
    pub async fn get_library_songs(&self) -> Result<Vec<LibrarySong>, String> {
        if !self.has_cookies() {
            return Err("Authentication required to access library".to_string());
        }

        info!("Fetching library songs...");

        let body = json!({
            "browseId": "FEmusic_liked_videos"
        });

        let response = self.post("browse", body).await?;
        let songs = Self::parse_library_songs(&response);

        info!("Found {} library songs", songs.len());
        Ok(songs)
    }

    /// Parse library songs from the browse response.
    fn parse_library_songs(response: &serde_json::Value) -> Vec<LibrarySong> {
        let mut songs = Vec::new();

        let contents = response
            .pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents")
            .or_else(|| response.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicShelfRenderer/contents"))
            .or_else(|| response.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicPlaylistShelfRenderer/contents"))
            .or_else(|| response.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents/0/musicShelfRenderer/contents"))
            .and_then(|c| c.as_array());

        let items = match contents {
            Some(arr) => arr,
            None => {
                warn!("Unexpected library response structure");
                return songs;
            }
        };

        for item in items {
            if let Some(song) = Self::parse_library_item(item) {
                songs.push(song);
            }
        }

        songs
    }

    fn parse_library_item(item: &serde_json::Value) -> Option<LibrarySong> {
        let renderer = item.get("musicResponsiveListItemRenderer")?;

        let video_id = renderer
            .pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if video_id.is_empty() {
            return None;
        }

        let columns: Vec<Vec<String>> = (0..3)
            .map(|i| {
                let path = format!("/flexColumns/{i}/musicResponsiveListItemFlexColumnRenderer/text/runs");
                renderer
                    .pointer(&path)
                    .and_then(|runs| runs.as_array())
                    .map(|runs| {
                        runs.iter()
                            .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();

        let title = columns.first()
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        let col1_texts: Vec<&str> = columns.get(1)
            .map(|c| c.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        let artist = col1_texts.first().copied().unwrap_or("Unknown").to_string();
        let album = col1_texts.get(2).copied().unwrap_or("").to_string();
        let duration = col1_texts.last().copied().unwrap_or("").to_string();

        let thumbnail_url = renderer
            .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails/0/url")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Some(LibrarySong {
            video_id,
            title,
            artist,
            album,
            thumbnail_url,
            duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_library_requires_auth() {
        let client = YtMusicClient::anonymous();
        let res = client.get_library_songs().await;
        assert!(res.is_err(), "Anonymous client should fail library fetch");
        assert_eq!(res.unwrap_err(), "Authentication required to access library");
    }
}
