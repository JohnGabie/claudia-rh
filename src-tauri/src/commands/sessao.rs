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
            "UPDATE sessoes SET terminada_em = datetime('now'), motivo_termino = 'substituída' WHERE terminada_em IS NULL",
            [],
        ).map_err(|e| e.to_string())?;
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
    let prompt_file = workspace.join(".claude-system-prompt.txt");
    std::fs::write(&prompt_file, &sys_prompt).map_err(|e| e.to_string())?;

    let skip_permissions = ler_skip_permissions(&data_dir);
    let mcp_config = crate::commands::perfil::write_mcp_config(app);
    let debug_file = crate::diagnostics::PATHS
        .get()
        .map(|p| p.dir.join("claude-startup.log"));
    let query = "Inicia a sessao de candidaturas.".to_string();
    let args = build_claude_cli_args(
        skip_permissions,
        mcp_config.as_deref(),
        &prompt_file,
        debug_file.as_deref(),
        &query,
    );

    let claude = crate::commands::claude_program();
    if cfg!(windows) && claude == "claude" {
        // last-resort string; still try spawn, but warn
        let msg = "\r\n\x1b[1;31m[Claudia RH]\x1b[0m claude.exe not found on PATH/WinGet/npm — trying 'claude' anyway.\r\n";
        app.emit("pty-output", msg).ok();
    }

    let spawn = pty_manager::iniciar_claude(
        app.clone(),
        claude.clone(),
        args,
        24,
        80,
        session_id,
        Arc::clone(&db),
        workspace.to_string_lossy().into_owned(),
    );

    match spawn {
        Ok(()) => {
            let modo_txt = if skip_permissions { "autonomous" } else { "supervised" };
            let notice = format!(
                "\r\n\x1b[1;33m[Claudia RH]\x1b[0m Starting Claude session (reason: {motivo} · mode: {modo_txt} · bin: {claude})…\r\n"
            );
            app.emit("pty-output", notice).ok();
            app.emit("session-started", session_id).ok();
            crate::diagnostics::emit_event("session", "spawn_ok", Some(session_id), motivo);
            crate::diagnostics::emit_event("session", "started", Some(session_id), motivo);
            Ok(())
        }
        Err(e) => {
            let msg = format!(
                "\r\n\x1b[1;31m[Claudia RH]\x1b[0m Falha ao iniciar: {e}\r\n"
            );
            app.emit("pty-output", msg).ok();
            if let Ok(conn) = db.lock() {
                let _ = conn.execute(
                    "UPDATE sessoes SET terminada_em = datetime('now'), motivo_termino = 'spawn_err' WHERE id = ?1",
                    rusqlite::params![session_id],
                );
            }
            crate::diagnostics::emit_event("session", "spawn_err", Some(session_id), &e);
            app.emit("session-ended", "spawn_err".to_string()).ok();
            Err(e)
        }
    }
}

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
    let content = serde_json::to_string(&serde_json::json!({ "skip_permissions": ativo }))
        .map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("modo_autonomo.json"), content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn registar_pausa_sessao(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sessoes SET pausada_em = datetime('now')
         WHERE id = (SELECT id FROM sessoes WHERE terminada_em IS NULL AND pausada_em IS NULL ORDER BY id DESC LIMIT 1)",
        [],
    ).map_err(|e| e.to_string())?;
    crate::diagnostics::emit_event("session", "paused", None, "");
    Ok(())
}

#[tauri::command]
pub fn registar_retoma_sessao(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sessoes SET
            tempo_pausado_segundos = COALESCE(tempo_pausado_segundos, 0) +
                CAST((julianday(datetime('now')) - julianday(pausada_em)) * 86400 AS INTEGER),
            pausada_em = NULL
         WHERE id = (SELECT id FROM sessoes WHERE terminada_em IS NULL AND pausada_em IS NOT NULL ORDER BY id DESC LIMIT 1)",
        [],
    ).map_err(|e| e.to_string())?;
    crate::diagnostics::emit_event("session", "resumed", None, "");
    Ok(())
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

pub(crate) fn build_claude_cli_args(
    skip_permissions: bool,
    mcp_config: Option<&std::path::Path>,
    prompt_file: &std::path::Path,
    debug_file: Option<&std::path::Path>,
    initial_query: &str,
) -> Vec<String> {
    let mut args = Vec::new();
    if skip_permissions {
        args.push("--dangerously-skip-permissions".to_string());
    }
    args.push("--chrome".to_string());
    if let Some(mcp) = mcp_config {
        args.push("--mcp-config".to_string());
        args.push(mcp.to_string_lossy().into_owned());
        args.push("--strict-mcp-config".to_string());
    }
    args.push("--system-prompt-file".to_string());
    args.push(prompt_file.to_string_lossy().into_owned());
    if let Some(df) = debug_file {
        args.push("--debug-file".to_string());
        args.push(df.to_string_lossy().into_owned());
    }
    args.push(initial_query.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::build_claude_cli_args;
    use std::path::Path;

    #[test]
    fn args_use_prompt_file_not_body() {
        let args = build_claude_cli_args(
            true,
            Some(Path::new("C:/data/mcp-config.json")),
            Path::new("C:/data/workspace/.claude-system-prompt.txt"),
            Some(Path::new("C:/data/diagnostics/claude-startup.log")),
            "Inicia a sessao de candidaturas.",
        );
        let joined = args.join("\x1e");
        assert!(args.contains(&"--system-prompt-file".to_string()));
        assert!(!args.iter().any(|a| a == "--system-prompt"));
        assert!(!joined.contains("nome_completo"));
        assert!(!args.iter().any(|a| a.len() > 4000), "an arg is huge: {}", args.iter().map(|a| a.len()).max().unwrap_or(0));
        assert!(args.contains(&"--strict-mcp-config".to_string()));
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(args.contains(&"--chrome".to_string()));
        assert_eq!(args.last().unwrap(), "Inicia a sessao de candidaturas.");
        let i = args.iter().position(|a| a == "--system-prompt-file").unwrap();
        assert!(args[i + 1].replace('\\', "/").ends_with("workspace/.claude-system-prompt.txt"));
    }

    #[test]
    fn supervised_omits_skip_permissions() {
        let args = build_claude_cli_args(false, None, Path::new("p.txt"), None, "hi");
        assert!(!args.iter().any(|a| a == "--dangerously-skip-permissions"));
        assert!(!args.iter().any(|a| a == "--mcp-config"));
        assert!(!args.iter().any(|a| a == "--debug-file"));
        assert_eq!(args.last().unwrap(), "hi");
    }
}
