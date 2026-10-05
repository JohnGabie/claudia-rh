//! Document tools (spec section 7). Metadata, tags and text only: never a file name or a path.
//! Marking a preferred document is a Profile-session write. Nothing here reads or writes outside
//! the library, except save_cover_letter, which only ever writes inside documentos/pendentes/.
use crate::documentos::{self, biblioteca};
use crate::mcp::SessionKind;
use std::path::Path;

fn tipo_ok(tipo: &str) -> Result<(), String> {
    if tipo == "curriculo" || tipo == "carta" {
        Ok(())
    } else {
        Err(format!("tipo inválido: {tipo} (use curriculo ou carta)"))
    }
}

pub fn list_documentos(data_dir: &Path, tipo: &str) -> Result<String, String> {
    tipo_ok(tipo)?;
    let v = biblioteca::listar_view(&biblioteca::open_conn(data_dir)?, tipo)?;
    serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
}

pub fn get_documento(data_dir: &Path, id: i64) -> Result<String, String> {
    let d = biblioteca::detalhe(&biblioteca::open_conn(data_dir)?, data_dir, id)?;
    let texto = match (d.documento.origem.as_str(), &d.preview_html) {
        ("gerado", Some(h)) => documentos::texto::html_para_texto(h),
        ("gerado", None) => "(o HTML deste documento sumiu: gere de novo para ler o texto)".to_string(),
        _ => "(documento importado: o texto não é extraído; responda formulários pelo candidate_base.yaml)".to_string(),
    };
    serde_json::to_string_pretty(&serde_json::json!({ "documento": d.documento, "texto": texto })).map_err(|e| e.to_string())
}

pub fn set_documento_preferido(data_dir: &Path, id: i64, variante_id: Option<&str>, session: SessionKind) -> Result<String, String> {
    if session == SessionKind::Autonomous {
        return Err("Marcar o documento preferido só numa sessão de Perfil, onde o usuário acompanha a conversa.".into());
    }
    if let Some(v) = variante_id {
        documentos::variantes::validar(data_dir, v)?;
    }
    documentos::definir_preferido(&biblioteca::open_conn(data_dir)?, id, variante_id)?;
    Ok(match variante_id {
        Some(v) => format!("Documento {id} é o preferido da variante {v}."),
        None => format!("Documento {id} é o preferido geral."),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::temp_dir;

    fn lib_with_two() -> (std::path::PathBuf, i64, i64) {
        let d = temp_dir("mcp-docs");
        let c = biblioteca::open_conn(&d).unwrap();
        crate::db::apply(&c).unwrap();
        let docs = documentos::docs_dir(&d);
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("cv_g.pdf"), b"%PDF-1.7 g").unwrap();
        std::fs::write(docs.join("cv_g.html"), "<h1>Ana Lima</h1><p>Rust &amp; SQL</p>").unwrap();
        let g = documentos::inserir(&c, &documentos::NovoDocumento { tipo: "curriculo", nome: "Gerado", origem: "gerado", arquivo: "cv_g.pdf", arquivo_html: Some("cv_g.html"), formato: "pdf", idioma: Some("pt"), sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        let src = d.join("meu.pdf");
        std::fs::write(&src, b"%PDF-1.7 meu").unwrap();
        let i = documentos::importar::importar(&c, &d, "curriculo", &src, Some("Meu")).unwrap().id;
        (d, g, i)
    }

    #[test]
    fn no_document_tool_returns_a_path() {
        let (d, g, i) = lib_with_two();
        let home = d.to_string_lossy().to_string();
        for out in [list_documentos(&d, "curriculo").unwrap(), get_documento(&d, g).unwrap(), get_documento(&d, i).unwrap()] {
            for leak in [home.as_str(), "cv_g.pdf", "cv_g.html", "importado-", "arquivo", "sha256", "\\\\", ".pdf\""] {
                assert!(!out.contains(leak), "{leak} leaked: {out}");
            }
        }
    }

    #[test]
    fn a_generated_document_comes_with_its_text_an_imported_one_says_why_not() {
        let (d, g, i) = lib_with_two();
        assert!(get_documento(&d, g).unwrap().contains("Ana Lima\\nRust & SQL"));
        let imp = get_documento(&d, i).unwrap();
        assert!(imp.contains("candidate_base.yaml"), "{imp}");
        assert!(get_documento(&d, 999).is_err());
        assert!(list_documentos(&d, "outro").is_err());
    }

    #[test]
    fn marking_preferred_is_a_profile_session_write() {
        let (d, g, _) = lib_with_two();
        std::fs::write(d.join("search_variants.yaml"), "variantes:\n  - id: backend\n").unwrap();
        assert!(set_documento_preferido(&d, g, Some("backend"), SessionKind::Autonomous).unwrap_err().contains("sessão de Perfil"));
        let c = biblioteca::open_conn(&d).unwrap();
        assert_eq!(documentos::preferido(&c, "curriculo", Some("backend")), None);
        set_documento_preferido(&d, g, Some("backend"), SessionKind::Interactive).unwrap();
        assert_eq!(documentos::preferido(&c, "curriculo", Some("backend")), Some(g));
        assert!(set_documento_preferido(&d, g, Some("inexistente"), SessionKind::Interactive).is_err(), "unknown variant");
    }
}
