//! "Log in again" notices, shown in the Sites screen and in the sidebar badge.
use rusqlite::{Connection, OptionalExtension};
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

/// The user dismisses ONE notice ("log in again") without logging in: only the row with this id is
/// resolved. The site row is never touched (it stays `precisa_login` with the AI closed), and no other
/// notice of the same site is resolved. Returns the site key when a notice was actually dismissed, and
/// `None` for an unknown or already resolved id (idempotent, nothing changes). One statement, so the key
/// returned is exactly the row that was updated.
pub fn dispensar(conn: &Connection, id: i64) -> Result<Option<String>, String> {
    conn.query_row(
        "UPDATE avisos_sites SET resolvido = 1, resolvido_em = datetime('now') WHERE id = ?1 AND resolvido = 0 RETURNING chave",
        [id],
        |r| r.get::<_, String>(0),
    )
    .optional()
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

    const TWO_SITES: &str = r#"{"versao":1,"sites":[
        {"chave":"a","nome":"Alpha","url_login":"https://a.com/","dominios":["a.com"],"tipo":"geral","regiao":"EU"},
        {"chave":"b","nome":"Beta","url_login":"https://b.com/","dominios":["b.com"],"tipo":"geral","regiao":"EU"}]}"#;

    fn db2() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        merge(&c, &parse_seed(TWO_SITES).unwrap()).unwrap();
        c
    }

    fn notice_id(c: &Connection, chave: &str) -> i64 {
        c.query_row("SELECT id FROM avisos_sites WHERE chave = ?1 AND resolvido = 0", [chave], |r| r.get(0)).unwrap()
    }

    /// (estado, regra_login, ia_pode_usar, verificada_em) of the site row.
    fn site_row(c: &Connection, chave: &str) -> (String, String, i64, Option<String>) {
        c.query_row(
            "SELECT estado, regra_login, ia_pode_usar, verificada_em FROM contas_browser WHERE chave = ?1",
            [chave],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
    }

    fn resolved_at(c: &Connection, id: i64) -> (i64, Option<String>) {
        c.query_row("SELECT resolvido, resolvido_em FROM avisos_sites WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
    }

    #[test]
    fn dismissing_an_open_notice_resolves_it_and_returns_the_site_key() {
        let c = db2();
        crate::sites::repo::marcar_precisa_login(&c, "a").unwrap();
        criar(&c, "a").unwrap();
        criar(&c, "b").unwrap();
        let id = notice_id(&c, "a");
        assert_eq!(dispensar(&c, id).unwrap(), Some("a".to_string()));
        assert_eq!(contar(&c).unwrap(), 1);
        let open = listar_abertos(&c).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].chave, "b");
        let (resolvido, em) = resolved_at(&c, id);
        assert_eq!(resolvido, 1);
        assert!(em.is_some(), "resolvido_em is written like resolver_do_site does");
    }

    #[test]
    fn dismissing_never_touches_the_site_row() {
        let c = db2();
        crate::sites::repo::marcar_precisa_login(&c, "a").unwrap();
        criar(&c, "a").unwrap();
        let before = site_row(&c, "a");
        assert_eq!(before.0, "precisa_login");
        dispensar(&c, notice_id(&c, "a")).unwrap();
        assert_eq!(site_row(&c, "a"), before);
    }

    #[test]
    fn dismissing_only_resolves_the_notice_that_was_named() {
        let c = db2();
        // Old resolved notice of "a", then a new open one for the same site.
        criar(&c, "a").unwrap();
        let old = notice_id(&c, "a");
        c.execute("UPDATE avisos_sites SET resolvido = 1, resolvido_em = '2020-01-01 00:00:00' WHERE id = ?1", [old]).unwrap();
        criar(&c, "a").unwrap();
        criar(&c, "b").unwrap();
        let new_a = notice_id(&c, "a");
        let b = notice_id(&c, "b");
        assert_ne!(old, new_a);

        dispensar(&c, new_a).unwrap();
        assert_eq!(resolved_at(&c, old), (1, Some("2020-01-01 00:00:00".to_string())), "the old notice stays as it was");
        assert_eq!(resolved_at(&c, new_a).0, 1);
        assert_eq!(resolved_at(&c, b).0, 0, "another site's notice is not touched");

        // And dismissing the other site's notice does not touch this site's.
        criar(&c, "a").unwrap();
        let again_a = notice_id(&c, "a");
        dispensar(&c, b).unwrap();
        assert_eq!(resolved_at(&c, again_a).0, 0);
        assert_eq!(resolved_at(&c, new_a).0, 1);
    }

    #[test]
    fn an_unknown_or_already_resolved_id_is_ok_and_changes_nothing() {
        let c = db2();
        criar(&c, "a").unwrap();
        criar(&c, "b").unwrap();
        let a = notice_id(&c, "a");
        assert_eq!(dispensar(&c, 9999).unwrap(), None);
        assert_eq!(contar(&c).unwrap(), 2);
        assert_eq!(dispensar(&c, a).unwrap(), Some("a".to_string()));
        let first = resolved_at(&c, a);
        assert_eq!(dispensar(&c, a).unwrap(), None, "dismissing twice is a no-op");
        assert_eq!(resolved_at(&c, a), first);
        assert_eq!(contar(&c).unwrap(), 1);
    }

    #[test]
    fn a_new_login_wall_after_a_dismissal_raises_a_new_open_notice() {
        use crate::browser::sitegate::SiteGate;
        use crate::sites::gate::DbSiteGate;
        let dir = crate::mcp::tools::test_support::temp_dir("site-notice-dismiss");
        crate::mcp::tools::test_support::seed_db(&dir);
        let conn = Connection::open(dir.join("claudia_rh.db")).unwrap();
        merge(&conn, &parse_seed(r#"{"versao":1,"sites":[{"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}]}"#).unwrap()).unwrap();
        crate::sites::repo::marcar_logada(&conn, "linkedin").unwrap();
        let gate = DbSiteGate { data_dir: dir.clone() };
        gate.login_wall("https://www.linkedin.com/authwall");
        let first = notice_id(&conn, "linkedin");
        assert_eq!(dispensar(&conn, first).unwrap(), Some("linkedin".to_string()));
        assert_eq!(contar(&conn).unwrap(), 0);
        gate.login_wall("https://www.linkedin.com/authwall");
        gate.login_wall("https://www.linkedin.com/authwall");
        assert_eq!(contar(&conn).unwrap(), 1, "one open notice per site, as before");
        assert_ne!(notice_id(&conn, "linkedin"), first, "it is a NEW notice");
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM avisos_sites WHERE chave='linkedin'", [], |r| r.get(0)).unwrap();
        assert_eq!(total, 2);
    }
}
