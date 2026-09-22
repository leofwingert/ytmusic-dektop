pub mod auth;
pub mod library;
pub mod models;
pub mod search;

use reqwest::header::{HeaderMap, HeaderValue, COOKIE, ORIGIN, REFERER, USER_AGENT};
use reqwest::Client;
use serde_json::json;

/// Base YouTube Music API client.
/// Wraps reqwest::Client with proper headers for the YouTube Music internal API.
pub struct YtMusicClient {
    client: Client,
    cookies: String,
}

/// Base context payload required by all YouTube Music API requests.
fn base_context() -> serde_json::Value {
    json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.00.00",
                "hl": "en",
                "gl": "US"
            }
        }
    })
}

impl YtMusicClient {
    /// Create a new client. `cookies` is the raw Cookie header string.
    pub fn new(cookies: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        ));
        headers.insert(ORIGIN, HeaderValue::from_static("https://music.youtube.com"));
        headers.insert(REFERER, HeaderValue::from_static("https://music.youtube.com/"));
        if let Ok(cookie_val) = HeaderValue::from_str(&cookies) {
            headers.insert(COOKIE, cookie_val);
        }

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        Self { client, cookies }
    }

    /// Create a client without authentication (limited functionality).
    pub fn anonymous() -> Self {
        Self::new(String::new())
    }

    /// Make a POST request to a YouTube Music API endpoint.
    pub async fn post(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = format!("https://music.youtube.com/youtubei/v1/{endpoint}?prettyPrint=false");

        // Merge body with base context
        let mut payload = base_context();
        if let (Some(base), Some(extra)) = (payload.as_object_mut(), body.as_object()) {
            for (k, v) in extra {
                base.insert(k.clone(), v.clone());
            }
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("API returned status {}", resp.status()));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("JSON parse error: {e}"))
    }

    pub fn has_cookies(&self) -> bool {
        !self.cookies.is_empty()
    }
}
