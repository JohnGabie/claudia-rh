use crate::{pty_manager, DbState};
use serde::Serialize;
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize)]
pub struct VagaLinkedinRede {
    pub id: i64,
    pub titulo: String,
    pub empresa: String,
    pub url: String,
    pub fonte_conexao: Option<String>,
    pub descoberta_em: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct StatusLinkedinRede {
    pub ativo: bool,
    pub ultima_busca: Option<String>,
    pub vagas_encontradas: i64,
}

fn montar_prompt_linkedin(data_dir: &std::path::Path, db_path: &str) -> String {
    crate::commands::prompts::read_prompt(data_dir, "linkedin-rede")
        .replace("{{DB_PATH}}", db_path)
}

#[tauri::command]
pub fn iniciar_busca_linkedin_rede(
    app: AppHandle,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = data_dir.join("claudia_rh.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    let db = Arc::clone(&state.0);
    let session_id = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE sessoes SET terminada_em = datetime('now'), motivo_termino = 'substituída' WHERE terminada_em IS NULL",
            [],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO sessoes (iniciada_em, motivo_disparo) VALUES (datetime('now'), 'linkedin_rede')",
            [],
        ).map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    let workspace = data_dir.join("workspace");
    std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    if !workspace.join(".git").exists() {
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&workspace)
            .output()
            .ok();
    }

    let skip_permissions = crate::commands::sessao::ler_skip_permissions(&data_dir);
    let sys_prompt = montar_prompt_linkedin(&data_dir, &db_path_str);
    let workspace_str = workspace.to_string_lossy().into_owned();

    let mut args: Vec<String> = Vec::new();
    if skip_permissions {
        args.push("--dangerously-skip-permissions".to_string());
    }
    args.push("--chrome".to_string());
    // Expose claudia's typed tools (register_vaga with fonte_conexao, …)
    if let Some(mcp_config) = crate::commands::perfil::write_mcp_config(&app) {
        args.push("--mcp-config".to_string());
        args.push(mcp_config.to_string_lossy().into_owned());
    }
    args.push("--system-prompt".to_string());
    args.push(sys_prompt);

    app.emit(
        "pty-output",
        "\r\n\x1b[1;36m[Claudia RH]\x1b[0m A iniciar varredura da rede LinkedIn…\r\n",
    ).ok();
    app.emit("linkedin-session-started", session_id).ok();
    app.emit("session-started", session_id).ok();

    pty_manager::iniciar_claude(
        app.clone(),
        crate::commands::claude_program(),
        args,
        24,
        80,
        session_id,
        db,
        workspace_str,
        "Starting LinkedIn network scan for jobs shared by connections.".to_string(),
    )
}

#[tauri::command]
pub fn listar_vagas_linkedin_rede(
    state: State<'_, DbState>,
) -> Result<Vec<VagaLinkedinRede>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, titulo, empresa, url, fonte_conexao, descoberta_em, status
             FROM vagas WHERE plataforma = 'linkedin_rede'
             ORDER BY descoberta_em DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let vagas = stmt
        .query_map([], |row| {
            Ok(VagaLinkedinRede {
                id: row.get(0)?,
                titulo: row.get(1)?,
                empresa: row.get(2)?,
                url: row.get(3)?,
                fonte_conexao: row.get(4)?,
                descoberta_em: row.get(5)?,
                status: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(vagas)
}

#[tauri::command]
pub fn obter_status_linkedin_rede(
    state: State<'_, DbState>,
) -> Result<StatusLinkedinRede, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let ativo: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sessoes \
             WHERE motivo_disparo = 'linkedin_rede' AND terminada_em IS NULL",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    let ultima_busca: Option<String> = conn
        .query_row(
            "SELECT terminada_em FROM sessoes \
             WHERE motivo_disparo = 'linkedin_rede' AND terminada_em IS NOT NULL \
             ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();

    let vagas_encontradas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vagas WHERE plataforma = 'linkedin_rede'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(StatusLinkedinRede {
        ativo,
        ultima_busca,
        vagas_encontradas,
    })
}
