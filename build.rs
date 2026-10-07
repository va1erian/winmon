//! Embeds `winmon.rc`: the application manifest (Common Controls v6 for the
//! settings and setup windows, per-monitor-v2 DPI, `asInvoker` so a file named
//! `winmon-setup.exe` isn't auto-elevated) and the version resource that
//! Windows shows in file properties and Settings → Apps.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=winmon.rc");
    println!("cargo:rerun-if-changed=winmon.manifest");
    // rc.exe's command-line defines can't carry a quoted string reliably, so
    // the version is substituted into a generated copy of the script.
    let env = |k: &str| std::env::var(k).expect(k);
    let dir = PathBuf::from(env("CARGO_MANIFEST_DIR"));
    let manifest = dir.join("winmon.manifest").display().to_string();
    let rc = std::fs::read_to_string(dir.join("winmon.rc"))
        .expect("read winmon.rc")
        .replace("@MANIFEST@", &manifest.replace('\\', "/"))
        .replace("@MAJOR@", &env("CARGO_PKG_VERSION_MAJOR"))
        .replace("@MINOR@", &env("CARGO_PKG_VERSION_MINOR"))
        .replace("@PATCH@", &env("CARGO_PKG_VERSION_PATCH"))
        .replace("@VERSION@", &env("CARGO_PKG_VERSION"));
    let out = PathBuf::from(env("OUT_DIR")).join("winmon.rc");
    std::fs::write(&out, rc).expect("write generated winmon.rc");
    embed_resource::compile(&out, embed_resource::NONE)
        .manifest_required()
        .expect("compile winmon.rc");
}
