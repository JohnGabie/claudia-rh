//! Reads and writes of the stored site list. Every rule the UI shows is enforced here too: the
//! screen is never the only defence.
use super::dominio::{chave_from_url, host_and_path, host_matches, url_matches, valid_domain, DomainRules};
use super::modelo::{is_public_suffix, TIPOS};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Site {
    pub chave: String,
    pub nome: String,
    pub url_login: String,
    pub dominios: Vec<String>,
    pub dominios_ats: Vec<String>,
    pub caminho_ats: Option<String>,
    pub tipo: String,
    pub regiao: String,
    pub pais: Option<String>,
    pub papel: String,
    pub regra_login: String,
    pub ia_pode_usar: bool,
    pub embutida: bool,
    pub padrao_atual: bool,
    pub estado: String,
    pub verificada_em: Option<String>,
}

const COLS: &str = "chave, nome, url_login, dominios, dominios_ats, caminho_ats, tipo, regiao, pais, papel, \
                    regra_login, ia_pode_usar, embutida, padrao_atual, estado, verificada_em";

fn json_list(s: String) -> Vec<String> {
    serde_json::from_str(&s).unwrap_or_default()
}

fn row(r: &Row) -> rusqlite::Result<Site> {
    Ok(Site {
        chave: r.get(0)?,
        nome: r.get(1)?,
        url_login: r.get(2)?,
        dominios: json_list(r.get(3)?),
        dominios_ats: json_list(r.get(4)?),
        caminho_ats: r.get(5)?,
        tipo: r.get(6)?,
        regiao: r.get(7)?,
        pais: r.get(8)?,
        papel: r.get(9)?,
        regra_login: r.get(10)?,
        ia_pode_usar: r.get::<_, i64>(11)? != 0,
        embutida: r.get::<_, i64>(12)? != 0,
        padrao_atual: r.get::<_, i64>(13)? != 0,
        estado: r.get(14)?,
        verificada_em: r.get(15)?,
    })
}

pub fn listar(conn: &Connection) -> Result<Vec<Site>, String> {
    let mut st = conn
        .prepare(&format!("SELECT {COLS} FROM contas_browser ORDER BY nome COLLATE NOCASE"))
        .map_err(|e| e.to_string())?;
    let rows = st.query_map([], row).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

pub fn obter(conn: &Connection, chave: &str) -> Result<Option<Site>, String> {
    let mut st = conn.prepare(&format!("SELECT {COLS} FROM contas_browser WHERE chave = ?1")).map_err(|e| e.to_string())?;
    let mut rows = st.query_map([chave], row).map_err(|e| e.to_string())?;
    rows.next().transpose().map_err(|e| e.to_string())
}

fn exigir(conn: &Connection, chave: &str) -> Result<Site, String> {
    obter(conn, chave)?.ok_or_else(|| format!("site '{chave}' não existe"))
}

/// Turning the permission ON needs a site that is logged in (or that asks for no login). Turning it
/// OFF always works.
pub fn definir_ia_pode_usar(conn: &Connection, chave: &str, valor: bool) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if valor && s.regra_login != "nenhuma" && s.estado != "logada" {
        return Err("Entre neste site (passo 1 da página do site) antes de liberar a IA.".into());
    }
    conn.execute("UPDATE contas_browser SET ia_pode_usar = ?2 WHERE chave = ?1", params![chave, valor as i64])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn marcar_logada(conn: &Connection, chave: &str) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if s.regra_login == "nenhuma" {
        return Err("Este site não exige login.".into());
    }
    conn.execute("UPDATE contas_browser SET estado = 'logada', verificada_em = datetime('now') WHERE chave = ?1", [chave])
        .map_err(|e| e.to_string())?;
    let _ = super::avisos::resolver_do_site(conn, chave);
    Ok(())
}

/// valor = true: the site asks for no login. valor = false: back to manual login: the site is marked
/// precisa_login, the AI permission is closed and the relogin notice is raised, in one transaction.
pub fn definir_sem_login(conn: &Connection, chave: &str, valor: bool) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if s.regra_login == "cookie_sessao" {
        return Err("Este site é verificado pelo cookie da sessão; a opção \"não pede login\" não vale aqui.".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    if valor {
        tx.execute("UPDATE contas_browser SET regra_login = 'nenhuma' WHERE chave = ?1", [chave]).map_err(|e| e.to_string())?;
        super::avisos::resolver_do_site(&tx, chave)?;
    } else {
        // Back to manual login: the site always goes red (a login confirmed before "no login" no longer
        // counts), the AI permission closes and the relogin notice is raised, all in one transaction.
        tx.execute(
            "UPDATE contas_browser SET regra_login = 'manual', estado = 'precisa_login', \
             verificada_em = datetime('now'), ia_pode_usar = 0 WHERE chave = ?1",
            [chave],
        )
        .map_err(|e| e.to_string())?;
        super::avisos::criar(&tx, chave)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

/// "I am no longer logged in": estado = precisa_login and the AI permission closed.
pub fn marcar_nao_logado(conn: &Connection, chave: &str) -> Result<(), String> {
    exigir(conn, chave)?;
    conn.execute("UPDATE contas_browser SET estado = 'precisa_login', ia_pode_usar = 0 WHERE chave = ?1", [chave])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Login wall seen: estado = precisa_login and the AI permission closed, in ONE statement (a site that
/// needs login never has the AI on). `true` when anything changed; a site already red with the flag
/// off changes nothing (`verificada_em` is only renewed when the state itself flips).
pub fn marcar_precisa_login(conn: &Connection, chave: &str) -> Result<bool, String> {
    exigir(conn, chave)?;
    let n = conn
        .execute(
            "UPDATE contas_browser SET estado = 'precisa_login', ia_pode_usar = 0, \
             verificada_em = CASE WHEN estado <> 'precisa_login' THEN datetime('now') ELSE verificada_em END \
             WHERE chave = ?1 AND (estado <> 'precisa_login' OR ia_pode_usar <> 0)",
            [chave],
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

pub fn adicionar(conn: &Connection, nome: &str, url: &str, tipo: &str, pais: Option<&str>) -> Result<String, String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("Informe o nome do site.".into());
    }
    if !url.trim().to_ascii_lowercase().starts_with("https://") {
        return Err("O endereço precisa começar com https://".into());
    }
    let Some((host, _)) = host_and_path(url) else { return Err("Endereço inválido.".into()) };
    if !TIPOS.contains(&tipo) {
        return Err("Tipo desconhecido.".into());
    }
    if let Some(p) = pais {
        if p.len() != 2 || !p.chars().all(|c| c.is_ascii_uppercase()) {
            return Err("O país precisa ser o código de 2 letras (ex.: DK).".into());
        }
    }
    if let Some(dup) = resolver(conn, url)? {
        return Err(format!("Este endereço já está coberto pelo site \"{}\".", dup.nome));
    }
    let chave = chave_from_url(url).ok_or("Endereço inválido.")?;
    if obter(conn, &chave)?.is_some() {
        return Err("Já existe um site com este endereço.".into());
    }
    let dominio = host.strip_prefix("www.").unwrap_or(&host).to_string();
    if !valid_domain(&dominio) || is_public_suffix(&dominio) {
        return Err("Endereço inválido: use o endereço de um site (ex.: https://www.exemplo.com.br/).".into());
    }
    for s in listar(conn)? {
        if s.dominios.iter().chain(&s.dominios_ats).any(|d| host_matches(&dominio, d) || host_matches(d, &dominio)) {
            return Err(format!("Este endereço se sobrepõe ao site \"{}\".", s.nome));
        }
    }
    let dominios = serde_json::to_string(&vec![dominio]).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO contas_browser (chave, nome, url_login, dominios, tipo, regiao, pais, papel, regra_login, embutida) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'GLOBAL', ?6, 'candidatura', 'manual', 0)",
        params![chave, nome, url.trim(), dominios, tipo, pais],
    )
    .map_err(|e| e.to_string())?;
    Ok(chave)
}

pub fn remover(conn: &Connection, chave: &str) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if s.embutida {
        return Err("Sites que vêm com o app não podem ser removidos; desligue a IA neste site.".into());
    }
    conn.execute("DELETE FROM contas_browser WHERE chave = ?1 AND embutida = 0", [chave]).map_err(|e| e.to_string())?;
    Ok(())
}

/// The site `url` belongs to; with several matches the one with the longest matching domain wins.
pub fn resolver(conn: &Connection, url: &str) -> Result<Option<Site>, String> {
    Ok(resolver_em(&listar(conn)?, url).cloned())
}

/// Same rule as [`resolver`] over an already loaded list, so callers that resolve many URLs load the
/// sites once.
pub fn resolver_em<'a>(sites: &'a [Site], url: &str) -> Option<&'a Site> {
    let (host, _) = host_and_path(url)?;
    let mut best: Option<(usize, &'a Site)> = None;
    for s in sites {
        let rules = DomainRules { dominios: &s.dominios, dominios_ats: &s.dominios_ats, caminho_ats: s.caminho_ats.as_deref() };
        if !url_matches(&rules, url) {
            continue;
        }
        let weight = s.dominios.iter().chain(&s.dominios_ats).filter(|d| host_matches(&host, d)).map(|d| d.len()).max().unwrap_or(0);
        if best.as_ref().is_none_or(|(w, _)| weight > *w) {
            best = Some((weight, s));
        }
    }
    best.map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::Connection;

    fn db_with(json_sites: &str) -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        merge(&c, &parse_seed(&format!(r#"{{"versao":1,"sites":[{json_sites}]}}"#)).unwrap()).unwrap();
        c
    }
    const LI: &str = r#"{"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}"#;
    const PUB: &str = r#"{"chave":"pub","nome":"Pub","url_login":"https://pub.com/careers","dominios":["pub.com"],"tipo":"empresa","regiao":"EU","regra_login":"nenhuma"}"#;

    #[test]
    fn the_ai_permission_cannot_be_turned_on_for_a_site_that_needs_login() {
        let c = db_with(LI);
        let e = definir_ia_pode_usar(&c, "linkedin", true).unwrap_err();
        assert!(e.contains("Entre neste site"), "{e}");
        assert!(!obter(&c, "linkedin").unwrap().unwrap().ia_pode_usar);
    }

    #[test]
    fn the_refusal_to_turn_the_ai_on_points_to_step_one_of_the_site_page() {
        let c = db_with(LI);
        let e = definir_ia_pode_usar(&c, "linkedin", true).unwrap_err();
        assert_eq!(e, "Entre neste site (passo 1 da página do site) antes de liberar a IA.");
        assert!(!e.contains("Já estou logado") && !e.contains('\u{c3}') && !e.contains('\u{c2}'), "{e}");
    }

    #[test]
    fn the_ai_permission_turns_on_once_the_user_confirms_the_login() {
        let c = db_with(LI);
        marcar_logada(&c, "linkedin").unwrap();
        definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        let s = obter(&c, "linkedin").unwrap().unwrap();
        assert!(s.ia_pode_usar && s.estado == "logada" && s.verificada_em.is_some());
    }

    #[test]
    fn a_site_without_login_rule_can_be_turned_on_right_away() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        assert!(obter(&c, "pub").unwrap().unwrap().ia_pode_usar);
    }

    #[test]
    fn turning_the_permission_off_always_works() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        definir_ia_pode_usar(&c, "pub", false).unwrap();
        assert!(!obter(&c, "pub").unwrap().unwrap().ia_pode_usar);
    }

    #[test]
    fn confirming_the_login_clears_the_sites_open_notice() {
        let c = db_with(LI);
        marcar_precisa_login(&c, "linkedin").unwrap();
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 1);
        marcar_logada(&c, "linkedin").unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 0);
    }

    #[test]
    fn marking_login_needed_reports_whether_anything_changed() {
        let c = db_with(LI);
        marcar_logada(&c, "linkedin").unwrap();
        assert!(marcar_precisa_login(&c, "linkedin").unwrap());
        assert!(!marcar_precisa_login(&c, "linkedin").unwrap());
    }

    #[test]
    fn an_unknown_key_is_an_error_not_a_silent_success() {
        let c = db_with(LI);
        assert!(marcar_logada(&c, "nope").is_err());
        assert!(definir_ia_pode_usar(&c, "nope", false).is_err());
    }

    #[test]
    fn adding_a_site_validates_and_starts_closed_and_manual() {
        let c = db_with(LI);
        let k = adicionar(&c, "Jobindex", "https://www.jobindex.dk/", "geral", Some("DK")).unwrap();
        assert_eq!(k, "jobindex-dk");
        let s = obter(&c, &k).unwrap().unwrap();
        assert!(!s.embutida && !s.ia_pode_usar && s.regra_login == "manual" && s.estado == "desconhecido");
    }

    #[test]
    fn adding_rejects_bad_input_and_domains_that_are_already_covered() {
        let c = db_with(LI);
        for (nome, url, tipo, pais) in [
            ("", "https://a.com/", "geral", None),
            ("A", "http://a.com/", "geral", None),
            ("A", "javascript:alert(1)", "geral", None),
            ("A", "https://a.com/", "banana", None),
            ("A", "https://a.com/", "geral", Some("brazil")),
            ("Again", "https://www.linkedin.com/feed", "geral", None),
        ] {
            assert!(adicionar(&c, nome, url, tipo, pais).is_err(), "{nome} {url} {tipo}");
        }
    }

    #[test]
    fn adding_refuses_public_suffixes_and_single_label_hosts() {
        let c = db_with(LI);
        for url in ["https://co.uk/", "https://com/", "https://localhost/"] {
            assert!(adicionar(&c, "X", url, "geral", None).is_err(), "{url}");
        }
    }

    #[test]
    fn adding_refuses_a_shared_ats_host_of_an_existing_site() {
        let ats = r#"{"chave":"gh","nome":"GH","url_login":"https://job-boards.greenhouse.io/x","dominios":["gh.example.com"],"dominios_ats":["job-boards.greenhouse.io"],"caminho_ats":"/x","tipo":"empresa","regiao":"EU"}"#;
        let c = db_with(ats);
        let e = adicionar(&c, "X", "https://job-boards.greenhouse.io/", "geral", None).unwrap_err();
        assert!(e.contains("GH"), "{e}");
    }

    #[test]
    fn adding_refuses_a_parent_of_an_existing_domain() {
        let sub = r#"{"chave":"jl","nome":"JL","url_login":"https://jobs.linkedin.com/","dominios":["jobs.linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}"#;
        let c = db_with(sub);
        assert!(adicionar(&c, "X", "https://linkedin.com/", "geral", None).is_err());
        assert!(adicionar(&c, "Ok", "https://normal-site.org/", "geral", None).is_ok());
    }

    #[test]
    fn only_sites_the_user_added_can_be_removed() {
        let c = db_with(LI);
        assert!(remover(&c, "linkedin").is_err());
        let k = adicionar(&c, "Mine", "https://mine.org/", "geral", None).unwrap();
        remover(&c, &k).unwrap();
        assert!(obter(&c, &k).unwrap().is_none());
    }

    #[test]
    fn resolving_picks_the_most_specific_match_and_ignores_lookalikes() {
        let c = db_with(&format!("{LI},{PUB}"));
        assert_eq!(resolver(&c, "https://www.linkedin.com/jobs/1").unwrap().unwrap().chave, "linkedin");
        assert!(resolver(&c, "https://linkedin.com.evil.com/").unwrap().is_none());
        assert!(resolver(&c, "https://linkedin.com@evil.com/").unwrap().is_none());
        assert!(resolver(&c, "https://unknown.org/").unwrap().is_none());
    }

    fn ia(c: &Connection, k: &str) -> bool { obter(c, k).unwrap().unwrap().ia_pode_usar }

    #[test]
    fn turning_no_login_on_lets_the_ai_be_released_without_a_login() {
        let c = db_with(LI); // LI has the default manual rule
        definir_sem_login(&c, "linkedin", true).unwrap();
        assert_eq!(obter(&c, "linkedin").unwrap().unwrap().regra_login, "nenhuma");
        definir_ia_pode_usar(&c, "linkedin", true).unwrap();
    }

    fn estado(c: &Connection, k: &str) -> String { obter(c, k).unwrap().unwrap().estado }
    fn regra(c: &Connection, k: &str) -> String { obter(c, k).unwrap().unwrap().regra_login }

    #[test]
    fn going_back_to_manual_without_being_logged_in_closes_the_ai() {
        let c = db_with(PUB); // regra_login = nenhuma, estado = desconhecido
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        definir_sem_login(&c, "pub", false).unwrap();
        assert_eq!(regra(&c, "pub"), "manual");
        assert_eq!(estado(&c, "pub"), "precisa_login");
        assert!(!ia(&c, "pub"));
        // The gate refuses a manual site that is not logged in.
        assert!(matches!(crate::sites::gate::decidir(&c, "https://pub.com/careers"), crate::sites::gate::Decision::Refuse(_)));
    }

    #[test]
    fn going_back_to_manual_while_needing_login_also_closes_the_ai() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        marcar_precisa_login(&c, "pub").unwrap();
        definir_sem_login(&c, "pub", false).unwrap();
        assert!(!ia(&c, "pub"));
        assert_eq!(estado(&c, "pub"), "precisa_login");
    }

    #[test]
    fn going_back_to_manual_while_logged_in_still_marks_the_site_red_and_closes_the_ai() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        c.execute("UPDATE contas_browser SET estado='logada' WHERE chave='pub'", []).unwrap();
        definir_sem_login(&c, "pub", false).unwrap();
        assert_eq!(regra(&c, "pub"), "manual");
        assert_eq!(estado(&c, "pub"), "precisa_login");
        assert!(!ia(&c, "pub"));
    }

    #[test]
    fn going_back_to_manual_raises_exactly_one_relogin_notice() {
        let c = db_with(PUB);
        definir_sem_login(&c, "pub", false).unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 1);
        let open = crate::sites::avisos::listar_abertos(&c).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].chave, "pub");
        // Toggle on, off, on, off: never more than one open notice.
        definir_sem_login(&c, "pub", true).unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 0);
        definir_sem_login(&c, "pub", false).unwrap();
        definir_sem_login(&c, "pub", true).unwrap();
        definir_sem_login(&c, "pub", false).unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 1);
        // Repeating "off" while the notice is open does not duplicate it.
        definir_sem_login(&c, "pub", false).unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 1);
    }

    #[test]
    fn going_back_to_manual_is_atomic() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        c.execute("UPDATE contas_browser SET estado='logada' WHERE chave='pub'", []).unwrap();
        c.execute("DROP TABLE avisos_sites", []).unwrap();
        assert!(definir_sem_login(&c, "pub", false).is_err());
        assert_eq!(regra(&c, "pub"), "nenhuma");
        assert_eq!(estado(&c, "pub"), "logada");
        assert!(ia(&c, "pub"));
    }

    #[test]
    fn a_cookie_session_site_refuses_the_toggle_and_stays_untouched() {
        let c = db_with(LI);
        c.execute("UPDATE contas_browser SET regra_login='cookie_sessao' WHERE chave='linkedin'", []).unwrap();
        assert!(definir_sem_login(&c, "linkedin", true).is_err());
        assert_eq!(obter(&c, "linkedin").unwrap().unwrap().regra_login, "cookie_sessao");
        assert!(definir_sem_login(&c, "nope", true).is_err());
    }

    #[test]
    fn the_cookie_session_refusal_message_is_pinned_with_its_accents() {
        let c = db_with(LI);
        c.execute("UPDATE contas_browser SET regra_login='cookie_sessao' WHERE chave='linkedin'", []).unwrap();
        let e = definir_sem_login(&c, "linkedin", true).unwrap_err();
        assert_eq!(e, "Este site é verificado pelo cookie da sessão; a opção \"não pede login\" não vale aqui.");
    }

    #[test]
    fn turning_no_login_on_resolves_the_open_relogin_notice() {
        let c = db_with(LI);
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        definir_sem_login(&c, "linkedin", true).unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 0);
    }

    #[test]
    fn not_logged_in_anymore_closes_the_ai_and_marks_the_site() {
        let c = db_with(LI);
        marcar_logada(&c, "linkedin").unwrap();
        definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        marcar_nao_logado(&c, "linkedin").unwrap();
        let s = obter(&c, "linkedin").unwrap().unwrap();
        assert_eq!(s.estado, "precisa_login");
        assert!(!s.ia_pode_usar);
        assert!(marcar_nao_logado(&c, "pub-que-nao-existe").is_err());
    }
}
