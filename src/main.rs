// Release builds have no console window; debug builds keep one for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use winmon::app::Exit;
use winmon::cli::{Command, parse};
use winmon::config::{self, Config};
use winmon::{app, cli, log, platform, settings};

fn main() -> ExitCode {
    log::install_panic_hook();
    // DPI awareness before any monitor query, so rects are in real pixels.
    win32ui::init();
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut args = match parse(raw.iter().cloned()) {
        Ok(a) => a,
        Err(e) => {
            platform::attach_parent_console();
            eprintln!("winmon: {e}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    if raw.is_empty() && std::env::current_exe().is_ok_and(|exe| cli::is_setup_exe(&exe)) {
        args.command = Command::Install;
    }
    if !matches!(
        args.command,
        Command::Run | Command::Settings | Command::Install | Command::Uninstall
    ) || args.quiet
    {
        platform::attach_parent_console();
    }
    let config_path = args.config.clone().or_else(config::default_path);
    let loaded = Config::load(args.config.as_deref());

    match args.command {
        Command::Settings => {
            let Some(path) = config_path else {
                log::error("settings: %APPDATA% is not set");
                return ExitCode::FAILURE;
            };
            // A broken file still opens the dialog, on defaults, so it can be
            // fixed; a missing one is just a first run.
            let (config, error) = match loaded {
                Ok(c) => (c, None),
                Err(_) if !path.exists() => (Config::default(), None),
                Err(e) => (Config::default(), Some(format!("{e:#}"))),
            };
            match settings::run(config, path, error) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    log::error(&format!("settings: {e}"));
                    ExitCode::FAILURE
                }
            }
        }
        command => {
            let config = match loaded {
                Ok(c) => c,
                Err(e) => {
                    platform::attach_parent_console();
                    eprintln!("winmon: config: {e:#}");
                    log::error(&format!("config: {e:#}"));
                    return ExitCode::FAILURE;
                }
            };
            if command == Command::Run {
                run_dashboard(config, config_path, args.windowed, &raw)
            } else {
                cli::run_command(&args, &config)
            }
        }
    }
}

fn run_dashboard(
    config: Config,
    config_path: Option<std::path::PathBuf>,
    windowed: bool,
    raw_args: &[String],
) -> ExitCode {
    let Some(guard) = platform::single_instance() else {
        log::info("already running");
        return ExitCode::SUCCESS;
    };
    platform::lower_priority();
    match app::run(config, config_path, windowed) {
        Ok(Exit::Quit) => ExitCode::SUCCESS,
        Ok(Exit::Restart) => {
            // A fresh process is the simplest way to rebuild every worker and
            // the window from the new config. Release the instance lock first.
            drop(guard);
            match std::env::current_exe()
                .and_then(|exe| std::process::Command::new(exe).args(raw_args).spawn())
            {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    log::error(&format!("restart: {e}"));
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            log::error(&format!("run: {e}"));
            ExitCode::FAILURE
        }
    }
}
