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
