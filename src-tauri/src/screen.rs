//! How much of the screen Castle takes. Every sizing decision lives here.

use crate::settings::{self, SettingsStore};
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, Result, WebviewWindow};

/// How much of its screen Castle takes (Settings' `window_layout`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Full,
    Left,
    Right,
    /// The member sizes the window; Castle only remembers it.
    Free,
}

impl Layout {
    pub fn from_setting(value: &str) -> Self {
        match value {
            "left" => Self::Left,
            "right" => Self::Right,
            "free" => Self::Free,
            _ => Self::Full,
        }
    }
}

fn layout_of(app: &AppHandle) -> (Layout, Option<[i32; 4]>) {
    let s = app.state::<SettingsStore>().get();
    (Layout::from_setting(&s.window_layout), s.window_bounds)
}

/// The part of a work area a layout takes; None for the free layout (the member's own size).
fn target(area: DesktopArea, layout: Layout) -> Option<DesktopArea> {
    let half = area.width / 2;
    match layout {
        Layout::Full => Some(area),
        Layout::Left => Some(DesktopArea { width: half, ..area }),
        Layout::Right => Some(DesktopArea { x: area.x + half as i32, width: area.width - half, ..area }),
        Layout::Free => None,
    }
}

/// The free layout's first size when nothing is saved: two thirds of the work area, centred.
fn first_free(area: DesktopArea) -> DesktopArea {
    let (width, height) = (area.width * 2 / 3, area.height * 5 / 6);
    DesktopArea { x: area.x + ((area.width - width) / 2) as i32, y: area.y + ((area.height - height) / 2) as i32, width, height }
}

/// Puts the main window where its layout says, on the monitor whose work area is `area`.
fn place(window: &WebviewWindow, area: DesktopArea) -> Result<()> {
    let (layout, bounds) = layout_of(window.app_handle());
    window.set_resizable(layout == Layout::Free)?;
    match target(area, layout) {
        Some(part) => apply_desktop_area(window, part),
        None => {
            let free = bounds
                .filter(|b| b[2] > 200 && b[3] > 200)
                .map(|b| DesktopArea { x: b[0], y: b[1], width: b[2] as u32, height: b[3] as u32 })
                .unwrap_or_else(|| first_free(area));
            apply_desktop_area(window, free)
        }
    }
}

/// The layout menu: saves the member's choice and applies it at once.
#[tauri::command]
pub fn castle_window_layout(app: AppHandle, layout: String) -> std::result::Result<(), String> {
    let store = app.state::<SettingsStore>();
    let mut next = store.get();
    next.window_layout = match Layout::from_setting(&layout) {
        Layout::Full => "full",
        Layout::Left => "left",
        Layout::Right => "right",
        Layout::Free => "free",
    }
    .into();
    settings::save(&store.path, &next).map_err(|_| "The window layout could not be saved.".to_string())?;
    *store.current.lock().unwrap() = next;
    let window = app.get_webview_window("main").ok_or("Castle's window is not open.")?;
    fill_desktop(&window).map_err(|_| "Castle's window could not be resized.".to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DesktopArea {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn desktop_area(window: &WebviewWindow) -> Result<Option<DesktopArea>> {
    let monitor = match window.current_monitor()? {
        Some(monitor) => Some(monitor),
        None => window.primary_monitor()?,
    };
    Ok(monitor.map(|monitor| {
        let area = monitor.work_area();
        DesktopArea { x: area.position.x, y: area.position.y, width: area.size.width, height: area.size.height }
    }))
}

fn apply_desktop_area(window: &WebviewWindow, area: DesktopArea) -> Result<()> {
    window.set_position(PhysicalPosition::new(area.x, area.y))?;
    window.set_size(PhysicalSize::new(area.width, area.height))?;
    Ok(())
}

fn needs_fit(previous: Option<DesktopArea>, current: DesktopArea) -> bool {
    current.width > 0 && current.height > 0 && previous != Some(current)
}

/// Fills a whole monitor, taskbar and all: a world window on one of the extra screens.
pub fn fill_monitor(window: &WebviewWindow, monitor: &Monitor) -> Result<()> {
    window.set_position(PhysicalPosition::new(monitor.position().x, monitor.position().y))?;
    window.set_size(PhysicalSize::new(monitor.size().width, monitor.size().height))?;
    Ok(())
}

/// Places Castle on one given monitor's work area (the screen the member chose), in its layout.
pub fn fill_work_area(window: &WebviewWindow, monitor: &Monitor) -> Result<()> {
    let area = monitor.work_area();
    place(window, DesktopArea { x: area.position.x, y: area.position.y, width: area.size.width, height: area.size.height })
}

/// Places Castle on its monitor's work area in its layout: by default the whole desktop, with the
/// Windows taskbar still showing.
pub fn fill_desktop(window: &WebviewWindow) -> Result<()> {
    if let Some(area) = desktop_area(window)? {
        place(window, area)?;
    }
    Ok(())
}

/// Windows can change its work area without resizing our manually placed, borderless window
/// (for example when taskbar auto-hide is toggled). Query fresh monitor bounds, but only move
/// the main window after a change. World windows intentionally keep their full-monitor bounds.
pub fn watch_desktop(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut fitted = None;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let Some(window) = app.get_webview_window("main") else { break };
            if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(true) {
                // Refit on return, even if the work area changed and then changed back.
                fitted = None;
                continue;
            }
            let (layout, bounds) = layout_of(&app);
            if layout == Layout::Free {
                // the member's own size: only remember where they put it
                fitted = None;
                remember_bounds(&app, &window, bounds);
                continue;
            }
            if let Ok(Some(area)) = desktop_area(&window) {
                let Some(part) = target(area, layout) else { continue };
                if needs_fit(fitted, part) {
                    match apply_desktop_area(&window, part) {
                        Ok(()) => fitted = Some(part),
                        Err(error) => eprintln!("MemoryCore Castle: desktop size could not be refreshed: {error}"),
                    }
                }
            }
        }
    });
}

/// The free layout: saves the window's place and size when the member has moved or resized it.
fn remember_bounds(app: &AppHandle, window: &WebviewWindow, saved: Option<[i32; 4]>) {
    let (Ok(at), Ok(size)) = (window.outer_position(), window.outer_size()) else { return };
    let now = [at.x, at.y, size.width as i32, size.height as i32];
    if saved == Some(now) || window.is_maximized().unwrap_or(false) {
        return;
    }
    let store = app.state::<SettingsStore>();
    let mut next = store.get();
    next.window_bounds = Some(now);
    if settings::save(&store.path, &next).is_ok() {
        *store.current.lock().unwrap() = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layout_takes_its_part_of_the_work_area() {
        let wide = DesktopArea { x: 0, y: 25, width: 5120, height: 1415 };
        assert_eq!(target(wide, Layout::Full), Some(wide));
        assert_eq!(target(wide, Layout::Left), Some(DesktopArea { width: 2560, ..wide }));
        assert_eq!(target(wide, Layout::Right), Some(DesktopArea { x: 2560, width: 2560, ..wide }));
        assert_eq!(target(wide, Layout::Free), None);
        // an odd width leaves no gap between the halves
        let odd = DesktopArea { x: -1921, y: 0, width: 1921, height: 1080 };
        let (left, right) = (target(odd, Layout::Left).unwrap(), target(odd, Layout::Right).unwrap());
        assert_eq!(left.width + right.width, 1921);
        assert_eq!(right.x, left.x + left.width as i32);
        assert_eq!(Layout::from_setting(""), Layout::Full, "older settings files keep the whole screen");
        assert_eq!(Layout::from_setting("free"), Layout::Free);
        let first = first_free(wide);
        assert!(first.width < wide.width && first.x > 0);
    }

    #[test]
    fn taskbar_changes_refit_in_both_directions_without_repeating_unchanged_bounds() {
        let shown = DesktopArea { x: 0, y: 0, width: 1920, height: 1040 };
        let hidden = DesktopArea { height: 1080, ..shown };
        assert!(needs_fit(Some(shown), hidden));
        assert!(needs_fit(Some(hidden), shown));
        assert!(!needs_fit(Some(hidden), hidden));
        assert!(needs_fit(None, hidden));
        assert!(!needs_fit(None, DesktopArea { height: 0, ..shown }));
    }

    #[test]
    fn side_taskbars_and_secondary_monitor_origins_are_part_of_the_bounds() {
        let desktop = DesktopArea { x: -1920, y: 0, width: 1920, height: 1080 };
        assert!(needs_fit(Some(desktop), DesktopArea { x: -1880, width: 1880, ..desktop }));
        assert!(needs_fit(Some(desktop), DesktopArea { y: 40, height: 1040, ..desktop }));
        assert!(needs_fit(Some(desktop), DesktopArea { x: 0, ..desktop }));
    }
}
