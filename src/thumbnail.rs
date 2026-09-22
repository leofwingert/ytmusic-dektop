use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Manages downloading, disk caching, and egui texture creation for track thumbnails.
pub struct ThumbnailManager {
    textures: HashMap<String, TextureHandle>,
    pending: HashSet<String>,
    tx: Sender<(String, ColorImage)>,
    rx: Receiver<(String, ColorImage)>,
    cache_dir: PathBuf,
}

impl ThumbnailManager {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let cache_dir = dirs::cache_dir()
            .map(|p| p.join("ytmusic-rs").join("thumbnails"))
            .unwrap_or_else(|| PathBuf::from(".thumbnails_cache"));

        if let Err(e) = fs::create_dir_all(&cache_dir) {
            error!("Failed to create thumbnail cache directory {:?}: {e}", cache_dir);
        } else {
            info!("Thumbnail cache directory: {:?}", cache_dir);
        }

        Self {
            textures: HashMap::new(),
            pending: HashSet::new(),
            tx,
            rx,
            cache_dir,
        }
    }

    /// Try to get a texture for a thumbnail URL.
    /// If not yet loaded, schedules background fetch and returns None.
    pub fn get_or_load(&mut self, ctx: &Context, url: &str) -> Option<&TextureHandle> {
        if url.is_empty() {
            return None;
        }

        // Process any newly loaded images from background tasks
        while let Ok((loaded_url, color_image)) = self.rx.try_recv() {
            self.pending.remove(&loaded_url);
            let handle = ctx.load_texture(&loaded_url, color_image, TextureOptions::LINEAR);
            self.textures.insert(loaded_url, handle);
        }

        if self.textures.contains_key(url) {
            return self.textures.get(url);
        }

        // Trigger background fetch if not already in progress
        if !self.pending.contains(url) {
            self.pending.insert(url.to_string());
            let url_clone = url.to_string();
            let tx_clone = self.tx.clone();
            let cache_dir_clone = self.cache_dir.clone();
            let ctx_clone = ctx.clone();

            std::thread::spawn(move || {
                if let Some(color_image) = Self::fetch_and_decode(&url_clone, &cache_dir_clone) {
                    let _ = tx_clone.send((url_clone, color_image));
                    ctx_clone.request_repaint();
                }
            });
        }

        None
    }

    /// Hash a URL to a safe filename for disk caching.
    fn url_to_filename(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:016x}.png", hasher.finish())
    }

    /// Fetch image from disk cache or network, and decode into egui::ColorImage.
    fn fetch_and_decode(url: &str, cache_dir: &PathBuf) -> Option<ColorImage> {
        let filename = Self::url_to_filename(url);
        let cached_path = cache_dir.join(&filename);

        // Try reading from disk cache first
        let image_bytes = if cached_path.exists() {
            match fs::read(&cached_path) {
                Ok(bytes) => {
                    debug!("Loaded thumbnail from cache: {:?}", cached_path);
                    bytes
                }
                Err(e) => {
                    error!("Failed to read cached thumbnail {:?}: {e}", cached_path);
                    Self::download_and_cache(url, &cached_path)?
                }
            }
        } else {
            Self::download_and_cache(url, &cached_path)?
        };

        // Decode using the `image` crate
        match image::load_from_memory(&image_bytes) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                Some(ColorImage::from_rgba_unmultiplied(size, &pixels))
            }
            Err(e) => {
                error!("Failed to decode image from {url}: {e}");
                None
            }
        }
    }

    /// Download thumbnail from network and save to disk cache.
    fn download_and_cache(url: &str, cached_path: &PathBuf) -> Option<Vec<u8>> {
        debug!("Downloading thumbnail: {url}");
        let resp = reqwest::blocking::get(url).ok()?;
        if !resp.status().is_success() {
            return None;
        }

        let bytes = resp.bytes().ok()?.to_vec();

        // Write to disk cache
        if let Err(e) = fs::write(cached_path, &bytes) {
            error!("Failed to save thumbnail to {:?}: {e}", cached_path);
        } else {
            debug!("Saved thumbnail to {:?}", cached_path);
        }

        Some(bytes)
    }
}
