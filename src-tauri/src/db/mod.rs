pub mod queries;

use rusqlite::{Connection, Result};
use std::path::Path;

const SCHEMA: &str = include_str!("schema.sql");

/// Applies schema and idempotent migrations to an open connection.
/// Extracted so tests can call it on an in-memory connection without touching the filesystem.
pub(crate) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;
    // Idempotent migrations — "duplicate column" errors are expected and safe to ignore
    for (sql, label) in [
        ("ALTER TABLE vagas ADD COLUMN variante_id TEXT;", "vagas.variante_id"),
        ("ALTER TABLE candidaturas ADD COLUMN resultado TEXT;", "candidaturas.resultado"),
        ("ALTER TABLE sessoes ADD COLUMN tempo_pausado_segundos INTEGER DEFAULT 0;", "sessoes.tempo_pausado_segundos"),
        ("ALTER TABLE sessoes ADD COLUMN pausada_em TEXT;", "sessoes.pausada_em"),
        ("ALTER TABLE vagas ADD COLUMN fonte_conexao TEXT;", "vagas.fonte_conexao"),
    ] {
        if let Err(e) = conn.execute_batch(sql) {
            if !e.to_string().contains("duplicate column") {
                eprintln!("[db migration] {label}: {e}");
            }
        }
    }
    // Close any ghost sessions left open by a previous crash or forced quit
    let _ = conn.execute_batch(
        "UPDATE sessoes SET terminada_em = datetime('now'), motivo_termino = 'encerrada_no_arranque' WHERE terminada_em IS NULL;"
    );
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS feedbacks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            gerado_em TEXT NOT NULL,
            gatilho TEXT NOT NULL,
            resumo TEXT NOT NULL,
            conteudo_completo TEXT NOT NULL,
            candidaturas_ate_aqui INTEGER NOT NULL DEFAULT 0
        );",
    )?;
    Ok(())
}

pub fn init(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    apply(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        apply(&conn).expect("apply schema");
        conn
    }

    #[test]
    fn schema_creates_all_tables() {
        let conn = mem();
        let mut tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        tables.retain(|t| t != "sqlite_sequence");
        for expected in ["candidaturas", "feedbacks", "pendencias", "propostas_perfil", "sessoes", "vagas"] {
            assert!(tables.contains(&expected.to_string()), "missing table: {expected}");
        }
    }

    #[test]
    fn apply_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply(&conn).expect("first apply");
        apply(&conn).expect("second apply must not fail");
    }

    #[test]
    fn ghost_sessions_closed_on_startup() {
        let conn = mem();
        conn.execute(
            "INSERT INTO sessoes (iniciada_em, motivo_disparo) VALUES (datetime('now', '-1 hour'), 'manual')",
            [],
        ).unwrap();
        // Simulate restart: apply closes sessions with no terminada_em
        apply(&conn).unwrap();
        let open: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessoes WHERE terminada_em IS NULL",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(open, 0, "ghost session must be closed on startup");
    }
}
