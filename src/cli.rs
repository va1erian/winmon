//! Command-line parsing and the one-shot commands.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::config::Config;
use crate::temps::lhm;
use crate::{install, log, monitor, platform, setup};

pub const USAGE: &str = "\
usage: winmon [--config <path>] [--windowed]
       winmon --settings
       winmon --list-monitors | --list-sensors
       winmon --install | --uninstall [--quiet]
       winmon --quit

  --config <path>   config file (default: %APPDATA%\\winmon\\winmon.toml)
  --windowed        run in a normal 480x800 window, for development
  --settings        open the settings dialog
  --list-monitors   print the attached monitors and exit
  --list-sensors    print LibreHardwareMonitor temperature sensors and exit
  --install         install for the current user (setup window)
  --uninstall       remove winmon (what Settings > Apps runs)
  --quiet           with --install/--uninstall: no window, default choices
  --quit            ask a running winmon to exit

A copy named winmon-setup.exe opens the setup window when started.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    ListMonitors,
    ListSensors,
    Settings,
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
    /// `--quiet`: install/uninstall without a window.
    pub quiet: bool,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut out = Args {
        command: Command::Run,
        config: None,
        windowed: false,
        quiet: false,
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
            "--quiet" => {
                out.quiet = true;
                continue;
            }
            "--settings" => Command::Settings,
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
    if out.quiet && !matches!(out.command, Command::Install | Command::Uninstall) {
        return Err("--quiet only goes with --install or --uninstall".into());
    }
    Ok(out)
}

/// Whether `exe` is named like an installer (`winmon-setup.exe`): started
/// with no arguments, such a copy opens the setup window.
pub fn is_setup_exe(exe: &std::path::Path) -> bool {
    exe.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.to_ascii_lowercase().contains("setup"))
}

pub fn run_command(args: &Args, config: &Config) -> ExitCode {
    match args.command {
        Command::Run | Command::Settings => unreachable!("handled in main"),
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
        Command::Install if args.quiet => report(
            install::install(install::Options::default())
                .map(|exe| format!("winmon installed: {}", exe.display())),
        ),
        Command::Uninstall if args.quiet => {
            report(install::uninstall(false).map(|()| "winmon removed".to_string()))
        }
        Command::Install => window(setup::run(setup::Mode::Install)),
        Command::Uninstall => window(setup::run(setup::Mode::Uninstall)),
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

fn report(result: anyhow::Result<String>) -> ExitCode {
    match result {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("winmon: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn window(result: win32ui::Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error(&format!("window: {e}"));
            ExitCode::FAILURE
        }
    }
}
