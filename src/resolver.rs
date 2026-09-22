use crate::state::TrackMeta;
use log::{debug, error, info};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Resolve a YouTube video ID or URL into a direct audio stream URL
/// and track metadata by calling yt-dlp as a subprocess.
pub struct Resolver;

impl Resolver {
    /// Find the yt-dlp binary path, checking standard locations if not in PATH.
    pub fn find_ytdlp_bin() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            let local_bin = home.join(".local/bin/yt-dlp");
            if local_bin.exists() {
                return local_bin;
            }
            let user_bin = home.join("bin/yt-dlp");
            if user_bin.exists() {
                return user_bin;
            }
        }
        for fallback in ["/usr/local/bin/yt-dlp", "/usr/bin/yt-dlp"] {
            let p = PathBuf::from(fallback);
            if p.exists() {
                return p;
            }
        }
        PathBuf::from("yt-dlp")
    }

    /// Check if yt-dlp is installed and accessible.
    pub async fn check_ytdlp() -> Result<String, String> {
        let bin = Self::find_ytdlp_bin();
        let output = Command::new(&bin)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("yt-dlp not found (checked {:?}): {e}. Please install yt-dlp.", bin))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info!("yt-dlp ready at {:?}: version {version}", bin);
            Ok(version)
        } else {
            Err("yt-dlp returned an error".to_string())
        }
    }

    /// Resolve a YouTube URL or video ID into a direct audio stream URL.
    pub async fn resolve_stream_url(input: &str) -> Result<String, String> {
        let url = Self::normalize_url(input);
        info!("Resolving stream URL for: {url}");

        let bin = Self::find_ytdlp_bin();
        let output = Command::new(&bin)
            .args([
                "--format", "bestaudio",
                "--get-url",
                &url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run yt-dlp at {:?}: {e}", bin))?;

        if output.status.success() {
            let stream_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            debug!("Resolved stream URL ({} bytes)", stream_url.len());
            Ok(stream_url)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            error!("yt-dlp error: {stderr}");
            Err(format!("yt-dlp error: {stderr}"))
        }
    }

    /// Resolve metadata for a YouTube video (title, artist, duration, thumbnail).
    pub async fn resolve_metadata(input: &str) -> Result<TrackMeta, String> {
        let url = Self::normalize_url(input);
        info!("Resolving metadata for: {url}");

        let bin = Self::find_ytdlp_bin();
        let output = Command::new(&bin)
            .args([
                "--dump-json",
                "--no-download",
                &url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run yt-dlp at {:?}: {e}", bin))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("yt-dlp metadata error: {stderr}"));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse yt-dlp JSON: {e}"))?;

        let title = json["title"]
            .as_str()
            .unwrap_or("Unknown Title")
            .to_string();
        let artist = json["artist"]
            .as_str()
            .or_else(|| json["uploader"].as_str())
            .unwrap_or("Unknown Artist")
            .to_string();
        let thumbnail_url = json["thumbnail"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let duration_secs = json["duration"]
            .as_f64()
            .unwrap_or(0.0) as f32;
        let video_id = json["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(TrackMeta {
            title,
            artist,
            thumbnail_url,
            duration_secs,
            video_id,
        })
    }

    /// Normalize user input into a full YouTube URL.
    /// Accepts: full URLs, youtube.com/watch?v=..., music.youtube.com, or bare video IDs.
    fn normalize_url(input: &str) -> String {
        let input = input.trim();
        if input.starts_with("http://") || input.starts_with("https://") {
            input.to_string()
        } else if input.len() == 11 && input.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            format!("https://www.youtube.com/watch?v={input}")
        } else {
            // Try as URL anyway
            format!("https://www.youtube.com/watch?v={input}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_ytdlp() {
        let version = Resolver::check_ytdlp().await;
        assert!(version.is_ok(), "yt-dlp check should succeed, got {:?}", version);
    }

    #[tokio::test]
    async fn test_resolve_stream() {
        let url = Resolver::resolve_stream_url("fa5IWHDbftI").await;
        assert!(url.is_ok(), "resolve_stream_url should succeed, got {:?}", url);
        let stream = url.unwrap();
        assert!(stream.starts_with("https://"), "Stream URL should start with https://");
        println!("Stream URL resolved successfully (length: {})", stream.len());
    }
}
