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

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn allowing_the_front_does_not_fail_or_panic() {
        super::let_it_come_forward();
    }
}
