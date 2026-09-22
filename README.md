# YouTube Music Desktop (Rust)

A native, blazing-fast YouTube Music desktop client for Linux and Windows built with Rust, egui, rodio, and tokio. Completely free of Electron or WebView.

![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)
![egui](https://img.shields.io/badge/UI-egui%200.31-blue.svg)
![rodio](https://img.shields.io/badge/Audio-rodio%200.20-green.svg)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)

---

## ✨ Features

- **Pure Native Desktop App**: No Electron, no WebView, ultra-low memory footprint (~30-50MB RAM).
- **Search & Playback**: Instant search across YouTube Music catalog with album covers, artist metadata, and duration.
- **Audio Streaming & Seeking**: High-quality audio streaming via `yt-dlp` subprocess + Symphonia audio decoding with smooth seeking.
- **Queue Management**: Upcoming tracks queue with auto-advance, reordering, shuffle mode (🔀), and 3-state repeat (🔁 Off, All, One).
- **Global Media Keys & MPRIS**: Control playback using keyboard media keys or OS media widgets on Linux/Windows.
- **System Tray**: System tray icon with quick controls (Play/Pause, Next, Previous, Quit).
- **Persistent Settings**: Remembers volume, repeat mode, shuffle, and theme in `~/.config/ytmusic-rs/settings.json`.
- **Theme Switcher**: Dark mode, AMOLED pure black, and Light theme.
- **Thumbnail Disk Cache**: Covers cached locally at `~/.cache/ytmusic-rs/thumbnails/` for fast offline loading.
- **Optional Browser Auth**: Load your library and playlists by exporting your cookies to `~/.config/ytmusic-rs/cookies.json`.

---

## 🛠 Prerequisites

- [Rust & Cargo](https://rustup.rs/) (edition 2021)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) (auto-detected from `PATH`, `~/.local/bin`, or `~/bin`)
- ALSA / PulseAudio development libraries on Linux (standard on most distros)

---

## 🚀 Getting Started

### Development
```bash
cargo run
```

### Production Build
```bash
cargo build --release
```
The optimized executable will be in `target/release/ytmusic-rs`.

### Desktop Integration (Linux)
```bash
# Copy binary
mkdir -p ~/.local/bin
cp target/release/ytmusic-rs ~/.local/bin/

# Install .desktop launcher
mkdir -p ~/.local/share/applications
cp ytmusic-desktop.desktop ~/.local/share/applications/
```

---

## 🔐 Authentication (Optional)

To browse your personal library:
1. Export cookies from [music.youtube.com](https://music.youtube.com) using an extension (like "Get cookies.txt LOCALLY").
2. Save the file to:
   - `~/.config/ytmusic-rs/cookies.json` or `~/.config/ytmusic-rs/cookies.txt`

The app automatically picks up the session on startup.

---

## 🧪 Testing

Run the automated test suite:
```bash
cargo test
```

---

## 📜 License

Dual-licensed under MIT and Apache-2.0.
