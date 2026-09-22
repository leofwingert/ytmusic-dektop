use super::models::{SearchResult, SearchResultType};
use super::YtMusicClient;
use log::{info, warn};
use serde_json::json;

impl YtMusicClient {
    /// Search YouTube Music for songs, videos, albums, artists, or playlists.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        info!("Searching for: {query}");

        let body = json!({
            "query": query,
            "params": "EgWKAQIIAWoKEAkQBRAKEAMQBA%3D%3D"  // filter for songs
        });

        let response = self.post("search", body).await?;
        let results = Self::parse_search_results(&response);

        info!("Found {} results", results.len());
        Ok(results)
    }

    /// Parse the complex YouTube Music search response into a flat list.
    fn parse_search_results(response: &serde_json::Value) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // Navigate the deeply nested YouTube Music response structure
        let contents = response
            .pointer("/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
            .or_else(|| response.pointer("/contents/sectionListRenderer/contents"));

        let sections = match contents.and_then(|c| c.as_array()) {
            Some(arr) => arr,
            None => {
                warn!("Unexpected search response structure");
                return results;
            }
        };

        for section in sections {
            let items = section
                .pointer("/musicShelfRenderer/contents")
                .and_then(|c| c.as_array());

            if let Some(items) = items {
                for item in items {
                    if let Some(result) = Self::parse_search_item(item) {
                        results.push(result);
                    }
                }
            }
        }

        results
    }

    /// Parse a single search result item from the YouTube Music response.
    fn parse_search_item(item: &serde_json::Value) -> Option<SearchResult> {
        let renderer = item.get("musicResponsiveListItemRenderer")?;

        // Extract video ID from navigation endpoint
        let video_id = renderer
            .pointer("/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId")
            .or_else(|| renderer.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if video_id.is_empty() {
            return None;
        }

        // Extract text runs from flex columns
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

        // Second column typically has: artist • album • duration
        let col1_texts: Vec<&str> = columns.get(1)
            .map(|c| c.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        let artist = col1_texts.first().copied().unwrap_or("Unknown Artist").to_string();
        let album = col1_texts.get(2).copied().unwrap_or("").to_string();

        // Duration is usually the last text in column 1
        let duration = col1_texts.last().copied().unwrap_or("").to_string();

        // Thumbnail
        let thumbnail_url = renderer
            .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails/0/url")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Some(SearchResult {
            video_id,
            title,
            artist,
            album,
            duration,
            thumbnail_url,
            result_type: SearchResultType::Song,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_songs() {
        let client = YtMusicClient::anonymous();
        let res = client.search("Daft Punk").await;
        assert!(res.is_ok(), "Search should return Ok, got {:?}", res);
        let songs = res.unwrap();
        println!("Got {} songs", songs.len());
        assert!(!songs.is_empty(), "Should find at least 1 song");
        println!("First song: {} - {} (id: {})", songs[0].artist, songs[0].title, songs[0].video_id);
    }
}
