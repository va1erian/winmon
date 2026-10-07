# winmon

A tiny, always-on system dashboard for a small secondary monitor on Windows 11:
CPU and memory history, temperatures, clock, and weather, drawn fullscreen with
[win32ui](https://github.com/va1erian/win32ui) and Direct2D.

Status: PLAN.md phases 0–7, plus the installer, the settings dialog and
config hot reload. See [PLAN.md](PLAN.md) for the architecture and what's left.

## Install

winmon is its own installer. Run `winmon-setup.exe` (built by
`scripts\package.ps1`, see *Development*) and choose the options:

- it installs for your account only, into `%LOCALAPPDATA%\Programs\winmon`,
  without administrator rights;
- Start menu shortcuts **winmon** and **winmon settings**;
- optionally, start at sign-in (`HKCU\...\Run`);
- an entry in Settings → Apps, which uninstalls it (`winmon --uninstall`).

Running the setup again updates the installed copy, stopping the dashboard
first. Uninstalling keeps your settings unless you tick *Also delete my
settings and log*. For scripts: `winmon --install --quiet` and
`winmon --uninstall --quiet`.

## Settings

**winmon settings** in the Start menu (or `winmon --settings`) edits the config:
the target monitor, the clock format, sign-in start, the temperature sensors
(*Detect sensors* asks LibreHardwareMonitor for the list), the weather
location (search for a city by name), the units, and the refresh intervals.

A running dashboard watches its config file. When the file changes, saved by
the dialog or by a text editor, the dashboard restarts with the new settings
within about a second. A file that doesn't parse is logged and ignored.

## Run from a build

```
cargo build --release
target\release\winmon.exe               # fullscreen on the small monitor
target\release\winmon.exe --windowed    # 480x800 window, for development
target\release\winmon.exe --settings    # the settings dialog
target\release\winmon.exe --list-monitors
target\release\winmon.exe --quit        # stop a running instance
```

With no config, winmon picks the smallest non-primary monitor. If only the
primary is attached it stays windowed, and it moves back to the panel when it
is plugged in again. The fullscreen window is a tool window: it is not in the
taskbar or Alt-Tab, and clicking it doesn't take the focus.

Only one instance runs at a time.

## Config file

The settings dialog writes `%APPDATA%\winmon\winmon.toml`, or the file given
with `--config <path>`. [winmon.example.toml](winmon.example.toml) documents
every key, and all of them are optional. The weather location defaults to Paris.
Saving from the dialog rewrites the file, so comments added by hand are lost.

## Temperatures: LibreHardwareMonitor setup

Windows has no user-mode API for CPU temperatures (PLAN.md §6), so winmon reads
them from [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor):

1. Install LibreHardwareMonitor (latest release).
2. Options → enable *Run On Windows Startup*, *Start Minimized*,
   *Minimize To Tray*, and *Remote Web Server* → Run (default port 8085).
3. Restrict the web server to localhost if your version allows it, or block
   port 8085 inbound in Windows Firewall.
4. In winmon settings → Temperatures, click *Detect sensors* and pick the CPU
   and GPU sensors if the automatic guesses are wrong. `winmon --list-sensors`
   prints the same list in a console.

Without LibreHardwareMonitor the tiles show "—" and a red status dot; they
recover on their own once it runs. Build with `--features nvml` to read an
NVIDIA GPU's temperature directly.

## Development

```
cargo test
cargo clippy --all-targets -- -D warnings
powershell -ExecutionPolicy Bypass -File scripts\package.ps1   # dist\winmon-setup.exe
```

The package is the release exe under another name: started with no arguments
under a name that contains "setup", winmon opens the setup window.
`build.rs` embeds the version resource and the manifest: Common Controls v6,
per-monitor DPI, and `asInvoker`, which stops Windows from auto-elevating an
exe named `*setup*`.

### Releasing

Bump `version` in `Cargo.toml`, merge, then publish a GitHub release with
the tag `v<version>` (e.g. `v0.2.0`). The [release workflow](.github/workflows/release.yml)
runs clippy and the tests on Windows, builds `winmon-setup-v0.2.0.exe`, and
attaches it and its `.sha256` to the release. If the tag doesn't match
`Cargo.toml`, the workflow fails without attaching anything. Tags that don't
start with `v` are ignored.

To check rendering without looking at the panel, set `WINMON_SNAPSHOT=out.bmp` (optionally `WINMON_SNAPSHOT_TICKS=8`).
winmon then captures its window after that many ticks and exits. This also
works for `--settings` (`WINMON_SNAPSHOT_TAB=0..3` picks the tab), `--install`
and `--uninstall`.
