//! Command-line parsing and the one-shot commands.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::config::Config;
use crate::temps::lhm;
use crate::{monitor, platform};

pub const USAGE: &str = "\
usage: winmon [--config <path>] [--windowed]
       winmon --list-monitors | --list-sensors
       winmon --install | --uninstall | --quit

  --config <path>   config file (default: %APPDATA%\\winmon\\winmon.toml)
  --windowed        run in a normal 480x800 window, for development
  --list-monitors   print the attached monitors and exit
  --list-sensors    print LibreHardwareMonitor temperature sensors and exit
  --install         start winmon at logon (current user)
  --uninstall       remove the logon entry
  --quit            ask a running winmon to exit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    ListMonitors,
    ListSensors,
    Install,
    Uninstall,
    Quit,
    Help,
}

#[derive(Debug)]
pub struct Args {
    pub command: Command,
    pub config: Option<PathBuf>,
    pub windowed: bool,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut out = Args {
        command: Command::Run,
        config: None,
        windowed: false,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let command = match arg.as_str() {
            "--config" => {
                out.config = Some(args.next().ok_or("--config needs a path")?.into());
                continue;
            }
            "--windowed" => {
                out.windowed = true;
                continue;
            }
            "--list-monitors" => Command::ListMonitors,
            "--list-sensors" => Command::ListSensors,
            "--install" => Command::Install,
            "--uninstall" => Command::Uninstall,
            "--quit" => Command::Quit,
            "-h" | "--help" => Command::Help,
            other => return Err(format!("unknown argument `{other}`")),
        };
        if out.command != Command::Run {
            return Err("only one command at a time".into());
        }
        out.command = command;
    }
    Ok(out)
}

pub fn run_command(command: Command, config: &Config) -> ExitCode {
    match command {
        Command::Run => unreachable!(),
        Command::Help => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Command::ListMonitors => {
            let monitors = win32ui::monitors();
            for m in &monitors {
                println!("{}", monitor::describe(m));
            }
            match monitor::select(&monitors, &config.display) {
                Some(m) => println!("\nwinmon would use: {}", m.device_name),
                None => println!("\nno secondary monitor matches: winmon would run windowed"),
            }
            ExitCode::SUCCESS
        }
        Command::ListSensors => list_sensors(config),
        Command::Install => autostart(true),
        Command::Uninstall => autostart(false),
        Command::Quit => {
            if platform::signal_quit() {
                ExitCode::SUCCESS
            } else {
                eprintln!("winmon is not running");
                ExitCode::FAILURE
            }
        }
    }
}

fn list_sensors(config: &Config) -> ExitCode {
    let json = match lhm::fetch(&lhm::agent(), &config.temps.lhm_url) {
        Ok(j) => j,
        Err(e) => {
            eprintln!(
                "could not read {}: {e:#}\n\nIs LibreHardwareMonitor running with Options → Remote Web Server enabled?",
                config.temps.lhm_url
            );
            return ExitCode::FAILURE;
        }
    };
    let sensors = lhm::sensors(&json);
    let temps: Vec<_> = sensors.iter().filter(|s| lhm::is_temperature(s)).collect();
    if temps.is_empty() {
        println!("no temperature sensors found");
    }
    for s in &temps {
        let value = s.value.map_or("—".to_string(), |v| format!("{v:.1} °C"));
        println!("{:<36} {:>9}  {} / {}", s.id, value, s.hardware, s.name);
    }
    let guess = |s: Option<&lhm::Sensor>| s.map_or("(none)".to_string(), |s| s.id.clone());
    println!(
        "\ndefault picks: cpu_sensor = {}, gpu_sensor = {}",
        guess(lhm::guess_cpu(&sensors)),
        guess(lhm::guess_gpu(&sensors))
    );
    ExitCode::SUCCESS
}

/// Autostart through the per-user Run key: no elevation needed.
fn autostart(install: bool) -> ExitCode {
    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let mut cmd = std::process::Command::new("reg");
    if install {
        let Ok(exe) = std::env::current_exe() else {
            return ExitCode::FAILURE;
        };
        cmd.args(["add", KEY, "/v", "winmon", "/t", "REG_SZ", "/f", "/d"])
            .arg(format!("\"{}\"", exe.display()));
    } else {
        cmd.args(["delete", KEY, "/v", "winmon", "/f"]);
    }
    match cmd.status() {
        Ok(s) if s.success() => {
            println!(
                "{}",
                if install {
                    "winmon will start at logon"
                } else {
                    "autostart removed"
                }
            );
            ExitCode::SUCCESS
        }
        _ => ExitCode::FAILURE,
    }
}
