//! The settings dialog (`winmon --settings`): edits the TOML config with
//! native controls. A running dashboard watches the file and restarts itself
//! with the new settings, so Save applies them right away.

pub mod choices;
mod form;

use std::path::PathBuf;
use std::result::Result;

use win32ui::prelude::*;

use crate::config::{Config, TempSourceKind};
use crate::snapshot::Snapshot;
use crate::temps::lhm::{self, Sensor};
use crate::weather::geocode::{self, Place};
use crate::{install, platform};

use form::Form;

pub enum Msg {
    SourceChanged(TempSourceKind),
    DetectSensors,
    Sensors(Result<Vec<Sensor>, String>),
    WeatherToggled(bool),
    SearchPlace,
    Places(Result<Vec<Place>, String>),
    PlacePicked(Place),
    Save,
    Close,
    SnapshotTick,
}

struct Settings {
    form: Form,
    /// What was loaded (or last saved): keys the dialog doesn't show survive.
    base: Config,
    path: PathBuf,
    autostart: bool,
    snapshot: Option<Snapshot>,
}

/// Opens the dialog on `config`, saving to `path`. `load_error` (the current
/// file didn't parse) is shown so the user knows Save will replace it.
pub fn run(config: Config, path: PathBuf, load_error: Option<String>) -> win32ui::Result<()> {
    let spec = WindowSpec::new("winmon settings")
        .size(dip(520.0), dip(520.0))
        .theme(Theme::system());
    run_app(spec, move |ui| {
        ui.follow_system_theme(true);
        platform::dialog_frame(ui.hwnd().raw());
        let autostart = install::autostart_enabled();
        let mut form = Form::new(ui, &config, autostart);
        let tabs = form.lay_out(ui, &path.display().to_string());
        let snapshot = Snapshot::arm(ui, || Msg::SnapshotTick);
        if let Some(tab) = snapshot.as_ref().and(Snapshot::tab()) {
            tabs.set_selected(tab);
        }
        if let Some(e) = load_error {
            form.status.set_text(&format!(
                "The config file has an error; Save replaces it. {e}"
            ));
        }
        if config.temps.source == TempSourceKind::Lhm {
            ui.emit(Msg::DetectSensors);
        }
        Settings {
            form,
            base: config,
            path,
            autostart,
            snapshot,
        }
    })
}

impl App for Settings {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        let f = &mut self.form;
        match msg {
            Msg::SourceChanged(kind) => f.source_changed(kind),
            Msg::DetectSensors => {
                let url = f.lhm_url.text().trim().to_string();
                f.sensors_status
                    .set_text("Looking for LibreHardwareMonitor…");
                f.detect.set_enabled(false);
                let proxy = ui.proxy();
                std::thread::spawn(move || {
                    let r = lhm::fetch(&lhm::agent(), &url)
                        .map(|json| lhm::sensors(&json))
                        .map_err(|e| format!("{e:#}"));
                    let _ = proxy.send(Msg::Sensors(r));
                });
            }
            Msg::Sensors(result) => {
                f.detect.set_enabled(true);
                match result {
                    Ok(sensors) => {
                        let n = sensors.iter().filter(|s| lhm::is_temperature(s)).count();
                        f.set_sensors(&sensors);
                        f.sensors_status
                            .set_text(&format!("Found {n} temperature sensors."));
                    }
                    Err(_) => f
                        .sensors_status
                        .set_text("Not reachable. Is LibreHardwareMonitor's web server on?"),
                }
            }
            Msg::WeatherToggled(on) => f.weather_toggled(on),
            Msg::SearchPlace => {
                let name = f.city.text();
                if name.trim().is_empty() {
                    f.city.focus();
                    return;
                }
                f.status.set_text("Searching…");
                f.search.set_enabled(false);
                let proxy = ui.proxy();
                std::thread::spawn(move || {
                    let r = geocode::search(&name).map_err(|e| format!("{e:#}"));
                    let _ = proxy.send(Msg::Places(r));
                });
            }
            Msg::Places(result) => {
                f.search.set_enabled(true);
                match result {
                    Ok(places) if places.is_empty() => f.status.set_text("No place found."),
                    Ok(places) => {
                        let n = places.len();
                        f.set_places(places);
                        f.status.set_text(&format!(
                            "{n} place{} found; pick one from the list.",
                            if n == 1 { "" } else { "s" }
                        ));
                    }
                    Err(e) => f.status.set_text(&format!("Search failed: {e}")),
                }
            }
            Msg::PlacePicked(place) => f.place_picked(&place),
            Msg::Save => self.save(),
            Msg::Close => ui.close(),
            Msg::SnapshotTick => Snapshot::on_tick(&mut self.snapshot, ui),
        }
    }
}

impl Settings {
    fn save(&mut self) {
        let f = &self.form;
        let config = match f.read(&self.base) {
            Ok(c) => c,
            Err(e) => {
                f.status.set_text(&e);
                return;
            }
        };
        if let Err(e) = config.save(&self.path) {
            f.status.set_text(&format!("Couldn't save: {e:#}"));
            return;
        }
        self.base = config;

        let autostart = f.autostart.is_checked();
        if autostart != self.autostart {
            // Autostart runs the installed copy when there is one.
            let exe = install::installed_exe()
                .ok()
                .filter(|p| p.exists())
                .or_else(|| std::env::current_exe().ok());
            match exe.map(|exe| install::set_autostart(autostart, &exe)) {
                Some(Ok(())) => self.autostart = autostart,
                Some(Err(e)) => {
                    f.status
                        .set_text(&format!("Saved, but autostart failed: {e:#}"));
                    return;
                }
                None => {}
            }
        }
        f.status.set_text(if platform::is_running() {
            "Saved. winmon is applying the changes."
        } else {
            "Saved. They apply the next time winmon starts."
        });
    }
}
