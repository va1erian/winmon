//! Start menu shortcuts (`.lnk` files) through the shell's `IShellLinkW`.

use std::path::Path;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::core::{HSTRING, Interface};

/// Writes a shortcut at `lnk` that runs `target args`.
pub fn create_shortcut(
    lnk: &Path,
    target: &Path,
    args: &str,
    description: &str,
) -> windows::core::Result<()> {
    let dir = target.parent().unwrap_or(target);
    // SAFETY: COM is initialised on this thread before use (a repeat or a
    // different mode is fine: the shell link object is apartment-neutral
    // in-proc); every string outlives the call it's passed to.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(target.as_os_str()))?;
        link.SetArguments(&HSTRING::from(args))?;
        link.SetDescription(&HSTRING::from(description))?;
        link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        link.SetIconLocation(&HSTRING::from(target.as_os_str()), 0)?;
        link.cast::<IPersistFile>()?
            .Save(&HSTRING::from(lnk.as_os_str()), true)
    }
}
