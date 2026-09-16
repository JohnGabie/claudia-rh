// Pendências tools — direct SQLite, replacing the old Python-script hack.

use std::path::Path;

use super::open_db;

/// Marks one pendência as resolved. Errors if the id doesn't exist or is
/// already resolved, so the model gets honest feedback.
pub fn close_pendencia(data_dir: &Path, id: i64, resolucao: &str) -> Result<String, String> {
    let conn = open_db(data_dir)?;
    let n = conn
        .execute(
            "UPDATE pendencias SET resolvida = 1, resolvida_em = datetime('now'), resolucao = ?1 \
             WHERE id = ?2 AND resolvida = 0",
            rusqlite::params![resolucao, id],
        )
        .map_err(|e| format!("erro ao atualizar pendência: {e}"))?;
    if n == 0 {
        Err(format!("Pendência {id} não encontrada ou já resolvida."))
    } else {
        Ok(format!("Pendência {id} fechada: {resolucao}"))
    }
}

/// Closes ALL open pendências of one vaga (used by the application session
/// when it finishes a vaga that was in pendente_revisao).
pub fn close_pendencias_vaga(data_dir: &Path, vaga_id: i64, resolucao: &str) -> Result<String, String> {
    let conn = open_db(data_dir)?;
    let n = conn
        .execute(
            "UPDATE pendencias SET resolvida = 1, resolvida_em = datetime('now'), \
             resolucao = COALESCE(NULLIF(resolucao, ''), ?1) \
             WHERE vaga_id = ?2 AND resolvida = 0",
            rusqlite::params![resolucao, vaga_id],
        )
        .map_err(|e| format!("erro ao atualizar pendências: {e}"))?;
    Ok(format!("{n} pendência(s) da vaga {vaga_id} fechada(s)."))
}

/// Returns the most recent open pendência of a vaga (id + user resolution, if
/// any) — the model reads how the user unblocked the vaga before resuming it.
pub fn get_pendencia_vaga(data_dir: &Path, vaga_id: i64) -> Result<String, String> {
    let conn = open_db(data_dir)?;
    let row = conn
        .query_row(
            "SELECT id, categoria, descricao, COALESCE(resolucao, '') FROM pendencias \
             WHERE vaga_id = ?1 AND resolvida = 0 ORDER BY criada_em DESC LIMIT 1",
            [vaga_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        );
    match row {
        Ok((id, cat, desc, res)) => {
            let res_txt = if res.is_empty() {
                "(sem resolução do utilizador ainda)".to_string()
            } else {
                format!("resolução do utilizador: {res}")
            };
            Ok(format!("Pendência {id} [{cat}]: {desc} — {res_txt}"))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Ok(format!("(sem pendências abertas para a vaga {vaga_id})"))
        }
        Err(e) => Err(format!("erro ao consultar pendência: {e}")),
    }
}

/// Lists open pendências with their ids, so the model always acts on fresh data.
pub fn list_pendencias(data_dir: &Path) -> Result<String, String> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.categoria, p.descricao, v.titulo, v.empresa \
             FROM pendencias p LEFT JOIN vagas v ON p.vaga_id = v.id \
             WHERE p.resolvida = 0 ORDER BY p.criada_em DESC",
        )
        .map_err(|e| format!("erro ao consultar pendências: {e}"))?;

    let rows: Vec<String> = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let cat: String = row.get(1)?;
            let desc: String = row.get(2)?;
            let titulo: Option<String> = row.get(3)?;
            let empresa: Option<String> = row.get(4)?;
            Ok(format!(
                "- ID {id}: [{cat}] {desc} (vaga: {} @ {})",
                titulo.as_deref().unwrap_or("?"),
                empresa.as_deref().unwrap_or("?"),
            ))
        })
        .map_err(|e| format!("erro ao ler pendências: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        Ok("(sem pendências abertas)".to_string())
    } else {
        Ok(rows.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    #[test]
    fn close_marks_resolved_and_rejects_double_close() {
        let dir = temp_dir("pend-close");
        seed_db(&dir);
        let msg = close_pendencia(&dir, 10, "Salário definido: 8k").unwrap();
        assert!(msg.contains("10"));
        assert!(close_pendencia(&dir, 10, "de novo").is_err());
        assert!(close_pendencia(&dir, 999, "x").is_err());
    }

    #[test]
    fn list_shows_open_and_hides_resolved() {
        let dir = temp_dir("pend-list");
        seed_db(&dir);
        let out = list_pendencias(&dir).unwrap();
        assert!(out.contains("ID 10"));
        assert!(out.contains("ACME"));
        close_pendencia(&dir, 10, "ok").unwrap();
        assert_eq!(list_pendencias(&dir).unwrap(), "(sem pendências abertas)");
    }

    #[test]
    fn get_pendencia_vaga_shows_resolution_then_none() {
        let dir = temp_dir("pend-get");
        seed_db(&dir);
        let out = get_pendencia_vaga(&dir, 1).unwrap();
        assert!(out.contains("sem resolução do utilizador"), "got: {out}");
        // Simulate the user answering via UI
        let conn = open_db(&dir).unwrap();
        conn.execute("UPDATE pendencias SET resolucao='pode oferecer 8k' WHERE id=10", [])
            .unwrap();
        let out = get_pendencia_vaga(&dir, 1).unwrap();
        assert!(out.contains("pode oferecer 8k"));
        close_pendencias_vaga(&dir, 1, "vaga concluída").unwrap();
        let out = get_pendencia_vaga(&dir, 1).unwrap();
        assert!(out.contains("sem pendências abertas"));
    }

    #[test]
    fn close_vaga_keeps_user_resolution_text() {
        let dir = temp_dir("pend-closevaga");
        seed_db(&dir);
        let conn = open_db(&dir).unwrap();
        conn.execute("UPDATE pendencias SET resolucao='resposta do utilizador' WHERE id=10", [])
            .unwrap();
        close_pendencias_vaga(&dir, 1, "Resolvida pelo agente").unwrap();
        let res: String = conn
            .query_row("SELECT resolucao FROM pendencias WHERE id=10", [], |r| r.get(0))
            .unwrap();
        assert_eq!(res, "resposta do utilizador");
    }
}
