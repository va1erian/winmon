use winmon::config::Temps;
use winmon::temps::lhm::{self, LhmSource};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/lhm.json")).unwrap()
}

#[test]
fn walks_every_sensor_with_hardware_names() {
    let sensors = lhm::sensors(&fixture());
    assert_eq!(sensors.len(), 6);
    let tctl = sensors
        .iter()
        .find(|s| s.id == "/amdcpu/0/temperature/2")
        .unwrap();
    assert_eq!(tctl.hardware, "AMD Ryzen 7 5800X");
    assert_eq!(tctl.name, "Core (Tctl/Tdie)");
    assert_eq!(tctl.value, Some(45.3));
    let board = sensors.iter().find(|s| s.id.starts_with("/lpc")).unwrap();
    assert_eq!(board.hardware, "Nuvoton NCT6798D");
    assert_eq!(board.value, Some(31.5), "comma decimal separator");
    let hot_spot = sensors
        .iter()
        .find(|s| s.id == "/gpu-nvidia/0/temperature/2")
        .unwrap();
    assert_eq!(hot_spot.value, None, "empty value");
}

#[test]
fn parses_values_defensively() {
    assert_eq!(lhm::parse_value("45.0 °C"), Some(45.0));
    assert_eq!(lhm::parse_value("45,5 °C"), Some(45.5));
    assert_eq!(lhm::parse_value(" 7 °C"), Some(7.0));
    assert_eq!(lhm::parse_value("-3.5 °C"), Some(-3.5));
    assert_eq!(lhm::parse_value(""), None);
    assert_eq!(lhm::parse_value("n/a"), None);
}

#[test]
fn guesses_cpu_and_gpu() {
    let sensors = lhm::sensors(&fixture());
    assert_eq!(
        lhm::guess_cpu(&sensors).unwrap().id,
        "/amdcpu/0/temperature/2"
    );
    assert_eq!(
        lhm::guess_gpu(&sensors).unwrap().id,
        "/gpu-nvidia/0/temperature/0"
    );
}

#[test]
fn readings_follow_config() {
    let sensors = lhm::sensors(&fixture());
    let guessed = LhmSource::new(&Temps::default(), true).readings(&sensors);
    assert_eq!(guessed.len(), 2);
    assert_eq!(guessed[0].celsius, Some(45.3));
    assert_eq!(guessed[1].celsius, Some(52.0));

    let config = Temps {
        cpu_sensor: Some("/amdcpu/0/temperature/3".into()),
        gpu_sensor: Some("/gpu-nvidia/9/temperature/0".into()),
        ..Temps::default()
    };
    let configured = LhmSource::new(&config, true).readings(&sensors);
    assert_eq!(configured[0].celsius, Some(43.0));
    assert_eq!(
        configured[1].celsius, None,
        "configured but missing shows —"
    );

    let no_gpu = LhmSource::new(&Temps::default(), false).readings(&sensors);
    assert_eq!(no_gpu.len(), 1);
}
