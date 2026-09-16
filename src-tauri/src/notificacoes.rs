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

fn query_proposta_ids(db: &Arc<Mutex<Connection>>) -> Vec<i64> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare("SELECT id FROM propostas_perfil WHERE promovida = 0") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| r.get::<_, i64>(0))
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotifyDecision {
    pub send_native: bool,
    pub emit_event: bool,
}

pub fn decide_notify(
    first_seen: bool,
    still_open: bool,
    last_native: Option<std::time::Instant>,
    now: std::time::Instant,
    interval: std::time::Duration,
    native_enabled: bool,
) -> NotifyDecision {
    if !still_open {
        return NotifyDecision {
            send_native: false,
            emit_event: false,
        };
    }
    if first_seen {
        return NotifyDecision {
            emit_event: true,
            send_native: native_enabled,
        };
    }
    if !native_enabled {
        return NotifyDecision {
            send_native: false,
            emit_event: false,
        };
    }
    let due = match last_native {
        None => true,
        Some(t) => now.duration_since(t) >= interval,
    };
    NotifyDecision {
        send_native: due,
        emit_event: false,
    }
}

pub fn start(app: AppHandle, db: Arc<Mutex<Connection>>) {
    tauri::async_runtime::spawn(async move {
        let mut seen_pendencias: HashSet<i64> = HashSet::new();
        let mut seen_propostas: HashSet<i64> = HashSet::new();

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            // Pendências
            let pend_ids: HashSet<i64> = query_pending_ids(&db).into_iter().collect();
            seen_pendencias.retain(|id| pend_ids.contains(id));
            for &id in &pend_ids {
                if !seen_pendencias.contains(&id) {
                    seen_pendencias.insert(id);
                    let _ = app.emit("nova-pendencia", id);
                }
            }

            // Propostas de perfil
            let prop_ids: HashSet<i64> = query_proposta_ids(&db).into_iter().collect();
            seen_propostas.retain(|id| prop_ids.contains(id));
            for &id in &prop_ids {
                if !seen_propostas.contains(&id) {
                    seen_propostas.insert(id);
                    let _ = app.emit("nova-proposta", id);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn first_seen_enabled_emits_and_sends() {
        let now = t0();
        let d = decide_notify(true, true, None, now, Duration::from_secs(600), true);
        assert_eq!(
            d,
            NotifyDecision {
                send_native: true,
                emit_event: true
            }
        );
    }

    #[test]
    fn first_seen_disabled_emits_but_no_native() {
        let now = t0();
        let d = decide_notify(true, true, None, now, Duration::from_secs(600), false);
        assert_eq!(
            d,
            NotifyDecision {
                send_native: false,
                emit_event: true
            }
        );
    }

    #[test]
    fn repeat_after_interval_native_only() {
        let t0 = t0();
        let later = t0 + Duration::from_secs(600);
        let d = decide_notify(false, true, Some(t0), later, Duration::from_secs(600), true);
        assert_eq!(
            d,
            NotifyDecision {
                send_native: true,
                emit_event: false
            }
        );
    }

    #[test]
    fn inside_interval_sends_nothing() {
        let t0 = t0();
        let later = t0 + Duration::from_secs(60);
        let d = decide_notify(false, true, Some(t0), later, Duration::from_secs(600), true);
        assert_eq!(
            d,
            NotifyDecision {
                send_native: false,
                emit_event: false
            }
        );
    }

    #[test]
    fn closed_sends_nothing() {
        let now = t0();
        let d = decide_notify(true, false, None, now, Duration::from_secs(600), true);
        assert_eq!(
            d,
            NotifyDecision {
                send_native: false,
                emit_event: false
            }
        );
    }

    #[test]
    fn not_first_seen_never_toasted_sends_native_if_enabled() {
        let now = t0();
        let d = decide_notify(false, true, None, now, Duration::from_secs(600), true);
        assert_eq!(
            d,
            NotifyDecision {
                send_native: true,
                emit_event: false
            }
        );
    }
}
