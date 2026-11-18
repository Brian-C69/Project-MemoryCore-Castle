//! Handing something to Windows Explorer so its window comes up in front.
//!
//! Windows only lets a program bring a window to the front if it has been allowed to. Castle
//! starts `explorer.exe`, which usually passes the job to the Explorer already running, and that one
//! has no such right - so the folder opened behind Castle. Before starting Explorer, Castle (the
//! program the member just clicked in) says any program may now take the front.

/// Allows the program Castle is about to start to come to the front. Call it right before starting
/// Explorer for something the member just asked to see.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn let_it_come_forward() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
        // A failure only means the window may open behind, as it did before; nothing to report.
        unsafe {
            AllowSetForegroundWindow(ASFW_ANY);
        }
    }
}

/// A notification click starts a second Castle with Castle's own address (memorycore-castle://...).
/// Windows lets THAT process, started by the member's click, bring a window to the front - not the
/// Castle that is already running, which only gets its taskbar button flashed when it tries. So the new
/// process does it itself, before it hands the request over: it finds the running Castle's window,
/// opens it if it is hidden or minimised and brings it forward. Returns whether a window was found.
#[cfg(windows)]
pub fn bring_running_castle_forward() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, FindWindowW, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, ASFW_ANY, SW_RESTORE, SW_SHOW,
    };
    // the main window's title (tauri.conf.json); the WORLD and identify windows have longer ones
    let title: Vec<u16> = "MemoryCore Castle\0".encode_utf16().collect();
    // SAFETY: plain Win32 calls with a NUL-terminated title and a window handle from FindWindowW.
    unsafe {
        let window = FindWindowW(std::ptr::null(), title.as_ptr());
        if window.is_null() {
            return false;
        }
        // let the running Castle take the front as well when it shows itself a moment later
        AllowSetForegroundWindow(ASFW_ANY);
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        } else if IsWindowVisible(window) == 0 {
            ShowWindow(window, SW_SHOW);
        }
        SetForegroundWindow(window);
        true
    }
}

#[cfg(not(windows))]
pub fn bring_running_castle_forward() -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn allowing_the_front_does_not_fail_or_panic() {
        super::let_it_come_forward();
    }
}
