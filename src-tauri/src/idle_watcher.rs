use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;

use crate::IdleConfig;
use crate::commands::sessao::iniciar_sessao;

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

fn tem_orcamento(db: &Arc<Mutex<rusqlite::Connection>>) -> bool {
    const BUDGET: i64 = 10;
    if let Ok(conn) = db.lock() {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM candidaturas WHERE date(enviada_em)=date('now')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        count < BUDGET
    } else {
        false
    }
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

            let (ativo, limiar_secs) = {
                let cfg = idle_config.lock().unwrap();
                (cfg.ativo, cfg.limiar_minutos * 60)
            };

            let secs = idle_secs();

            if secs < 10 {
                fired_this_idle = false;
                continue;
            }

            if !ativo || fired_this_idle {
                continue;
            }

            if secs >= limiar_secs && tem_orcamento(&db) {
                fired_this_idle = true;
                let _ = iniciar_sessao(Arc::clone(&db), &app, "inatividade");
            }
        }
    });
}
