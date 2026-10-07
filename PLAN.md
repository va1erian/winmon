# winmon — implementation plan

A tiny, always-on system dashboard for a small secondary monitor (e.g. a 4"
480×800 USB/HDMI panel), built on [win32ui](https://github.com/va1erian/win32ui).

The plan is written to be executed phase by phase, by hand or by a coding agent
running locally on Windows 11. Every phase ends in something that builds, runs,
and can be checked against its acceptance criteria before moving on.

---

## 1. Goals and non-goals

**Goals**

- Runs on Windows 11, borderless fullscreen on a monitor chosen in config.
- Shows CPU and memory usage as a history graph plus an instantaneous bar.
- Shows temperatures (CPU, GPU at minimum).
- Shows the time and date.
- Shows the weather forecast for the next few days.
- Uses minimal resources: target under 15 MB working set and roughly 0% CPU at
  idle (1 redraw per second).

**Non-goals (v1)**

- No in-app settings UI: configuration is a TOML file.
- No kernel driver of our own: temperatures come from existing sources (§6).
- No themes or layout editor: one layout tuned for portrait 480×800, scaled
  proportionally for other sizes.
- No touch interaction.

---

## 2. Architecture

```
            ┌──────────────── UI thread (win32ui) ─────────────────┐
            │                                                      │
 1 Hz timer ──► Msg::Tick ──► sampler.sample()   (CPU, mem: ~µs)   │
            │                    │                                 │
            │                    ▼                                 │
            │               state: Model ──► Dashboard.invalidate()│
            │                    ▲                                 │
            └────────────────────┼─────────────────────────────────┘
                                 │ Proxy::send(Msg::…)
       ┌─────────────────────────┴──────────────────────────┐
       │                                                    │
 temps worker (every 2 s)                     weather worker (every 30 min)
 LHM JSON / NVML / WMI                        Open-Meteo HTTPS
```

- **One window, one widget.** The whole dashboard is a single `CustomWidget`
  with `Renderer::Direct2D` that fills the window. There are no native controls.
- **The UI thread does only cheap work.** CPU and memory sampling are single
  syscalls, so they run inline on the tick. Anything that can block (HTTP,
  WMI, LHM) runs on worker threads and reports back through `Proxy::send`.
- **The model is plain data.** `Model` owns ring buffers and the latest
  readings. The widget holds an `Rc<RefCell<Model>>` (or receives a snapshot) and
  only reads it in `paint_d2d`.

### Crate layout

```
winmon/
├── Cargo.toml
├── build.rs                 # app manifest (DPI awareness, icon) via embed-resource
├── winmon.example.toml
├── src/
│   ├── main.rs              # arg parsing, config load, run_app
│   ├── app.rs               # App impl, Msg enum, timer wiring, monitor selection
│   ├── config.rs            # serde config + defaults
│   ├── model.rs             # Model, RingBuffer<f32>, reading types
│   ├── sampler/
│   │   ├── mod.rs
│   │   ├── cpu.rs           # GetSystemTimes deltas (+ optional per-core)
│   │   └── memory.rs        # GlobalMemoryStatusEx
│   ├── temps/
│   │   ├── mod.rs           # TempSource trait, worker thread
│   │   ├── lhm.rs           # LibreHardwareMonitor web-server JSON
│   │   ├── nvml.rs          # NVIDIA GPU (feature "nvml")
│   │   └── wmi_acpi.rs      # MSAcpi_ThermalZoneTemperature fallback (optional)
│   ├── weather/
│   │   ├── mod.rs           # worker thread, retry/backoff
│   │   ├── openmeteo.rs     # request + response types
│   │   └── icons.rs         # WMO code → icon drawn with PathBuilder
│   ├── ui/
│   │   ├── mod.rs           # Dashboard: CustomWidget
│   │   ├── layout.rs        # pure fn: bounds → section rects (unit-tested)
│   │   ├── graph.rs         # history graph + bar
│   │   ├── clock.rs
│   │   ├── temps.rs
│   │   ├── weather.rs
│   │   └── palette.rs
│   └── platform.rs          # tool-window styles, EcoQoS, coalescable timer
└── tests/
    ├── ring_buffer.rs
    ├── layout.rs
    ├── openmeteo_parse.rs   # against a checked-in JSON fixture
    └── lhm_parse.rs         # against a checked-in JSON fixture
```

### Dependencies

| Crate | Why |
|---|---|
| `win32ui` (git) | window, fullscreen, monitors, timers, Direct2D, `Proxy` |
| `windows` 0.62 | the extra Win32 namespaces win32ui doesn't enable (see below) |
| `serde`, `toml` | config |
| `serde_json` | Open-Meteo and LHM payloads |
| `ureq` (rustls) | blocking HTTPS client for the two workers |
| `nvml-wrapper` | optional, behind feature `nvml` |
| `anyhow` | error plumbing in the workers |

`windows` features to add on top of what win32ui pulls in:
`Win32_System_SystemInformation` (memory), `Win32_System_Threading`
(`GetSystemTimes`, EcoQoS), `Win32_UI_WindowsAndMessaging` (extended styles,
already present in win32ui but needed by our own direct calls).

Alternative for the HTTP client: WinHTTP through `windows`
(`Win32_Networking_WinHttp`) to avoid `ureq` and rustls. It saves around 1 MB of
binary but costs about 150 lines of boilerplate. Start with `ureq`; revisit in
Phase 8 if the binary size matters.

---

## 3. Config

`%APPDATA%\winmon\winmon.toml`, overridable with `--config <path>`. Every key
has a default so an empty file works.

```toml
[display]
# Match by friendly name (substring, case-insensitive) or exact device name.
# If unset or not found: the smallest non-primary monitor, else the primary.
monitor = "USB Display"
# monitor_device = "\\\\.\\DISPLAY3"

[refresh]
tick_ms = 1000          # redraw + CPU/mem sample interval
history_seconds = 120   # graph width in samples
temps_ms = 2000
weather_minutes = 30

[temps]
source = "lhm"          # "lhm" | "wmi" | "none"
lhm_url = "http://127.0.0.1:8085/data.json"
# Sensor ids as printed by `winmon --list-sensors`
cpu_sensor = "/amdcpu/0/temperature/2"
gpu_sensor = "/gpu-nvidia/0/temperature/0"
nvml = true             # read NVIDIA GPU temp directly, overrides gpu_sensor

[weather]
latitude = 0.0
longitude = 0.0
days = 5
units = "metric"        # "metric" | "imperial"

[clock]
format_24h = true
show_seconds = false
```

**CLI**

- `winmon`: run.
- `winmon --list-monitors`: print `device_name`, `friendly_name`, rect,
  primary, dpi for each monitor, then exit.
- `winmon --list-sensors`: fetch the LHM JSON and print every temperature
  sensor with its id and current value, then exit.
- `winmon --windowed`: run in a normal 480×800 window on the primary monitor,
  for development.

---

## 4. Phases

Each phase: **Do**, then **Done when**. Commit at the end of each phase.

### Phase 0 — Skeleton

**Do**

- `cargo new winmon`, edition 2024. Add `win32ui = { git = "https://github.com/va1erian/win32ui" }`.
- Copy win32ui's `build.rs` approach (embed-resource) for a manifest that
  declares per-monitor-v2 DPI awareness and Common Controls v6, if win32ui
  doesn't already provide it to dependents.
- `main.rs`: `run_app(WindowSpec::new("winmon").size(dip(480.0), dip(800.0)), |ui| …)`
  with an empty `Dashboard` custom widget that clears to black.
- Release profile: `opt-level = "s"`, `lto = true`, `codegen-units = 1`,
  `panic = "abort"`, `strip = true`.
- Set the `windows_subsystem = "windows"` attribute only for release builds, so
  debug builds keep a console for logs.

**Done when** `cargo run` opens a black 480×800 window, and `cargo build --release`
produces an executable. Record its size.

### Phase 1 — Monitor selection and fullscreen

**Do**

- `config.rs`: load TOML with defaults; `--config`, `--windowed`.
- `--list-monitors` using `win32ui::monitors()`.
- Selection order: `monitor_device` exact match, then `monitor` substring of
  `friendly_name`, then the smallest-area non-primary monitor, then the primary.
- After the window is built: `ui.enter_fullscreen(&monitor)` (skip with
  `--windowed`). Call `ui.hide_cursor_when_idle(2000)`.
- `ui.on_display_change(|| Some(Msg::DisplayChanged))`. On that message,
  re-run the selection and call `enter_fullscreen` again. This handles hotplug,
  resolution changes, and resume from sleep.
- `platform.rs`, called once after the window is created:
  - Add `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` and remove `WS_EX_APPWINDOW`
    with `SetWindowLongPtrW(GWL_EXSTYLE)` on `HWND(ui.hwnd().raw() as _)`.
    The window then stays out of the taskbar and Alt-Tab and never takes focus.
  - Call `SetWindowPos(..., SWP_FRAMECHANGED | SWP_NOACTIVATE)` afterwards.
  - Later, upstream this as a `WindowSpec` option in win32ui and delete the
    raw call.
- Handle exit: `Escape` while focused, and a `--quit`-style tray-free path
  (Task Manager is acceptable for v1). Unplugging the monitor must **not**
  crash: fall back to windowed on the primary until it returns.

**Done when**

- `--list-monitors` lists the small screen.
- With the screen plugged in, the window covers it entirely and doesn't
  appear in the taskbar or Alt-Tab.
- Clicking the panel doesn't steal focus from the app you were using.
- Unplugging and replugging the screen moves the window off and back on
  without a restart.

### Phase 2 — Model and samplers (CPU, memory)

**Do**

- `model.rs`: `RingBuffer<f32>` with fixed capacity (`history_seconds`),
  `push`, `iter_oldest_first`, `len`, `latest`. No allocation after
  construction.
- `sampler/cpu.rs`: keep the previous `(idle, kernel, user)` from
  `GetSystemTimes`.
  `usage = 1 − Δidle / (Δkernel + Δuser)` (kernel time includes idle). Clamp
  to 0..=1. The first sample returns `None`.
- `sampler/memory.rs`: `GlobalMemoryStatusEx` → used = total − avail, plus
  percent. Also keep the absolute GB values for the label.
- `app.rs`: `ui.set_timer(tick_ms)` and `ui.on_timer(|_| Some(Msg::Tick))`.
  On `Tick`: sample, push into the ring buffers, then `dashboard.invalidate()`.
- Optional (stretch): per-core usage through PDH
  `\Processor Information(*)\% Processor Utility`
  (`Win32_System_Performance`). It matches Task Manager better than
  `GetSystemTimes` on CPUs with turbo boost.

**Done when**

- Unit tests pass for `RingBuffer` (wraparound, order, capacity 1).
- In debug, log lines show CPU % tracking Task Manager within ~5 points
  under a load (e.g. a `cargo build`).

### Phase 3 — Dashboard rendering

**Do**

- `ui/layout.rs`: a pure function `fn sections(bounds: RectF) -> Sections`
  returning rects for clock, cpu, mem, temps, weather. Portrait layout
  (top to bottom):
  1. clock + date: ~15%
  2. CPU graph + bar: ~22%
  3. memory graph + bar: ~22%
  4. temps row: ~11%
  5. weather, N day columns: ~30%

  If `width > height`, switch to a 2-column landscape layout. Unit-test both
  orientations and that sections never overlap or leave the bounds.
- `ui/palette.rs`: dark background (OLED/LCD-friendly, low glow), one accent
  per metric, a dim grid color, and text colors. Use the `Color::hex` consts.
- `ui/graph.rs`:
  - Graph area: build one `PathBuilder` path from the ring buffer (x evenly
    spaced, y = value × height). Stroke it at 2 DIP. Close a second path down
    to the baseline and fill it with the accent at ~25% alpha.
  - Draw horizontal grid lines at 25/50/75%.
  - Bar: rounded track plus a filled portion, with the % text right-aligned in
    the bar.
  - Build paths per paint; at 120 points this is negligible. Only cache if the
    profiler says so.
- `ui/clock.rs`: `HH:MM` in a large font (≈ 72 DIP at 480 width), date below.
  Create fonts once through `TextSystem::new()` + `system.font(&FontSpec::new("Segoe UI Variable Display", size))`
  and store them in the widget. Cache the `Layout` for the clock string and
  rebuild it only when the text changes.
- Scale every size from a single `unit = bounds.width / 480.0` so other panel
  sizes work.

**Done when**

- In `--windowed` mode, the dashboard shows a live clock and moving CPU/memory
  graphs and bars.
- No layout overlaps at 480×800, 800×480, 320×480, and 1024×600 (the layout
  tests cover this).
- Task Manager shows winmon at about 0% CPU.

### Phase 4 — Temperatures

**Do**

- `temps/mod.rs`:

  ```rust
  pub struct TempReading { pub label: String, pub celsius: f32 }
  pub trait TempSource: Send {
      fn read(&mut self) -> anyhow::Result<Vec<TempReading>>;
  }
  ```

  A worker thread loops: read every `temps_ms`, then
  `proxy.send(Msg::Temps(readings))`. Errors are sent as `Msg::TempsError(String)`
  and shown as "—" plus a small status dot rather than crashing. The worker
  exits when `send` returns `Err` (the window is gone).
- `temps/lhm.rs`: GET `lhm_url`, then walk the JSON tree
  (`Children` arrays; nodes with `SensorId`, `Type`, `Value`).
  Match configured sensor ids. Parse `"45.0 °C"`-style values defensively,
  without depending on the locale's decimal separator. Keep one `ureq::Agent`
  for the connection pool.
  `--list-sensors` reuses this walker.
- `temps/nvml.rs` (feature `nvml`): `Nvml::init()` once, then device 0
  `temperature(TemperatureSensor::Gpu)`. If init fails (no NVIDIA GPU or
  driver), log once and disable.
- `temps/wmi_acpi.rs` (optional): `MSAcpi_ThermalZoneTemperature` through
  WMI. Needs admin and is often wrong on desktops; document it as a last
  resort.
- Render: one tile per reading with the label and value. Color the value on a
  gradient (accent → orange → red at configurable thresholds, defaults 60/80 °C).
  Optionally add a 60-sample sparkline per temperature.

**Setup (document in the README)**

1. Install LibreHardwareMonitor (latest release).
2. Options → enable *Run On Windows Startup*, *Start Minimized*,
   *Minimize To Tray*, and *Remote Web Server* → Run (default port 8085).
3. Restrict the web server to localhost if your version allows it, or block
   port 8085 inbound in Windows Firewall.
4. `winmon --list-sensors`, then copy the ids into the config.

**Done when**

- `--list-sensors` prints CPU package and GPU temperatures.
- The dashboard shows them, updating every 2 s.
- Stopping LibreHardwareMonitor shows "—" within one interval. Restarting it
  recovers without restarting winmon.

### Phase 5 — Weather

**Do**

- `weather/openmeteo.rs`: GET

  ```
  https://api.open-meteo.com/v1/forecast
    ?latitude={lat}&longitude={lon}
    &current=temperature_2m,weather_code
    &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max
    &timezone=auto&forecast_days={days}
  ```

  Add `&temperature_unit=fahrenheit` for imperial. Deserialize into
  typed structs. Check a captured response into `tests/fixtures/` and
  unit-test the parse.
- `weather/mod.rs`: the worker fetches on start and then every
  `weather_minutes`. On failure, retry with backoff (1, 2, 5, 10 min, capped
  at the normal interval). Keep the last good forecast and show its age when
  it's older than 2 intervals.
- `weather/icons.rs`: map WMO codes to ~8 icon kinds (clear, partly cloudy,
  cloudy, fog, drizzle, rain, snow, thunder). Draw them with `PathBuilder`:
  circles, arcs, and lines in the palette colors. No image assets, no
  emoji fonts. Each icon is a function `fn draw(canvas, rect, kind)`.
- Render: current temperature + icon, then N columns with the day name
  (localized with `GetDateFormatEx`, or a simple English/French table),
  icon, max/min, and rain %.

**Done when**

- The weather section shows today plus the next 4 days.
- With the network disabled, the last forecast stays visible with an age
  label, and the app reports no errors other than the age.
- Fixture parse test passes.

### Phase 6 — Minimal resource pass

**Do**

- EcoQoS on startup: `SetProcessInformation(GetCurrentProcess(), ProcessPowerThrottling, …)`
  with `ControlMask = StateMask = PROCESS_POWER_THROTTLING_EXECUTION_SPEED`.
  Also `SetPriorityClass(BELOW_NORMAL_PRIORITY_CLASS)`.
- Replace the 1 s timer with `SetCoalescableTimer` (tolerance ~100 ms) through
  a raw call on the window's HWND, or add `set_coalescable_timer` to win32ui.
  `WM_TIMER` delivery stays the same, so `on_timer` keeps working.
- If `show_seconds = false`, the clock only needs to change once a minute, but
  the graphs still tick at 1 Hz. Keep 1 Hz as the single cadence.
- Workers: make sure they sleep with no busy loops, use one `ureq::Agent`
  each, and that the threads use a small stack (`std::thread::Builder::stack_size(256 * 1024)`).
- Check `paint_d2d` allocations: fonts and text layouts are cached, and paths
  are rebuilt per frame (fine) but don't grow unbounded.
- Measure with Task Manager, then optionally with
  `wpr -start CPU` / Windows Performance Analyzer over 60 s.

**Done when**, measured over 10 minutes on the target machine:

- Working set under 15 MB (stretch: under 10 MB).
- Average CPU under 0.1%.
- No growth in handles or GDI objects (Task Manager → Details → add the
  columns).
- Release binary size recorded; under 3 MB with `ureq`.

### Phase 7 — Autostart and robustness

**Do**

- Autostart with Task Scheduler: an `--install` command that writes a task
  (trigger: at logon of the current user; *Run only when user is logged on*;
  no elevation needed for winmon itself). Use `schtasks /Create /XML` with an
  embedded template, and `--uninstall` to remove it.
- Single instance: a named mutex `Local\winmon`. A second launch exits
  immediately.
- Session events: after resume from sleep, re-fetch weather right away and
  reset the CPU sampler's previous values (the first delta after a long sleep
  is meaningless). Use `on_display_change` plus a
  `WM_POWERBROADCAST`/`PBT_APMRESUMEAUTOMATIC` hook through `ui.on_raw_message`.
- Logging: to `%LOCALAPPDATA%\winmon\winmon.log`, rotated at 1 MB, errors
  only in release.
- Panics: `panic = "abort"` plus a panic hook that writes to the log first.

**Done when**

- After a reboot, winmon appears on the small screen without user action.
- Launching it twice leaves one instance.
- Sleep/resume and a 24-hour soak run cause no drift, leaks, or frozen data.

### Phase 8 — Polish (optional)

- Upstream to win32ui: a `WindowSpec` option for tool/no-activate windows
  and a coalescable timer, then remove the raw calls from `platform.rs`.
- Burn-in protection for OLED panels: shift the whole drawing by ±2 px every
  few minutes.
- Night dimming: lower the brightness of the palette between configurable
  hours.
- Config hot reload: watch the TOML file (`ReadDirectoryChangesW`) and apply
  the changes.
- Swap `ureq` for WinHTTP if the binary size matters.
- Extra tiles: network up/down (`GetIfTable2` deltas), disk usage, GPU load
  from NVML.

---

## 5. Risks and mitigations

| Risk | Mitigation |
|---|---|
| No CPU temperature without a kernel driver | Use LibreHardwareMonitor as the sensor service (§6); show "—" when it's absent; GPU through NVML works without it. |
| LHM JSON format changes between versions | Parse defensively (ids, types, and values as strings); fixture tests; `--list-sensors` for quick checks. |
| Small screen reported with a generic or empty friendly name | Fall back to `monitor_device` or "smallest non-primary"; `--list-monitors` to find the name. |
| Panel's native orientation is portrait but Windows shows it landscape | Rotate it in Windows display settings; the layout also handles landscape. |
| `enter_fullscreen` is topmost and could cover a dialog on that screen | The panel is dedicated; acceptable. The tool-window style keeps it out of Alt-Tab. |
| Open-Meteo rate limits or outages | 30-minute interval is far below the free tier limit; keep the last good forecast. |
| win32ui API changes (git dependency) | Pin to a commit `rev` in `Cargo.toml`; bump deliberately. |

---

## 6. Why LibreHardwareMonitor for temperatures

Windows has no user-mode API for CPU temperatures; the sensors are read through
model-specific registers and the SMBus, which need a signed kernel driver. The
alternatives considered:

- **HWiNFO shared memory**: the free version requires re-enabling shared memory
  every 12 hours, which doesn't fit an always-on display.
- **WMI `MSAcpi_ThermalZoneTemperature`**: needs admin, and on most desktop
  boards the value is static, fake, or missing.
- **Our own driver**: requires EV-signed driver submission to Microsoft. Out of
  scope.
- **LibreHardwareMonitor**: open source, maintained, handles the driver, and
  exposes all sensors as JSON on localhost. winmon stays a small user-mode
  process.

GPU temperatures don't need any of this: NVML (NVIDIA) works from user mode.
ADLX for AMD GPUs could be added the same way later.

---

## 7. Working with a coding agent

To execute with an agent locally, give it one phase at a time:

> Read PLAN.md. Implement Phase N only. Run `cargo build`, `cargo test`, and
> `cargo clippy -- -D warnings`. Then show me what to run to check the phase's
> "Done when" criteria, and stop.

Review and commit after each phase before starting the next one. The win32ui
source is the reference for its API: the `examples/demo/app/` widgets
(`document.rs`, `grid.rs`, `text_specimen.rs`) show `CustomWidget::paint_d2d`,
`TextSystem`, and `FontSpec` usage.
