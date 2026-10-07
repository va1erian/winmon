//! Raw Win32 calls win32ui doesn't cover (yet). Everything `unsafe` in the
//! app lives here, behind safe functions.

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    BELOW_NORMAL_PRIORITY_CLASS, CreateEventW, CreateMutexW, GetCurrentProcess, INFINITE,
    OpenEventW, PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
    PROCESS_POWER_THROTTLING_STATE, ProcessPowerThrottling, SYNCHRONIZATION_ACCESS_RIGHTS,
    SetEvent, SetPriorityClass, SetProcessInformation, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, WM_POWERBROADCAST,
};
use windows::core::w;

/// EcoQoS plus below-normal priority: the dashboard never needs to be fast.
pub fn lower_priority() {
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
    };
    // SAFETY: the pseudo-handle is always valid; `state` is the documented
    // struct for ProcessPowerThrottling and outlives the call.
    unsafe {
        let process = GetCurrentProcess();
        let _ = SetProcessInformation(
            process,
            ProcessPowerThrottling,
            (&raw const state).cast(),
            size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
        let _ = SetPriorityClass(process, BELOW_NORMAL_PRIORITY_CLASS);
    }
}

/// The single-instance guard. Held for the process lifetime.
pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // SAFETY: we own this handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// `None` when another winmon already runs in this session.
pub fn single_instance() -> Option<InstanceGuard> {
    // SAFETY: plain named-mutex creation; the last-error read is immediate.
    unsafe {
        let handle = CreateMutexW(None, false, w!("Local\\winmon")).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            return None;
        }
        Some(InstanceGuard(handle))
    }
}

/// Waits (on a small thread) for `winmon --quit` and calls `on_quit` once.
pub fn listen_for_quit(on_quit: impl FnOnce() + Send + 'static) {
    // SAFETY: plain named auto-reset event creation.
    let Ok(event) = (unsafe { CreateEventW(None, false, false, w!("Local\\winmon-quit")) }) else {
        return;
    };
    let event = event.0 as usize;
    let _ = std::thread::Builder::new()
        .name("quit".into())
        .stack_size(64 * 1024)
        .spawn(move || {
            // SAFETY: the event handle stays open for the process lifetime.
            if unsafe { WaitForSingleObject(HANDLE(event as _), INFINITE) } == WAIT_OBJECT_0 {
                on_quit();
            }
        });
}

/// Asks a running instance to exit. Returns whether one was listening.
pub fn signal_quit() -> bool {
    const EVENT_MODIFY_STATE: u32 = 0x0002;
    // SAFETY: open + set + close a named event we don't otherwise hold.
    unsafe {
        let Ok(event) = OpenEventW(
            SYNCHRONIZATION_ACCESS_RIGHTS(EVENT_MODIFY_STATE),
            false,
            w!("Local\\winmon-quit"),
        ) else {
            return false;
        };
        let ok = SetEvent(event).is_ok();
        let _ = CloseHandle(event);
        ok
    }
}

/// Whether the raw message `msg` (a `*const MSG`, as given by
/// `Ui::on_raw_message`) reports a resume from sleep.
pub fn is_resume(msg: *const std::ffi::c_void) -> bool {
    if msg.is_null() {
        return false;
    }
    // SAFETY: win32ui passes a pointer to a live MSG for the call's duration.
    let msg = unsafe { &*(msg as *const MSG) };
    msg.message == WM_POWERBROADCAST
        && matches!(
            msg.wParam.0 as u32,
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND
        )
}

/// Lets a GUI-subsystem release build print `--list-*` output to the console
/// it was started from.
pub fn attach_parent_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // SAFETY: no preconditions; failure (no parent console) is fine.
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
