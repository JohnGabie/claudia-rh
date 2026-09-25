// Profile-proposal tool — the write side of `propostas_perfil`.
//
// The table has been in schema.sql since the beginning, with a nullable
// vaga_id, and the whole UI around it (counter, notification, sidebar badge)
// was already wired. Nothing in production ever inserted into it, which is why
// global questions ended up in search_variants.yaml instead.

use std::path::Path;

use super::open_db;

/// Records a change the agent thinks the profile needs, for the user to review.
///
/// `vaga_id` is None for a question that is not about one job — the case the
/// pendencias table cannot represent, because its vaga_id is NOT NULL.
pub fn propose_profile_change(
    data_dir: &Path,
    pergunta: &str,
    contexto: Option<&str>,
    vaga_id: Option<i64>,
) -> Result<String, String> {
    if pergunta.trim().is_empty() {
        return Err("parâmetro 'pergunta' vazio".to_string());
    }
    let conn = open_db(data_dir)?;
    conn.execute(
        "INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, contexto, promovida) \
         VALUES (?1, datetime('now'), ?2, ?3, 0)",
        rusqlite::params![vaga_id, pergunta.trim(), contexto],
    )
    .map_err(|e| format!("erro ao registar proposta de perfil: {e}"))?;

    Ok(format!("Proposta registada para o usuário rever: {}", pergunta.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    /// Review Focus 5: the global question is the case this table exists for.
    #[test]
    fn proposal_without_vaga_stores_null() {
        let dir = temp_dir("prop-global");
        seed_db(&dir);
        propose_profile_change(&dir, "Aceita viajar?", Some("3 vagas pediram"), None).unwrap();
        let conn = open_db(&dir).unwrap();
        let vaga: Option<i64> = conn
            .query_row(
                "SELECT vaga_id FROM propostas_perfil WHERE pergunta = 'Aceita viajar?'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vaga, None, "a global question must not be pinned to a vaga");
    }

    #[test]
    fn proposal_with_vaga_keeps_the_link() {
        let dir = temp_dir("prop-vaga");
        seed_db(&dir);
        propose_profile_change(&dir, "Tem visto?", None, Some(1)).unwrap();
        let conn = open_db(&dir).unwrap();
        let vaga: Option<i64> = conn
            .query_row(
                "SELECT vaga_id FROM propostas_perfil WHERE pergunta = 'Tem visto?'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vaga, Some(1));
    }

    #[test]
    fn proposal_starts_unpromoted_and_is_counted() {
        let dir = temp_dir("prop-count");
        seed_db(&dir);
        propose_profile_change(&dir, "Pergunta", None, None).unwrap();
        let conn = open_db(&dir).unwrap();
        let open: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propostas_perfil WHERE promovida = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(open, 1);
    }

    #[test]
    fn empty_question_is_rejected() {
        let dir = temp_dir("prop-empty");
        seed_db(&dir);
        assert!(propose_profile_change(&dir, "   ", None, None).is_err());
    }
}
