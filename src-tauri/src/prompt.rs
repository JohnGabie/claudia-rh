use rusqlite::Connection;
use std::path::Path;

pub fn montar_prompt_sistema(
    conn: &Connection,
    data_dir: &Path,
    db_path: &Path,
) -> String {
    let candidate_base_yaml = std::fs::read_to_string(data_dir.join("candidate_base.yaml"))
        .unwrap_or_default();

    let search_variants_yaml = std::fs::read_to_string(data_dir.join("search_variants.yaml"))
        .unwrap_or_default();

    let strategy_md = std::fs::read_to_string(data_dir.join("strategy.md"))
        .unwrap_or_default();

    let memory_summary = build_memory_summary(conn);
    let db_path_str = db_path.to_string_lossy().to_string();

    crate::commands::prompts::read_prompt(data_dir, "runtime")
        .replace("{{CANDIDATE_BASE_YAML}}", &candidate_base_yaml)
        .replace("{{SEARCH_VARIANTS_YAML}}", &search_variants_yaml)
        .replace("{{STRATEGY_MD}}", &strategy_md)
        .replace("{{RECENT_MEMORY_SUMMARY}}", &memory_summary)
        .replace("{{DB_PATH}}", &db_path_str)
}

fn build_memory_summary(conn: &Connection) -> String {
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
                    "(nenhuma vaga pulada recentemente)".to_string()
                } else {
                    rows.join("\n")
                }
            }
            Err(_) => "(nenhuma vaga pulada recentemente)".to_string(),
        }
    };

    format!(
        "Candidaturas hoje: {}\nCandidaturas esta semana: {}\nPendências por resolver: {}\n\nVagas puladas recentemente:\n{}",
        hoje, semana, pendentes, puladas_txt
    )
}
