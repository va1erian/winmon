// Release builds have no console window; debug builds keep one for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use winmon::cli::{Command, parse};
use winmon::config::Config;
use winmon::{app, cli, log, platform};

fn main() -> ExitCode {
    log::install_panic_hook();
    // DPI awareness before any monitor query, so rects are in real pixels.
    win32ui::init();
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            platform::attach_parent_console();
            eprintln!("winmon: {e}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    if args.command != Command::Run {
        platform::attach_parent_console();
    }
    let config = match Config::load(args.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            platform::attach_parent_console();
            eprintln!("winmon: config: {e:#}");
            log::error(&format!("config: {e:#}"));
            return ExitCode::FAILURE;
        }
    };

    match args.command {
        Command::Run => {
            let Some(_guard) = platform::single_instance() else {
                log::info("already running");
                return ExitCode::SUCCESS;
            };
            platform::lower_priority();
            if let Err(e) = app::run(config, args.windowed) {
                log::error(&format!("run: {e}"));
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        other => cli::run_command(other, &config),
    }
}
