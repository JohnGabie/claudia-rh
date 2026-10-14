//! The embedded seed and its merge into the stored list.
use super::modelo::{validate_all, SeedFile, SeedSite};

pub const SEED_JSON: &str = include_str!("../../sites-padrao.json");

pub fn parse_seed(json: &str) -> Result<Vec<SeedSite>, String> {
    let f: SeedFile = serde_json::from_str(json).map_err(|e| format!("seed ilegível: {e}"))?;
    validate_all(&f.sites)?;
    Ok(f.sites)
}

pub fn embedded() -> Result<Vec<SeedSite>, String> {
    parse_seed(SEED_JSON)
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
}
