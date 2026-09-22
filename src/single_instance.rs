use log::{info, warn};
use std::fs;
use std::path::PathBuf;
use std::process;

/// Enforces a single running instance of the application.
pub struct SingleInstance {
    lock_path: PathBuf,
}

impl SingleInstance {
    pub fn try_acquire() -> Result<Self, String> {
        let lock_path = dirs::config_dir()
            .map(|p| p.join("ytmusic-rs").join("app.lock"))
            .unwrap_or_else(|| PathBuf::from(".app.lock"));

        if let Some(parent) = lock_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if lock_path.exists() {
            if let Ok(content) = fs::read_to_string(&lock_path) {
                if let Ok(pid) = content.trim().parse::<u32>() {
                    // Check if process is still alive (Linux /proc/PID)
                    let proc_path = format!("/proc/{pid}");
                    if std::path::Path::new(&proc_path).exists() {
                        return Err(format!(
                            "Another instance of YouTube Music Desktop is already running (PID {pid})."
                        ));
                    } else {
                        warn!("Removing stale lock file from PID {pid}");
                    }
                }
            }
        }

        let pid = process::id();
        if let Err(e) = fs::write(&lock_path, pid.to_string()) {
            warn!("Failed to write lock file {:?}: {e}", lock_path);
        } else {
            info!("Acquired single instance lock (PID {pid})");
        }

        Ok(Self { lock_path })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
        info!("Released single instance lock");
    }
}
