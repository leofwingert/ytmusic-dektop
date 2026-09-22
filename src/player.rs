use log::{debug, info};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::{BufReader, Cursor};
use std::time::{Duration, Instant};

/// Manages audio playback using rodio.
/// Lives on the backend thread — never touched by UI directly.
pub struct Player {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
    current_data: Option<Vec<u8>>,
    current_volume: f32,
    playback_start: Option<Instant>,
    offset_secs: f32,
    duration_secs: f32,
    is_paused: bool,
    pause_instant: Option<Instant>,
}

impl Player {
    pub fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to open audio output: {e}"))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {e}"))?;

        info!("Audio player initialized");

        Ok(Self {
            _stream: stream,
            stream_handle,
            sink,
            current_data: None,
            current_volume: 0.75,
            playback_start: None,
            offset_secs: 0.0,
            duration_secs: 0.0,
            is_paused: false,
            pause_instant: None,
        })
    }

    /// Load and play audio from raw bytes.
    pub fn play_bytes(&mut self, data: Vec<u8>, duration_secs: f32) -> Result<(), String> {
        info!("Playing audio ({} bytes, {:.1}s)", data.len(), duration_secs);

        self.stop();

        let cursor = Cursor::new(data.clone());
        let buf_reader = BufReader::new(cursor);
        let source = Decoder::new(buf_reader)
            .map_err(|e| format!("Failed to decode audio: {e}"))?;

        self.sink.set_volume(self.current_volume);
        self.sink.append(source);
        self.sink.play();
        self.current_data = Some(data);
        self.duration_secs = duration_secs;
        self.playback_start = Some(Instant::now());
        self.offset_secs = 0.0;
        self.is_paused = false;
        self.pause_instant = None;

        Ok(())
    }

    /// Seek to a specific timestamp in seconds.
    pub fn seek(&mut self, pos_secs: f32) {
        let pos = pos_secs.clamp(0.0, self.duration_secs);
        let data = match &self.current_data {
            Some(d) => d.clone(),
            None => return,
        };

        self.sink.stop();
        if let Ok(new_sink) = Sink::try_new(&self.stream_handle) {
            new_sink.set_volume(self.current_volume);
            self.sink = new_sink;
        }

        let cursor = Cursor::new(data);
        let buf_reader = BufReader::new(cursor);
        if let Ok(source) = Decoder::new(buf_reader) {
            let skipped = source.skip_duration(Duration::from_secs_f32(pos));
            self.sink.append(skipped);
            if !self.is_paused {
                self.sink.play();
            } else {
                self.sink.pause();
            }
            self.playback_start = Some(Instant::now());
            self.offset_secs = pos;
            if self.is_paused {
                self.pause_instant = Some(Instant::now());
            }
            info!("Seeked to {pos:.1}s");
        }
    }

    pub fn pause(&mut self) {
        if !self.is_paused {
            self.sink.pause();
            self.is_paused = true;
            self.pause_instant = Some(Instant::now());
            debug!("Playback paused");
        }
    }

    pub fn resume(&mut self) {
        if self.is_paused {
            if let (Some(start), Some(pause_at)) = (self.playback_start, self.pause_instant) {
                let paused_duration = Instant::now() - pause_at;
                self.playback_start = Some(start + paused_duration);
            }
            self.sink.play();
            self.is_paused = false;
            self.pause_instant = None;
            debug!("Playback resumed");
        }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        if let Ok(new_sink) = Sink::try_new(&self.stream_handle) {
            new_sink.set_volume(self.current_volume);
            self.sink = new_sink;
        }
        self.playback_start = None;
        self.offset_secs = 0.0;
        self.is_paused = false;
        self.pause_instant = None;
        debug!("Playback stopped");
    }

    pub fn set_volume(&mut self, volume: f32) {
        let vol = volume.clamp(0.0, 1.0);
        self.current_volume = vol;
        self.sink.set_volume(vol);
        debug!("Volume set to {vol:.2}");
    }

    pub fn progress_secs(&self) -> f32 {
        if let Some(start) = self.playback_start {
            let elapsed = if self.is_paused {
                self.pause_instant
                    .map(|p| p - start)
                    .unwrap_or(Duration::ZERO)
            } else {
                Instant::now() - start
            };
            (self.offset_secs + elapsed.as_secs_f32()).min(self.duration_secs)
        } else {
            0.0
        }
    }

    #[allow(dead_code)]
    pub fn duration_secs(&self) -> f32 {
        self.duration_secs
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty() && self.playback_start.is_some()
    }

    #[allow(dead_code)]
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        self.playback_start.is_some() && !self.is_paused && !self.sink.empty()
    }
}
