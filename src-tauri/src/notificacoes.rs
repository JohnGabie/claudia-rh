use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use tauri::{AppHandle, Emitter};

fn query_pending_ids(db: &Arc<Mutex<Connection>>) -> Vec<i64> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare("SELECT id FROM pendencias WHERE resolvida = 0") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| r.get::<_, i64>(0))
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Polls unresolved pendências every 30 seconds and emits `nova-pendencia` whenever
/// a new one appears. No Windows toast notifications are sent.
pub fn start(app: AppHandle, db: Arc<Mutex<Connection>>) {
    tauri::async_runtime::spawn(async move {
        let mut seen: HashSet<i64> = HashSet::new();

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let ids = query_pending_ids(&db);
            let current: HashSet<i64> = ids.into_iter().collect();

            // Remove resolved items from seen
            seen.retain(|id| current.contains(id));

            // Emit for each new item
            for &id in &current {
                if !seen.contains(&id) {
                    seen.insert(id);
                    let _ = app.emit("nova-pendencia", id);
                }
            }
        }
    });
}
