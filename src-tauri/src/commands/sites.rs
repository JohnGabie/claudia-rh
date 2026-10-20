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
pub(crate) fn do_dispensar(c: &Connection, id: i64) -> Result<Option<String>, String> { avisos::dispensar(c, id) }

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

/// "Dismiss" on a log-in-again notice: resolves only that notice. The site stays red with the AI closed.
/// Emits the same event as `marcar_site_logado` (payload = site key) only when a notice was dismissed.
#[tauri::command]
pub fn dispensar_aviso_site(app: AppHandle, state: State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(chave) = do_dispensar(&conn, id)? {
        let _ = app.emit("aviso-site-resolvido", chave);
    }
    Ok(())
}

/// The user says the site asks for no login (valor = true) or that it asks for login again
/// (valor = false). Going back to "asks for login" marks the site red, closes the AI and raises the
/// relogin notice.
#[tauri::command]
pub fn definir_site_sem_login(app: AppHandle, state: State<'_, DbState>, chave: String, valor: bool) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repo::definir_sem_login(&conn, &chave, valor)?;
    if valor {
        // Turning it on resolves the open re-login notice, like marcar_site_logado does.
        let _ = app.emit("aviso-site-resolvido", chave);
    } else if let Some(id) = open_notice_id(&conn, &chave) {
        // Same event and payload (the notice id) the notifications poller emits for a new notice; the
        // sidebar badge and the Pendências list refresh on it.
        let _ = app.emit("novo-aviso-site", id);
    }
    Ok(())
}

/// Id of the site's open relogin notice, if any.
fn open_notice_id(conn: &Connection, chave: &str) -> Option<i64> {
    avisos::listar_abertos(conn).ok()?.into_iter().find(|a| a.chave == chave).map(|a| a.id)
}

/// "I am no longer logged in": marks the site as needing login and closes the AI permission.
#[tauri::command]
pub fn marcar_site_nao_logado(state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repo::marcar_nao_logado(&conn, &chave)
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
    fn the_notice_id_for_the_event_is_the_open_notice_of_that_site() {
        let c = db();
        assert!(open_notice_id(&c, "linkedin").is_none());
        crate::sites::repo::definir_sem_login(&c, "linkedin", false).unwrap();
        let id = open_notice_id(&c, "linkedin").unwrap();
        assert_eq!(crate::sites::avisos::listar_abertos(&c).unwrap()[0].id, id);
        assert!(open_notice_id(&c, "outro").is_none());
    }

    #[test]
    fn dismissing_gives_the_key_for_the_event_only_when_a_notice_was_dismissed() {
        let c = db();
        crate::sites::repo::marcar_precisa_login(&c, "linkedin").unwrap();
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        let id = open_notice_id(&c, "linkedin").unwrap();
        assert_eq!(do_dispensar(&c, id).unwrap(), Some("linkedin".to_string()));
        assert_eq!(do_dispensar(&c, id).unwrap(), None, "no event for a second dismissal");
        assert_eq!(do_dispensar(&c, 424242).unwrap(), None);
        assert_eq!(crate::sites::repo::obter(&c, "linkedin").unwrap().unwrap().estado, "precisa_login");
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
