use crate::models::LayoutOrientation;
use tauri::{AppHandle, LogicalSize, Manager, Size};

const PORTRAIT: (f64, f64) = (440.0, 700.0);
const LANDSCAPE: (f64, f64) = (920.0, 560.0);

pub fn apply_layout(app: &AppHandle, layout: &LayoutOrientation) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let (width, height) = match layout {
        LayoutOrientation::Portrait => PORTRAIT,
        LayoutOrientation::Landscape => LANDSCAPE,
    };
    let size = Size::Logical(LogicalSize { width, height });
    window.set_min_size(Some(size)).map_err(|error| error.to_string())?;
    window.set_max_size(Some(size)).map_err(|error| error.to_string())?;
    window.set_size(size).map_err(|error| error.to_string())?;
    window.center().map_err(|error| error.to_string())?;
    Ok(())
}
