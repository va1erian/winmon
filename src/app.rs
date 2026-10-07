//! The win32ui app: messages, timer, workers, monitor placement.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use win32ui::prelude::*;
use win32ui::{Custom, MonitorInfo, Proxy, column};

use crate::config::Config;
use crate::model::Model;
use crate::sampler::{self, cpu::CpuSampler, memory};
use crate::snapshot::Snapshot;
use crate::temps::{self, TempReading};
use crate::ui::{Dashboard, Event, Options};
use crate::weather::{self, Forecast, WeatherWorker};
use crate::{log, monitor, platform};

/// How late the 1 Hz tick may fire so Windows can batch it with other timers.
const TICK_TOLERANCE_MS: u32 = 100;

pub enum Msg {
    Tick,
    Temps(std::result::Result<Vec<TempReading>, String>),
    Weather(std::result::Result<Forecast, String>),
    DisplayChanged,
    Resumed,
    Quit,
}

/// Why the dashboard's loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Quit,
    /// The config file changed: start again with the new settings.
    Restart,
}

/// Watches the config file's modification time (one stat per tick).
struct ConfigWatch {
    path: PathBuf,
    modified: Option<SystemTime>,
    /// The running config, to ignore saves that change nothing.
    current: String,
}

impl ConfigWatch {
    fn new(path: PathBuf, config: &Config) -> Self {
        ConfigWatch {
            modified: modified(&path),
            path,
            current: config.to_toml(),
        }
    }

    /// Whether the file now holds a different, valid config.
    fn changed(&mut self) -> bool {
        let now = modified(&self.path);
        if now == self.modified {
            return false;
        }
        self.modified = now;
        if now.is_none() {
            return false; // deleted: keep running as is
        }
        match Config::load(Some(&self.path)) {
            Ok(c) => c.to_toml() != self.current,
            Err(e) => {
                log::error(&format!("config reload: {e:#}"));
                false
            }
        }
    }
}

fn modified(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

pub struct WinMon {
    dashboard: Custom<Dashboard, Msg>,
    cpu: CpuSampler,
    config: Config,
    windowed: bool,
    /// Device name of the monitor we're fullscreen on, if any.
    placed_on: Option<String>,
    weather: Option<WeatherWorker>,
    snapshot: Option<Snapshot>,
    watch: Option<ConfigWatch>,
    exit: Rc<Cell<Exit>>,
}

/// Runs the dashboard until it quits. `config_path` is watched: when the
/// file changes to a valid config, this returns [`Exit::Restart`].
pub fn run(config: Config, config_path: Option<PathBuf>, windowed: bool) -> win32ui::Result<Exit> {
    // Fullscreen, the window stays out of the taskbar and Alt-Tab and clicks
    // never take the focus; windowed (development) it is a normal window.
    let spec = WindowSpec::new("winmon")
        .size(dip(480.0), dip(800.0))
        .theme(Theme::dark())
        .tool_window(!windowed)
        .no_activate(!windowed);
    let exit = Rc::new(Cell::new(Exit::Quit));
    let app_exit = exit.clone();
    run_app(spec, move |ui| {
        let watch = config_path.map(|p| ConfigWatch::new(p, &config));
        WinMon::new(ui, config, windowed, watch, app_exit)
    })?;
    Ok(exit.get())
}

impl WinMon {
    fn new(
        ui: &mut Ui<Msg>,
        config: Config,
        windowed: bool,
        watch: Option<ConfigWatch>,
        exit: Rc<Cell<Exit>>,
    ) -> WinMon {
        let mut model = Model::new(config.refresh.history_seconds);
        model.now = sampler::local_time();
        let dashboard = Custom::new(ui, Dashboard::new(model, Options::from_config(&config)))
            .expect("dashboard widget")
            .on_event(|e| match e {
                Event::Quit => Some(Msg::Quit),
            });
        ui.set_layout(column![dashboard.fill(1)]);

        // One 1 Hz cadence drives sampling and redraw. It needn't be exact, so
        // let Windows coalesce it with other timers to save wake-ups.
        match ui.set_coalescable_timer(config.refresh.tick_ms, TICK_TOLERANCE_MS) {
            Ok(tick) => ui.on_timer(move |id| (id == tick).then_some(Msg::Tick)),
            Err(e) => log::error(&format!("set_coalescable_timer: {e}")),
        }
        ui.on_display_change(|| Some(Msg::DisplayChanged));
        let proxy = ui.proxy();
        {
            let proxy = proxy.clone();
            ui.on_raw_message(move |msg| {
                if platform::is_resume(msg) {
                    let _ = proxy.send(Msg::Resumed);
                }
                false
            });
        }
        {
            let proxy = proxy.clone();
            platform::listen_for_quit(move || {
                let _ = proxy.send(Msg::Quit);
            });
        }

        if !windowed {
            let _ = ui.hide_cursor_when_idle(2000);
        }

        spawn_temps(&config, proxy.clone());
        let weather = config
            .weather
            .enabled
            .then(|| spawn_weather(&config, proxy));

        // Place the window once the loop runs and it has been shown.
        ui.emit(Msg::DisplayChanged);

        let mut app = WinMon {
            dashboard,
            cpu: CpuSampler::new(),
            config,
            windowed,
            placed_on: None,
            weather,
            snapshot: Snapshot::from_env(),
            watch,
            exit,
        };
        app.cpu.sample(); // prime the delta
        app
    }

    fn tick(&mut self) {
        let cpu = self.cpu.sample();
        let mem = memory::sample();
        let widget = self.dashboard.widget();
        let mut d = widget.borrow_mut();
        let m = &mut d.model;
        if let Some(cpu) = cpu {
            m.cpu.push(cpu);
        }
        if let Some(mem) = mem {
            m.memory = mem;
            m.mem.push(mem.fraction());
        }
        m.now = sampler::local_time();
    }

    fn take_snapshot(&mut self, ui: &mut Ui<Msg>) {
        if let Some(snap) = self.snapshot.take() {
            snap.capture(ui);
        }
    }

    fn place(&mut self, ui: &mut Ui<Msg>) {
        if self.windowed {
            return;
        }
        let monitors = win32ui::monitors();
        match monitor::select(&monitors, &self.config.display) {
            Some(target) => self.enter(ui, target),
            None => {
                if ui.is_fullscreen() {
                    log::info("target monitor gone: falling back to windowed");
                    let _ = ui.leave_fullscreen();
                }
                self.placed_on = None;
            }
        }
    }

    fn enter(&mut self, ui: &mut Ui<Msg>, target: &MonitorInfo) {
        log::info(&format!("fullscreen on {}", monitor::describe(target)));
        // Re-entering just moves/resizes, which also handles resolution changes.
        match ui.enter_fullscreen(target) {
            Ok(()) => self.placed_on = Some(target.device_name.clone()),
            Err(e) => log::error(&format!("enter_fullscreen: {e}")),
        }
    }
}

impl App for WinMon {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        match msg {
            Msg::Tick => {
                if self.watch.as_mut().is_some_and(ConfigWatch::changed) {
                    log::info("config changed: restarting");
                    self.exit.set(Exit::Restart);
                    ui.quit();
                    return;
                }
                self.tick();
                if self.snapshot.as_mut().is_some_and(Snapshot::tick) {
                    self.dashboard.invalidate();
                    self.take_snapshot(ui);
                    return;
                }
            }
            Msg::Temps(result) => {
                let widget = self.dashboard.widget();
                let mut d = widget.borrow_mut();
                match result {
                    Ok(readings) => {
                        d.model.temps = readings;
                        d.model.temps_error = None;
                    }
                    Err(e) => {
                        if d.model.temps_error.as_ref() != Some(&e) {
                            log::info(&format!("temps: {e}"));
                        }
                        d.model.temps_error = Some(e);
                    }
                }
            }
            Msg::Weather(result) => {
                let widget = self.dashboard.widget();
                let mut d = widget.borrow_mut();
                match result {
                    Ok(forecast) => {
                        d.model.forecast = Some(forecast);
                        d.model.forecast_at = Some(Instant::now());
                        d.model.weather_error = None;
                    }
                    Err(e) => {
                        log::info(&format!("weather: {e}"));
                        d.model.weather_error = Some(e);
                    }
                }
            }
            Msg::DisplayChanged => self.place(ui),
            Msg::Resumed => {
                log::info("resumed from sleep");
                self.cpu.reset();
                self.cpu.sample();
                if let Some(w) = &self.weather {
                    w.refresh();
                }
                self.place(ui);
            }
            Msg::Quit => {
                ui.quit();
                return;
            }
        }
        self.dashboard.invalidate();
    }
}

fn spawn_temps(config: &Config, proxy: Proxy<Msg>) {
    temps::spawn(
        &config.temps,
        Duration::from_millis(config.refresh.temps_ms),
        move |r| proxy.send(Msg::Temps(r)).is_ok(),
    );
}

fn spawn_weather(config: &Config, proxy: Proxy<Msg>) -> WeatherWorker {
    let interval = Duration::from_secs(config.refresh.weather_minutes * 60);
    weather::spawn(&config.weather, interval, move |r| {
        proxy.send(Msg::Weather(r)).is_ok()
    })
}
