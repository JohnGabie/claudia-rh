use crate::db::queries::{self, Candidatura, Pendencia, Proposta, ResumoMemoria, Vaga, VagaAtual};
use crate::DbState;
use tauri::{AppHandle, Emitter, State};
use std::process::Command;

#[tauri::command]
pub fn listar_vagas(
    state: State<'_, DbState>,
    filtro: Option<String>,
) -> Result<Vec<Vaga>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::listar_vagas(&conn, filtro).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_pendencias(
    state: State<'_, DbState>,
    apenas_nao_resolvidas: bool,
) -> Result<Vec<Pendencia>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    // Checkpoint WAL so this long-lived connection sees writes from external processes (e.g. the Claude agent)
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
    queries::listar_pendencias(&conn, apenas_nao_resolvidas).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resumo_memoria_recente(
    state: State<'_, DbState>,
    dias: u32,
) -> Result<ResumoMemoria, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::resumo_memoria_recente(&conn, dias).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn candidaturas_hoje(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::candidaturas_hoje(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn atividade_recente(state: State<'_, DbState>) -> Result<Vec<Vaga>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::atividade_recente(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn contar_pendencias(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
    queries::contar_pendencias_nao_resolvidas(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolver_pendencia(
    state: State<'_, DbState>,
    app: AppHandle,
    id: i64,
    resolucao: String,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE pendencias SET resolvida = 1, resolvida_em = datetime('now'), resolucao = ?1 WHERE id = ?2",
        rusqlite::params![resolucao, id],
    )
    .map_err(|e| e.to_string())?;
    let _ = app.emit("pendencia-resolvida", id);
    Ok(())
}

#[tauri::command]
pub fn listar_candidaturas(state: State<'_, DbState>) -> Result<Vec<Candidatura>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::listar_candidaturas_historico(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn contar_propostas(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::contar_propostas(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_propostas(state: State<'_, DbState>) -> Result<Vec<Proposta>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
    queries::listar_propostas(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ignorar_proposta(state: State<'_, DbState>, app: AppHandle, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE propostas_perfil SET promovida = 1, promovida_em = datetime('now') WHERE id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    let _ = app.emit("proposta-resolvida", id);
    Ok(())
}

#[tauri::command]
pub fn vaga_atual_sessao(state: State<'_, DbState>) -> Result<Option<VagaAtual>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::vaga_candidatando(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vagas_analisadas_hoje(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::vagas_analisadas_hoje(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vagas_analisadas_total(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::vagas_analisadas_total(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tempo_sessoes_hoje(state: State<'_, DbState>) -> Result<f64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    queries::tempo_sessoes_hoje_minutos(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn abrir_pasta(caminho: String) -> Result<(), String> {
    Command::new("explorer")
        .arg(&caminho)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pular_pendencia(
    state: State<'_, DbState>,
    app: AppHandle,
    id: i64,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let vaga_id: Result<i64, _> = conn.query_row(
        "SELECT vaga_id FROM pendencias WHERE id = ?1",
        [id],
        |r| r.get::<_, i64>(0),
    );
    conn.execute(
        "UPDATE pendencias SET resolvida = 1, resolvida_em = datetime('now'), resolucao = 'Vaga pulada' WHERE id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    if let Ok(vid) = vaga_id {
        let _ = conn.execute(
            "UPDATE vagas SET status = 'pulada', motivo_status = 'Skipped by user' WHERE id = ?1",
            [vid],
        );
    }
    let _ = app.emit("pendencia-resolvida", id);
    Ok(())
}
