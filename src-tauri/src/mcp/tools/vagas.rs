// Vagas/candidaturas tools for the main application session — replaces the
// raw `sqlite3` CLI writes the runtime prompt used to instruct.

use std::path::Path;

use super::open_db;

/// Status values the agent is allowed to set, mirroring the runtime prompt's
/// state machine. Anything else is rejected with the valid list.
const VALID_STATUS: &[&str] = &[
    "descoberta",
    "analisada",
    "candidatando",
    "aplicada",
    "pulada",
    "pendente_revisao",
];

/// Registers a newly discovered vaga (status starts as 'descoberta').
/// Duplicate URLs are reported as already-known, not as an error.
#[allow(clippy::too_many_arguments)]
pub fn register_vaga(
    data_dir: &Path,
    titulo: &str,
    empresa: &str,
    plataforma: &str,
    url: &str,
    localizacao: Option<&str>,
    modelo_trabalho: Option<&str>,
    fonte_conexao: Option<&str>,
    variante_id: Option<&str>,
) -> Result<String, String> {
    if titulo.trim().is_empty() || empresa.trim().is_empty() || url.trim().is_empty() {
        return Err("titulo, empresa e url são obrigatórios".to_string());
    }
    if let Some(v) = variante_id {
        crate::documentos::variantes::validar(data_dir, v)?;
    }
    let conn = open_db(data_dir)?;
    let n = conn
        .execute(
            "INSERT OR IGNORE INTO vagas \
             (titulo, empresa, plataforma, url, localizacao, modelo_trabalho, fonte_conexao, variante_id, descoberta_em, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'), 'descoberta')",
            rusqlite::params![titulo, empresa, plataforma, url, localizacao, modelo_trabalho, fonte_conexao, variante_id],
        )
        .map_err(|e| format!("erro ao registar vaga: {e}"))?;

    if n == 0 {
        let id: i64 = conn
            .query_row("SELECT id FROM vagas WHERE url = ?1", [url], |r| r.get(0))
            .map_err(|e| format!("erro ao consultar vaga existente: {e}"))?;
        Ok(format!("Vaga já registada anteriormente (ID {id}) — não duplicada."))
    } else {
        let id = conn.last_insert_rowid();
        Ok(format!("Vaga registada (ID {id}): {titulo} @ {empresa}"))
    }
}

/// Advances a vaga's status. `detalhe` fills match_score for 'analisada' and
/// motivo_status for 'pulada'/'pendente_revisao'.
pub fn update_vaga_status(
    data_dir: &Path,
    vaga_id: i64,
    status: &str,
    detalhe: Option<&str>,
    variante_id: Option<&str>,
) -> Result<String, String> {
    if !VALID_STATUS.contains(&status) {
        return Err(format!(
            "status inválido: '{status}'. Válidos: {}",
            VALID_STATUS.join(", ")
        ));
    }
    if let Some(v) = variante_id {
        crate::documentos::variantes::validar(data_dir, v)?;
    }
    let conn = open_db(data_dir)?;
    let n = match status {
        "analisada" => conn.execute(
            "UPDATE vagas SET status = ?1, match_score = COALESCE(?2, match_score) WHERE id = ?3",
            rusqlite::params![status, detalhe, vaga_id],
        ),
        "pulada" | "pendente_revisao" => conn.execute(
            "UPDATE vagas SET status = ?1, motivo_status = COALESCE(?2, motivo_status) WHERE id = ?3",
            rusqlite::params![status, detalhe, vaga_id],
        ),
        _ => conn.execute(
            "UPDATE vagas SET status = ?1 WHERE id = ?2",
            rusqlite::params![status, vaga_id],
        ),
    }
    .map_err(|e| format!("erro ao atualizar vaga: {e}"))?;

    if n == 0 {
        return Err(format!("Vaga {vaga_id} não encontrada."));
    }
    if let Some(v) = variante_id {
        conn.execute("UPDATE vagas SET variante_id = ?1 WHERE id = ?2", rusqlite::params![v, vaga_id])
            .map_err(|e| format!("erro ao gravar a variante: {e}"))?;
    }
    Ok(format!("Vaga {vaga_id} → {status}"))
}

/// Records which variant the agent chose for a vaga (spec 2.1). Validated against search_variants.yaml.
pub fn set_vaga_variante(data_dir: &Path, vaga_id: i64, variante_id: &str) -> Result<String, String> {
    crate::documentos::variantes::validar(data_dir, variante_id)?;
    let conn = open_db(data_dir)?;
    let n = conn
        .execute("UPDATE vagas SET variante_id = ?1 WHERE id = ?2", rusqlite::params![variante_id, vaga_id])
        .map_err(|e| format!("erro ao gravar a variante: {e}"))?;
    if n == 0 {
        return Err(format!("Vaga {vaga_id} não encontrada."));
    }
    Ok(format!("Vaga {vaga_id}: variante {variante_id}."))
}

/// Records a submitted application and flips the vaga to 'aplicada' in one
/// transaction, so the two writes can never diverge.
pub fn register_candidatura(
    data_dir: &Path,
    vaga_id: i64,
    pasta_arquivos: &str,
    metodo: &str,
) -> Result<String, String> {
    let mut conn = open_db(data_dir)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) \
         VALUES (?1, datetime('now'), ?2, ?3)",
        rusqlite::params![vaga_id, pasta_arquivos, metodo],
    )
    .map_err(|e| format!("erro ao registar candidatura: {e}"))?;
    let n = tx
        .execute("UPDATE vagas SET status = 'aplicada' WHERE id = ?1", [vaga_id])
        .map_err(|e| format!("erro ao atualizar vaga: {e}"))?;
    if n == 0 {
        return Err(format!("Vaga {vaga_id} não encontrada — candidatura NÃO registada."));
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(format!("Candidatura registada para a vaga {vaga_id} (método: {metodo}). Vaga → aplicada."))
}

/// Opens a pendência (pause condition) and flips the vaga to pendente_revisao
/// in one transaction.
pub fn create_pendencia(
    data_dir: &Path,
    vaga_id: i64,
    categoria: &str,
    descricao: &str,
) -> Result<String, String> {
    if categoria.trim().is_empty() || descricao.trim().is_empty() {
        return Err("categoria e descricao são obrigatórias".to_string());
    }
    let mut conn = open_db(data_dir)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO pendencias (vaga_id, criada_em, categoria, descricao) \
         VALUES (?1, datetime('now'), ?2, ?3)",
        rusqlite::params![vaga_id, categoria, descricao],
    )
    .map_err(|e| format!("erro ao criar pendência: {e}"))?;
    let id = tx.last_insert_rowid();
    let n = tx
        .execute("UPDATE vagas SET status = 'pendente_revisao' WHERE id = ?1", [vaga_id])
        .map_err(|e| format!("erro ao atualizar vaga: {e}"))?;
    if n == 0 {
        return Err(format!("Vaga {vaga_id} não encontrada — pendência NÃO criada."));
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(format!("Pendência {id} criada [{categoria}]. Vaga {vaga_id} → pendente_revisao."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    #[test]
    fn register_vaga_inserts_and_dedupes_by_url() {
        let dir = temp_dir("vaga-reg");
        seed_db(&dir);
        let msg = register_vaga(&dir, "Backend Dev", "Initech", "linkedin", "https://x/2", None, Some("remoto"), None, None).unwrap();
        assert!(msg.contains("registada (ID"), "got: {msg}");
        let msg = register_vaga(&dir, "Backend Dev", "Initech", "linkedin", "https://x/2", None, None, None, None).unwrap();
        assert!(msg.contains("já registada"), "got: {msg}");
        assert!(register_vaga(&dir, "", "Initech", "l", "https://x/3", None, None, None, None).is_err());
    }

    fn with_variants(dir: &std::path::Path) {
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n  - id: backend\n  - id: devops\n").unwrap();
    }

    fn variante_of(dir: &std::path::Path, id: i64) -> Option<String> {
        open_db(dir).unwrap().query_row("SELECT variante_id FROM vagas WHERE id = ?1", [id], |r| r.get(0)).unwrap()
    }

    #[test]
    fn the_agent_records_the_vagas_variant_three_ways() {
        let dir = temp_dir("vaga-var");
        seed_db(&dir);
        with_variants(&dir);
        register_vaga(&dir, "SRE", "Initech", "linkedin", "https://x/9", None, None, None, Some("devops")).unwrap();
        let id: i64 = open_db(&dir).unwrap().query_row("SELECT id FROM vagas WHERE url='https://x/9'", [], |r| r.get(0)).unwrap();
        assert_eq!(variante_of(&dir, id).as_deref(), Some("devops"));
        update_vaga_status(&dir, 1, "analisada", Some("ok"), Some("backend")).unwrap();
        assert_eq!(variante_of(&dir, 1).as_deref(), Some("backend"));
        set_vaga_variante(&dir, 1, "devops").unwrap();
        assert_eq!(variante_of(&dir, 1).as_deref(), Some("devops"));
    }

    #[test]
    fn an_unknown_variant_is_refused_and_nothing_is_written() {
        let dir = temp_dir("vaga-var-bad");
        seed_db(&dir);
        with_variants(&dir);
        assert!(register_vaga(&dir, "X", "Y", "l", "https://x/10", None, None, None, Some("frontend")).is_err());
        let n: i64 = open_db(&dir).unwrap().query_row("SELECT COUNT(*) FROM vagas WHERE url='https://x/10'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "no vaga with an invented variant");
        assert!(update_vaga_status(&dir, 1, "analisada", None, Some("frontend")).is_err());
        let st: String = open_db(&dir).unwrap().query_row("SELECT status FROM vagas WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(st, "descoberta", "the status did not move either");
        assert!(set_vaga_variante(&dir, 1, "frontend").is_err());
        assert!(set_vaga_variante(&dir, 999, "backend").is_err(), "unknown vaga");
        assert_eq!(variante_of(&dir, 1), None);
    }

    #[test]
    fn update_status_validates_and_fills_detail_columns() {
        let dir = temp_dir("vaga-status");
        seed_db(&dir);
        assert!(update_vaga_status(&dir, 1, "status_inventado", None, None).is_err());
        assert!(update_vaga_status(&dir, 999, "analisada", None, None).is_err());

        update_vaga_status(&dir, 1, "analisada", Some("cobre 3/4 must-haves"), None).unwrap();
        let conn = open_db(&dir).unwrap();
        let (st, ms): (String, String) = conn
            .query_row("SELECT status, match_score FROM vagas WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(st, "analisada");
        assert!(ms.contains("must-haves"));

        update_vaga_status(&dir, 1, "pulada", Some("sem visto"), None).unwrap();
        let motivo: String = conn
            .query_row("SELECT motivo_status FROM vagas WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(motivo, "sem visto");
    }

    #[test]
    fn register_candidatura_is_transactional() {
        let dir = temp_dir("vaga-cand");
        seed_db(&dir);
        register_candidatura(&dir, 1, "/tmp/acme", "formulario").unwrap();
        let conn = open_db(&dir).unwrap();
        let st: String = conn.query_row("SELECT status FROM vagas WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(st, "aplicada");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM candidaturas", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
        // Unknown vaga: nothing written at all (transaction rolled back)
        assert!(register_candidatura(&dir, 999, "/tmp/x", "form").is_err());
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM candidaturas", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn create_pendencia_flips_vaga_and_is_transactional() {
        let dir = temp_dir("vaga-pend");
        seed_db(&dir);
        let msg = create_pendencia(&dir, 1, "visto", "Formulário pede visto americano").unwrap();
        assert!(msg.contains("pendente_revisao"));
        let conn = open_db(&dir).unwrap();
        let st: String = conn.query_row("SELECT status FROM vagas WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(st, "pendente_revisao");
        assert!(create_pendencia(&dir, 999, "visto", "x").is_err());
        assert!(create_pendencia(&dir, 1, "", "x").is_err());
    }
}
