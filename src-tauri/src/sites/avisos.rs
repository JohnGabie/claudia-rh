//! "Log in again" notices, shown in the Sites screen and in the sidebar badge.
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Aviso {
    pub id: i64,
    pub chave: String,
    pub nome: String,
    pub criado_em: String,
}

/// Idempotent: a site has at most one open notice.
pub fn criar(conn: &Connection, chave: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO avisos_sites (chave, criado_em) SELECT ?1, datetime('now') \
         WHERE NOT EXISTS (SELECT 1 FROM avisos_sites WHERE chave = ?1 AND resolvido = 0)",
        [chave],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn contar(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM avisos_sites WHERE resolvido = 0", [], |r| r.get(0)).map_err(|e| e.to_string())
}

pub fn listar_abertos(conn: &Connection) -> Result<Vec<Aviso>, String> {
    let mut st = conn
        .prepare("SELECT a.id, a.chave, c.nome, a.criado_em FROM avisos_sites a JOIN contas_browser c ON c.chave = a.chave WHERE a.resolvido = 0 ORDER BY a.criado_em")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map([], |r| Ok(Aviso { id: r.get(0)?, chave: r.get(1)?, nome: r.get(2)?, criado_em: r.get(3)? }))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

pub fn resolver_do_site(conn: &Connection, chave: &str) -> Result<(), String> {
    conn.execute("UPDATE avisos_sites SET resolvido = 1, resolvido_em = datetime('now') WHERE chave = ?1 AND resolvido = 0", [chave])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        merge(&c, &parse_seed(r#"{"versao":1,"sites":[{"chave":"a","nome":"Alpha","url_login":"https://a.com/","dominios":["a.com"],"tipo":"geral","regiao":"EU"}]}"#).unwrap()).unwrap();
        c
    }

    #[test]
    fn one_open_notice_per_site_even_if_created_twice() {
        let c = db();
        criar(&c, "a").unwrap();
        criar(&c, "a").unwrap();
        assert_eq!(contar(&c).unwrap(), 1);
        assert_eq!(listar_abertos(&c).unwrap()[0].nome, "Alpha");
    }

    #[test]
    fn confirming_the_login_resolves_the_notice() {
        let c = db();
        criar(&c, "a").unwrap();
        resolver_do_site(&c, "a").unwrap();
        assert_eq!(contar(&c).unwrap(), 0);
    }

    #[test]
    fn removing_the_site_removes_its_notices() {
        let c = db();
        c.execute("PRAGMA foreign_keys=ON", []).unwrap();
        c.execute("UPDATE contas_browser SET embutida = 0 WHERE chave='a'", []).unwrap();
        criar(&c, "a").unwrap();
        crate::sites::repo::remover(&c, "a").unwrap();
        assert_eq!(contar(&c).unwrap(), 0);
    }
}
