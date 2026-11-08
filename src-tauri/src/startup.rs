//! Starting with Windows: the sign-in entry, and knowing when this launch came from it.
//! Off by default; the member turns it on in setup or Settings.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// The argument the sign-in entry passes, so Castle knows Windows started it.
pub const SIGN_IN_ARG: &str = "--autostart";

pub fn is_sign_in_launch<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().any(|arg| arg.as_ref() == SIGN_IN_ARG)
}

/// The companion's session starts by itself only when Windows started Castle and the switch is on.
pub fn should_auto_wake(at_sign_in: bool, start_session_on_launch: bool) -> bool {
    at_sign_in && start_session_on_launch
}

/// Makes the Windows sign-in entry match the setting. Returns a plain note when it was not applied.
pub fn sync(app: &AppHandle, wanted: bool) -> Option<String> {
    // A test build lives in target\debug; pointing Windows at it would leave a broken entry behind.
    if cfg!(debug_assertions) {
        return Some("Start with Windows takes effect in the installed Castle, not in this test build.".into());
    }
    let launcher = app.autolaunch();
    if launcher.is_enabled().ok() == Some(wanted) {
        return None;
    }
    let result = if wanted { launcher.enable() } else { launcher.disable() };
    result.err().map(|_| "Windows did not accept the change to start-up.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sign_in_argument_is_recognised() {
        assert!(is_sign_in_launch(["memorycore-castle.exe", "--autostart"]));
        assert!(!is_sign_in_launch(["memorycore-castle.exe"]));
        assert!(!is_sign_in_launch(["memorycore-castle.exe", "--autostart-later"]));
    }

    #[test]
    fn the_session_starts_by_itself_only_at_sign_in_with_the_switch_on() {
        assert!(should_auto_wake(true, true));
        assert!(!should_auto_wake(true, false));
        assert!(!should_auto_wake(false, true));
    }
}
