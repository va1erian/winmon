use winmon::config::{Units, Weather};
use winmon::weather::icons::{IconKind, kind};
use winmon::weather::openmeteo::{parse, url, weekday_of};

#[test]
fn parses_fixture() {
    let f = parse(include_str!("fixtures/openmeteo.json")).unwrap();
    assert_eq!(f.current_temp, Some(19.1));
    assert_eq!(f.current_code, Some(3));
    assert_eq!(f.days.len(), 5);
    let d0 = &f.days[0];
    assert_eq!(d0.weekday, Some(3), "2026-10-07 is a Wednesday");
    assert_eq!(d0.code, Some(80));
    assert_eq!(d0.max, Some(19.1));
    assert_eq!(d0.min, Some(14.0));
    assert_eq!(d0.rain_pct, Some(100.0));
}

#[test]
fn tolerates_nulls_and_short_arrays() {
    let json = r#"{"daily":{"time":["2026-01-01","2026-01-02"],"weather_code":[null,2],"temperature_2m_max":[5.0]}}"#;
    let f = parse(json).unwrap();
    assert_eq!(f.current_temp, None);
    assert_eq!(f.days[0].code, None);
    assert_eq!(f.days[1].code, Some(2));
    assert_eq!(f.days[1].max, None);
    assert_eq!(f.days[1].rain_pct, None);
}

#[test]
fn weekdays() {
    assert_eq!(weekday_of("2000-01-01"), Some(6));
    assert_eq!(weekday_of("2024-02-29"), Some(4));
    assert_eq!(weekday_of("2026-10-11"), Some(0));
    assert_eq!(weekday_of("garbage"), None);
    assert_eq!(weekday_of("2026-13-01"), None);
}

#[test]
fn builds_url() {
    let mut w = Weather {
        latitude: 1.5,
        longitude: -2.25,
        days: 5,
        ..Weather::default()
    };
    let u = url(&w);
    assert!(u.contains("latitude=1.5&longitude=-2.25"));
    assert!(u.contains("forecast_days=5"));
    assert!(!u.contains("fahrenheit"));
    w.units = Units::Imperial;
    assert!(url(&w).ends_with("&temperature_unit=fahrenheit"));
}

#[test]
fn icon_kinds() {
    assert_eq!(kind(0), IconKind::Clear);
    assert_eq!(kind(3), IconKind::Cloudy);
    assert_eq!(kind(45), IconKind::Fog);
    assert_eq!(kind(53), IconKind::Drizzle);
    assert_eq!(kind(81), IconKind::Rain);
    assert_eq!(kind(75), IconKind::Snow);
    assert_eq!(kind(95), IconKind::Thunder);
}
