//! The embedded seed and its merge into the stored list.
use super::modelo::{validate_all, SeedFile, SeedSite};
use rusqlite::{params, Connection};
use std::collections::HashSet;

pub const SEED_JSON: &str = include_str!("../../sites-padrao.json");

pub fn parse_seed(json: &str) -> Result<Vec<SeedSite>, String> {
    let f: SeedFile = serde_json::from_str(json).map_err(|e| format!("seed ilegível: {e}"))?;
    validate_all(&f.sites)?;
    Ok(f.sites)
}

pub fn embedded() -> Result<Vec<SeedSite>, String> {
    parse_seed(SEED_JSON)
}

/// Merges the seed by `chave`, idempotently. New sites arrive closed (`ia_pode_usar = 0`). For a
/// seed site already stored it refreshes only the catalog fields, and only when the stored row is a
/// seed row (`embutida = 1`): a site the user added with the same key is never touched. It never
/// changes `ia_pode_usar`, `estado`, `regra_login` or `verificada_em`. A seed row that left the seed
/// is kept and flagged `padrao_atual = 0`.
pub fn merge(conn: &Connection, seed: &[SeedSite]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    for s in seed {
        let dominios = serde_json::to_string(&s.dominios).unwrap_or_else(|_| "[]".into());
        let ats = serde_json::to_string(&s.dominios_ats).unwrap_or_else(|_| "[]".into());
        tx.execute(
            "INSERT INTO contas_browser \
             (chave, nome, url_login, dominios, dominios_ats, caminho_ats, tipo, regiao, pais, papel, regra_login, embutida, padrao_atual) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,1) \
             ON CONFLICT(chave) DO UPDATE SET \
               nome=excluded.nome, url_login=excluded.url_login, dominios=excluded.dominios, \
               dominios_ats=excluded.dominios_ats, caminho_ats=excluded.caminho_ats, tipo=excluded.tipo, \
               regiao=excluded.regiao, pais=excluded.pais, papel=excluded.papel, padrao_atual=1 \
             WHERE contas_browser.embutida = 1",
            params![s.chave, s.nome, s.url_login, dominios, ats, s.caminho_ats, s.tipo, s.regiao, s.pais, s.papel, s.regra_login],
        )?;
    }
    let in_seed: HashSet<&str> = seed.iter().map(|s| s.chave.as_str()).collect();
    let stored: Vec<String> = {
        let mut st = tx.prepare("SELECT chave FROM contas_browser WHERE embutida = 1")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for chave in stored.iter().filter(|k| !in_seed.contains(k.as_str())) {
        tx.execute("UPDATE contas_browser SET padrao_atual = 0 WHERE chave = ?1", [chave])?;
    }
    tx.commit()
}

/// Reads the shipped seed and merges it. Called once when the app starts.
pub fn merge_embedded(conn: &Connection) -> Result<(), String> {
    let seed = embedded()?;
    merge(conn, &seed).map_err(|e| format!("erro ao mesclar o seed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(chave: &str) -> String {
        format!(r#"{{"chave":"{chave}","nome":"X","url_login":"https://www.x.com/","dominios":["x.com"],"tipo":"geral","regiao":"GLOBAL"}}"#)
    }
    fn file(sites: &[String]) -> String {
        format!(r#"{{"versao":1,"sites":[{}]}}"#, sites.join(","))
    }

    #[test]
    fn the_embedded_seed_is_valid() {
        let s = embedded().expect("the shipped seed must parse and validate");
        assert!(s.len() >= 9);
    }

    #[test]
    fn defaults_are_candidatura_and_manual() {
        let s = parse_seed(&file(&[site("x")])).unwrap();
        assert_eq!((s[0].papel.as_str(), s[0].regra_login.as_str()), ("candidatura", "manual"));
    }

    #[test]
    fn a_repeated_key_is_rejected() {
        assert!(parse_seed(&file(&[site("x"), site("x")])).unwrap_err().contains("repetida"));
    }

    #[test]
    fn invalid_fields_are_rejected_with_the_site_key_in_the_message() {
        let cases = [
            (r#""tipo":"geral""#, r#""tipo":"banana""#),
            (r#""regiao":"GLOBAL""#, r#""regiao":"MARS""#),
            (r#""url_login":"https://www.x.com/""#, r#""url_login":"http://www.x.com/""#),
            (r#""url_login":"https://www.x.com/""#, r#""url_login":"https://other.org/""#),
            (r#""dominios":["x.com"]"#, r#""dominios":[]"#),
            (r#""dominios":["x.com"]"#, r#""dominios":["https://x.com"]"#),
        ];
        for (from, to) in cases {
            let bad = file(&[site("x")]).replace(from, to);
            let e = parse_seed(&bad).unwrap_err();
            assert!(e.contains("x"), "{to}: {e}");
        }
    }

    #[test]
    fn an_ats_path_needs_an_ats_domain() {
        let bad = file(&[site("x")]).replace(r#""tipo":"geral""#, r#""tipo":"empresa","caminho_ats":"/x""#);
        assert!(parse_seed(&bad).unwrap_err().contains("caminho_ats"));
    }

    #[test]
    fn a_public_suffix_is_not_a_valid_site_domain() {
        let bad = file(&[site("x")])
            .replace(r#""dominios":["x.com"]"#, r#""dominios":["co.uk"]"#)
            .replace(r#""url_login":"https://www.x.com/""#, r#""url_login":"https://www.co.uk/""#);
        assert!(parse_seed(&bad).unwrap_err().contains("sufixo público"));
    }

    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }
    fn one(chave: &str, nome: &str) -> SeedSite {
        parse_seed(&file(&[site(chave).replace(r#""nome":"X""#, &format!(r#""nome":"{nome}""#))])).unwrap().remove(0)
    }
    fn count(c: &Connection) -> i64 {
        c.query_row("SELECT COUNT(*) FROM contas_browser", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn merge_inserts_embedded_sites_closed_for_the_ai() {
        let c = db();
        merge(&c, &[one("x", "X")]).unwrap();
        let (ia, estado, emb): (i64, String, i64) = c
            .query_row("SELECT ia_pode_usar, estado, embutida FROM contas_browser WHERE chave='x'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!((ia, estado.as_str(), emb), (0, "desconhecido", 1));
    }

    #[test]
    fn merging_twice_gives_the_same_table() {
        let c = db();
        merge(&c, &[one("x", "X"), one("y", "Y")]).unwrap();
        merge(&c, &[one("x", "X"), one("y", "Y")]).unwrap();
        assert_eq!(count(&c), 2);
    }

    #[test]
    fn merge_updates_the_catalog_fields_but_never_the_users_choices() {
        let c = db();
        merge(&c, &[one("x", "Old")]).unwrap();
        c.execute("UPDATE contas_browser SET ia_pode_usar=1, estado='logada', regra_login='cookie_sessao', verificada_em='2026-01-01 00:00:00' WHERE chave='x'", []).unwrap();
        merge(&c, &[one("x", "New")]).unwrap();
        let (nome, ia, estado, regra): (String, i64, String, String) = c
            .query_row("SELECT nome, ia_pode_usar, estado, regra_login FROM contas_browser WHERE chave='x'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap();
        assert_eq!((nome.as_str(), ia, estado.as_str(), regra.as_str()), ("New", 1, "logada", "cookie_sessao"));
    }

    #[test]
    fn a_site_the_user_added_with_the_same_key_is_left_alone() {
        let c = db();
        c.execute("INSERT INTO contas_browser (chave,nome,url_login,dominios,tipo,regiao,embutida) VALUES ('x','Mine','https://mine.org/','[\"mine.org\"]','geral','GLOBAL',0)", []).unwrap();
        merge(&c, &[one("x", "FromSeed")]).unwrap();
        let (nome, url): (String, String) = c.query_row("SELECT nome, url_login FROM contas_browser WHERE chave='x'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((nome.as_str(), url.as_str()), ("Mine", "https://mine.org/"));
    }

    #[test]
    fn a_site_that_left_the_seed_is_kept_and_flagged_not_current() {
        let c = db();
        merge(&c, &[one("x", "X"), one("y", "Y")]).unwrap();
        c.execute("UPDATE contas_browser SET ia_pode_usar=1 WHERE chave='y'", []).unwrap();
        merge(&c, &[one("x", "X")]).unwrap();
        let (padrao, ia): (i64, i64) = c.query_row("SELECT padrao_atual, ia_pode_usar FROM contas_browser WHERE chave='y'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((padrao, ia), (0, 1));
    }
}
