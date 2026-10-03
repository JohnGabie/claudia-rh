// Tauri commands for the dev-mode Activity log. The app's own UI is the only reader: no MCP tool
// exposes any of this to an AI.
use base64::Engine;
use std::path::Path;
use tauri::{AppHandle, Manager};

use crate::activity_log::{self, shots, ActivityLog, EventRow, Filter};

fn log_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("dev"))
}

/// The image at `rel` (a path stored in the log) as a data URL, or an error if the path is not
/// inside the captures folder.
pub(crate) fn image_data_url(dir: &Path, rel: &str) -> Result<String, String> {
    let path = shots::resolve(dir, rel)?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn activity_dev_mode_get(app: AppHandle) -> bool {
    app.path().app_data_dir().map(|d| activity_log::is_enabled(&d)).unwrap_or(false)
}

#[tauri::command]
pub fn activity_dev_mode_set(app: AppHandle, on: bool) -> Result<(), String> {
    let d = app.path().app_data_dir().map_err(|e| e.to_string())?;
    activity_log::set_enabled(&d, on)
}

#[tauri::command]
pub fn activity_log_list(
    app: AppHandle,
    session_id: Option<i64>,
    vaga_id: Option<i64>,
    kind: Option<String>,
) -> Result<Vec<EventRow>, String> {
    // Never create the log folder just because the screen was opened.
    let dir = log_dir(&app)?;
    if !dir.join("activity-log.db").exists() {
        return Ok(vec![]);
    }
    ActivityLog::open(&dir)?.list(&Filter { session_id, vaga_id, kind, limit: 500 })
}

#[tauri::command]
pub fn activity_log_get(app: AppHandle, id: i64) -> Result<Option<EventRow>, String> {
    let dir = log_dir(&app)?;
    if !dir.join("activity-log.db").exists() {
        return Ok(None);
    }
    ActivityLog::open(&dir)?.get(id)
}

#[tauri::command]
pub fn activity_log_image(app: AppHandle, path: String) -> Result<String, String> {
    image_data_url(&log_dir(&app)?, &path)
}

#[tauri::command]
pub fn activity_log_clear(app: AppHandle) -> Result<(), String> {
    let dir = log_dir(&app)?;
    if !dir.join("activity-log.db").exists() {
        return Ok(());
    }
    ActivityLog::open(&dir)?.clear()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-img-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(d.join("capturas").join("20261002")).unwrap();
        d
    }

    #[test]
    fn a_valid_capture_becomes_a_jpeg_data_url() {
        let d = temp("ok");
        std::fs::write(d.join("capturas/20261002/1-before.jpg"), b"JPEGDATA").unwrap();
        let url = image_data_url(&d, "capturas/20261002/1-before.jpg").unwrap();
        assert_eq!(url, format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(b"JPEGDATA")));
    }

    #[test]
    fn paths_outside_the_captures_folder_are_refused_even_if_the_file_exists() {
        let d = temp("bad");
        std::fs::write(d.join("activity-log.db"), b"secret rows").unwrap();
        std::fs::write(d.join("capturas/20261002/1-before.jpg"), b"x").unwrap();
        for bad in ["../x.jpg", "activity-log.db", "capturas/../activity-log.db", "/etc/passwd", "C:/Windows/win.ini"] {
            assert!(image_data_url(&d, bad).is_err(), "{bad}");
        }
        assert!(image_data_url(&d, "capturas/20261002/missing.jpg").is_err());
    }
}
