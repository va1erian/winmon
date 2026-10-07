//! Per-user install: no elevation, nothing outside the user's profile.
//!
//! - the binary goes to `%LOCALAPPDATA%\Programs\winmon\winmon.exe`;
//! - Start menu shortcuts "winmon" and "winmon settings";
//! - optional autostart through `HKCU\...\Run`;
//! - an entry under `HKCU\...\Uninstall`, so Settings → Apps lists winmon and
//!   can remove it (it runs `winmon.exe --uninstall`).
//!
//! The config (`%APPDATA%\winmon`) and log (`%LOCALAPPDATA%\winmon`) are
//! separate from the program folder; uninstall keeps them unless asked.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, anyhow};

use crate::platform::{self, registry, shell};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\winmon";
const VALUE: &str = "winmon";
/// How long to wait for a running dashboard to exit before replacing it.
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Start the dashboard at logon.
    pub autostart: bool,
    /// Add Start menu shortcuts.
    pub start_menu: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            autostart: true,
            start_menu: true,
        }
    }
}

fn env_dir(var: &str) -> anyhow::Result<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("%{var}% is not set"))
}

/// `%LOCALAPPDATA%\Programs\winmon`: where per-user apps install.
pub fn install_dir() -> anyhow::Result<PathBuf> {
    Ok(env_dir("LOCALAPPDATA")?.join("Programs").join("winmon"))
}

pub fn installed_exe() -> anyhow::Result<PathBuf> {
    Ok(install_dir()?.join("winmon.exe"))
}

fn start_menu_dir() -> anyhow::Result<PathBuf> {
    Ok(env_dir("APPDATA")?.join(r"Microsoft\Windows\Start Menu\Programs"))
}

fn shortcuts() -> anyhow::Result<[PathBuf; 2]> {
    let dir = start_menu_dir()?;
    Ok([dir.join("winmon.lnk"), dir.join("winmon settings.lnk")])
}

/// The installed version, if winmon is installed.
pub fn installed_version() -> Option<String> {
    registry::get_string(UNINSTALL_KEY, "DisplayVersion")
}

/// Whether the running binary is the installed one.
pub fn running_installed() -> bool {
    let (Ok(me), Ok(installed)) = (std::env::current_exe(), installed_exe()) else {
        return false;
    };
    same_file(&me, &installed)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Whether winmon starts at logon.
pub fn autostart_enabled() -> bool {
    registry::get_string(RUN_KEY, VALUE).is_some()
}

/// Turns autostart on (running `exe`) or off.
pub fn set_autostart(on: bool, exe: &Path) -> anyhow::Result<()> {
    if on {
        registry::set_string(RUN_KEY, VALUE, &format!("\"{}\"", exe.display()))
    } else {
        registry::delete_value(RUN_KEY, VALUE)
    }
    .context("autostart registry value")
}

/// Installs (or updates) the running binary. Returns the installed exe.
pub fn install(options: Options) -> anyhow::Result<PathBuf> {
    let me = std::env::current_exe().context("locate winmon.exe")?;
    let dir = install_dir()?;
    let exe = installed_exe()?;

    if !same_file(&me, &exe) {
        // A running dashboard locks its exe; stop it before replacing it.
        if !platform::stop_running(STOP_TIMEOUT) {
            return Err(anyhow!(
                "winmon is running and didn't exit; close it and retry"
            ));
        }
        std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        std::fs::copy(&me, &exe).with_context(|| format!("copy to {}", exe.display()))?;
    }

    let [app_lnk, settings_lnk] = shortcuts()?;
    if options.start_menu {
        shell::create_shortcut(&app_lnk, &exe, "", "System dashboard for a small monitor")
            .context("Start menu shortcut")?;
        shell::create_shortcut(&settings_lnk, &exe, "--settings", "Configure winmon")
            .context("Start menu shortcut")?;
    } else {
        remove_files(&[app_lnk, settings_lnk]);
    }
    set_autostart(options.autostart, &exe)?;
    register_uninstall(&dir, &exe).context("Apps & features entry")?;
    Ok(exe)
}

fn register_uninstall(dir: &Path, exe: &Path) -> std::io::Result<()> {
    let k = UNINSTALL_KEY;
    let exe_q = format!("\"{}\"", exe.display());
    let size_kb = std::fs::metadata(exe).map_or(0, |m| m.len().div_ceil(1024)) as u32;
    registry::set_string(k, "DisplayName", "winmon")?;
    registry::set_string(k, "DisplayVersion", env!("CARGO_PKG_VERSION"))?;
    registry::set_string(k, "DisplayIcon", &exe.display().to_string())?;
    registry::set_string(k, "InstallLocation", &dir.display().to_string())?;
    registry::set_string(k, "UninstallString", &format!("{exe_q} --uninstall"))?;
    registry::set_string(
        k,
        "QuietUninstallString",
        &format!("{exe_q} --uninstall --quiet"),
    )?;
    registry::set_string(k, "Comments", env!("CARGO_PKG_DESCRIPTION"))?;
    registry::set_dword(k, "EstimatedSize", size_kb)?;
    registry::set_dword(k, "NoModify", 1)?;
    registry::set_dword(k, "NoRepair", 1)
}

fn remove_files(paths: &[PathBuf]) {
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}

/// Removes winmon: stops the dashboard, then deletes the shortcuts, the
/// registry entries and the program folder. With `remove_settings`, the
/// config and log folders go too.
pub fn uninstall(remove_settings: bool) -> anyhow::Result<()> {
    platform::stop_running(STOP_TIMEOUT);
    registry::delete_value(RUN_KEY, VALUE).context("autostart registry value")?;
    remove_files(&shortcuts()?);
    registry::delete_key(UNINSTALL_KEY).context("Apps & features entry")?;

    if remove_settings {
        for (var, sub) in [("APPDATA", "winmon"), ("LOCALAPPDATA", "winmon")] {
            if let Ok(d) = env_dir(var) {
                let _ = std::fs::remove_dir_all(d.join(sub));
            }
        }
    }

    let dir = install_dir()?;
    if !dir.exists() {
        return Ok(());
    }
    if running_installed() {
        // A running exe can't delete itself: let a hidden shell do it once
        // this process has exited.
        delete_after_exit(&dir)
    } else {
        std::fs::remove_dir_all(&dir).with_context(|| format!("delete {}", dir.display()))
    }
}

fn delete_after_exit(dir: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // `ping` is the classic console-free sleep; ~2 s is plenty for us to exit.
    std::process::Command::new("cmd")
        .raw_arg(format!(
            "/d /c ping -n 3 127.0.0.1 >nul & rmdir /s /q \"{}\"",
            dir.display()
        ))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("schedule program folder removal")?;
    Ok(())
}

/// Starts `exe` with `args` as a detached process.
pub fn launch(exe: &Path, args: &[&str]) -> anyhow::Result<()> {
    std::process::Command::new(exe)
        .args(args)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .spawn()
        .with_context(|| format!("start {}", exe.display()))?;
    Ok(())
}
