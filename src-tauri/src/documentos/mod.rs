//! The library of documents the agent may attach. Files live in <app data>/documentos/; the database
//! holds only bare file names, so a row can never point outside that folder.
pub mod biblioteca;
pub mod escolha;
pub mod fonte;
pub mod importar;
pub mod migrar;
pub mod pdf;
pub mod registrar;
pub mod seguranca;
pub mod texto;
pub mod variantes;

use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub fn docs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("documentos")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Documento {
    pub id: i64,
    pub tipo: String,
    pub nome: String,
    pub origem: String,
    pub arquivo: String,
    pub arquivo_html: Option<String>,
    pub formato: String,
    pub idioma: Option<String>,
    pub tags: String,
    pub sha256: String,
    pub tamanho: i64,
    pub vaga_id: Option<i64>,
    pub variante_id: Option<String>,
    pub criado_em: String,
}

pub struct NovoDocumento<'a> {
    pub tipo: &'a str,
    pub nome: &'a str,
    pub origem: &'a str,
    pub arquivo: &'a str,
    pub arquivo_html: Option<&'a str>,
    pub formato: &'a str,
    pub idioma: Option<&'a str>,
    pub sha256: &'a str,
    pub tamanho: i64,
    pub variante_id: Option<&'a str>,
}

const COLS: &str = "id, tipo, nome, origem, arquivo, arquivo_html, formato, idioma, tags, sha256, tamanho, vaga_id, variante_id, criado_em";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Documento> {
    Ok(Documento {
        id: r.get(0)?,
        tipo: r.get(1)?,
        nome: r.get(2)?,
        origem: r.get(3)?,
        arquivo: r.get(4)?,
        arquivo_html: r.get(5)?,
        formato: r.get(6)?,
        idioma: r.get(7)?,
        tags: r.get(8)?,
        sha256: r.get(9)?,
        tamanho: r.get(10)?,
        vaga_id: r.get(11)?,
        variante_id: r.get(12)?,
        criado_em: r.get(13)?,
    })
}

pub fn inserir(conn: &Connection, d: &NovoDocumento) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO documentos (tipo, nome, origem, arquivo, arquivo_html, formato, idioma, sha256, tamanho, variante_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![d.tipo, d.nome, d.origem, d.arquivo, d.arquivo_html, d.formato, d.idioma, d.sha256, d.tamanho, d.variante_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn obter(conn: &Connection, id: i64) -> rusqlite::Result<Option<Documento>> {
    conn.query_row(&format!("SELECT {COLS} FROM documentos WHERE id = ?1"), [id], row).optional()
}

pub fn listar(conn: &Connection, tipo: &str) -> rusqlite::Result<Vec<Documento>> {
    let mut st = conn.prepare(&format!("SELECT {COLS} FROM documentos WHERE tipo = ?1 ORDER BY id DESC"))?;
    let rows = st.query_map([tipo], row)?;
    rows.collect()
}

pub fn definir_preferido(conn: &Connection, id: i64, variante_id: Option<&str>) -> Result<(), String> {
    let doc = obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("documento {id} não existe"))?;
    conn.execute(
        "INSERT INTO preferidos (tipo, variante_id, documento_id) VALUES (?1, ?2, ?3)
         ON CONFLICT (tipo, variante_id) DO UPDATE SET documento_id = excluded.documento_id",
        params![doc.tipo, variante_id.unwrap_or(""), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<i64> {
    conn.query_row(
        "SELECT documento_id FROM preferidos WHERE tipo = ?1 AND variante_id = ?2",
        params![tipo, variante_id.unwrap_or("")],
        |r| r.get(0),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    pub(crate) fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }

    pub(crate) fn novo<'a>(tipo: &'a str, arquivo: &'a str, variante: Option<&'a str>) -> NovoDocumento<'a> {
        NovoDocumento { tipo, nome: "CV", origem: "gerado", arquivo, arquivo_html: None, formato: "pdf", idioma: Some("pt"), sha256: "00", tamanho: 1, variante_id: variante }
    }

    #[test]
    fn insert_get_and_list_newest_first() {
        let c = mem();
        let a = inserir(&c, &novo("curriculo", "a.pdf", None)).unwrap();
        let b = inserir(&c, &novo("curriculo", "b.pdf", None)).unwrap();
        inserir(&c, &novo("carta", "c.pdf", None)).unwrap();
        assert_eq!(obter(&c, a).unwrap().unwrap().arquivo, "a.pdf");
        let l = listar(&c, "curriculo").unwrap();
        assert_eq!(l.iter().map(|d| d.id).collect::<Vec<_>>(), vec![b, a], "only CVs, newest first");
        assert!(obter(&c, 999).unwrap().is_none());
    }

    #[test]
    fn one_preferred_per_type_and_variant_the_last_mark_wins() {
        let c = mem();
        let a = inserir(&c, &novo("curriculo", "a.pdf", None)).unwrap();
        let b = inserir(&c, &novo("curriculo", "b.pdf", None)).unwrap();
        definir_preferido(&c, a, None).unwrap();
        definir_preferido(&c, b, None).unwrap();
        assert_eq!(preferido(&c, "curriculo", None), Some(b));
        definir_preferido(&c, a, Some("backend")).unwrap();
        assert_eq!(preferido(&c, "curriculo", Some("backend")), Some(a));
        assert_eq!(preferido(&c, "curriculo", None), Some(b), "the general mark is separate from the variant's");
        assert_eq!(preferido(&c, "carta", None), None);
    }

    #[test]
    fn marking_an_unknown_document_is_an_error() {
        let c = mem();
        assert!(definir_preferido(&c, 42, None).is_err());
    }
}
