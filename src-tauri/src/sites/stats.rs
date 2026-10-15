//! Per-site numbers for the Sites screen: re-login notices, opens, vagas and applications.
//! Vagas are tied to a site only by resolving `vagas.url` with the same domain rules the gate uses.
use super::repo::{self, resolver_em};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SiteStats {
    pub relogins: i64, // notices ever created for the site (open or resolved)
    pub aberturas: i64,
    pub ultima_abertura: Option<String>,
    pub vagas: i64,        // vagas whose URL resolves to this site
    pub candidaturas: i64, // candidaturas of those vagas
    pub ultima_candidatura: Option<String>,
}

pub fn estatisticas(conn: &Connection, chave: &str) -> Result<SiteStats, String> {
    // Load the site list once; every vaga URL is resolved against it in memory.
    let sites = repo::listar(conn)?;
    let (aberturas, ultima_abertura): (i64, Option<String>) = conn
        .query_row("SELECT aberturas, ultima_abertura FROM contas_browser WHERE chave = ?1", [chave], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("site '{chave}' não existe"),
            other => other.to_string(),
        })?;
    let relogins: i64 = conn
        .query_row("SELECT COUNT(*) FROM avisos_sites WHERE chave = ?1", [chave], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let mut vagas = 0;
    let mut st = conn.prepare("SELECT url FROM vagas").map_err(|e| e.to_string())?;
    let urls = st.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
    for url in urls {
        if resolver_em(&sites, &url.map_err(|e| e.to_string())?).is_some_and(|s| s.chave == chave) {
            vagas += 1;
        }
    }

    let mut candidaturas = 0;
    let mut ultima_candidatura: Option<String> = None;
    let mut st = conn
        .prepare("SELECT v.url, c.enviada_em FROM candidaturas c JOIN vagas v ON v.id = c.vaga_id")
        .map_err(|e| e.to_string())?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
    for row in rows {
        let (url, quando) = row.map_err(|e| e.to_string())?;
        if resolver_em(&sites, &url).is_some_and(|s| s.chave == chave) {
            candidaturas += 1;
            if ultima_candidatura.as_ref().is_none_or(|m| quando > *m) {
                ultima_candidatura = Some(quando);
            }
        }
    }

    Ok(SiteStats { relogins, aberturas, ultima_abertura, vagas, candidaturas, ultima_candidatura })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::params;

    const LI: &str = r#"{"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}"#;
    const ACME: &str = r#"{"chave":"acme","nome":"Acme","url_login":"https://acme.example.com/careers","dominios":["acme.example.com"],"dominios_ats":["boards.example-ats.com"],"caminho_ats":"/acme","tipo":"empresa","regiao":"EU"}"#;
    const GLOBEX: &str = r#"{"chave":"globex","nome":"Globex","url_login":"https://globex.example.com/careers","dominios":["globex.example.com"],"dominios_ats":["boards.example-ats.com"],"caminho_ats":"/globex","tipo":"empresa","regiao":"EU"}"#;

    fn db_with(json_sites: &str) -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        merge(&c, &parse_seed(&format!(r#"{{"versao":1,"sites":[{json_sites}]}}"#)).unwrap()).unwrap();
        c
    }
    fn db_with_linkedin_and_ats() -> Connection {
        db_with(&format!("{LI},{ACME}"))
    }
    fn db_with_two_ats_companies() -> Connection {
        db_with(&format!("{ACME},{GLOBEX}"))
    }

    fn vaga(c: &Connection, url: &str) -> i64 {
        c.execute("INSERT INTO vagas (titulo, empresa, plataforma, url, descoberta_em) VALUES ('t','e','p',?1,datetime('now'))", [url]).unwrap();
        c.last_insert_rowid()
    }
    fn candidatura(c: &Connection, vaga_id: i64, quando: &str) {
        c.execute("INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES (?1, ?2, '', 'x')", params![vaga_id, quando]).unwrap();
    }

    #[test]
    fn counts_only_the_vagas_and_candidaturas_that_resolve_to_the_site() {
        let c = db_with_linkedin_and_ats();
        let a = vaga(&c, "https://www.linkedin.com/jobs/view/1");
        let _b = vaga(&c, "https://www.linkedin.com/jobs/view/2");
        let outra = vaga(&c, "https://elsewhere.example/job/9");
        // a vaga of ANOTHER known site: it must not count for linkedin (an always-true match would)
        let da_acme = vaga(&c, "https://boards.example-ats.com/acme/jobs/5");
        candidatura(&c, da_acme, "2026-10-08 10:00:00");
        candidatura(&c, a, "2026-10-01 10:00:00");
        candidatura(&c, outra, "2026-10-02 10:00:00");
        let s = estatisticas(&c, "linkedin").unwrap();
        assert_eq!((s.vagas, s.candidaturas), (2, 1));
        assert_eq!(s.ultima_candidatura.as_deref(), Some("2026-10-01 10:00:00"));
    }

    #[test]
    fn a_vaga_on_a_shared_ats_host_counts_only_for_the_site_that_owns_the_path() {
        // two companies on the same ATS host, told apart by caminho_ats
        let c = db_with_two_ats_companies();
        // Both directions: with only an acme vaga the test passes even when the path is ignored,
        // because acme sorts first and wins the tie. The globex vaga is what exposes that.
        vaga(&c, "https://boards.example-ats.com/acme/jobs/1");
        vaga(&c, "https://boards.example-ats.com/globex/jobs/2");
        assert_eq!(estatisticas(&c, "acme").unwrap().vagas, 1);
        assert_eq!(estatisticas(&c, "globex").unwrap().vagas, 1);
    }

    #[test]
    fn relogins_count_every_notice_open_or_resolved() {
        let c = db_with_linkedin_and_ats();
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        crate::sites::avisos::resolver_do_site(&c, "linkedin").unwrap();
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        assert_eq!(estatisticas(&c, "linkedin").unwrap().relogins, 2);
    }

    #[test]
    fn a_site_that_does_not_exist_is_an_error_and_an_unused_site_is_all_zero() {
        let c = db_with_linkedin_and_ats();
        assert!(estatisticas(&c, "nope").is_err());
        let s = estatisticas(&c, "linkedin").unwrap();
        assert_eq!((s.relogins, s.aberturas, s.vagas, s.candidaturas), (0, 0, 0, 0));
        assert!(s.ultima_abertura.is_none() && s.ultima_candidatura.is_none());
    }

    // Extra: opens read from the column the gate writes, and the latest application wins.
    #[test]
    fn opens_come_from_the_site_row_and_the_latest_application_date_is_the_max() {
        let c = db_with_linkedin_and_ats();
        c.execute("UPDATE contas_browser SET aberturas = 3, ultima_abertura = '2026-10-05 09:00:00' WHERE chave = 'linkedin'", []).unwrap();
        let a = vaga(&c, "https://www.linkedin.com/jobs/view/1");
        candidatura(&c, a, "2026-10-03 10:00:00");
        candidatura(&c, a, "2026-10-09 10:00:00");
        candidatura(&c, a, "2026-10-04 10:00:00");
        let s = estatisticas(&c, "linkedin").unwrap();
        assert_eq!((s.aberturas, s.ultima_abertura.as_deref()), (3, Some("2026-10-05 09:00:00")));
        assert_eq!((s.candidaturas, s.ultima_candidatura.as_deref()), (3, Some("2026-10-09 10:00:00")));
    }
}
