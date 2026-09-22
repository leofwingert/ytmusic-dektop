use log::{info, warn};
use std::path::PathBuf;

/// Manages cookie-based authentication for the YouTube Music API.
/// Reads cookies from cookies.json or cookies.txt in ~/.config/ytmusic-rs/.
pub struct Auth;

impl Auth {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ytmusic-rs")
    }

    /// Primary cookie file path: ~/.config/ytmusic-rs/cookies.json
    pub fn cookie_file_path() -> PathBuf {
        Self::config_dir().join("cookies.json")
    }

    /// Load cookies from ~/.config/ytmusic-rs/cookies.json or cookies.txt.
    pub fn load_cookies() -> Result<String, String> {
        let dir = Self::config_dir();
        let json_path = dir.join("cookies.json");
        let txt_path = dir.join("cookies.txt");

        if json_path.exists() {
            let content = std::fs::read_to_string(&json_path)
                .map_err(|e| format!("Failed to read {}: {e}", json_path.display()))?;
            let cookies = Self::parse_json_cookies(&content)?;
            info!("Loaded {} cookies from {}", cookies.len(), json_path.display());
            return Ok(cookies.join("; "));
        }

        if txt_path.exists() {
            let content = std::fs::read_to_string(&txt_path)
                .map_err(|e| format!("Failed to read {}: {e}", txt_path.display()))?;
            let cookies = Self::parse_netscape_cookies(&content);
            if cookies.is_empty() {
                return Err(format!("No valid YouTube cookies in {}", txt_path.display()));
            }
            info!("Loaded {} cookies from {}", cookies.len(), txt_path.display());
            return Ok(cookies.join("; "));
        }

        Err(format!(
            "No cookie file found at {} or {}. \
             Export your browser cookies to authenticate.",
            json_path.display(),
            txt_path.display()
        ))
    }

    /// Parse JSON cookies (supports array of cookie objects or key-value map).
    pub fn parse_json_cookies(content: &str) -> Result<Vec<String>, String> {
        let val: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("JSON parse error: {e}"))?;

        let mut cookies = Vec::new();

        if let Some(arr) = val.as_array() {
            for item in arr {
                if let (Some(name), Some(value)) = (
                    item.get("name").and_then(|v| v.as_str()),
                    item.get("value").and_then(|v| v.as_str()),
                ) {
                    let domain = item.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                    if domain.is_empty() || domain.contains("youtube.com") || domain.contains(".google.com") {
                        cookies.push(format!("{name}={value}"));
                    }
                }
            }
        } else if let Some(map) = val.as_object() {
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    cookies.push(format!("{k}={s}"));
                }
            }
        }

        if cookies.is_empty() {
            return Err("No valid cookies found in JSON".to_string());
        }

        Ok(cookies)
    }

    /// Parse Netscape-format cookies.txt into name=value pairs.
    pub fn parse_netscape_cookies(content: &str) -> Vec<String> {
        content
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 7 {
                    let domain = parts[0];
                    let name = parts[5];
                    let value = parts[6];
                    if domain.contains("youtube.com") || domain.contains(".google.com") {
                        Some(format!("{name}={value}"))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Try to load cookies, returning None if not available (anonymous mode).
    pub fn try_load_cookies() -> Option<String> {
        match Self::load_cookies() {
            Ok(cookies) => Some(cookies),
            Err(e) => {
                warn!("Auth: {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_cookies_array() {
        let json = r#"[
            {"name": "SAPISID", "value": "12345", "domain": ".youtube.com"},
            {"name": "LOGIN_INFO", "value": "abcdef", "domain": ".google.com"}
        ]"#;
        let res = Auth::parse_json_cookies(json).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0], "SAPISID=12345");
        assert_eq!(res[1], "LOGIN_INFO=abcdef");
    }

    #[test]
    fn test_parse_netscape_cookies() {
        let txt = ".youtube.com\tTRUE\t/\tTRUE\t1700000000\tSID\tsecret_sid\n";
        let res = Auth::parse_netscape_cookies(txt);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], "SID=secret_sid");
    }
}
