use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;

use crate::IdleConfig;
use crate::commands::sessao::iniciar_sessao;
use chrono::{Datelike, Local};

#[cfg(target_os = "windows")]
fn idle_secs() -> u32 {
    use std::mem;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct LASTINPUTINFO {
        cbSize: u32,
        dwTime: u32,
    }

    extern "system" {
        fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
        fn GetTickCount() -> u32;
    }

    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut info) != 0 {
            GetTickCount().wrapping_sub(info.dwTime) / 1000
        } else {
            0
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn idle_secs() -> u32 {
    0
}

fn pode_disparar(db: &Arc<Mutex<rusqlite::Connection>>, config: &IdleConfig) -> bool {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return false,
    };

    // 1. Limite diário de candidaturas
    let cands_hoje: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM candidaturas WHERE date(enviada_em)=date('now')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if cands_hoje >= config.limite_diario as i64 {
        return false;
    }

    // 2. Limite de tempo de sessão (0 = sem limite)
    if config.limite_tempo_minutos > 0 {
        let mins: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(
                    (julianday(COALESCE(terminada_em, datetime('now'))) - julianday(iniciada_em)) * 1440
                 ), 0.0) FROM sessoes WHERE date(iniciada_em) = date('now')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        if mins >= config.limite_tempo_minutos as f64 {
            return false;
        }
    }

    // 3. Janela de agendamento (se não houver janelas, sempre permitido)
    if !config.janelas.is_empty() {
        let agora = Local::now();
        let dia = agora.weekday().num_days_from_sunday() as u8;
        let hhmm = agora.format("%H:%M").to_string();
        let dentro = config.janelas.iter().any(|j| {
            j.ativo && j.dia_semana == dia && hhmm >= j.inicio && hhmm < j.fim
        });
        if !dentro {
            return false;
        }
    }

    true
}

/// Spawns the background idle-watcher task.
/// Polls every 30 seconds. Only fires once per idle session (resets when the
/// user becomes active again, i.e. idle time drops below 10 s).
pub fn start(
    app: AppHandle,
    idle_config: Arc<Mutex<IdleConfig>>,
    db: Arc<Mutex<rusqlite::Connection>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut fired_this_idle = false;

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let cfg_snapshot = {
                let cfg = idle_config.lock().unwrap();
                cfg.clone()
            };
            let ativo = cfg_snapshot.ativo;
            let limiar_secs = cfg_snapshot.limiar_minutos * 60;

            let secs = idle_secs();

            if secs < 10 {
                fired_this_idle = false;
                continue;
            }

            if !ativo || fired_this_idle {
                continue;
            }

            if secs >= limiar_secs && pode_disparar(&db, &cfg_snapshot) {
                fired_this_idle = true;
                let _ = iniciar_sessao(Arc::clone(&db), &app, "inatividade");
            }
        }
    });
}
