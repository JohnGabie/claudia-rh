use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use tauri::{AppHandle, Emitter};

/// Watches the SQLite DB every 5 seconds for changes made by the claude session
/// (which writes via the sqlite3 CLI, outside of Tauri's connection).
/// Emits `db-atualizada` to the frontend whenever the vagas or candidaturas tables change.
pub fn start(app: AppHandle, db: Arc<Mutex<Connection>>) {
    tauri::async_runtime::spawn(async move {
        let mut last_marker = String::new();

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

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
                format!("{}-{}-{}", v_ts, c_count, p_resolved)
            };

            if marker != last_marker {
                last_marker = marker;
                let _ = app.emit("db-atualizada", ());
            }
        }
    });
}
