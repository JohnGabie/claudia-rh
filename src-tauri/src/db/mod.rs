pub mod queries;

use rusqlite::{Connection, Result};
use std::path::Path;

const SCHEMA: &str = include_str!("schema.sql");

pub fn init(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;
    // Idempotent migrations — "duplicate column" errors are expected and safe to ignore
    if let Err(e) = conn.execute_batch("ALTER TABLE vagas ADD COLUMN variante_id TEXT;") {
        if !e.to_string().contains("duplicate column") { eprintln!("[db migration] vagas.variante_id: {e}"); }
    }
    if let Err(e) = conn.execute_batch("ALTER TABLE candidaturas ADD COLUMN resultado TEXT;") {
        if !e.to_string().contains("duplicate column") { eprintln!("[db migration] candidaturas.resultado: {e}"); }
    }
    if let Err(e) = conn.execute_batch("ALTER TABLE sessoes ADD COLUMN tempo_pausado_segundos INTEGER DEFAULT 0;") {
        if !e.to_string().contains("duplicate column") { eprintln!("[db migration] sessoes.tempo_pausado_segundos: {e}"); }
    }
    if let Err(e) = conn.execute_batch("ALTER TABLE sessoes ADD COLUMN pausada_em TEXT;") {
        if !e.to_string().contains("duplicate column") { eprintln!("[db migration] sessoes.pausada_em: {e}"); }
    }
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
    Ok(conn)
}
