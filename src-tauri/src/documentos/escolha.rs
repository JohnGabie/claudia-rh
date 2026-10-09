//! Which document to attach (spec section 4). The user's marks always win over what was generated.
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq)]
pub struct Escolha {
    pub id: i64,
    pub regra: &'static str,
}

pub fn escolher(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<Escolha> {
    if let Some(v) = variante_id {
        if let Some(id) = super::preferido(conn, tipo, Some(v)) {
            return Some(Escolha { id, regra: "preferido da variante" });
        }
    }
    if let Some(id) = super::preferido(conn, tipo, None) {
        return Some(Escolha { id, regra: "preferido geral" });
    }
    let v = variante_id?;
    conn.query_row(
        "SELECT id FROM documentos WHERE tipo = ?1 AND origem = 'gerado' AND variante_id = ?2 ORDER BY id DESC LIMIT 1",
        params![tipo, v],
        |r| r.get(0),
    )
    .ok()
    .map(|id| Escolha { id, regra: "gerado mais recente da variante" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{definir_preferido, inserir, NovoDocumento};

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }

    fn doc(c: &rusqlite::Connection, origem: &str, variante: Option<&str>) -> i64 {
        let arquivo = format!("x{}.pdf", c.query_row("SELECT COUNT(*) FROM documentos", [], |r| r.get::<_, i64>(0)).unwrap());
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem, arquivo: &arquivo, arquivo_html: None, formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: variante }).unwrap()
    }

    #[test]
    fn the_variant_preference_wins_over_everything() {
        let c = mem();
        let geral = doc(&c, "importado", None);
        let var = doc(&c, "importado", None);
        doc(&c, "gerado", Some("backend"));
        definir_preferido(&c, geral, None).unwrap();
        definir_preferido(&c, var, Some("backend")).unwrap();
        let e = escolher(&c, "curriculo", Some("backend")).unwrap();
        assert_eq!((e.id, e.regra), (var, "preferido da variante"));
    }

    #[test]
    fn without_a_variant_preference_the_general_one() {
        let c = mem();
        let geral = doc(&c, "importado", None);
        doc(&c, "gerado", Some("backend"));
        definir_preferido(&c, geral, None).unwrap();
        assert_eq!(escolher(&c, "curriculo", Some("backend")).unwrap().regra, "preferido geral");
        assert_eq!(escolher(&c, "curriculo", None).unwrap().id, geral);
    }

    #[test]
    fn without_preferences_the_newest_generated_for_that_variant() {
        let c = mem();
        doc(&c, "gerado", Some("backend"));
        let novo = doc(&c, "gerado", Some("backend"));
        doc(&c, "gerado", Some("devops"));
        doc(&c, "importado", None);
        // Newer, same variant, but imported: rule 3 is about what was GENERATED for the variant.
        doc(&c, "importado", Some("backend"));
        let e = escolher(&c, "curriculo", Some("backend")).unwrap();
        assert_eq!((e.id, e.regra), (novo, "gerado mais recente da variante"));
    }

    #[test]
    fn nothing_to_pick_returns_none() {
        let c = mem();
        doc(&c, "importado", None);
        doc(&c, "gerado", Some("devops"));
        assert!(escolher(&c, "curriculo", Some("backend")).is_none(), "no preference, nothing generated for backend");
        assert!(escolher(&c, "curriculo", None).is_none(), "no variant and no general preference");
        assert!(escolher(&c, "carta", None).is_none());
    }
}
