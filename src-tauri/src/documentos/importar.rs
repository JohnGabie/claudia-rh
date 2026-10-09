//! Bringing the user's own CV into the library: copied (never moved or referenced), checked by
//! extension AND content, at most 10 MB, stored under a generated bare name.
use super::{docs_dir, inserir, obter, Documento, NovoDocumento};
use rusqlite::Connection;
use std::path::Path;

pub const MAX_BYTES: u64 = 10 * 1024 * 1024;

pub(crate) fn dentro_do_limite(len: u64) -> bool {
    len <= MAX_BYTES
}

/// A .docx is a ZIP with a Word document inside, not any ZIP renamed.
fn parece_docx(bytes: &[u8]) -> bool {
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else { return false };
    let tipos = z.by_name("[Content_Types].xml").is_ok();
    let word = z.by_name("word/document.xml").is_ok();
    tipos && word
}

/// Creates the file; fails if anything already has that name (never overwrites).
pub(crate) fn gravar_novo(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(dest).map_err(|_| "conflito de nome ao importar; tente de novo".to_string())?;
    f.write_all(bytes).map_err(|e| {
        let _ = std::fs::remove_file(dest);
        e.to_string()
    })
}

pub fn importar(conn: &Connection, data_dir: &Path, tipo: &str, origem: &Path, nome: Option<&str>) -> Result<Documento, String> {
    if tipo != "curriculo" && tipo != "carta" {
        return Err(format!("tipo inválido: {tipo}"));
    }
    let ext = origem.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let formato = match ext.as_str() {
        "pdf" => "pdf",
        "docx" => "docx",
        _ => return Err("só é possível importar PDF ou DOCX".into()),
    };
    let unreadable = || "não foi possível ler o arquivo escolhido".to_string();
    let meta = std::fs::metadata(origem).map_err(|_| unreadable())?;
    if !dentro_do_limite(meta.len()) {
        return Err("o arquivo passa de 10 MB".into());
    }
    let bytes = std::fs::read(origem).map_err(|_| unreadable())?;
    // The file may have grown between the metadata and the read.
    if !dentro_do_limite(bytes.len() as u64) {
        return Err("o arquivo passa de 10 MB".into());
    }
    let real = if formato == "pdf" { bytes.starts_with(b"%PDF-") } else { bytes.starts_with(b"PK\x03\x04") && parece_docx(&bytes) };
    if !real {
        return Err(format!("o arquivo não é um {} de verdade", formato.to_uppercase()));
    }
    let dir = docs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    // The stored name is always generated: whatever the user called the file never becomes a path.
    let arquivo = format!("importado-{nanos}.{formato}");
    let dest = dir.join(&arquivo);
    gravar_novo(&dest, &bytes)?;
    let sha = super::seguranca::sha256_file(&dest).map_err(|e| e.to_string())?;
    let stem = origem.file_stem().and_then(|s| s.to_str()).unwrap_or("Currículo");
    let nome = nome.unwrap_or(stem).trim();
    let nome = if nome.is_empty() { "Currículo" } else { nome };
    let novo = NovoDocumento { tipo, nome, origem: "importado", arquivo: &arquivo, arquivo_html: None, formato, idioma: None, sha256: &sha, tamanho: bytes.len() as i64, variante_id: None };
    let id = inserir(conn, &novo).map_err(|e| {
        let _ = std::fs::remove_file(&dest);
        e.to_string()
    })?;
    obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| "falha ao registrar o documento".into())
}

#[cfg(test)]
mod tests {
    use super::*;


    fn setup(name: &str) -> (rusqlite::Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-imp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        (c, dir)
    }

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn a_real_pdf_is_copied_registered_and_resolvable() {
        let (c, dir) = setup("pdf");
        let src = file(&dir, "Meu CV.pdf", b"%PDF-1.7 conteudo");
        let d = importar(&c, &dir, "curriculo", &src, None).unwrap();
        assert_eq!((d.origem.as_str(), d.formato.as_str(), d.nome.as_str()), ("importado", "pdf", "Meu CV"));
        assert!(src.exists(), "the user's original is never moved");
        crate::documentos::seguranca::resolve_attachable(&c, &dir, d.id).unwrap();
    }

    fn docx_bytes(with_word: bool) -> Vec<u8> {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("[Content_Types].xml", o).unwrap();
        z.write_all(b"<Types/>").unwrap();
        if with_word {
            z.start_file("word/document.xml", o).unwrap();
            z.write_all(b"<w:document/>").unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn a_real_docx_is_accepted() {
        let (c, dir) = setup("docx");
        let src = file(&dir, "cv.docx", &docx_bytes(true));
        assert_eq!(importar(&c, &dir, "curriculo", &src, Some("Word")).unwrap().formato, "docx");
    }

    #[test]
    fn a_zip_renamed_docx_is_refused() {
        let (c, dir) = setup("zipdocx");
        let src = file(&dir, "cv.docx", &docx_bytes(false));
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("não é um DOCX"));
        let src = file(&dir, "cv2.docx", b"PK\x03\x04 not a zip at all");
        assert!(importar(&c, &dir, "curriculo", &src, None).is_err());
    }

    #[test]
    fn an_existing_destination_is_never_overwritten() {
        let (_c, dir) = setup("noover");
        let dest = dir.join("ja-existe.pdf");
        std::fs::write(&dest, b"%PDF-1.7 original").unwrap();
        assert!(gravar_novo(&dest, b"%PDF-1.7 outro").is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"%PDF-1.7 original");
    }

    #[test]
    fn the_size_limit_is_checked_on_the_bytes_read() {
        assert!(dentro_do_limite(MAX_BYTES) && !dentro_do_limite(MAX_BYTES + 1));
    }

    #[test]
    fn a_fake_pdf_is_refused() {
        let (c, dir) = setup("fake");
        let src = file(&dir, "cv.pdf", b"MZ not a pdf");
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("não é um PDF"));
    }

    #[test]
    fn other_extensions_are_refused() {
        let (c, dir) = setup("ext");
        let src = file(&dir, "cv.exe", b"%PDF-1.7");
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("PDF ou DOCX"));
    }

    #[test]
    fn a_file_over_ten_megabytes_is_refused() {
        let (c, dir) = setup("big");
        let mut big = b"%PDF-1.7".to_vec();
        big.resize((MAX_BYTES + 1) as usize, b'x');
        let src = file(&dir, "cv.pdf", &big);
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("10 MB"));
    }

    #[test]
    fn import_sanitizes_names_and_never_overwrites() {
        let (c, dir) = setup("names");
        let src = file(&dir, "cv.pdf", b"%PDF-1.7 a");
        let a = importar(&c, &dir, "curriculo", &src, Some("..\\..\\x")).unwrap();
        let b = importar(&c, &dir, "curriculo", &src, Some("..\\..\\x")).unwrap();
        assert_ne!(a.arquivo, b.arquivo, "the same import twice gets two files");
        for d in [&a, &b] {
            assert!(!d.arquivo.contains(['\\', '/', ':']) && !d.arquivo.starts_with(".."), "{}", d.arquivo);
            crate::documentos::seguranca::resolve_attachable(&c, &dir, d.id).unwrap();
        }
    }

    #[test]
    fn a_wrong_type_is_refused() {
        let (c, dir) = setup("tipo");
        let src = file(&dir, "cv.pdf", b"%PDF-1.7");
        assert!(importar(&c, &dir, "foto", &src, None).is_err());
    }
}
