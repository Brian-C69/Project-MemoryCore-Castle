//! Castle in the tray: the icon, its menu, and the one way the window is shown or hidden.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

const MAIN_WINDOW: &str = "main";
/// Tells the page to start the companion's session; the page does it, because it must be listening first.
pub const WAKE_EVENT: &str = "castle-wake";

/// Brings Castle to the front, whatever state it was in.
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    // the WORLD on the other screens comes back with Castle
    crate::world::set_visible(app, true);
}

/// Hides Castle when it is in front, otherwise brings it forward.
pub fn toggle_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let in_front = window.is_visible().unwrap_or(false)
            && !window.is_minimized().unwrap_or(false)
            && window.is_focused().unwrap_or(false);
        if in_front {
            let _ = window.hide();
            crate::world::set_visible(app, false);
        } else {
            show_main(app);
        }
    }
}

/// Quits at once when no session is running; asks first when one is.
pub fn quit(app: &AppHandle) {
    if !crate::sessions_running(app) {
        app.exit(0);
        return;
    }
    show_main(app);
    let handle = app.clone();
    app.dialog()
        .message("Your companion's session will end with Castle.")
        .title("Quit MemoryCore Castle?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Quit".into(), "Stay".into()))
        .show(move |quit| {
            if quit {
                handle.exit(0);
            }
        });
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Castle", true, None::<&str>)?;
    let wake = MenuItem::with_id(app, "wake", "Wake companion", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &wake, &quit_item])?;

    let mut tray = TrayIconBuilder::with_id("castle")
        .tooltip("MemoryCore Castle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "wake" => {
                show_main(app);
                let _ = app.emit(WAKE_EVENT, ());
            }
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
