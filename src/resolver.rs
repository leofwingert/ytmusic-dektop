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

    /// Check if Node.js or Deno is available on the system for YouTube JS challenges.
    pub fn has_js_runtime() -> bool {
        if PathBuf::from("/usr/bin/node").exists()
            || PathBuf::from("/usr/local/bin/node").exists()
            || PathBuf::from("/usr/bin/deno").exists()
        {
            return true;
        }
        std::process::Command::new("node")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Locate cookies file for yt-dlp (cookies.txt or cookies.json).
    pub fn find_cookies_path() -> Option<PathBuf> {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ytmusic-rs");
        let txt = dir.join("cookies.txt");
        if txt.exists() {
            return Some(txt);
        }
        let json = dir.join("cookies.json");
        if json.exists() {
            return Some(json);
        }
        None
    }

    /// Local directory for caching downloaded audio files: ~/.cache/ytmusic-rs/audio/
    pub fn audio_cache_dir() -> PathBuf {
        let dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("ytmusic-rs/audio");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            error!("Failed to create audio cache directory {:?}: {e}", dir);
        }
        dir
    }

    /// Extract video ID from URL or return the trimmed ID.
    pub fn extract_video_id(input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.len() == 11 && trimmed.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return trimmed.to_string();
        }
        if let Some(pos) = trimmed.find("v=") {
            let rest = &trimmed[pos + 2..];
            let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
            if !id.is_empty() {
                return id;
            }
        }
        if let Some(pos) = trimmed.find("youtu.be/") {
            let rest = &trimmed[pos + 9..];
            let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
            if !id.is_empty() {
                return id;
            }
        }
        trimmed.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_")
    }

    /// Fetch or download track audio as MP3 bytes.
    /// Checks local disk cache first (~/.cache/ytmusic-rs/audio/<video_id>.mp3).
    /// If not present, downloads & converts via yt-dlp in ~2-4s.
    pub async fn get_or_download_audio(video_id: &str, title_hint: Option<&str>) -> Result<Vec<u8>, String> {
        let clean_id = Self::extract_video_id(video_id);
        let cache_dir = Self::audio_cache_dir();
        let cache_file = cache_dir.join(format!("{clean_id}.mp3"));

        // 1. Check disk cache
        if cache_file.exists() {
            if let Ok(meta) = std::fs::metadata(&cache_file) {
                if meta.len() > 1000 {
                    info!(
                        "Loaded audio from disk cache for '{}' ({:.2} MB, path: {:?})",
                        title_hint.unwrap_or(video_id),
                        meta.len() as f64 / (1024.0 * 1024.0),
                        cache_file
                    );
                    return std::fs::read(&cache_file)
                        .map_err(|e| format!("Failed to read cached audio file: {e}"));
                }
            }
        }

        // 2. Download and convert via yt-dlp
        let url = format!("https://www.youtube.com/watch?v={clean_id}");
        info!(
            "Downloading and converting audio for '{}' via yt-dlp...",
            title_hint.unwrap_or(video_id)
        );

        let bin = Self::find_ytdlp_bin();
        let mut cmd = Command::new(&bin);

        if Self::has_js_runtime() {
            cmd.args(["--js-runtimes", "node"]);
        }

        if let Some(cookies) = Self::find_cookies_path() {
            cmd.arg("--cookies").arg(cookies);
        }

        let output_template = cache_dir.join(format!("{clean_id}.%(ext)s"));

        cmd.args([
            "--no-playlist",
            "-x",
            "--audio-format", "mp3",
            "--audio-quality", "0",
            "-o", output_template.to_str().unwrap(),
            &url,
        ]);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to execute yt-dlp at {:?}: {e}", bin))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("yt-dlp download failed: {stderr}");
            return Err(format!("yt-dlp download failed: {stderr}"));
        }

        if cache_file.exists() {
            let bytes = std::fs::read(&cache_file)
                .map_err(|e| format!("Failed to read downloaded audio file: {e}"))?;
            info!(
                "Successfully downloaded and cached '{}' ({} bytes)",
                title_hint.unwrap_or(video_id),
                bytes.len()
            );
            Ok(bytes)
        } else {
            Err(format!("Downloaded file not found at {:?}", cache_file))
        }
    }

    /// Resolve a YouTube URL or video ID into a direct audio stream URL.
    pub async fn resolve_stream_url(input: &str) -> Result<String, String> {
        let url = Self::normalize_url(input);
        info!("Resolving stream URL for: {url}");

        let bin = Self::find_ytdlp_bin();
        let mut cmd = Command::new(&bin);

        if Self::has_js_runtime() {
            cmd.args(["--js-runtimes", "node"]);
        }
        if let Some(cookies) = Self::find_cookies_path() {
            cmd.arg("--cookies").arg(cookies);
        }

        cmd.args([
            "--format", "bestaudio",
            "--get-url",
            &url,
        ]);

        let output = cmd
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
        let mut cmd = Command::new(&bin);

        if Self::has_js_runtime() {
            cmd.args(["--js-runtimes", "node"]);
        }
        if let Some(cookies) = Self::find_cookies_path() {
            cmd.arg("--cookies").arg(cookies);
        }

        cmd.args([
            "--dump-json",
            "--no-download",
            &url,
        ]);

        let output = cmd
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
    pub fn normalize_url(input: &str) -> String {
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

    #[test]
    fn test_extract_video_id() {
        assert_eq!(Resolver::extract_video_id("eMGRt0A9Yns"), "eMGRt0A9Yns");
        assert_eq!(Resolver::extract_video_id("https://www.youtube.com/watch?v=eMGRt0A9Yns"), "eMGRt0A9Yns");
        assert_eq!(Resolver::extract_video_id("https://music.youtube.com/watch?v=eMGRt0A9Yns&list=RD123"), "eMGRt0A9Yns");
        assert_eq!(Resolver::extract_video_id("https://youtu.be/eMGRt0A9Yns?t=10"), "eMGRt0A9Yns");
    }

    #[tokio::test]
    async fn test_get_or_download_audio() {
        // Test with a short test video: jNQXAC9IVRw (First video on YouTube, ~19s)
        let res = Resolver::get_or_download_audio("jNQXAC9IVRw", Some("Me at the zoo")).await;
        assert!(res.is_ok(), "Failed to download audio: {:?}", res.err());
        let bytes = res.unwrap();
        assert!(!bytes.is_empty(), "Downloaded audio should not be empty");

        // Verify cache file exists
        let cache_file = Resolver::audio_cache_dir().join("jNQXAC9IVRw.mp3");
        assert!(cache_file.exists(), "Cache file should exist");

        // Second call should load from cache
        let cached_res = Resolver::get_or_download_audio("jNQXAC9IVRw", Some("Me at the zoo")).await;
        assert!(cached_res.is_ok());
        assert_eq!(cached_res.unwrap().len(), bytes.len());
    }
}


