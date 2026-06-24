use crate::{prompt, pty_manager, DbState, IdleConfig, IdleState};
use rusqlite::Connection;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Inserts a sessoes row, assembles the system prompt, and spawns
/// `claude --dangerously-skip-permissions --chrome` inside the embedded PTY.
/// Called by both the Tauri command (manual) and the idle watcher (inatividade).
pub fn iniciar_sessao(
    db: Arc<Mutex<Connection>>,
    app: &AppHandle,
    motivo: &str,
) -> Result<(), String> {
    let session_id = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO sessoes (iniciada_em, motivo_disparo) VALUES (datetime('now'), ?1)",
            [motivo],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = data_dir.join("claudia_rh.db");

    // Use a dedicated workspace directory that is a git repo.
    // Claude Code's trust-folder prompt is suppressed in git repositories,
    // so this avoids the manual "Do you trust this folder?" dialog every session.
    let workspace = data_dir.join("workspace");
    std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    let git_dir = workspace.join(".git");
    if !git_dir.exists() {
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&workspace)
            .output()
            .ok();
    }

    let sys_prompt = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        prompt::montar_prompt_sistema(&conn, &data_dir, &db_path)
    };

    let notice = format!(
        "\r\n\x1b[1;33m[Claudia RH]\x1b[0m A iniciar sessão Claude (motivo: {})…\r\n",
        motivo
    );
    app.emit("pty-output", notice).ok();
    app.emit("session-started", session_id).ok();

    pty_manager::iniciar_claude(
        app.clone(),
        crate::commands::claude_program(),
        vec![
            "--dangerously-skip-permissions".to_string(),
            "--chrome".to_string(),
            "--system-prompt".to_string(),
            sys_prompt,
        ],
        24,
        80,
        session_id,
        db,
        workspace.to_string_lossy().into_owned(),
        "Inicia a sessao de candidaturas.".to_string(),
    )
}

#[tauri::command]
pub fn disparar_sessao(
    app: AppHandle,
    state: State<'_, DbState>,
    motivo: String,
) -> Result<(), String> {
    iniciar_sessao(Arc::clone(&state.0), &app, &motivo)
}

#[derive(Serialize)]
pub struct ConfigDisparo {
    pub ativo: bool,
    pub limiar_minutos: u32,
    pub limite_diario: u32,
    pub limite_tempo_minutos: u32,
    pub janelas: Vec<crate::JanelaAgendamento>,
}

impl From<&IdleConfig> for ConfigDisparo {
    fn from(c: &IdleConfig) -> Self {
        ConfigDisparo {
            ativo: c.ativo,
            limiar_minutos: c.limiar_minutos,
            limite_diario: c.limite_diario,
            limite_tempo_minutos: c.limite_tempo_minutos,
            janelas: c.janelas.clone(),
        }
    }
}

#[tauri::command]
pub fn obter_config_disparo(idle: State<'_, IdleState>) -> Result<ConfigDisparo, String> {
    let cfg = idle.0.lock().map_err(|e| e.to_string())?;
    Ok(ConfigDisparo::from(&*cfg))
}

#[tauri::command]
pub fn configurar_disparo(
    app: AppHandle,
    idle: State<'_, IdleState>,
    ativo: bool,
    limiar_minutos: u32,
    limite_diario: Option<u32>,
    limite_tempo_minutos: Option<u32>,
    janelas: Option<Vec<crate::JanelaAgendamento>>,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut cfg = idle.0.lock().map_err(|e| e.to_string())?;
    cfg.ativo = ativo;
    cfg.limiar_minutos = limiar_minutos;
    if let Some(ld) = limite_diario { cfg.limite_diario = ld; }
    if let Some(lt) = limite_tempo_minutos { cfg.limite_tempo_minutos = lt; }
    if let Some(j) = janelas { cfg.janelas = j; }
    salvar_config_disparo(&data_dir, &cfg)
}

#[tauri::command]
pub fn configurar_limite_diario(
    app: AppHandle,
    idle: State<'_, IdleState>,
    limite: u32,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut cfg = idle.0.lock().map_err(|e| e.to_string())?;
    cfg.limite_diario = limite;
    salvar_config_disparo(&data_dir, &cfg)
}

pub fn salvar_config_disparo(
    data_dir: &std::path::Path,
    cfg: &IdleConfig,
) -> Result<(), String> {
    let content = serde_json::to_string(cfg).map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("disparo.json"), content).map_err(|e| e.to_string())
}
