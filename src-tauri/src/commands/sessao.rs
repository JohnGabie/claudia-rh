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

    // Modo autónomo: quando ativo, passa --dangerously-skip-permissions e o agente
    // age sem pedir confirmação. Quando inativo (predefinição segura), o Claude pede
    // permissão no terminal embutido antes de cada ação — o utilizador mantém controlo.
    let skip_permissions = ler_skip_permissions(&data_dir);
    let mut args: Vec<String> = Vec::new();
    if skip_permissions {
        args.push("--dangerously-skip-permissions".to_string()); // Adicionado um botão de segurança nas configurações
    }
    args.push("--chrome".to_string());
    args.push("--system-prompt".to_string());
    args.push(sys_prompt);

    let modo_txt = if skip_permissions {
        "autónomo (sem confirmações)"
    } else {
        "supervisionado (pede permissão)"
    };
    let notice = format!(
        "\r\n\x1b[1;33m[Claudia RH]\x1b[0m A iniciar sessão Claude (motivo: {} · modo: {})…\r\n",
        motivo, modo_txt
    );
    app.emit("pty-output", notice).ok();
    app.emit("session-started", session_id).ok();

    pty_manager::iniciar_claude(
        app.clone(),
        "claude".to_string(),
        args,
        24,
        80,
        session_id,
        db,
        workspace.to_string_lossy().into_owned(),
        "Inicia a sessao de candidaturas.".to_string(),
    )
}

/// Lê a preferência de "modo autónomo" (skip de permissões) de `modo_autonomo.json`.
/// Predefinição segura: `false` (o Claude pede confirmação antes de agir).
pub fn ler_skip_permissions(data_dir: &std::path::Path) -> bool {
    std::fs::read_to_string(data_dir.join("modo_autonomo.json"))
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|v| v.get("skip_permissions").and_then(|x| x.as_bool()))
        .unwrap_or(false)
}

#[tauri::command]
pub fn obter_modo_autonomo(app: AppHandle) -> Result<bool, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(ler_skip_permissions(&data_dir))
}

#[tauri::command]
pub fn configurar_modo_autonomo(app: AppHandle, ativo: bool) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let payload = serde_json::json!({ "skip_permissions": ativo });
    let content = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("modo_autonomo.json"), content).map_err(|e| e.to_string())
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
}

impl From<&IdleConfig> for ConfigDisparo {
    fn from(c: &IdleConfig) -> Self {
        ConfigDisparo { ativo: c.ativo, limiar_minutos: c.limiar_minutos }
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
) -> Result<(), String> {
    {
        let mut cfg = idle.0.lock().map_err(|e| e.to_string())?;
        cfg.ativo = ativo;
        cfg.limiar_minutos = limiar_minutos;
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    salvar_config_disparo(&data_dir, ativo, limiar_minutos)
}

pub fn salvar_config_disparo(
    data_dir: &std::path::Path,
    ativo: bool,
    limiar_minutos: u32,
) -> Result<(), String> {
    let payload = serde_json::json!({ "ativo": ativo, "limiar_minutos": limiar_minutos });
    let content = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("disparo.json"), content).map_err(|e| e.to_string())
}
