# winmon

A tiny, always-on system dashboard for a small secondary monitor on Windows 11:
CPU and memory history, temperatures, clock, and weather, drawn fullscreen with
[win32ui](https://github.com/va1erian/win32ui) and Direct2D.

Status: working prototype (PLAN.md phases 0–5, plus parts of 6–7). See
[PLAN.md](PLAN.md) for the architecture and the remaining phases.

## Build and run

```
cargo build --release
target\release\winmon.exe               # fullscreen on the small monitor
target\release\winmon.exe --windowed    # 480x800 window, for development
target\release\winmon.exe --list-monitors
target\release\winmon.exe --quit        # stop a running instance
```

With no config, winmon picks the smallest non-primary monitor. If only the
primary is attached it stays windowed, and it moves back to the panel when it
is plugged in again. The fullscreen window is a tool window: it is not in the
taskbar or Alt-Tab, and clicking it doesn't take the focus.

Only one instance runs at a time. `--install` / `--uninstall` add or remove a
per-user logon entry (`HKCU\...\Run`, no elevation needed).

## Config

Copy [winmon.example.toml](winmon.example.toml) to
`%APPDATA%\winmon\winmon.toml` (or pass `--config <path>`). Every key is
optional. Set `[weather] latitude`/`longitude`; the default is Paris.

## Temperatures: LibreHardwareMonitor setup

Windows has no user-mode API for CPU temperatures (PLAN.md §6), so winmon reads
them from [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor):

1. Install LibreHardwareMonitor (latest release).
2. Options → enable *Run On Windows Startup*, *Start Minimized*,
   *Minimize To Tray*, and *Remote Web Server* → Run (default port 8085).
3. Restrict the web server to localhost if your version allows it, or block
   port 8085 inbound in Windows Firewall.
4. Run `winmon --list-sensors`. It prints every temperature sensor and the ids
   winmon picks by default. Copy other ids into `cpu_sensor` / `gpu_sensor` if
   the guesses are wrong.

Without LibreHardwareMonitor the tiles show "—" and a red status dot; they
recover on their own once it runs. Build with `--features nvml` to read an
NVIDIA GPU's temperature directly.

## Development

```
cargo test
cargo clippy --all-targets -- -D warnings
```

To check rendering without looking at the panel, build with the `snapshot`
feature and set `WINMON_SNAPSHOT=out.bmp` (optionally `WINMON_SNAPSHOT_TICKS=8`).
winmon then captures its window after that many ticks and exits.
