//! What the agent may open. Every failure to read the list is a refusal (fail closed).
use super::dominio::host_and_path;
use super::repo;
use crate::browser::sitegate::SiteGate;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum Decision {
    /// The agent may open the URL; carries the site key.
    Allow(String),
    /// The message is shown to the agent as the tool result.
    Refuse(String),
}

pub const MSG_FORA_DA_LISTA: &str = "RECUSADO: este site não está na lista de sites da Claudia, então você não pode abri-lo. \
Salve a vaga com register_vaga, marque com update_vaga_status como 'pulada' com o motivo \"site fora da lista: <domínio>\" \
e siga para a próxima. Não tente outro endereço do mesmo site. Chame browser_contas para ver onde você pode abrir páginas.";

/// Counts a refused domain. Stores only the host and the URL without its query string or fragment.
pub fn registrar_recusa(conn: &Connection, url: &str) {
    let Some((host, path)) = host_and_path(url) else { return };
    let limpa = format!("https://{host}{path}");
    let _ = conn.execute(
        "INSERT INTO dominios_recusados (dominio, contagem, ultima_vez, ultima_url) VALUES (?1, 1, datetime('now'), ?2) \
         ON CONFLICT(dominio) DO UPDATE SET contagem = contagem + 1, ultima_vez = datetime('now'), ultima_url = excluded.ultima_url",
        params![host, limpa],
    );
}

pub fn decidir(conn: &Connection, url: &str) -> Decision {
    let site = match repo::resolver(conn, url) {
        Ok(s) => s,
        Err(e) => return Decision::Refuse(format!("RECUSADO: não foi possível consultar a lista de sites ({e}). Tente de novo.")),
    };
    let Some(s) = site else {
        registrar_recusa(conn, url);
        return Decision::Refuse(MSG_FORA_DA_LISTA.to_string());
    };
    if !s.ia_pode_usar {
        return Decision::Refuse(format!("RECUSADO: o usuário não liberou o site \"{}\" para a IA. Pule esta fonte e siga para a próxima.", s.nome));
    }
    if s.regra_login != "nenhuma" && s.estado != "logada" {
        return Decision::Refuse(format!("RECUSADO: o site \"{}\" não está com login confirmado. Pule esta fonte; o usuário precisa entrar nele de novo.", s.nome));
    }
    Decision::Allow(s.chave)
}

/// The gate backed by the main database file. Opens its own connection per call: the MCP server
/// runs in a separate process from the app.
pub struct DbSiteGate {
    pub data_dir: PathBuf,
}

impl DbSiteGate {
    fn conn(&self) -> Result<Connection, String> {
        let c = Connection::open(self.data_dir.join("claudia_rh.db")).map_err(|e| e.to_string())?;
        c.busy_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())?;
        Ok(c)
    }
}

impl SiteGate for DbSiteGate {
    fn check_open(&self, url: &str) -> Result<String, String> {
        let conn = self.conn().map_err(|e| format!("RECUSADO: não foi possível consultar a lista de sites ({e}). Tente de novo."))?;
        match decidir(&conn, url) {
            Decision::Allow(k) => Ok(k),
            Decision::Refuse(m) => Err(m),
        }
    }

    fn login_wall(&self, url: &str) {
        let Ok(conn) = self.conn() else { return };
        let Ok(Some(site)) = repo::resolver(&conn, url) else { return };
        if site.regra_login == "nenhuma" {
            return; // a public site that happens to show a sign-in box is not "logged out"
        }
        if repo::marcar_precisa_login(&conn, &site.chave).unwrap_or(false) {
            let _ = conn.execute("INSERT INTO avisos_sites (chave, criado_em) VALUES (?1, datetime('now'))", [&site.chave]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::repo;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        let seed = r#"{"versao":1,"sites":[
            {"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"},
            {"chave":"pub","nome":"Pub","url_login":"https://pub.com/careers","dominios":["pub.com"],"tipo":"empresa","regiao":"EU","regra_login":"nenhuma"}]}"#;
        merge(&c, &parse_seed(seed).unwrap()).unwrap();
        c
    }
    fn refused(d: Decision) -> String {
        match d { Decision::Refuse(m) => m, Decision::Allow(k) => panic!("allowed {k}") }
    }

    #[test]
    fn a_site_off_the_list_is_refused_and_counted() {
        let c = db();
        let m = refused(decidir(&c, "https://stranger.org/jobs/1?token=secret"));
        assert!(m.contains("register_vaga") && m.contains("pulada"), "{m}");
        decidir(&c, "https://stranger.org/other");
        let (n, url): (i64, String) = c.query_row("SELECT contagem, ultima_url FROM dominios_recusados WHERE dominio='stranger.org'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(n, 2);
        assert!(!url.contains("token") && !url.contains('?'), "the query string must not be stored: {url}");
    }

    #[test]
    fn a_listed_site_stays_closed_until_the_user_opens_it() {
        let c = db();
        assert!(refused(decidir(&c, "https://pub.com/careers")).contains("não liberou"));
    }

    #[test]
    fn a_site_that_needs_login_is_refused_until_it_is_logged_in() {
        let c = db();
        repo::marcar_logada(&c, "linkedin").unwrap();
        repo::definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        assert_eq!(decidir(&c, "https://www.linkedin.com/jobs/1"), Decision::Allow("linkedin".into()));
        repo::marcar_precisa_login(&c, "linkedin").unwrap();
        assert!(refused(decidir(&c, "https://www.linkedin.com/jobs/1")).contains("login"));
    }

    #[test]
    fn a_site_without_login_rule_only_needs_the_permission() {
        let c = db();
        repo::definir_ia_pode_usar(&c, "pub", true).unwrap();
        assert_eq!(decidir(&c, "https://pub.com/careers/1"), Decision::Allow("pub".into()));
    }

    #[test]
    fn tricky_urls_do_not_borrow_the_permission_of_a_listed_site() {
        let c = db();
        repo::marcar_logada(&c, "linkedin").unwrap();
        repo::definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        for url in ["https://linkedin.com@evil.com/", "https://evil.com\\@linkedin.com", "https://linkedin.com.evil.com/"] {
            assert!(matches!(decidir(&c, url), Decision::Refuse(_)), "{url}");
        }
    }

    #[test]
    fn an_unreadable_database_refuses_and_never_panics() {
        let c = Connection::open_in_memory().unwrap(); // no tables at all
        assert!(matches!(decidir(&c, "https://www.linkedin.com/"), Decision::Refuse(m) if m.contains("não foi possível consultar")));
    }

    #[test]
    fn the_db_gate_turns_a_login_wall_into_a_state_change_and_one_notice() {
        let dir = crate::mcp::tools::test_support::temp_dir("site-gate");
        crate::mcp::tools::test_support::seed_db(&dir);
        {
            let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            crate::sites::seed::merge(&conn, &crate::sites::seed::parse_seed(
                r#"{"versao":1,"sites":[{"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}]}"#).unwrap()).unwrap();
            repo::marcar_logada(&conn, "linkedin").unwrap();
        }
        let gate = DbSiteGate { data_dir: dir.clone() };
        gate.login_wall("https://www.linkedin.com/authwall");
        gate.login_wall("https://www.linkedin.com/authwall");
        gate.login_wall("https://stranger.org/login"); // off the list: ignored
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        assert_eq!(repo::obter(&conn, "linkedin").unwrap().unwrap().estado, "precisa_login");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM avisos_sites WHERE chave='linkedin'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "the notice is created once, not at every read");
    }

    #[test]
    fn a_public_site_never_becomes_login_needed() {
        let dir = crate::mcp::tools::test_support::temp_dir("site-gate-public");
        crate::mcp::tools::test_support::seed_db(&dir);
        let before = {
            let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            crate::sites::seed::merge(&conn, &crate::sites::seed::parse_seed(
                r#"{"versao":1,"sites":[{"chave":"pub","nome":"Pub","url_login":"https://pub.com/careers","dominios":["pub.com"],"tipo":"empresa","regiao":"EU","regra_login":"nenhuma"}]}"#).unwrap()).unwrap();
            repo::obter(&conn, "pub").unwrap().unwrap().estado
        };
        DbSiteGate { data_dir: dir.clone() }.login_wall("https://pub.com/careers/login");
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let site = repo::obter(&conn, "pub").unwrap().unwrap();
        assert_eq!(site.estado, before, "a site without a login rule keeps its state");
        assert_ne!(site.estado, "precisa_login");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM avisos_sites WHERE chave='pub'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }
}
