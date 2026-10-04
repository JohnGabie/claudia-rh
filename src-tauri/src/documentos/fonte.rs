//! What browser_upload asks: "which file for this vaga?". Answered from the library only.
use crate::mcp::tools::browser::{Attachment, DocumentSource};
use sha2::Digest;
use std::path::PathBuf;

/// A file name a recruiter can read, safe on any OS: letters and digits kept (accents too), anything
/// else becomes a single dash, at most 60 characters.
fn friendly_name(nome: &str, tipo: &str) -> String {
    let mut out = String::new();
    for ch in nome.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out.trim_matches('-').chars().take(60).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        if tipo == "carta" { "Carta".into() } else { "Curriculo".into() }
    } else {
        out
    }
}

pub struct DbDocumentSource {
    pub data_dir: PathBuf,
}

impl DocumentSource for DbDocumentSource {
    fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String> {
        let conn = crate::mcp::tools::open_db(&self.data_dir)?;
        let (id, regra) = match documento {
            Some(id) => {
                let d = super::obter(&conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
                if d.tipo != tipo {
                    return Err(format!("o documento {id} não é um {tipo}"));
                }
                (id, "escolhido pelo agente".to_string())
            }
            None => {
                let variante: Option<String> = conn
                    .query_row("SELECT variante_id FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0))
                    .ok()
                    .flatten();
                let e = super::escolha::escolher(&conn, tipo, variante.as_deref()).ok_or_else(|| {
                    format!("não há {tipo} para anexar: marque um como preferido no app (Perfil → Currículos) ou gere um para a variante da vaga")
                })?;
                (e.id, e.regra.to_string())
            }
        };
        let a = super::seguranca::resolve_attachable(&conn, &self.data_dir, id)?;
        // The site (and the recruiter) sees the file's own name: hand Chrome a copy named after the
        // document, made from bytes checked against the registered hash right now.
        let bytes = std::fs::read(&a.path).map_err(|e| e.to_string())?;
        if format!("{:x}", sha2::Sha256::digest(&bytes)) != a.sha256 {
            return Err(format!("o arquivo de \"{}\" mudou durante o envio: tente de novo", a.nome));
        }
        let dir = super::docs_dir(&self.data_dir).join("envio").join(a.id.to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.{}", friendly_name(&a.nome, tipo), a.formato));
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        Ok(Attachment { id: a.id, nome: a.nome, formato: a.formato, tamanho: a.tamanho, sha256: a.sha256, path, regra })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::browser::DocumentSource;

    fn setup(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-fonte-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        crate::db::apply(&c).unwrap();
        c.execute(
            "INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status, variante_id)
             VALUES (7, 't', 'e', 'p', 'https://x/7', datetime('now'), 'candidatando', 'backend')",
            [],
        )
        .unwrap();
        dir
    }

    #[test]
    fn picks_by_the_vagas_variant_and_resolves_safely() {
        let dir = setup("variant");
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("meu.pdf");
        std::fs::write(&src, b"%PDF-1.7 x").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, Some("Meu")).unwrap();
        crate::documentos::definir_preferido(&c, d.id, Some("backend")).unwrap();
        let a = DbDocumentSource { data_dir: dir.clone() }.for_upload(7, "curriculo", None).unwrap();
        assert_eq!((a.id, a.regra.as_str()), (d.id, "preferido da variante"));
    }

    #[test]
    fn an_explicit_document_must_be_of_the_right_type() {
        let dir = setup("type");
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("carta.pdf");
        std::fs::write(&src, b"%PDF-1.7 x").unwrap();
        let carta = crate::documentos::importar::importar(&c, &dir, "carta", &src, None).unwrap();
        let e = DbDocumentSource { data_dir: dir }.for_upload(7, "curriculo", Some(carta.id)).unwrap_err();
        assert!(e.contains("não é um curriculo"), "{e}");
    }

    #[test]
    fn the_site_receives_the_documents_name_with_the_same_bytes() {
        let dir = setup("name");
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("qualquer.pdf");
        std::fs::write(&src, b"%PDF-1.7 bytes do cv").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, Some("Meu CV / João: Final")).unwrap();
        crate::documentos::definir_preferido(&c, d.id, None).unwrap();
        let a = DbDocumentSource { data_dir: dir.clone() }.for_upload(7, "curriculo", None).unwrap();
        assert_eq!(a.path.file_name().unwrap().to_string_lossy(), "Meu-CV-João-Final.pdf", "a name a recruiter can read");
        assert_eq!(std::fs::read(&a.path).unwrap(), b"%PDF-1.7 bytes do cv", "exactly the verified bytes");
        let base = std::fs::canonicalize(crate::documentos::docs_dir(&dir)).unwrap();
        assert!(std::fs::canonicalize(&a.path).unwrap().starts_with(&base), "still inside the documents folder");
    }

    #[test]
    fn nothing_to_attach_explains_what_to_do() {
        let dir = setup("empty");
        let e = DbDocumentSource { data_dir: dir }.for_upload(7, "curriculo", None).unwrap_err();
        assert!(e.contains("preferido"), "{e}");
    }
}
