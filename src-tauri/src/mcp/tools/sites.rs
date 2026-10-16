//! `browser_contas`: tells the agent where it may open pages. Read-only.
use std::path::Path;

use super::open_db;
use crate::sites::repo;

pub fn browser_contas(data_dir: &Path) -> Result<String, String> {
    let conn = open_db(data_dir)?;
    let all = repo::listar(&conn)?;
    // Same predicate as the gate (sites::gate): released by the user AND no login pending. A site that
    // needs login is refused by the gate even if an old row still has `ia_pode_usar` on, so do not list it.
    let open: Vec<_> = all
        .iter()
        .filter(|s| s.ia_pode_usar && (s.regra_login == "nenhuma" || s.estado == "logada"))
        .collect();
    let closed = all.len() - open.len();
    if open.is_empty() {
        return Ok(format!("Nenhum site liberado pelo usuário ({closed} na lista, todos fechados ou esperando o login do usuário). Não abra páginas com browser_open; avise o usuário numa pendência se isso travar o trabalho."));
    }
    let mut out = format!("{} site(s) liberado(s) para você abrir com browser_open:\n", open.len());
    for s in open {
        let login = match (s.regra_login.as_str(), s.estado.as_str()) {
            ("nenhuma", _) => "sem login".to_string(),
            _ => "login confirmado".to_string(),
        };
        let papel = if s.papel == "descoberta" { ", só descoberta (as vagas levam a outro site)" } else { "" };
        out.push_str(&format!("- {} [{} · {}] {login}{papel} → {}\n", s.nome, s.tipo, s.regiao, s.url_login));
    }
    out.push_str(&format!("Outros {closed} site(s) da lista estão fechados ou esperam o login do usuário; qualquer endereço fora da lista é recusado."));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};
    use crate::sites::{repo, seed};

    #[test]
    fn it_lists_only_the_sites_the_user_opened_and_says_how_many_are_closed() {
        let dir = temp_dir("browser-contas");
        seed_db(&dir);
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        seed::merge(&conn, &seed::parse_seed(
            r#"{"versao":1,"sites":[
            {"chave":"a","nome":"Alpha","url_login":"https://a.com/","dominios":["a.com"],"tipo":"geral","regiao":"EU","regra_login":"nenhuma"},
            {"chave":"b","nome":"Beta","url_login":"https://b.com/","dominios":["b.com"],"tipo":"geral","regiao":"EU"}]}"#).unwrap()).unwrap();
        repo::definir_ia_pode_usar(&conn, "a", true).unwrap();
        let out = browser_contas(&dir).unwrap();
        assert!(out.contains("Alpha") && out.contains("https://a.com/"), "{out}");
        assert!(!out.contains("Beta"), "{out}");
        assert!(out.contains("1 site"), "it must say how many are closed: {out}");
    }

    #[test]
    fn it_says_so_when_nothing_is_open() {
        let dir = temp_dir("browser-contas-empty");
        seed_db(&dir);
        assert!(browser_contas(&dir).unwrap().contains("Nenhum site liberado"));
    }

    fn contas_with(sites: &str, dir: &std::path::Path) -> rusqlite::Connection {
        seed_db(dir);
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        seed::merge(&conn, &seed::parse_seed(sites).unwrap()).unwrap();
        conn
    }

    const DOIS: &str = r#"{"versao":1,"sites":[
        {"chave":"pub","nome":"Publico","url_login":"https://pub.com/","dominios":["pub.com"],"tipo":"geral","regiao":"EU","regra_login":"nenhuma"},
        {"chave":"man","nome":"Manual","url_login":"https://man.com/","dominios":["man.com"],"tipo":"geral","regiao":"EU","regra_login":"manual"}]}"#;

    #[test]
    fn it_does_not_list_a_site_that_needs_login_even_with_the_ai_flag_on() {
        let dir = temp_dir("browser-contas-login-wall");
        let conn = contas_with(DOIS, &dir);
        // Defensive: an old row with the flag on while the state is precisa_login.
        conn.execute("UPDATE contas_browser SET ia_pode_usar = 1 WHERE chave = 'man'", []).unwrap();
        let out = browser_contas(&dir).unwrap();
        assert!(!out.contains("Manual"), "{out}");
        assert!(out.contains("Nenhum site liberado"), "{out}");
    }

    #[test]
    fn it_lists_a_logged_site_and_a_public_site_with_the_ai_flag_on() {
        let dir = temp_dir("browser-contas-logged");
        let conn = contas_with(DOIS, &dir);
        conn.execute("UPDATE contas_browser SET ia_pode_usar = 1, estado = 'logada' WHERE chave = 'man'", []).unwrap();
        repo::definir_ia_pode_usar(&conn, "pub", true).unwrap();
        let out = browser_contas(&dir).unwrap();
        assert!(out.contains("Manual") && out.contains("login confirmado"), "{out}");
        assert!(out.contains("Publico") && out.contains("sem login"), "{out}");
    }
}
