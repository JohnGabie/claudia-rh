//! Tauri commands of the Sites screen. Each command is a thin wrapper over a function on `&Connection`
//! so the rules stay testable without a window.
use crate::sites::avisos::{self, Aviso};
use crate::sites::repo::{self, Site};
use crate::sites::stats::{self, SiteStats};
use crate::DbState;
use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State};

pub(crate) fn do_listar(c: &Connection) -> Result<Vec<Site>, String> { repo::listar(c) }
pub(crate) fn do_definir_ia(c: &Connection, chave: &str, valor: bool) -> Result<(), String> { repo::definir_ia_pode_usar(c, chave, valor) }
pub(crate) fn do_marcar_logado(c: &Connection, chave: &str) -> Result<(), String> { repo::marcar_logada(c, chave) }

#[tauri::command]
pub fn listar_sites(state: State<'_, DbState>) -> Result<Vec<Site>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    do_listar(&conn)
}

#[tauri::command]
pub fn definir_ia_pode_usar(state: State<'_, DbState>, chave: String, valor: bool) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    do_definir_ia(&conn, &chave, valor)
}

#[tauri::command]
pub fn marcar_site_logado(app: AppHandle, state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    do_marcar_logado(&conn, &chave)?;
    // The Pendências list and the sidebar badge listen to this event.
    let _ = app.emit("aviso-site-resolvido", chave);
    Ok(())
}

#[tauri::command]
pub fn adicionar_site(state: State<'_, DbState>, nome: String, url: String, tipo: String, pais: Option<String>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repo::adicionar(&conn, &nome, &url, &tipo, pais.as_deref())
}

#[tauri::command]
pub fn remover_site(state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repo::remover(&conn, &chave)
}

#[tauri::command]
pub fn estatisticas_site(state: State<'_, DbState>, chave: String) -> Result<SiteStats, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    stats::estatisticas(&conn, &chave)
}

#[tauri::command]
pub fn listar_avisos_sites(state: State<'_, DbState>) -> Result<Vec<Aviso>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    avisos::listar_abertos(&conn)
}

#[tauri::command]
pub fn contar_avisos_sites(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    avisos::contar(&conn)
}

/// "Open" button: starts the Claudia browser if needed and opens the site's entry page in a new tab,
/// so the user can log in by hand. The app never sees a password. Takes only the site KEY: the URL
/// always comes from the database, never from the frontend.
#[tauri::command]
pub async fn abrir_site(app: AppHandle, state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let url = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        repo::obter(&conn, &chave)?.ok_or_else(|| format!("site '{chave}' não existe"))?.url_login
    };
    crate::browser::service::open_tab_when_ready(&app, &url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        crate::sites::seed::merge_embedded(&c).unwrap();
        c
    }

    #[test]
    fn the_embedded_seed_shows_up_in_the_list_closed() {
        let c = db();
        let all = do_listar(&c).unwrap();
        assert!(all.len() >= 9 && all.iter().all(|s| !s.ia_pode_usar));
    }

    #[test]
    fn confirming_the_login_clears_the_notice_and_unlocks_the_permission() {
        let c = db();
        crate::sites::repo::marcar_precisa_login(&c, "linkedin").unwrap();
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        assert!(do_definir_ia(&c, "linkedin", true).is_err());
        do_marcar_logado(&c, "linkedin").unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 0);
        do_definir_ia(&c, "linkedin", true).unwrap();
    }
}
