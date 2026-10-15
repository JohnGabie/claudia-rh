use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::NotifConfig;

struct OpenItem {
    id: i64,
    body: String,
}

fn query_open_pendencias(db: &Arc<Mutex<Connection>>) -> Vec<OpenItem> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare(
        "SELECT id, COALESCE(categoria, ''), COALESCE(descricao, '') FROM pendencias WHERE resolvida = 0",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| {
        let id: i64 = r.get(0)?;
        let cat: String = r.get(1)?;
        let desc: String = r.get(2)?;
        let body = match (cat.trim(), desc.trim()) {
            ("", "") => "Tens uma pendência por resolver.".to_string(),
            ("", d) => d.to_string(),
            (c, "") => c.to_string(),
            (c, d) => format!("{c}: {d}"),
        };
        Ok(OpenItem { id, body })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn query_open_propostas(db: &Arc<Mutex<Connection>>) -> Vec<OpenItem> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare(
        "SELECT id, COALESCE(pergunta, '') FROM propostas_perfil WHERE promovida = 0",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| {
        let id: i64 = r.get(0)?;
        let pergunta: String = r.get(1)?;
        let body = if pergunta.trim().is_empty() {
            "Há uma pergunta nova para o teu perfil.".to_string()
        } else {
            pergunta
        };
        Ok(OpenItem { id, body })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn query_open_avisos_sites(db: &Arc<Mutex<Connection>>) -> Vec<OpenItem> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare(
        "SELECT a.id, c.nome FROM avisos_sites a JOIN contas_browser c ON c.chave = a.chave WHERE a.resolvido = 0",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| {
        let id: i64 = r.get(0)?;
        let nome: String = r.get(1)?;
        Ok(OpenItem { id, body: format!("Entre de novo em {nome} para a IA poder usar o site.") })
    })
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

pub fn start(
    app: AppHandle,
    db: Arc<Mutex<Connection>>,
    notif: Arc<Mutex<NotifConfig>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut seen_pendencias: HashSet<i64> = HashSet::new();
        let mut seen_propostas: HashSet<i64> = HashSet::new();
        let mut last_native_pend: HashMap<i64, Instant> = HashMap::new();
        let mut last_native_prop: HashMap<i64, Instant> = HashMap::new();
        let mut seen_avisos: HashSet<i64> = HashSet::new();
        let mut last_native_aviso: HashMap<i64, Instant> = HashMap::new();

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let (native_enabled, interval) = {
                match notif.lock() {
                    Ok(cfg) => (
                        cfg.ativo,
                        Duration::from_secs(cfg.intervalo_minutos.max(1) as u64 * 60),
                    ),
                    Err(_) => continue,
                }
            };
            let now = Instant::now();

            let pend = query_open_pendencias(&db);
            let pend_ids: HashSet<i64> = pend.iter().map(|p| p.id).collect();
            seen_pendencias.retain(|id| pend_ids.contains(id));
            last_native_pend.retain(|id, _| pend_ids.contains(id));
            for item in &pend {
                let first_seen = !seen_pendencias.contains(&item.id);
                let decision = decide_notify(
                    first_seen,
                    true,
                    last_native_pend.get(&item.id).copied(),
                    now,
                    interval,
                    native_enabled,
                );
                if decision.emit_event {
                    seen_pendencias.insert(item.id);
                    let _ = app.emit("nova-pendencia", item.id);
                } else if first_seen {
                    seen_pendencias.insert(item.id);
                }
                if decision.send_native {
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Claudia RH — Pendência")
                        .body(&item.body)
                        .show()
                    {
                        eprintln!("[notif] show pendencia: {e}");
                    }
                    last_native_pend.insert(item.id, now);
                }
            }

            let prop = query_open_propostas(&db);
            let prop_ids: HashSet<i64> = prop.iter().map(|p| p.id).collect();
            seen_propostas.retain(|id| prop_ids.contains(id));
            last_native_prop.retain(|id, _| prop_ids.contains(id));
            for item in &prop {
                let first_seen = !seen_propostas.contains(&item.id);
                let decision = decide_notify(
                    first_seen,
                    true,
                    last_native_prop.get(&item.id).copied(),
                    now,
                    interval,
                    native_enabled,
                );
                if decision.emit_event {
                    seen_propostas.insert(item.id);
                    let _ = app.emit("nova-proposta", item.id);
                } else if first_seen {
                    seen_propostas.insert(item.id);
                }
                if decision.send_native {
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Claudia RH — Perfil")
                        .body(&item.body)
                        .show()
                    {
                        eprintln!("[notif] show proposta: {e}");
                    }
                    last_native_prop.insert(item.id, now);
                }
            }

            let avisos = query_open_avisos_sites(&db);
            let aviso_ids: HashSet<i64> = avisos.iter().map(|p| p.id).collect();
            seen_avisos.retain(|id| aviso_ids.contains(id));
            last_native_aviso.retain(|id, _| aviso_ids.contains(id));
            for item in &avisos {
                let first_seen = !seen_avisos.contains(&item.id);
                let decision = decide_notify(
                    first_seen,
                    true,
                    last_native_aviso.get(&item.id).copied(),
                    now,
                    interval,
                    native_enabled,
                );
                if decision.emit_event {
                    seen_avisos.insert(item.id);
                    let _ = app.emit("novo-aviso-site", item.id);
                } else if first_seen {
                    seen_avisos.insert(item.id);
                }
                if decision.send_native {
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Claudia RH — Site")
                        .body(&item.body)
                        .show()
                    {
                        eprintln!("[notif] show aviso de site: {e}");
                    }
                    last_native_aviso.insert(item.id, now);
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

    fn mem_db() -> Arc<Mutex<Connection>> {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        crate::sites::seed::merge(&c, &crate::sites::seed::parse_seed(r#"{"versao":1,"sites":[{"chave":"a","nome":"Alpha","url_login":"https://a.com/","dominios":["a.com"],"tipo":"geral","regiao":"EU"}]}"#).unwrap()).unwrap();
        Arc::new(Mutex::new(c))
    }

    #[test]
    fn open_site_notices_name_the_site_and_resolved_ones_are_hidden() {
        let db = mem_db();
        crate::sites::avisos::criar(&db.lock().unwrap(), "a").unwrap();
        let items = query_open_avisos_sites(&db);
        assert_eq!(items.len(), 1);
        assert!(items[0].body.contains("Alpha"), "{}", items[0].body);
        crate::sites::avisos::resolver_do_site(&db.lock().unwrap(), "a").unwrap();
        assert!(query_open_avisos_sites(&db).is_empty());
    }

    #[test]
    fn a_missing_notices_table_gives_an_empty_list() {
        let db = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        assert!(query_open_avisos_sites(&db).is_empty());
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
