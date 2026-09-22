mod action;
mod api;
mod app;
mod backend;
mod media_keys;
mod player;
mod resolver;
mod settings;
mod single_instance;
mod state;
mod thumbnail;
mod tray;

use app::App;
use log::{error, info};
use single_instance::SingleInstance;

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    info!("Starting YouTube Music Desktop");

    // Single instance enforcement
    let _instance_lock = match SingleInstance::try_acquire() {
        Ok(lock) => lock,
        Err(e) => {
            error!("{e}");
            eprintln!("{e}");
            return Ok(());
        }
    };

    let icon_data = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .ok()
        .map(|img| {
            let rgba = img.to_rgba8();
            egui::IconData {
                width: rgba.width(),
                height: rgba.height(),
                rgba: rgba.into_raw(),
            }
        });

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1060.0, 700.0])
        .with_min_inner_size([720.0, 480.0])
        .with_title("YouTube Music Desktop");

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "YouTube Music Desktop",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_generate_icon() {
        let size = 256u32;
        let mut img = image::RgbaImage::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - 128.0;
                let dy = y as f32 - 128.0;
                let dist = (dx * dx + dy * dy).sqrt();

                let is_triangle = x >= 102 && x <= 182 && {
                    let top = 82.0 + (x as f32 - 102.0) * 0.58;
                    let bottom = 174.0 - (x as f32 - 102.0) * 0.58;
                    (y as f32) >= top && (y as f32) <= bottom
                };

                let pixel = if is_triangle {
                    image::Rgba([255, 255, 255, 255])
                } else if dist <= 120.0 {
                    let factor = (1.0 - (dist / 120.0) * 0.2).clamp(0.0, 1.0);
                    image::Rgba([(255.0 * factor) as u8, 0, 0, 255])
                } else {
                    image::Rgba([0, 0, 0, 0])
                };
                img.put_pixel(x, y, pixel);
            }
        }
        std::fs::create_dir_all("assets").unwrap();
        img.save("assets/icon.png").unwrap();
        assert!(std::path::Path::new("assets/icon.png").exists());
    }
}
