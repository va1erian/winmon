//! The setup window: install/update (`winmon --install`, or running a copy
//! named `winmon-setup.exe`) and uninstall (`winmon --uninstall`, which is
//! what Settings → Apps runs). The work itself is in [`crate::install`].

use std::path::PathBuf;
use std::result::Result;

use win32ui::prelude::*;
use win32ui::{Layout, column};

use crate::install::{self, Options};
use crate::platform;
use crate::snapshot::Snapshot;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Install,
    Uninstall,
}

pub enum Msg {
    Go,
    Cancel,
    Done(Result<Option<PathBuf>, String>),
    SnapshotTick,
}

struct Setup {
    mode: Mode,
    autostart: CheckBox<Msg>,
    start_menu: CheckBox<Msg>,
    launch: CheckBox<Msg>,
    settings: CheckBox<Msg>,
    remove_settings: CheckBox<Msg>,
    status: Label,
    go: Button<Msg>,
    cancel: Button<Msg>,
    _text: [Label; 2],
    snapshot: Option<Snapshot>,
}

pub fn run(mode: Mode) -> win32ui::Result<()> {
    let title = match mode {
        Mode::Install => "winmon setup",
        Mode::Uninstall => "Uninstall winmon",
    };
    let spec = WindowSpec::new(title)
        .size(dip(460.0), dip(360.0))
        .theme(Theme::system());
    run_app(spec, move |ui| Setup::new(ui, mode))
}

fn heading_and_body(mode: Mode) -> (String, String) {
    let dir = install::install_dir().map_or_else(|e| e.to_string(), |d| d.display().to_string());
    let new = env!("CARGO_PKG_VERSION");
    // The folder goes last: a long path may wrap without hiding the rest.
    match (mode, install::installed_version()) {
        (Mode::Install, None) => (
            format!("Install winmon {new}"),
            format!(
                "For your account only; no administrator rights are needed. \
                 Program folder: {dir}"
            ),
        ),
        (Mode::Install, Some(old)) => (
            if old == new {
                format!("Reinstall winmon {new}")
            } else {
                format!("Update winmon {old} to {new}")
            },
            format!("Your settings are kept. Program folder: {dir}"),
        ),
        (Mode::Uninstall, _) => (
            "Remove winmon?".into(),
            format!(
                "This stops winmon and removes its Start menu and sign-in entries \
                 and its program folder: {dir}"
            ),
        ),
    }
}

impl Setup {
    fn new(ui: &mut Ui<Msg>, mode: Mode) -> Setup {
        ui.follow_system_theme(true);
        platform::dialog_frame(ui.hwnd().raw());
        let (heading, body) = heading_and_body(mode);
        let check = |ui: &mut Ui<Msg>, text: &str, on: bool| {
            CheckBox::new(ui, text).expect("check").checked(on)
        };
        let fresh = install::installed_version().is_none();
        let snapshot = Snapshot::arm(ui, || Msg::SnapshotTick);
        let autostart = fresh || install::autostart_enabled();
        let setup = Setup {
            mode,
            autostart: check(ui, "Start winmon when I sign in", autostart),
            start_menu: check(ui, "Add winmon to the Start menu", true),
            launch: check(ui, "Start winmon now", true),
            settings: check(ui, "Open the settings", fresh),
            remove_settings: check(ui, "Also delete my settings and log", false),
            status: Label::new(ui, Rect::default(), "").expect("label"),
            go: Button::new(
                ui,
                if mode == Mode::Install {
                    "Install"
                } else {
                    "Uninstall"
                },
            )
            .expect("button")
            .default()
            .on_click(|| Some(Msg::Go)),
            cancel: Button::new(ui, "Cancel")
                .expect("button")
                .on_click(|| Some(Msg::Cancel)),
            _text: [
                Label::new(ui, Rect::default(), &heading).expect("label"),
                Label::new(ui, Rect::default(), &body).expect("label"),
            ],
            snapshot,
        };
        if mode == Mode::Install && !fresh {
            setup.go.set_text("Update");
        }

        let options = match mode {
            Mode::Install => column![
                setup.autostart.height(dip(24.0)),
                setup.start_menu.height(dip(24.0)),
                setup.launch.height(dip(24.0)),
                setup.settings.height(dip(24.0)),
            ],
            Mode::Uninstall => column![setup.remove_settings.height(dip(24.0))],
        };
        for c in [
            &setup.autostart,
            &setup.start_menu,
            &setup.launch,
            &setup.settings,
        ] {
            c.set_visible(mode == Mode::Install);
        }
        setup.remove_settings.set_visible(mode == Mode::Uninstall);

        ui.set_layout(
            column![
                setup._text[0].height(dip(24.0)),
                setup._text[1].height(dip(64.0)),
                options.spacing(dip(4.0)).height(dip(108.0)),
                setup.status.fill(1),
                Layout::row()
                    .item(Layout::row().fill(1))
                    .item(setup.go.width(dip(96.0)))
                    .item(Layout::row().width(dip(8.0)))
                    .item(setup.cancel.width(dip(96.0)))
                    .height(dip(28.0)),
            ]
            .spacing(dip(10.0))
            .margins(Insets::all(dip(16.0))),
        );
        setup
    }

    fn options(&self) -> Options {
        Options {
            autostart: self.autostart.is_checked(),
            start_menu: self.start_menu.is_checked(),
        }
    }
}

impl App for Setup {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        match msg {
            Msg::Go => {
                self.go.set_enabled(false);
                self.cancel.set_enabled(false);
                let proxy = ui.proxy();
                match self.mode {
                    Mode::Install => {
                        self.status.set_text("Installing…");
                        let options = self.options();
                        std::thread::spawn(move || {
                            let r = install::install(options)
                                .map(Some)
                                .map_err(|e| format!("{e:#}"));
                            let _ = proxy.send(Msg::Done(r));
                        });
                    }
                    Mode::Uninstall => {
                        self.status.set_text("Removing…");
                        let remove_settings = self.remove_settings.is_checked();
                        std::thread::spawn(move || {
                            let r = install::uninstall(remove_settings)
                                .map(|()| None)
                                .map_err(|e| format!("{e:#}"));
                            let _ = proxy.send(Msg::Done(r));
                        });
                    }
                }
            }
            Msg::Done(Ok(Some(exe))) => {
                let mut errors = Vec::new();
                if self.launch.is_checked() {
                    errors.extend(install::launch(&exe, &[]).err());
                }
                if self.settings.is_checked() {
                    errors.extend(install::launch(&exe, &["--settings"]).err());
                }
                match errors.first() {
                    Some(e) => self.fail(&format!("Installed, but {e:#}")),
                    None => ui.close(),
                }
            }
            Msg::Done(Ok(None)) => {
                self.status.set_text("winmon has been removed.");
                self.go.set_visible(false);
                self.cancel.set_text("Close");
                self.cancel.set_enabled(true);
            }
            Msg::Done(Err(e)) => self.fail(&e),
            Msg::Cancel => ui.close(),
            Msg::SnapshotTick => Snapshot::on_tick(&mut self.snapshot, ui),
        }
    }
}

impl Setup {
    fn fail(&self, error: &str) {
        self.status.set_text(error);
        self.go.set_enabled(true);
        self.cancel.set_enabled(true);
        self.cancel.set_text("Close");
    }
}
