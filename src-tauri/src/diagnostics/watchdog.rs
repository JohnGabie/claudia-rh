use rusqlite::Connection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

use super::emit_event;

static LAST_TICK_MS: AtomicU64 = AtomicU64::new(0);

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn tick() {
    LAST_TICK_MS.store(now_millis(), Ordering::Relaxed);
}

#[tauri::command]
pub fn diag_heartbeat() {
    tick();
}

pub fn watchdog_transition(
    session_active: bool,
    last_tick_ms: u64,
    now_ms: u64,
    already_missing: bool,
) -> Option<&'static str> {
    if !session_active {
        return None;
    }
    let stale = now_ms.saturating_sub(last_tick_ms) >= 60_000;
    match (stale, already_missing) {
        (true, false) => Some("heartbeat_miss"),
        (false, true) => Some("recovered"),
        _ => None,
    }
}

pub fn start(_app: AppHandle, db: Arc<Mutex<Connection>>) {
    tick();
    tauri::async_runtime::spawn(async move {
        let mut already_missing = false;
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let session_active = {
                let conn = match db.lock() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sessoes WHERE terminada_em IS NULL",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);
                count > 0
            };

            if !session_active {
                already_missing = false;
                continue;
            }

            let now_ms = now_millis();
            let last_tick_ms = LAST_TICK_MS.load(Ordering::Relaxed);
            match watchdog_transition(session_active, last_tick_ms, now_ms, already_missing) {
                Some("heartbeat_miss") => {
                    already_missing = true;
                    let duration = now_ms.saturating_sub(last_tick_ms);
                    emit_event("watchdog", "heartbeat_miss", None, &duration.to_string());
                }
                Some("recovered") => {
                    already_missing = false;
                    let duration = now_ms.saturating_sub(last_tick_ms);
                    emit_event("watchdog", "recovered", None, &duration.to_string());
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::watchdog_transition;

    #[test]
    fn active_stale_not_missing_is_heartbeat_miss() {
        assert_eq!(
            watchdog_transition(true, 0, 61_000, false),
            Some("heartbeat_miss")
        );
    }

    #[test]
    fn active_fresh_already_missing_is_recovered() {
        assert_eq!(
            watchdog_transition(true, 61_000, 62_000, true),
            Some("recovered")
        );
    }

    #[test]
    fn inactive_is_none() {
        assert_eq!(watchdog_transition(false, 0, 61_000, false), None);
        assert_eq!(watchdog_transition(false, 0, 61_000, true), None);
    }
}
