use rusqlite::Connection;
use std::path::Path;

/// Returns the runtime system prompt: the operating rules, and nothing else.
///
/// It used to interpolate the profile, search variants, strategy and a memory
/// snapshot. Those are MCP tools now (`get_candidate_profile` and friends). The
/// prompt reaches the CLI as a file, but it is still the whole context the
/// session starts with, so keeping it free of per-candidate data is what stops
/// it from growing without bound — and what keeps the CPF out of it.
pub fn montar_prompt_sistema(data_dir: &Path) -> String {
    crate::commands::prompts::read_prompt(data_dir, "runtime")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    fn assemble(dir: &std::path::Path, profile_yaml: &str) -> String {
        std::fs::write(dir.join("candidate_base.yaml"), profile_yaml).unwrap();
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n").unwrap();
        std::fs::write(dir.join("strategy.md"), "# Estratégia\n").unwrap();
        seed_db(dir);
        montar_prompt_sistema(dir)
    }

    /// The CreateProcessW break was not the 32767 ceiling, it was that the
    /// prompt grew with the candidate's career. Size must stop tracking the data.
    #[test]
    fn prompt_size_does_not_depend_on_profile_size() {
        let small = temp_dir("prompt-small");
        let big = temp_dir("prompt-big");
        let filler = "  # padding line to make this profile large\n".repeat(1000);

        let a = assemble(&small, "dados_pessoais:\n  nome_completo: \"Maria\"\n");
        let b = assemble(&big, &format!("dados_pessoais:\n  nome_completo: \"Maria\"\n{filler}"));

        assert_eq!(a.len(), b.len(), "prompt still grows with the profile");
    }

    /// Whatever carries the prompt, personal data must not ride along with it.
    /// This is the regression guard: re-inlining the YAML breaks this test.
    #[test]
    fn prompt_does_not_carry_personal_data() {
        let dir = temp_dir("prompt-pii");
        let prompt = assemble(
            &dir,
            "dados_pessoais:\n  email: \"maria@exemplo.com\"\n  cpf: \"000.111.222-33\"\n",
        );
        assert!(!prompt.contains("maria@exemplo.com"), "e-mail leaked into the prompt");
        assert!(!prompt.contains("000.111.222-33"), "CPF leaked into the prompt");
    }
}

pub(crate) fn build_memory_summary(conn: &Connection) -> String {
    let hoje: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM candidaturas WHERE date(enviada_em) = date('now')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let semana: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM candidaturas WHERE enviada_em >= datetime('now', '-7 days')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let pendentes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pendencias WHERE resolvida = 0",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let puladas_txt = {
        let mut stmt = conn.prepare(
            "SELECT titulo, empresa, motivo_status FROM vagas \
             WHERE status = 'pulada' AND descoberta_em >= datetime('now', '-7 days') LIMIT 5",
        );
        match stmt {
            Ok(ref mut s) => {
                let rows: Vec<String> = s
                    .query_map([], |r| {
                        Ok(format!(
                            "- {} em {}: {}",
                            r.get::<_, String>(0).unwrap_or_default(),
                            r.get::<_, String>(1).unwrap_or_default(),
                            r.get::<_, String>(2).unwrap_or_default(),
                        ))
                    })
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();
                if rows.is_empty() {
                    "(no jobs skipped recently)".to_string()
                } else {
                    rows.join("\n")
                }
            }
            Err(_) => "(no jobs skipped recently)".to_string(),
        }
    };

    format!(
        "Applications today: {}\nApplications this week: {}\nUnresolved pending items: {}\n\nRecently skipped jobs:\n{}",
        hoje, semana, pendentes, puladas_txt
    )
}
