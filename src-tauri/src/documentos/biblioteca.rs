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

/// A stored bare name resolved inside <app data>/documentos/, or None. Never follows a name that
/// escapes the folder (`..`, separators, drive letters, links pointing out).
pub fn dentro_da_pasta(data_dir: &Path, nome: &str) -> Option<std::path::PathBuf> {
    if !super::seguranca::is_bare_name(nome) {
        return None;
    }
    let base = std::fs::canonicalize(super::docs_dir(data_dir)).ok()?;
    let real = std::fs::canonicalize(base.join(nome)).ok()?;
    (real.starts_with(&base) && real.is_file()).then_some(real)
}

pub fn detalhe(conn: &Connection, data_dir: &Path, id: i64) -> Result<DocumentoDetalhe, String> {
    let d = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    let preview_html = d
        .arquivo_html
        .as_deref()
        .filter(|h| !h.is_empty())
        .and_then(|h| dentro_da_pasta(data_dir, h))
        .and_then(|p| std::fs::read_to_string(p).ok());
    Ok(DocumentoDetalhe { documento: view_de(conn, d)?, preview_html })
}

pub fn renomear(conn: &Connection, id: i64, nome: &str) -> Result<(), String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("o nome não pode ficar vazio".into());
    }
    let n = conn.execute("UPDATE documentos SET nome = ?1 WHERE id = ?2", params![nome, id]).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("o documento {id} não existe"));
    }
    Ok(())
}

pub fn remover_preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Result<(), String> {
    if tipo != "curriculo" && tipo != "carta" {
        return Err(format!("tipo inválido: {tipo}"));
    }
    conn.execute("DELETE FROM preferidos WHERE tipo = ?1 AND variante_id = ?2", params![tipo, variante_id.unwrap_or("")])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes by id only: the document's own files (only if they resolve inside the folder), the copy
/// handed to Chrome (envio/<id>/), then the row. Its preferred marks go by FK cascade (open_conn).
pub fn apagar(conn: &Connection, data_dir: &Path, id: i64) -> Result<(), String> {
    let d = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    for nome in [Some(d.arquivo.as_str()), d.arquivo_html.as_deref()].into_iter().flatten() {
        if let Some(p) = dentro_da_pasta(data_dir, nome) {
            std::fs::remove_file(&p).map_err(|e| format!("não foi possível apagar o arquivo: {e}"))?;
        }
    }
    let envio = super::docs_dir(data_dir).join("envio").join(id.to_string());
    if envio.is_dir() {
        let _ = std::fs::remove_dir_all(&envio);
    }
    conn.execute("DELETE FROM documentos WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
    Ok(())
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

    fn on_disk(name: &str) -> (Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-bib-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir)).unwrap();
        // Schema through a throwaway connection: the one under test must get its FK setting from
        // open_conn alone (db::apply would switch it on by itself and hide a missing PRAGMA).
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        (open_conn(&dir).unwrap(), dir)
    }

    fn real(c: &Connection, dir: &Path, base: &str) -> i64 {
        let docs = crate::documentos::docs_dir(dir);
        std::fs::write(docs.join(format!("{base}.pdf")), b"%PDF-1.7 x").unwrap();
        std::fs::write(docs.join(format!("{base}.html")), "<p>cv</p>").unwrap();
        let pdf = format!("{base}.pdf");
        let html = format!("{base}.html");
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "gerado", arquivo: &pdf, arquivo_html: Some(&html), formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: None }).unwrap()
    }

    #[test]
    fn the_preview_is_read_only_from_inside_the_folder() {
        let (c, dir) = on_disk("preview");
        let ok = real(&c, &dir, "cv_ok");
        assert_eq!(detalhe(&c, &dir, ok).unwrap().preview_html.as_deref(), Some("<p>cv</p>"));
        std::fs::write(dir.join("fora.html"), "<p>segredo</p>").unwrap();
        let bad = inserir(&c, &NovoDocumento { tipo: "curriculo", nome: "X", origem: "gerado", arquivo: "x.pdf", arquivo_html: Some("..\\fora.html"), formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        assert_eq!(detalhe(&c, &dir, bad).unwrap().preview_html, None, "a name that escapes the folder is never read");
    }

    #[test]
    fn deleting_removes_its_files_and_its_preferred_marks_only() {
        let (c, dir) = on_disk("delete");
        let a = real(&c, &dir, "cv_a");
        let b = real(&c, &dir, "cv_b");
        crate::documentos::definir_preferido(&c, a, None).unwrap();
        crate::documentos::definir_preferido(&c, a, Some("backend")).unwrap();
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir).join("envio").join(a.to_string())).unwrap();
        apagar(&c, &dir, a).unwrap();
        let docs = crate::documentos::docs_dir(&dir);
        assert!(!docs.join("cv_a.pdf").exists() && !docs.join("cv_a.html").exists());
        assert!(!docs.join("envio").join(a.to_string()).exists(), "the copy handed to Chrome goes too");
        assert!(docs.join("cv_b.pdf").exists(), "the other document is untouched");
        assert_eq!(crate::documentos::preferido(&c, "curriculo", None), None, "the marks went with it (FK cascade)");
        assert_eq!(crate::documentos::preferido(&c, "curriculo", Some("backend")), None);
        assert!(ver(&c, b).is_ok());
        assert!(apagar(&c, &dir, a).is_err(), "deleting twice says it does not exist");
    }

    #[test]
    fn deleting_never_touches_a_file_outside_the_folder() {
        let (c, dir) = on_disk("outside");
        std::fs::write(dir.join("segredo.pdf"), b"%PDF-1.7 do usuario").unwrap();
        let id = inserir(&c, &NovoDocumento { tipo: "curriculo", nome: "X", origem: "importado", arquivo: "..\\segredo.pdf", arquivo_html: Some("../segredo.pdf"), formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        apagar(&c, &dir, id).unwrap();
        assert!(dir.join("segredo.pdf").exists(), "never deleted outside documentos/");
        assert!(ver(&c, id).is_err(), "the bad row itself is gone");
    }

    #[test]
    fn rename_and_unmark() {
        let (c, dir) = on_disk("rename");
        let a = real(&c, &dir, "cv_r");
        renomear(&c, a, "  Meu CV  ").unwrap();
        assert_eq!(ver(&c, a).unwrap().nome, "Meu CV");
        assert!(renomear(&c, a, "   ").is_err());
        assert!(renomear(&c, 999, "x").is_err());
        crate::documentos::definir_preferido(&c, a, None).unwrap();
        crate::documentos::definir_preferido(&c, a, Some("backend")).unwrap();
        remover_preferido(&c, "curriculo", Some("backend")).unwrap();
        let v = ver(&c, a).unwrap();
        assert!(v.preferido_global && v.preferido_variantes.is_empty(), "only that mark went");
        assert!(remover_preferido(&c, "foto", None).is_err());
    }
}
