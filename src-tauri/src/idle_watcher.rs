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
                    - COALESCE(tempo_pausado_segundos, 0) / 60.0
                    - CASE WHEN pausada_em IS NOT NULL AND terminada_em IS NULL
                        THEN (julianday(datetime('now')) - julianday(pausada_em)) * 1440
                        ELSE 0 END
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

#[cfg(test)]
mod tests {
    use super::pode_disparar;
    use crate::{IdleConfig, JanelaAgendamento};
    use chrono::Datelike;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn mem_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply(&conn).unwrap();
        Arc::new(Mutex::new(conn))
    }

    fn cfg_padrao() -> IdleConfig {
        IdleConfig { ativo: true, limiar_minutos: 5, limite_diario: 10, limite_tempo_minutos: 0, janelas: vec![] }
    }

    fn insert_candidaturas_hoje(conn: &Connection, n: u32) {
        conn.execute(
            "INSERT INTO vagas (titulo, empresa, plataforma, url, descoberta_em, status) \
             VALUES ('dev', 'ACME', 'LinkedIn', 'https://idle-test.com', datetime('now'), 'aplicada')",
            [],
        ).unwrap();
        let vid = conn.last_insert_rowid();
        for _ in 0..n {
            conn.execute(
                "INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) \
                 VALUES (?1, datetime('now'), '/tmp', 'chrome')",
                [vid],
            ).unwrap();
        }
    }

    #[test]
    fn pode_disparar_sem_restricoes_retorna_true() {
        assert!(pode_disparar(&mem_db(), &cfg_padrao()));
    }

    #[test]
    fn pode_disparar_bloqueia_quando_limite_diario_atingido() {
        let db = mem_db();
        { let c = db.lock().unwrap(); insert_candidaturas_hoje(&c, 5); }
        assert!(!pode_disparar(&db, &IdleConfig { limite_diario: 5, ..cfg_padrao() }));
    }

    #[test]
    fn pode_disparar_permite_abaixo_do_limite_diario() {
        let db = mem_db();
        { let c = db.lock().unwrap(); insert_candidaturas_hoje(&c, 3); }
        assert!(pode_disparar(&db, &IdleConfig { limite_diario: 10, ..cfg_padrao() }));
    }

    #[test]
    fn pode_disparar_janela_ativa_para_todos_os_dias_retorna_true() {
        let db = mem_db();
        // Cover every day of the week, full day — use "99:99" fim so midnight never misses
        let janelas = (0u8..7)
            .map(|d| JanelaAgendamento { dia_semana: d, inicio: "00:00".into(), fim: "99:99".into(), ativo: true })
            .collect();
        assert!(pode_disparar(&db, &IdleConfig { janelas, ..cfg_padrao() }));
    }

    #[test]
    fn pode_disparar_janelas_todas_inativas_retorna_false() {
        let db = mem_db();
        let janelas = (0u8..7)
            .map(|d| JanelaAgendamento { dia_semana: d, inicio: "00:00".into(), fim: "99:99".into(), ativo: false })
            .collect();
        assert!(!pode_disparar(&db, &IdleConfig { janelas, ..cfg_padrao() }));
    }

    #[test]
    fn pode_disparar_janela_somente_para_outro_dia_retorna_false() {
        let db = mem_db();
        let hoje = chrono::Local::now().weekday().num_days_from_sunday() as u8;
        let outro = (hoje + 1) % 7;
        let janelas = vec![JanelaAgendamento {
            dia_semana: outro,
            inicio: "00:00".into(),
            fim: "99:99".into(),
            ativo: true,
        }];
        assert!(!pode_disparar(&db, &IdleConfig { janelas, ..cfg_padrao() }));
    }
}
