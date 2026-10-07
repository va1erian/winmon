use win32ui::{MonitorInfo, Rect};
use winmon::cli::{Command, parse};
use winmon::config::{Config, Display, TempSourceKind, Units};
use winmon::monitor::select;

#[test]
fn empty_config_is_all_defaults() {
    let c = Config::parse("").unwrap();
    assert_eq!(c.refresh.tick_ms, 1000);
    assert_eq!(c.refresh.history_seconds, 120);
    assert_eq!(c.temps.source, TempSourceKind::Lhm);
    assert_eq!(c.weather.units, Units::Metric);
    assert!(c.clock.format_24h);
}

#[test]
fn example_config_parses() {
    let c = Config::parse(include_str!("../winmon.example.toml")).unwrap();
    assert_eq!(c.display.monitor.as_deref(), Some("USB Display"));
    assert_eq!(c.weather.days, 5);
}

#[test]
fn rejects_typos_and_clamps() {
    assert!(Config::parse("[refresh]\ntick_mss = 5").is_err());
    let c = Config::parse("[refresh]\ntick_ms = 1\nhistory_seconds = 0").unwrap();
    assert_eq!(c.refresh.tick_ms, 100);
    assert_eq!(c.refresh.history_seconds, 2);
}

#[test]
fn cli() {
    let a = |v: &[&str]| parse(v.iter().map(|s| s.to_string()));
    let r = a(&[]).unwrap();
    assert_eq!(r.command, Command::Run);
    assert!(!r.windowed);
    let r = a(&["--windowed", "--config", "x.toml"]).unwrap();
    assert!(r.windowed);
    assert_eq!(r.config.unwrap().to_str(), Some("x.toml"));
    assert_eq!(
        a(&["--list-monitors"]).unwrap().command,
        Command::ListMonitors
    );
    assert!(a(&["--config"]).is_err());
    assert!(a(&["--bogus"]).is_err());
    assert!(a(&["--install", "--quit"]).is_err());
}

fn mon(device: &str, name: &str, w: i32, h: i32, primary: bool) -> MonitorInfo {
    let rect = Rect::new(0, 0, w, h);
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
fn monitor_selection() {
    let all = [
        mon(r"\\.\DISPLAY1", "DELL U2720Q", 3840, 2160, true),
        mon(r"\\.\DISPLAY2", "LG 24", 1920, 1080, false),
        mon(r"\\.\DISPLAY3", "USB Display", 480, 800, false),
    ];
    let pick = |d: Display| select(&all, &d).map(|m| m.device_name.clone());

    // Default: smallest non-primary.
    assert_eq!(pick(Display::default()).as_deref(), Some(r"\\.\DISPLAY3"));
    // Friendly-name substring, case-insensitive.
    assert_eq!(
        pick(Display {
            monitor: Some("lg".into()),
            ..Default::default()
        })
        .as_deref(),
        Some(r"\\.\DISPLAY2")
    );
    // Device name wins over friendly name.
    let both = Display {
        monitor: Some("LG".into()),
        monitor_device: Some(r"\\.\display1".into()),
    };
    assert_eq!(pick(both).as_deref(), Some(r"\\.\DISPLAY1"));
    // A name that matches nothing falls back to smallest non-primary.
    assert_eq!(
        pick(Display {
            monitor: Some("nope".into()),
            ..Default::default()
        })
        .as_deref(),
        Some(r"\\.\DISPLAY3")
    );
    // Only the primary attached: stay windowed.
    assert_eq!(select(&all[..1], &Display::default()), None);
    // ...unless the config asks for it explicitly.
    assert!(
        select(
            &all[..1],
            &Display {
                monitor: Some("dell".into()),
                ..Default::default()
            }
        )
        .is_some()
    );
}
