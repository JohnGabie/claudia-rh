// One file per domain (see .claude/MCP-DESIGN.md — extensibility contract):
// adding tool #10 must cost the same as tool #4. Shared helpers live here.

mod pendencias;
mod profile;
mod vagas;

pub use pendencias::{close_pendencia, close_pendencias_vaga, get_pendencia_vaga, list_pendencias};
pub use profile::update_profile;
pub use vagas::{create_pendencia, register_candidatura, register_vaga, update_vaga_status};

use std::path::Path;

pub(crate) fn open_db(data_dir: &Path) -> Result<rusqlite::Connection, String> {
    rusqlite::Connection::open(data_dir.join("claudia_rh.db"))
        .map_err(|e| format!("erro ao abrir a base de dados: {e}"))
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};

    pub fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "claudia-mcp-test-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Creates the real schema (schema.sql + the fonte_conexao migration from
    /// db::init) plus one seeded vaga + pendência.
    pub fn seed_db(dir: &Path) {
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        conn.execute_batch(include_str!("../../db/schema.sql")).unwrap();
        conn.execute_batch("ALTER TABLE vagas ADD COLUMN fonte_conexao TEXT;").unwrap();
        conn.execute_batch(
            "INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status)
               VALUES (1, 'Dev Rust', 'ACME', 'linkedin', 'https://x/1', datetime('now'), 'descoberta');
             INSERT INTO pendencias (id, vaga_id, criada_em, categoria, descricao)
               VALUES (10, 1, datetime('now'), 'salario', 'Confirmar pretensão salarial');",
        )
        .unwrap();
    }
}
