//! How much of the screen Castle takes. Every sizing decision lives here.

use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, Result, WebviewWindow};

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

/// Fills one given monitor's work area (Castle placed on the screen the member chose).
pub fn fill_work_area(window: &WebviewWindow, monitor: &Monitor) -> Result<()> {
    let area = monitor.work_area();
    window.set_position(PhysicalPosition::new(area.position.x, area.position.y))?;
    window.set_size(PhysicalSize::new(area.size.width, area.size.height))?;
    Ok(())
}

/// Fills the monitor's work area: the whole desktop, with the Windows taskbar still showing.
pub fn fill_desktop(window: &WebviewWindow) -> Result<()> {
    if let Some(area) = desktop_area(window)? {
        apply_desktop_area(window, area)?;
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
            if let Ok(Some(area)) = desktop_area(&window) {
                if needs_fit(fitted, area) {
                    match apply_desktop_area(&window, area) {
                        Ok(()) => fitted = Some(area),
                        Err(error) => eprintln!("MemoryCore Castle: desktop size could not be refreshed: {error}"),
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
