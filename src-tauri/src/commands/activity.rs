// Tauri commands for the Activity log (always on; the app records without any setting). The app's own UI is the only reader: no MCP tool
// exposes any of this to an AI.
use base64::Engine;
use std::path::Path;
use tauri::{AppHandle, Manager};

use crate::activity_log::{shots, ActivityLog, EventRow, Filter};

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

/// Rows per page: enough to read, small enough that a long log never loads all at once. The images are
/// NOT part of a page; the screen asks for them one row at a time (activity_log_image).
pub const PAGE_MAX: i64 = 200;
pub const PAGE_DEFAULT: i64 = 100;

pub(crate) fn page_size(limit: Option<i64>) -> i64 {
    limit.unwrap_or(PAGE_DEFAULT).clamp(1, PAGE_MAX)
}

/// Read-only: the screen only ever reads the log (the app trims it by itself, 30 days / 1 GB).
#[tauri::command]
pub fn activity_log_list(
    app: AppHandle,
    session_id: Option<i64>,
    vaga_id: Option<i64>,
    kind: Option<String>,
    before_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<EventRow>, String> {
    // Never create the log folder just because the screen was opened.
    let dir = log_dir(&app)?;
    if !dir.join("activity-log.db").exists() {
        return Ok(vec![]);
    }
    ActivityLog::open(&dir)?.list(&Filter { session_id, vaga_id, kind, before_id, limit: page_size(limit) })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-img-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(d.join("capturas").join("20261002")).unwrap();
        d
    }

    #[test]
    fn the_page_size_is_bounded_so_a_long_log_never_loads_at_once() {
        assert_eq!(page_size(None), PAGE_DEFAULT);
        assert_eq!(page_size(Some(50)), 50);
        assert_eq!(page_size(Some(1_000_000)), PAGE_MAX);
        assert_eq!(page_size(Some(0)), 1);
        assert_eq!(page_size(Some(-5)), 1);
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
