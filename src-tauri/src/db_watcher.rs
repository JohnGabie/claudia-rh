use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use tauri::{AppHandle, Emitter};

/// Polls for changes made by external processes (the claude PTY session, the
/// MCP tool subprocess) and emits `db-atualizada` to the frontend.
///
/// This is the fallback path — MCP tools also push instantly via the localhost
/// notify channel (see `start_mcp_notify_listener` in lib.rs). 2s keeps the
/// worst case snappy without meaningful cost (three trivial queries + a stat).
pub fn start(app: AppHandle, db: Arc<Mutex<Connection>>) {
    let profile_path = {
        use tauri::Manager;
        app.path().app_data_dir().map(|d| d.join("candidate_base.yaml")).ok()
    };

    tauri::async_runtime::spawn(async move {
        let mut last_marker = String::new();

        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let marker = {
                let conn = match db.lock() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let v_ts: String = conn
                    .query_row(
                        "SELECT COALESCE(MAX(descoberta_em), '') FROM vagas",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap_or_default();
                let c_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM candidaturas", [], |r| r.get(0))
                    .unwrap_or(0);
                let p_resolved: i64 = conn
                    .query_row("SELECT COUNT(*) FROM pendencias WHERE resolvida=1", [], |r| r.get(0))
                    .unwrap_or(0);
                // Profile file mtime, so external profile writes refresh the UI too
                let p_mtime = profile_path
                    .as_ref()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                format!("{}-{}-{}-{}", v_ts, c_count, p_resolved, p_mtime)
            };

            if marker != last_marker {
                last_marker = marker;
                let _ = app.emit("db-atualizada", ());
            }
        }
    });
}
