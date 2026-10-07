//! The settings window's controls and layout, and moving values between
//! them and a [`Config`].

use std::result::Result;

use win32ui::prelude::*;
use win32ui::{Layout, Tabs, column};

use super::Msg;
use super::choices::{self, MonitorChoice};
use crate::config::{Config, TempSourceKind, Units};
use crate::weather::geocode::Place;

const CAPTION_WIDTH: f32 = 150.0;
const ROW: f32 = 24.0;

/// Every control in the window. Captions are only kept alive.
pub(super) struct Form {
    // Display
    pub monitor: ComboBox<MonitorChoice, Msg>,
    pub format_24h: CheckBox<Msg>,
    pub show_seconds: CheckBox<Msg>,
    pub autostart: CheckBox<Msg>,
    // Temperatures
    pub source: ComboBox<TempSourceKind, Msg>,
    pub lhm_url: Edit<Msg>,
    pub cpu_sensor: ComboBox<Option<String>, Msg>,
    pub gpu_sensor: ComboBox<Option<String>, Msg>,
    pub detect: Button<Msg>,
    pub sensors_status: Label,
    pub nvml: CheckBox<Msg>,
    pub warm: Edit<Msg>,
    pub hot: Edit<Msg>,
    // Weather
    pub weather: CheckBox<Msg>,
    pub city: Edit<Msg>,
    pub search: Button<Msg>,
    /// Search results; `None` is the placeholder shown before a search.
    pub places: ComboBox<Option<Place>, Msg>,
    pub latitude: Edit<Msg>,
    pub longitude: Edit<Msg>,
    pub days: ComboBox<u32, Msg>,
    pub units: ComboBox<Units, Msg>,
    // Advanced
    pub tick_ms: Edit<Msg>,
    pub history_seconds: Edit<Msg>,
    pub temps_ms: Edit<Msg>,
    pub weather_minutes: Edit<Msg>,
    // Footer
    pub status: Label,
    pub save: Button<Msg>,
    pub close: Button<Msg>,
    captions: Vec<Label>,
}

fn edit(ui: &mut Ui<Msg>, text: &str) -> Edit<Msg> {
    let e = Edit::single_line(ui).expect("edit");
    e.set_text(text);
    e
}

fn label(ui: &mut Ui<Msg>, text: &str) -> Label {
    Label::new(ui, Rect::default(), text).expect("label")
}

impl Form {
    pub fn new(ui: &mut Ui<Msg>, c: &Config, autostart: bool) -> Form {
        let (monitor_items, monitor_current) =
            choices::monitor_choices(&win32ui::monitors(), &c.display);
        let sensor_items =
            |current: &Option<String>| choices::sensor_choices(&[], current.as_deref());
        let num = |v: &dyn std::fmt::Display| v.to_string();

        let nvml_label = if cfg!(feature = "nvml") {
            "Read the NVIDIA GPU directly (NVML)"
        } else {
            "Read the NVIDIA GPU directly (needs a build with --features nvml)"
        };
        let form = Form {
            monitor: ComboBox::new(ui, monitor_items)
                .expect("combo")
                .select(&monitor_current),
            format_24h: CheckBox::new(ui, "24-hour clock")
                .expect("check")
                .checked(c.clock.format_24h),
            show_seconds: CheckBox::new(ui, "Show seconds")
                .expect("check")
                .checked(c.clock.show_seconds),
            autostart: CheckBox::new(ui, "Start winmon when I sign in")
                .expect("check")
                .checked(autostart),

            source: ComboBox::new(
                ui,
                [
                    ("LibreHardwareMonitor", TempSourceKind::Lhm),
                    ("None (hide CPU temperature)", TempSourceKind::None),
                ],
            )
            .expect("combo")
            .select(&c.temps.source)
            .on_select(|k| Some(Msg::SourceChanged(*k))),
            lhm_url: edit(ui, &c.temps.lhm_url),
            cpu_sensor: ComboBox::new(ui, sensor_items(&c.temps.cpu_sensor))
                .expect("combo")
                .select(&c.temps.cpu_sensor),
            gpu_sensor: ComboBox::new(ui, sensor_items(&c.temps.gpu_sensor))
                .expect("combo")
                .select(&c.temps.gpu_sensor),
            detect: Button::new(ui, "Detect sensors")
                .expect("button")
                .on_click(|| Some(Msg::DetectSensors)),
            sensors_status: label(ui, ""),
            nvml: CheckBox::new(ui, nvml_label)
                .expect("check")
                .checked(c.temps.nvml),
            warm: edit(ui, &num(&c.temps.warm)),
            hot: edit(ui, &num(&c.temps.hot)),

            weather: CheckBox::new(ui, "Show the weather forecast")
                .expect("check")
                .checked(c.weather.enabled)
                .on_toggle(|on| Some(Msg::WeatherToggled(on))),
            city: Edit::single_line(ui)
                .expect("edit")
                .cue("City, e.g. Lyon")
                .on_submit(|| Some(Msg::SearchPlace)),
            search: Button::new(ui, "Search")
                .expect("button")
                .on_click(|| Some(Msg::SearchPlace)),
            places: ComboBox::new(ui, [("Search results appear here".to_string(), None)])
                .expect("combo")
                .select(&None)
                .on_select(|p| p.clone().map(Msg::PlacePicked)),
            latitude: edit(ui, &choices::coordinate(c.weather.latitude)),
            longitude: edit(ui, &choices::coordinate(c.weather.longitude)),
            days: ComboBox::new(ui, (1..=16).map(|d| (d.to_string(), d)))
                .expect("combo")
                .select(&c.weather.days),
            units: ComboBox::new(
                ui,
                [
                    ("Metric (°C)", Units::Metric),
                    ("Imperial (°F)", Units::Imperial),
                ],
            )
            .expect("combo")
            .select(&c.weather.units),

            tick_ms: edit(ui, &num(&c.refresh.tick_ms)),
            history_seconds: edit(ui, &num(&c.refresh.history_seconds)),
            temps_ms: edit(ui, &num(&c.refresh.temps_ms)),
            weather_minutes: edit(ui, &num(&c.refresh.weather_minutes)),

            status: label(ui, ""),
            save: Button::new(ui, "Save")
                .expect("button")
                .default()
                .on_click(|| Some(Msg::Save)),
            close: Button::new(ui, "Close")
                .expect("button")
                .on_click(|| Some(Msg::Close)),
            captions: Vec::new(),
        };
        // Tab switches re-show every control, so disable rather than hide.
        form.places.set_enabled(false);
        form.source_changed(c.temps.source);
        form.weather_toggled(c.weather.enabled);
        form
    }

    /// Builds and installs the window's layout. Returns the tabs, for
    /// switching pages later.
    pub fn lay_out(&mut self, ui: &mut Ui<Msg>, config_path: &str) -> Tabs {
        let display = column![
            self.field(ui, "Monitor", self.monitor.fill(1)),
            self.note(
                ui,
                "winmon --list-monitors prints the details of each monitor."
            ),
            self.check_row(&self.format_24h),
            self.check_row(&self.show_seconds),
            self.check_row(&self.autostart),
            Layout::column().fill(1),
        ];
        let temps = column![
            self.field(ui, "Source", self.source.fill(1)),
            self.field(ui, "Web server address", self.lhm_url.fill(1)),
            self.field(ui, "CPU sensor", self.cpu_sensor.fill(1)),
            self.field(ui, "GPU sensor", self.gpu_sensor.fill(1)),
            Layout::row()
                .item(self.detect.width(dip(130.0)))
                .item(Layout::row().width(dip(10.0)))
                .item(self.sensors_status.fill(1))
                .height(dip(ROW)),
            self.check_row(&self.nvml),
            self.field(ui, "Orange from (°C)", self.warm.width(dip(80.0))),
            self.field(ui, "Red from (°C)", self.hot.width(dip(80.0))),
            Layout::column().fill(1),
        ];
        let weather = column![
            self.check_row(&self.weather),
            Layout::row()
                .item(self.caption(ui, "Find a place"))
                .item(self.city.fill(1))
                .item(Layout::row().width(dip(8.0)))
                .item(self.search.width(dip(80.0)))
                .height(dip(ROW)),
            self.field(ui, "", self.places.fill(1)),
            self.field(ui, "Latitude", self.latitude.width(dip(120.0))),
            self.field(ui, "Longitude", self.longitude.width(dip(120.0))),
            self.field(ui, "Days", self.days.width(dip(80.0))),
            self.field(ui, "Units", self.units.width(dip(160.0))),
            self.note(ui, "Forecasts come from open-meteo.com."),
            Layout::column().fill(1),
        ];
        let advanced = column![
            self.field(ui, "Redraw every (ms)", self.tick_ms.width(dip(100.0))),
            self.field(
                ui,
                "Graph history (s)",
                self.history_seconds.width(dip(100.0))
            ),
            self.field(
                ui,
                "Temperatures every (ms)",
                self.temps_ms.width(dip(100.0))
            ),
            self.field(
                ui,
                "Weather every (min)",
                self.weather_minutes.width(dip(100.0))
            ),
            self.note(ui, &format!("Saved to {config_path}")),
            Layout::column().fill(1),
        ];
        let page = |l: Layout| l.spacing(dip(10.0)).margins(Insets::all(dip(14.0)));
        let tabs = Tabs::new()
            .page("Display", page(display))
            .page("Temperatures", page(temps))
            .page("Weather", page(weather))
            .page("Advanced", page(advanced));
        let footer = Layout::row()
            .item(self.status.fill(1))
            .item(self.save.width(dip(88.0)))
            .item(Layout::row().width(dip(8.0)))
            .item(self.close.width(dip(88.0)))
            .height(dip(28.0));
        ui.set_layout(
            Layout::column()
                .item(&tabs)
                .item(footer)
                .spacing(dip(12.0))
                .margins(Insets::all(dip(12.0))),
        );
        tabs
    }

    fn caption(&mut self, ui: &mut Ui<Msg>, text: &str) -> LayoutItem {
        let l = label(ui, text);
        // Nudge the static text down to line up with the field's text.
        let item = Layout::column()
            .margins(Insets::new(dip(0.0), dip(4.0), dip(0.0), dip(0.0)))
            .item(l.fill(1))
            .width(dip(CAPTION_WIDTH));
        self.captions.push(l);
        item
    }

    fn field(&mut self, ui: &mut Ui<Msg>, text: &str, field: LayoutItem) -> LayoutItem {
        Layout::row()
            .item(self.caption(ui, text))
            .item(field)
            .height(dip(ROW))
    }

    fn check_row(&self, check: &CheckBox<Msg>) -> LayoutItem {
        check.height(dip(24.0))
    }

    fn note(&mut self, ui: &mut Ui<Msg>, text: &str) -> LayoutItem {
        let l = label(ui, text);
        let item = l.height(dip(34.0));
        self.captions.push(l);
        item
    }

    pub fn source_changed(&self, source: TempSourceKind) {
        let on = source == TempSourceKind::Lhm;
        self.lhm_url.set_enabled(on);
        self.cpu_sensor.set_enabled(on);
        self.gpu_sensor.set_enabled(on);
        self.detect.set_enabled(on);
    }

    pub fn weather_toggled(&self, on: bool) {
        self.city.set_enabled(on);
        self.search.set_enabled(on);
        self.latitude.set_enabled(on);
        self.longitude.set_enabled(on);
        self.days.set_enabled(on);
        self.units.set_enabled(on);
        self.places
            .set_enabled(on && self.places.selected().is_some_and(Option::is_some));
    }

    /// Replaces the sensor drop-downs' items, keeping the current choices.
    pub fn set_sensors(&mut self, sensors: &[crate::temps::lhm::Sensor]) {
        for combo in [&mut self.cpu_sensor, &mut self.gpu_sensor] {
            let current = combo.selected().cloned().flatten();
            combo.set_items(choices::sensor_choices(sensors, current.as_deref()));
            combo.set_selected(&current);
        }
    }

    pub fn set_places(&mut self, places: Vec<Place>) {
        let first = places.first().cloned();
        self.places
            .set_items(places.into_iter().map(|p| (p.label(), Some(p))));
        if let Some(p) = first {
            self.places.set_enabled(true);
            self.place_picked(&p);
            self.places.set_selected(&Some(p));
        }
    }

    pub fn place_picked(&self, place: &Place) {
        self.latitude.set_text(&choices::coordinate(place.latitude));
        self.longitude
            .set_text(&choices::coordinate(place.longitude));
    }

    /// The config the fields describe, on top of `base` (which keeps any key
    /// the dialog doesn't show). The error is a sentence for the status line.
    pub fn read(&self, base: &Config) -> Result<Config, String> {
        use choices::number;
        let mut c = base.clone();
        let monitor = self
            .monitor
            .selected()
            .cloned()
            .unwrap_or(MonitorChoice::Auto);
        (c.display.monitor, c.display.monitor_device) = monitor.to_config();
        c.clock.format_24h = self.format_24h.is_checked();
        c.clock.show_seconds = self.show_seconds.is_checked();

        let t = &mut c.temps;
        t.source = self
            .source
            .selected()
            .copied()
            .unwrap_or(TempSourceKind::Lhm);
        t.lhm_url = self.lhm_url.text().trim().to_string();
        if t.source == TempSourceKind::Lhm && !t.lhm_url.starts_with("http") {
            return Err("The web server address must start with http://.".into());
        }
        t.cpu_sensor = self.cpu_sensor.selected().cloned().flatten();
        t.gpu_sensor = self.gpu_sensor.selected().cloned().flatten();
        t.nvml = self.nvml.is_checked();
        t.warm = number("Orange from", &self.warm.text(), -50.0, 150.0)?;
        t.hot = number("Red from", &self.hot.text(), -50.0, 150.0)?;
        if t.hot < t.warm {
            return Err("Red must start at or above orange.".into());
        }

        let w = &mut c.weather;
        w.enabled = self.weather.is_checked();
        w.latitude = number("Latitude", &self.latitude.text(), -90.0, 90.0)?;
        w.longitude = number("Longitude", &self.longitude.text(), -180.0, 180.0)?;
        w.days = self.days.selected().copied().unwrap_or(5);
        w.units = self.units.selected().copied().unwrap_or(Units::Metric);

        let r = &mut c.refresh;
        r.tick_ms = number("Redraw interval", &self.tick_ms.text(), 100, 60_000)?;
        r.history_seconds = number("Graph history", &self.history_seconds.text(), 2, 3600)?;
        r.temps_ms = number("Temperature interval", &self.temps_ms.text(), 250, 600_000)?;
        r.weather_minutes = number("Weather interval", &self.weather_minutes.text(), 5, 1440)?;
        Ok(c)
    }
}
