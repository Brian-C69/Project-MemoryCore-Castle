//! How much of the screen Castle takes. Every sizing decision lives here.

use tauri::{Monitor, PhysicalPosition, PhysicalSize, Result, WebviewWindow};

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
    let monitor = match window.current_monitor()? {
        Some(monitor) => Some(monitor),
        None => window.primary_monitor()?,
    };
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        window.set_position(PhysicalPosition::new(area.position.x, area.position.y))?;
        window.set_size(PhysicalSize::new(area.size.width, area.size.height))?;
    }
    Ok(())
}
