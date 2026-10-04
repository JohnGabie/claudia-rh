//! The only way from a document id to a file on disk. The agent never gives a path; this checks that
//! the stored name stays inside <app data>/documentos/, that the file is there, and that it is the
//! same file the user registered (sha256).
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Anexavel {
    pub id: i64,
    pub nome: String,
    pub formato: String,
    pub tamanho: u64,
    pub sha256: String,
    pub path: PathBuf,
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn is_bare_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && !name.starts_with("..")
}

pub fn resolve_attachable(conn: &Connection, data_dir: &Path, id: i64) -> Result<Anexavel, String> {
    let doc = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    let outside = || format!("o documento {id} aponta para fora da pasta de documentos: recusado");
    if !is_bare_name(&doc.arquivo) {
        return Err(outside());
    }
    let base = std::fs::canonicalize(super::docs_dir(data_dir)).map_err(|_| "a pasta de documentos não existe".to_string())?;
    let real = std::fs::canonicalize(base.join(&doc.arquivo)).map_err(|_| format!("o arquivo de \"{}\" sumiu: gere ou importe de novo", doc.nome))?;
    // A link inside the folder that points elsewhere resolves outside it: refused too.
    if !real.starts_with(&base) {
        return Err(outside());
    }
    let meta = std::fs::metadata(&real).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(outside());
    }
    let sha = sha256_file(&real).map_err(|e| e.to_string())?;
    if sha != doc.sha256 {
        return Err(format!("o arquivo de \"{}\" mudou depois de registrado: gere ou importe de novo", doc.nome));
    }
    Ok(Anexavel { id, nome: doc.nome, formato: doc.formato, tamanho: meta.len(), sha256: sha, path: real })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{inserir, NovoDocumento};


    fn setup(name: &str) -> (rusqlite::Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-docs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir)).unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        (c, dir)
    }

    fn add(c: &rusqlite::Connection, dir: &Path, arquivo: &str, bytes: &[u8]) -> i64 {
        let p = crate::documentos::docs_dir(dir).join(arquivo);
        if !arquivo.contains("..") && !arquivo.contains('/') && !arquivo.contains('\\') && !arquivo.contains(':') {
            std::fs::write(&p, bytes).unwrap();
        }
        let sha = if p.exists() { sha256_file(&p).unwrap() } else { "00".into() };
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "importado", arquivo, arquivo_html: None, formato: "pdf", idioma: None, sha256: &sha, tamanho: bytes.len() as i64, variante_id: None }).unwrap()
    }

    #[test]
    fn a_registered_file_resolves_inside_the_folder() {
        let (c, dir) = setup("ok");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 hello");
        let a = resolve_attachable(&c, &dir, id).unwrap();
        assert!(a.path.ends_with("cv.pdf"));
        assert_eq!(a.tamanho, 14);
    }

    #[test]
    fn a_name_that_escapes_the_folder_is_refused() {
        let (c, dir) = setup("escape");
        std::fs::write(dir.join("segredo.pdf"), b"%PDF-1.7 x").unwrap();
        for bad in ["..\\segredo.pdf", "../segredo.pdf", "C:\\Windows\\win.ini", "sub/cv.pdf"] {
            let id = add(&c, &dir, bad, b"");
            let e = resolve_attachable(&c, &dir, id).unwrap_err();
            assert!(e.contains("fora da pasta"), "{bad}: {e}");
        }
    }

    #[test]
    fn missing_file_is_refused() {
        let (c, dir) = setup("missing");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 x");
        std::fs::remove_file(crate::documentos::docs_dir(&dir).join("cv.pdf")).unwrap();
        assert!(resolve_attachable(&c, &dir, id).unwrap_err().contains("sumiu"));
    }

    #[test]
    fn edited_file_is_refused() {
        let (c, dir) = setup("edited");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 original");
        std::fs::write(crate::documentos::docs_dir(&dir).join("cv.pdf"), b"%PDF-1.7 changed").unwrap();
        assert!(resolve_attachable(&c, &dir, id).unwrap_err().contains("mudou"));
    }

    #[test]
    fn unknown_id_is_refused() {
        let (c, dir) = setup("unknown");
        assert!(resolve_attachable(&c, &dir, 77).unwrap_err().contains("não existe"));
    }
}
