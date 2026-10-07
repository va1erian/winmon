//! A tiny file log: `%LOCALAPPDATA%\winmon\winmon.log`, rotated at 1 MB.
//! Debug builds also echo to stderr and log `info`; release logs errors only.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_BYTES: u64 = 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

fn path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("winmon").join("winmon.log"))
}

fn write(level: &str, msg: &str) {
    if cfg!(debug_assertions) {
        eprintln!("[{level}] {msg}");
    }
    let Some(path) = path() else { return };
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let _ = writeln!(f, "{secs} [{level}] {msg}");
    }
}

pub fn info(msg: &str) {
    if cfg!(debug_assertions) {
        write("info", msg);
    }
}

pub fn error(msg: &str) {
    write("error", msg);
}

/// Logs panics before `panic = "abort"` takes the process down.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error(&format!("panic: {info}"));
        default(info);
    }));
}
