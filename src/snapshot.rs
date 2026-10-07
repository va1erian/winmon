//! Development aid: `WINMON_SNAPSHOT=out.bmp` captures the window to a BMP
//! after `WINMON_SNAPSHOT_TICKS` ticks (default 5), then exits. Lets the
//! dashboard be checked without access to the physical screen. The settings
//! and setup windows honour it too (one tick per second); for settings,
//! `WINMON_SNAPSHOT_TAB=<n>` picks the tab.

use win32ui::{RgbaImage, Ui};

use crate::log;

pub struct Snapshot {
    path: String,
    ticks_left: u32,
}

impl Snapshot {
    pub fn from_env() -> Option<Snapshot> {
        let path = std::env::var("WINMON_SNAPSHOT")
            .ok()
            .filter(|p| !p.is_empty())?;
        let ticks_left = std::env::var("WINMON_SNAPSHOT_TICKS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(5);
        Some(Snapshot { path, ticks_left })
    }

    /// Counts a tick; `true` when it's time to capture.
    pub fn tick(&mut self) -> bool {
        self.ticks_left = self.ticks_left.saturating_sub(1);
        self.ticks_left == 0
    }

    pub fn save(&self, image: &RgbaImage) -> std::io::Result<()> {
        std::fs::write(&self.path, bmp(image))
    }

    /// Captures `ui`'s window, saves it and ends the loop.
    pub fn capture<M: 'static>(&self, ui: &Ui<M>) {
        match ui
            .capture()
            .map_err(|e| e.to_string())
            .and_then(|img| self.save(&img).map_err(|e| e.to_string()))
        {
            Ok(()) => log::info("snapshot saved"),
            Err(e) => log::error(&format!("snapshot: {e}")),
        }
        ui.quit();
    }

    /// For windows without their own tick: when a snapshot is requested,
    /// maps a 1 s timer to `msg`; call [`Snapshot::on_tick`] on it.
    pub fn arm<M: 'static>(ui: &Ui<M>, msg: impl Fn() -> M + 'static) -> Option<Snapshot> {
        let snap = Snapshot::from_env()?;
        let id = ui.set_timer(1000).ok()?;
        ui.on_timer(move |t| (t == id).then(&msg));
        Some(snap)
    }

    /// Counts a tick of an [`arm`](Snapshot::arm)ed window; captures when due.
    pub fn on_tick<M: 'static>(snap: &mut Option<Snapshot>, ui: &Ui<M>) {
        if let Some(s) = snap.as_mut()
            && s.tick()
        {
            s.capture(ui);
            *snap = None;
        }
    }

    /// `WINMON_SNAPSHOT_TAB`, for the settings window.
    pub fn tab() -> Option<usize> {
        std::env::var("WINMON_SNAPSHOT_TAB").ok()?.parse().ok()
    }
}

/// A 32-bit top-down BMP.
fn bmp(image: &RgbaImage) -> Vec<u8> {
    let (w, h) = (image.width, image.height);
    let data_len = w * h * 4;
    let mut out = Vec::with_capacity(54 + data_len as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + data_len).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(-(h as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 24]);
    for px in image.pixels.as_chunks::<4>().0 {
        out.extend_from_slice(&[px[2], px[1], px[0], 255]);
    }
    out
}
