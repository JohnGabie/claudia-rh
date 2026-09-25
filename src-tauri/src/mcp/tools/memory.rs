// Memory tools — the session's own recent history, read live instead of frozen.

use std::path::Path;

/// Counts of recent applications, open pending items and recently skipped jobs.
///
/// The runtime prompt used to carry this, computed once at spawn. That snapshot
/// went stale as soon as the session applied to anything, so the model reasoned
/// about a state that no longer existed. As a tool it is recomputed per call.
pub fn get_memory_summary(data_dir: &Path) -> Result<String, String> {
    let conn = super::open_db(data_dir)?;
    Ok(crate::prompt::build_memory_summary(&conn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    #[test]
    fn summary_reports_counts_from_the_database() {
        let dir = temp_dir("memory-summary");
        seed_db(&dir);
        let summary = get_memory_summary(&dir).unwrap();
        assert!(summary.contains("Applications today: 0"), "got: {summary}");
        assert!(summary.contains("Unresolved pending items: 1"), "got: {summary}");
    }

    /// The prompt baked this in once at spawn, so counters went stale mid-session.
    /// As a tool it must reflect writes that happened after the session started.
    #[test]
    fn summary_reflects_writes_made_after_the_session_started() {
        let dir = temp_dir("memory-fresh");
        seed_db(&dir);
        assert!(get_memory_summary(&dir).unwrap().contains("Applications today: 0"));

        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        conn.execute(
            "INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo)
               VALUES (1, datetime('now'), 'x', 'form')",
            [],
        )
        .unwrap();

        assert!(get_memory_summary(&dir).unwrap().contains("Applications today: 1"));
    }
}
