//! A document the app generated (CV or cover letter): the preview HTML and the attachable PDF live side
//! by side in <app data>/documentos/ under bare names, and the row exists only once the PDF is real.
use super::biblioteca::{self, Tag};
use sha2::Digest;
use std::future::Future;
use std::path::{Path, PathBuf};

pub struct NovoGerado {
    pub tipo: String,
    pub nome: String,
    /// Bare file stem, e.g. "cv_classic-ats_2026-10-04_10-00-00": the files become <base>.html and <base>.pdf.
    pub base: String,
    pub html: String,
    pub idioma: Option<String>,
    pub variante_id: Option<String>,
    pub vaga_id: Option<i64>,
    pub tags: Vec<Tag>,
}

fn base_valida(base: &str) -> bool {
    !base.is_empty() && !base.contains(['/', '\\', ':']) && !base.starts_with('.')
}

/// Writes the HTML, asks `print` for the PDF, checks it, and only then registers the document.
/// Any failure leaves no row and no file behind. No database handle is held across the await, so the
/// future stays `Send` (Tauri async commands need that).
pub async fn registrar_gerado<F, Fut>(data_dir: &Path, novo: NovoGerado, print: F) -> Result<i64, String>
where
    F: FnOnce(PathBuf, PathBuf) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    if !base_valida(&novo.base) {
        return Err("nome de arquivo inválido".into());
    }
    let dir = super::docs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let html_name = format!("{}.html", novo.base);
    let pdf_name = format!("{}.pdf", novo.base);
    let html_path = dir.join(&html_name);
    let pdf_path = dir.join(&pdf_name);
    if html_path.exists() || pdf_path.exists() {
        return Err("já existe um documento com esse nome; tente de novo".into());
    }
    let limpar = || {
        let _ = std::fs::remove_file(&html_path);
        let _ = std::fs::remove_file(&pdf_path);
    };
    if let Err(e) = std::fs::write(&html_path, &novo.html) {
        limpar();
        return Err(e.to_string());
    }
    let printed = print(html_path.clone(), pdf_path.clone()).await;
    let bytes = match printed.and_then(|_| std::fs::read(&pdf_path).map_err(|_| "o PDF não foi gravado".to_string())) {
        Ok(b) if b.starts_with(b"%PDF-") => b,
        Ok(_) => {
            limpar();
            return Err("o PDF gerado é inválido".into());
        }
        Err(e) => {
            limpar();
            return Err(e);
        }
    };
    let sha = format!("{:x}", sha2::Sha256::digest(&bytes));
    let r = (|| -> Result<i64, String> {
        let mut conn = biblioteca::open_conn(data_dir)?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let id = super::inserir(
            &tx,
            &super::NovoDocumento {
                tipo: &novo.tipo,
                nome: &novo.nome,
                origem: "gerado",
                arquivo: &pdf_name,
                arquivo_html: Some(&html_name),
                formato: "pdf",
                idioma: novo.idioma.as_deref(),
                sha256: &sha,
                tamanho: bytes.len() as i64,
                variante_id: novo.variante_id.as_deref(),
            },
        )
        .map_err(|e| e.to_string())?;
        if let Some(v) = novo.vaga_id {
            tx.execute("UPDATE documentos SET vaga_id = ?1 WHERE id = ?2", rusqlite::params![v, id]).map_err(|e| e.to_string())?;
        }
        biblioteca::gravar_tags(&tx, id, &novo.tags)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    })();
    if r.is_err() {
        limpar();
    }
    r
}
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-reg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        dir
    }

    fn novo(base: &str) -> NovoGerado {
        NovoGerado {
            tipo: "curriculo".into(),
            nome: "Classic ATS (pt)".into(),
            base: base.into(),
            html: "<html><body>CV</body></html>".into(),
            idioma: Some("pt".into()),
            variante_id: Some("backend".into()),
            vaga_id: None,
            tags: vec![Tag { tipo: "template".into(), valor: "classic-ats".into() }],
        }
    }

    fn count(dir: &Path) -> i64 {
        rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap().query_row("SELECT COUNT(*) FROM documentos", [], |r| r.get(0)).unwrap()
    }

    #[tokio::test]
    async fn a_generated_document_gets_its_html_and_pdf_side_by_side() {
        let dir = setup("ok");
        let id = registrar_gerado(&dir, novo("cv_a"), |_h, p| async move { std::fs::write(p, b"%PDF-1.7 fake").map_err(|e| e.to_string()) }).await.unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let d = crate::documentos::obter(&c, id).unwrap().unwrap();
        assert_eq!((d.arquivo.as_str(), d.arquivo_html.as_deref(), d.origem.as_str()), ("cv_a.pdf", Some("cv_a.html"), "gerado"));
        assert_eq!(d.variante_id.as_deref(), Some("backend"));
        assert_eq!(crate::documentos::biblioteca::ver(&c, id).unwrap().tags.len(), 1);
        crate::documentos::seguranca::resolve_attachable(&c, &dir, id).expect("the PDF is attachable as registered");
    }

    #[tokio::test]
    async fn a_failed_pdf_registers_nothing_and_leaves_no_files() {
        let dir = setup("fail");
        let e = registrar_gerado(&dir, novo("cv_b"), |_h, _p| async { Err::<(), String>("boom".into()) }).await.unwrap_err();
        assert!(e.contains("boom"), "{e}");
        assert_eq!(count(&dir), 0);
        let docs = crate::documentos::docs_dir(&dir);
        assert!(!docs.join("cv_b.html").exists() && !docs.join("cv_b.pdf").exists());
    }

    #[tokio::test]
    async fn an_invalid_pdf_is_refused() {
        let dir = setup("invalid");
        let e = registrar_gerado(&dir, novo("cv_c"), |_h, p| async move { std::fs::write(p, b"<html>").map_err(|e| e.to_string()) }).await.unwrap_err();
        assert!(e.contains("inválido"), "{e}");
        assert_eq!(count(&dir), 0);
    }

    #[tokio::test]
    async fn a_name_with_a_path_is_refused() {
        let dir = setup("path");
        for bad in ["../cv", "..\\cv", "C:cv", "sub/cv", ""] {
            assert!(registrar_gerado(&dir, novo(bad), |_h, _p| async { Ok(()) }).await.is_err(), "{bad}");
        }
    }
}
