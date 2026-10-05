//! The library as the app's screens and the agent's tools see it (spec section 6): no file names, no
//! paths, the user's marks and tags already decoded.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The kinds of tag a document can carry (spec section 2).
pub const TAG_TIPOS: &[&str] = &["variante", "template", "cor", "idioma", "livre"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub tipo: String,
    pub valor: String,
}

/// `Documento` of the spec's contract. Never carries `arquivo`, `arquivo_html`, `sha256` or a path.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentoView {
    pub id: i64,
    pub tipo: String,
    pub nome: String,
    pub origem: String,
    pub formato: String,
    pub idioma: Option<String>,
    pub tags: Vec<Tag>,
    pub tamanho: i64,
    pub criado_em: String,
    pub vaga_id: Option<i64>,
    pub preferido_global: bool,
    pub preferido_variantes: Vec<String>,
    pub tem_preview: bool,
}

/// `obter_documento`: the document plus its preview HTML (generated documents only).
#[derive(Debug, Clone, Serialize)]
pub struct DocumentoDetalhe {
    #[serde(flatten)]
    pub documento: DocumentoView,
    pub preview_html: Option<String>,
}

/// `listar_variantes`: what the variant pickers need.
#[derive(Debug, Clone, Serialize)]
pub struct VarianteResumo {
    pub id: String,
    pub nome_exibicao: String,
    pub ativa: bool,
}

/// Every connection the document code opens: foreign keys ON, so deleting a document also drops its
/// preferred marks (preferidos.documento_id ... ON DELETE CASCADE).
pub fn open_conn(data_dir: &Path) -> Result<Connection, String> {
    let c = crate::mcp::tools::open_db(data_dir)?;
    c.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;
    Ok(c)
}

/// Trims, refuses unknown kinds and empty values, drops duplicates (first one wins).
pub fn normalizar_tags(tags: &[Tag]) -> Result<Vec<Tag>, String> {
    let mut out: Vec<Tag> = Vec::new();
    for t in tags {
        let tipo = t.tipo.trim();
        let valor = t.valor.trim();
        if !TAG_TIPOS.contains(&tipo) {
            return Err(format!("tipo de tag inválido: {tipo} (use {})", TAG_TIPOS.join(", ")));
        }
        if valor.is_empty() {
            return Err("uma tag precisa de um valor".into());
        }
        let tag = Tag { tipo: tipo.to_string(), valor: valor.to_string() };
        if !out.contains(&tag) {
            out.push(tag);
        }
    }
    Ok(out)
}

/// Replaces the document's tags. Tags are labels only: they never change `variante_id`.
pub fn gravar_tags(conn: &Connection, id: i64, tags: &[Tag]) -> Result<(), String> {
    let tags = normalizar_tags(tags)?;
    let json = serde_json::to_string(&tags).map_err(|e| e.to_string())?;
    let n = conn.execute("UPDATE documentos SET tags = ?1 WHERE id = ?2", params![json, id]).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("o documento {id} não existe"));
    }
    Ok(())
}

fn view_de(conn: &Connection, d: super::Documento) -> Result<DocumentoView, String> {
    let mut st = conn
        .prepare("SELECT variante_id FROM preferidos WHERE documento_id = ?1 ORDER BY variante_id")
        .map_err(|e| e.to_string())?;
    let marks: Vec<String> = st
        .query_map([d.id], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    Ok(DocumentoView {
        id: d.id,
        tipo: d.tipo,
        nome: d.nome,
        origem: d.origem,
        formato: d.formato,
        idioma: d.idioma.filter(|s| !s.is_empty()),
        tags: serde_json::from_str(&d.tags).unwrap_or_default(),
        tamanho: d.tamanho,
        criado_em: d.criado_em,
        vaga_id: d.vaga_id,
        preferido_global: marks.iter().any(|m| m.is_empty()),
        preferido_variantes: marks.into_iter().filter(|m| !m.is_empty()).collect(),
        tem_preview: d.arquivo_html.as_deref().is_some_and(|h| !h.is_empty()),
    })
}

pub fn ver(conn: &Connection, id: i64) -> Result<DocumentoView, String> {
    let d = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    view_de(conn, d)
}

pub fn listar_view(conn: &Connection, tipo: &str) -> Result<Vec<DocumentoView>, String> {
    super::listar(conn, tipo).map_err(|e| e.to_string())?.into_iter().map(|d| view_de(conn, d)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{definir_preferido, inserir, NovoDocumento};

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }

    fn doc(c: &Connection, html: Option<&str>) -> i64 {
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "gerado", arquivo: "segredo-123.pdf", arquivo_html: html, formato: "pdf", idioma: Some("pt"), sha256: "abcdef", tamanho: 10, variante_id: Some("backend") }).unwrap()
    }

    #[test]
    fn the_view_reads_the_marks_and_carries_no_file_name() {
        let c = mem();
        let id = doc(&c, Some("segredo-123.html"));
        definir_preferido(&c, id, None).unwrap();
        definir_preferido(&c, id, Some("devops")).unwrap();
        let v = ver(&c, id).unwrap();
        assert!(v.preferido_global);
        assert_eq!(v.preferido_variantes, vec!["devops".to_string()]);
        assert!(v.tem_preview);
        let json = serde_json::to_string(&v).unwrap();
        for leak in ["segredo-123", "arquivo", "sha256", "abcdef"] {
            assert!(!json.contains(leak), "{leak} leaked: {json}");
        }
        assert!(!ver(&c, doc(&c, None)).unwrap().tem_preview, "no HTML, no preview");
    }

    #[test]
    fn tags_are_checked_trimmed_and_deduplicated() {
        let c = mem();
        let id = doc(&c, None);
        let t = |tipo: &str, valor: &str| Tag { tipo: tipo.into(), valor: valor.into() };
        gravar_tags(&c, id, &[t("variante", "backend"), t("variante", " backend "), t("livre", " sênior ")]).unwrap();
        assert_eq!(ver(&c, id).unwrap().tags, vec![t("variante", "backend"), t("livre", "sênior")]);
        assert!(gravar_tags(&c, id, &[t("cor_favorita", "x")]).is_err(), "unknown kind");
        assert!(gravar_tags(&c, id, &[t("livre", "   ")]).is_err(), "empty value");
        assert!(gravar_tags(&c, 999, &[]).is_err(), "unknown document");
        let var: Option<String> = c.query_row("SELECT variante_id FROM documentos WHERE id = ?1", [id], |r| r.get(0)).unwrap();
        assert_eq!(var.as_deref(), Some("backend"), "tags never change the variant field");
    }

    #[test]
    fn open_conn_turns_foreign_keys_on() {
        let dir = std::env::temp_dir().join(format!("claudia-bib-fk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let c = open_conn(&dir).unwrap();
        let on: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(on, 1);
    }
}
