//! The settings dialog's pure parts, config writing, and the setup CLI.

use win32ui::{MonitorInfo, Rect};
use winmon::cli::{Command, is_setup_exe, parse};
use winmon::config::{Config, Display, Units};
use winmon::settings::choices::{
    MonitorChoice, coordinate, monitor_choices, number, sensor_choices,
};
use winmon::temps::lhm::Sensor;
use winmon::weather::geocode;

fn mon(device: &str, name: &str, primary: bool) -> MonitorInfo {
    let rect = Rect::new(0, 0, 480, 800);
    MonitorInfo {
        device_name: device.into(),
        friendly_name: name.into(),
        rect,
        work_area: rect,
        primary,
        dpi: 96,
    }
}

#[test]
fn config_round_trips_through_toml() {
    let mut c = Config::parse(include_str!("../winmon.example.toml")).unwrap();
    c.weather.units = Units::Imperial;
    c.temps.cpu_sensor = Some("/amdcpu/0/temperature/2".into());
    let back = Config::parse(&c.to_toml()).unwrap();
    assert_eq!(back.to_toml(), c.to_toml());
    assert_eq!(back.weather.units, Units::Imperial);
    // Unset optional keys stay unset rather than becoming "".
    assert_eq!(back.temps.gpu_sensor, None);
    assert_eq!(back.display.monitor_device, None);
}

#[test]
fn default_config_writes_and_reads_back() {
    let text = Config::default().to_toml();
    assert!(text.starts_with("# winmon config"));
    Config::parse(&text).unwrap();
}

#[test]
fn clamp_drops_blank_monitor_names() {
    let c = Config::parse("[display]\nmonitor = \"  \"\n[weather]\nlatitude = 120").unwrap();
    assert_eq!(c.display.monitor, None);
    assert_eq!(c.weather.latitude, 90.0);
}

#[test]
fn monitor_choices_use_names_and_keep_unplugged() {
    let monitors = [
        mon(r"\\.\DISPLAY1", "Dell U2720Q", true),
        mon(r"\\.\DISPLAY2", "USB Display", false),
        mon(r"\\.\DISPLAY3", "Generic PnP Monitor", false),
        mon(r"\\.\DISPLAY4", "Generic PnP Monitor", false),
    ];
    let (items, current) = monitor_choices(&monitors, &Display::default());
    assert_eq!(current, MonitorChoice::Auto);
    assert_eq!(items.len(), 5);
    assert_eq!(items[2].1, MonitorChoice::Name("USB Display".into()));
    // Ambiguous names fall back to the device.
    assert_eq!(items[3].1, MonitorChoice::Device(r"\\.\DISPLAY3".into()));

    // A substring in the config selects the matching monitor's item.
    let usb = Display {
        monitor: Some("usb".into()),
        monitor_device: None,
    };
    let (items, current) = monitor_choices(&monitors, &usb);
    assert_eq!(current, MonitorChoice::Name("USB Display".into()));
    assert_eq!(items.len(), 5);

    // An unplugged monitor gets its own item, so Save keeps it.
    let gone = Display {
        monitor: Some("Panel 4in".into()),
        monitor_device: None,
    };
    let (items, current) = monitor_choices(&monitors, &gone);
    assert_eq!(current, MonitorChoice::Name("Panel 4in".into()));
    assert!(items.last().unwrap().0.contains("not connected"));
    assert_eq!(current.to_config(), (Some("Panel 4in".into()), None));
}

#[test]
fn sensor_choices_list_temperatures_and_keep_missing_ids() {
    let sensor = |id: &str, kind: &str| Sensor {
        id: id.into(),
        name: "Core".into(),
        hardware: "CPU".into(),
        kind: kind.into(),
        value: Some(41.6),
    };
    let sensors = [
        sensor("/cpu/0/temperature/0", "Temperature"),
        sensor("/cpu/0/load/0", "Load"),
    ];
    let items = sensor_choices(&sensors, Some("/gpu/0/temperature/0"));
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].1, None);
    assert_eq!(items[1].0, "CPU / Core — 42 °C");
    assert!(items[2].0.contains("not found"));
    assert_eq!(
        sensor_choices(&sensors, Some("/cpu/0/temperature/0")).len(),
        2
    );
}

#[test]
fn number_fields() {
    assert_eq!(number("Latitude", " 45,75 ", -90.0, 90.0), Ok(45.75));
    assert!(number("Latitude", "91", -90.0, 90.0).is_err());
    assert!(
        number::<u32>("Days", "abc", 1, 16)
            .unwrap_err()
            .contains("Days")
    );
    assert_eq!(coordinate(48.8566 + 1e-12), "48.8566");
    assert_eq!(coordinate(2.0), "2");
    assert_eq!(coordinate(-0.00001), "0");
}

#[test]
fn geocode_fixture_parses() {
    let places = geocode::parse(include_str!("fixtures/geocode.json")).unwrap();
    assert_eq!(places.len(), 2);
    assert_eq!(places[0].name, "Springfield");
    assert!(places[0].label().ends_with("United States"));
    assert!(
        geocode::parse(r#"{"generationtime_ms":0.5}"#)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn setup_cli() {
    let a = |v: &[&str]| parse(v.iter().map(|s| s.to_string()));
    assert_eq!(a(&["--settings"]).unwrap().command, Command::Settings);
    let r = a(&["--install", "--quiet"]).unwrap();
    assert_eq!(r.command, Command::Install);
    assert!(r.quiet);
    assert!(a(&["--quiet"]).is_err());
    assert!(a(&["--settings", "--quiet"]).is_err());
    assert!(is_setup_exe("C:/dl/winmon-setup.exe".as_ref()));
    assert!(is_setup_exe("Winmon-Setup-0.2.exe".as_ref()));
    assert!(!is_setup_exe("C:/Programs/winmon/winmon.exe".as_ref()));
}
