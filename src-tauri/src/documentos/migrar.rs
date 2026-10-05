//! Old generated CVs (<data>/curriculos/*.html) and cover letters (<data>/cover_letters/*.html) join the
//! library: the HTML is copied (as text) into documentos/ and gets its PDF. Nothing in the old folders is
//! ever deleted, moved or changed. Idempotent: a file whose migrated PDF is registered is skipped; a
//! failure is retried on the next start.
use super::biblioteca::{self, Tag};
use super::registrar::{registrar_gerado, NovoGerado};
use std::future::Future;
use std::path::{Path, PathBuf};

pub struct Antigo {
    pub tipo: &'static str,
    pub origem: PathBuf,
    /// Marker key in `migrados`: "<pasta>/<file>.html".
    pub chave: String,
    pub base: String,
    pub nome: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Default)]
pub struct Relatorio {
    pub migrados: usize,
    pub falhas: Vec<String>,
}

fn template_de(stem: &str) -> Option<(&'static str, &'static str)> {
    [("classic-ats", "Classic ATS"), ("hybrid-skills", "Hybrid Skills"), ("dev-compact", "Dev Compact")]
        .into_iter()
        .find(|(id, _)| stem.starts_with(&format!("cv_{id}_")))
}

/// "2026-09-30_10-00" at the end of the stem, shown as "2026-09-30 10:00". `get` (not indexing): a
/// stem with accents near the end must not panic on a char boundary.
fn data_de(stem: &str) -> String {
    match stem.len().checked_sub(16).and_then(|i| stem.get(i..)) {
        Some(d) if d.is_ascii() => format!("{} {}", &d[..10], d[11..].replace('-', ":")),
        _ => String::new(),
    }
}

fn ja_migrado(conn: &rusqlite::Connection, base: &str) -> bool {
    conn.query_row("SELECT 1 FROM documentos WHERE arquivo = ?1", [format!("{base}.pdf")], |_| Ok(()))
        .is_ok()
}

fn marcado(conn: &rusqlite::Connection, chave: &str) -> bool {
    conn.query_row("SELECT 1 FROM migrados WHERE origem = ?1", [chave], |_| Ok(())).is_ok()
}

fn marcar(conn: &rusqlite::Connection, chave: &str) {
    let _ = conn.execute("INSERT OR IGNORE INTO migrados (origem) VALUES (?1)", [chave]);
}

/// Only one migration at a time: the start-up run and the "documentos" listener run can overlap.
static TRAVA: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn pendentes(data_dir: &Path) -> Result<Vec<Antigo>, String> {
    let conn = biblioteca::open_conn(data_dir)?;
    let mut out = Vec::new();
    for (pasta, tipo) in [("curriculos", "curriculo"), ("cover_letters", "carta")] {
        let Ok(entries) = std::fs::read_dir(data_dir.join(pasta)) else { continue };
        let mut files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        files.sort_by_key(|e| e.file_name());
        for e in files {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_file() || ft.is_symlink() {
                continue;
            }
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("html") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
            let base = format!("migrado-{stem}");
            let chave = format!("{pasta}/{}", e.file_name().to_string_lossy());
            if marcado(&conn, &chave) {
                continue;
            }
            if ja_migrado(&conn, &base) {
                // Migrated by a build without markers: record it now so deleting it later sticks.
                marcar(&conn, &chave);
                continue;
            }
            let quando = data_de(&stem);
            let (nome, tags) = if tipo == "curriculo" {
                match template_de(&stem) {
                    Some((id, rotulo)) => (format!("{rotulo} ({quando})"), vec![Tag { tipo: "template".into(), valor: id.into() }]),
                    None => (format!("Currículo ({quando})"), vec![]),
                }
            } else {
                let empresa = stem.strip_prefix("cl_").unwrap_or(&stem);
                let empresa = empresa.len().checked_sub(17).and_then(|i| empresa.get(..i)).unwrap_or(empresa);
                (format!("Carta {} ({quando})", empresa.replace('-', " ")), vec![])
            };
            out.push(Antigo { tipo, origem: path, chave, base, nome, tags });
        }
    }
    Ok(out)
}

/// Metadata the MCP process leaves next to a pending letter (contract with `save_cover_letter`).
#[derive(serde::Deserialize)]
struct PendenteDoc {
    tipo: String,
    nome: String,
    idioma: Option<String>,
    variante_id: Option<String>,
    vaga_id: Option<i64>,
    #[serde(default)]
    tags: Vec<Tag>,
}

/// Letters the agent saved while the app had no WebView: `documentos/pendentes/<n>.json` + `<n>.html`.
/// Those files are the app's own queue, so they are removed once registered; a failure leaves them
/// for the next start.
async fn processar_pendentes<F, Fut>(data_dir: &Path, print: &F, r: &mut Relatorio)
where
    F: Fn(PathBuf, PathBuf) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    let pasta = super::docs_dir(data_dir).join("pendentes");
    let Ok(entries) = std::fs::read_dir(&pasta) else { return };
    let mut nomes: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let p = e.path();
            (p.extension().and_then(|x| x.to_str()) == Some("json")).then(|| p.file_stem().and_then(|s| s.to_str()).map(str::to_string)).flatten()
        })
        .collect();
    nomes.sort();
    for n in nomes {
        let json_p = pasta.join(format!("{n}.json"));
        let html_p = pasta.join(format!("{n}.html"));
        let apagar = || {
            let _ = std::fs::remove_file(&json_p);
            let _ = std::fs::remove_file(&html_p);
        };
        let ja = biblioteca::open_conn(data_dir).map(|c| ja_migrado(&c, &n)).unwrap_or(false);
        if ja {
            // Registered in an earlier run that died before cleaning the queue.
            apagar();
            continue;
        }
        let lido = std::fs::read_to_string(&json_p)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<PendenteDoc>(&s).map_err(|e| e.to_string()))
            .and_then(|m| std::fs::read_to_string(&html_p).map(|h| (m, h)).map_err(|e| e.to_string()));
        let (m, html) = match lido {
            Ok(x) => x,
            Err(e) => {
                r.falhas.push(format!("pendentes/{n}: {e}"));
                continue;
            }
        };
        let novo = NovoGerado { tipo: m.tipo, nome: m.nome, base: n.clone(), html, idioma: m.idioma, variante_id: m.variante_id, vaga_id: m.vaga_id, tags: m.tags };
        match registrar_gerado(data_dir, novo, print).await {
            Ok(_) => {
                apagar();
                r.migrados += 1;
            }
            Err(e) => r.falhas.push(format!("pendentes/{n}: {e}")),
        }
    }
}

pub async fn migrar_com<F, Fut>(data_dir: &Path, print: F) -> Relatorio
where
    F: Fn(PathBuf, PathBuf) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    let _guarda = TRAVA.lock().await;
    let mut r = Relatorio::default();
    let lista = match pendentes(data_dir) {
        Ok(l) => l,
        Err(e) => {
            r.falhas.push(e);
            return r;
        }
    };
    let docs = super::docs_dir(data_dir);
    for a in lista {
        let chave = a.chave.clone();
        let html = match std::fs::read_to_string(&a.origem) {
            Ok(h) => h,
            Err(e) => {
                r.falhas.push(format!("{}: {e}", a.origem.display()));
                continue;
            }
        };
        // Re-check right before touching files: a document registered since the list was built owns them.
        if biblioteca::open_conn(data_dir).map(|c| ja_migrado(&c, &a.base)).unwrap_or(false) {
            continue;
        }
        // Leftovers of an interrupted run are ours (migrado-* has no row): clear them so it can retry.
        let _ = std::fs::remove_file(docs.join(format!("{}.html", a.base)));
        let _ = std::fs::remove_file(docs.join(format!("{}.pdf", a.base)));
        let novo = NovoGerado { tipo: a.tipo.into(), nome: a.nome, base: a.base.clone(), html, idioma: None, variante_id: None, vaga_id: None, tags: a.tags };
        match registrar_gerado(data_dir, novo, &print).await {
            Ok(_) => {
                r.migrados += 1;
                if let Ok(c) = biblioteca::open_conn(data_dir) {
                    marcar(&c, &chave);
                }
            }
            Err(e) => r.falhas.push(format!("{}: {e}", a.base)),
        }
    }
    processar_pendentes(data_dir, &print, &mut r).await;
    r
}

pub const EVENTO_ATUALIZADOS: &str = "documentos-atualizados";

pub async fn ao_iniciar(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    let Ok(data_dir) = app.path().app_data_dir() else { return };
    let r = migrar_com(&data_dir, |h: PathBuf, p: PathBuf| {
        let app = app.clone();
        async move { super::pdf::html_to_pdf(&app, &h, &p).await }
    })
    .await;
    if r.migrados > 0 || !r.falhas.is_empty() {
        crate::diagnostics::emit_event("documentos", "migracao", None, &format!("migrados={} falhas={}", r.migrados, r.falhas.len()));
    }
    // The screen reloads the library on this exact event name (contract with the frontend).
    if r.migrados > 0 {
        let _ = app.emit(EVENTO_ATUALIZADOS, serde_json::json!({ "novos": r.migrados }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-mig-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("curriculos")).unwrap();
        std::fs::create_dir_all(dir.join("cover_letters")).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        std::fs::write(dir.join("curriculos").join("cv_classic-ats_2026-09-30_10-00.html"), "<p>cv antigo</p>").unwrap();
        std::fs::write(dir.join("cover_letters").join("cl_acme_2026-09-30_11-00.html"), "<p>carta antiga</p>").unwrap();
        dir
    }

    async fn fake_pdf(_h: PathBuf, p: PathBuf) -> Result<(), String> {
        std::fs::write(p, b"%PDF-1.7 migrado").map_err(|e| e.to_string())
    }

    fn rows(dir: &Path) -> Vec<(String, String, String)> {
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let mut st = c.prepare("SELECT tipo, origem, arquivo FROM documentos ORDER BY id").unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(|r| r.unwrap()).collect()
    }

    #[tokio::test]
    async fn old_cvs_and_letters_become_documents_and_the_originals_stay() {
        let dir = setup("ok");
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (2, 0), "{:?}", r.falhas);
        assert_eq!(rows(&dir), vec![
            ("curriculo".into(), "gerado".into(), "migrado-cv_classic-ats_2026-09-30_10-00.pdf".into()),
            ("carta".into(), "gerado".into(), "migrado-cl_acme_2026-09-30_11-00.pdf".into()),
        ]);
        assert_eq!(std::fs::read_to_string(dir.join("curriculos").join("cv_classic-ats_2026-09-30_10-00.html")).unwrap(), "<p>cv antigo</p>", "nothing is deleted or changed");
        assert!(dir.join("cover_letters").join("cl_acme_2026-09-30_11-00.html").exists());
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let v = crate::documentos::biblioteca::ver(&c, 1).unwrap();
        assert!(v.tags.iter().any(|t| t.tipo == "template" && t.valor == "classic-ats"), "{:?}", v.tags);
    }

    #[test]
    fn the_event_name_is_the_frontend_contract() {
        assert_eq!(EVENTO_ATUALIZADOS, "documentos-atualizados");
    }

    #[tokio::test]
    async fn running_twice_migrates_once() {
        let dir = setup("twice");
        migrar_com(&dir, fake_pdf).await;
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (0, 0), "{:?}", r.falhas);
        assert_eq!(rows(&dir).len(), 2);
    }

    #[tokio::test]
    async fn a_failed_pdf_skips_that_file_and_the_rest_still_migrate() {
        let dir = setup("partial");
        let r = migrar_com(&dir, |h: PathBuf, p: PathBuf| async move {
            if h.to_string_lossy().contains("cl_acme") { Err("boom".to_string()) } else { fake_pdf(h, p).await }
        })
        .await;
        assert_eq!((r.migrados, r.falhas.len()), (1, 1));
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!(r.migrados, 1, "the failed one is retried on the next start");
        assert_eq!(rows(&dir).len(), 2);
    }

    #[tokio::test]
    async fn a_half_done_migration_is_retried() {
        let dir = setup("half");
        let docs = crate::documentos::docs_dir(&dir);
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("migrado-cv_classic-ats_2026-09-30_10-00.html"), "leftover").unwrap();
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (2, 0), "{:?}", r.falhas);
    }

    #[test]
    fn only_html_files_at_the_top_level_are_taken() {
        let dir = std::env::temp_dir().join(format!("claudia-mig-only-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("curriculos").join("sub")).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        std::fs::write(dir.join("curriculos").join("notas.txt"), "x").unwrap();
        std::fs::write(dir.join("curriculos").join("sub").join("cv_x_2026-01-01_00-00.html"), "x").unwrap();
        assert!(pendentes(&dir).unwrap().is_empty());
    }

    #[test]
    fn odd_file_names_never_panic() {
        let dir = std::env::temp_dir().join(format!("claudia-mig-odd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("cover_letters")).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        for name in ["cl_ção.html", "cl_empresa-ação_çççççççççç.html", "x.html"] {
            std::fs::write(dir.join("cover_letters").join(name), "x").unwrap();
        }
        assert_eq!(pendentes(&dir).unwrap().len(), 3);
    }

    /// A print that yields, so two migrations running concurrently really interleave.
    async fn slow_pdf(_h: PathBuf, p: PathBuf) -> Result<(), String> {
        tokio::task::yield_now().await;
        std::fs::write(p, b"%PDF-1.7 migrado").map_err(|e| e.to_string())
    }

    #[tokio::test]
    async fn two_concurrent_migrations_register_each_file_once_and_keep_the_files() {
        let dir = setup("conc");
        let (a, b) = tokio::join!(migrar_com(&dir, slow_pdf), migrar_com(&dir, slow_pdf));
        assert!(a.falhas.is_empty() && b.falhas.is_empty(), "{:?} {:?}", a.falhas, b.falhas);
        assert_eq!(a.migrados + b.migrados, 2);
        let r = rows(&dir);
        assert_eq!(r.len(), 2, "{r:?}");
        let docs = crate::documentos::docs_dir(&dir);
        for (_, _, arq) in r {
            assert!(docs.join(&arq).exists(), "{arq} must exist");
            assert!(docs.join(arq.replace(".pdf", ".html")).exists());
        }
    }

    #[tokio::test]
    async fn a_migrated_document_the_user_deletes_does_not_come_back() {
        let dir = setup("deleted");
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!(r.migrados, 2);
        {
            let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            biblioteca::apagar(&c, &dir, 1).unwrap();
        }
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (0, 0), "{:?}", r.falhas);
        assert_eq!(rows(&dir).len(), 1, "the deleted one stays deleted");
        assert!(dir.join("curriculos").join("cv_classic-ats_2026-09-30_10-00.html").exists(), "the original is never touched");
    }

    #[tokio::test]
    async fn documents_migrated_before_the_marker_table_existed_are_backfilled() {
        let dir = setup("backfill");
        migrar_com(&dir, fake_pdf).await;
        {
            let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            c.execute("DELETE FROM migrados", []).unwrap(); // as if migrated by an older build
        }
        migrar_com(&dir, fake_pdf).await; // sees the rows, records the markers
        {
            let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            biblioteca::apagar(&c, &dir, 1).unwrap();
        }
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!(r.migrados, 0);
    }

    fn pendente(dir: &Path, n: &str, json: &str) {
        let p = crate::documentos::docs_dir(dir).join("pendentes");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join(format!("{n}.html")), "<p>carta do agente</p>").unwrap();
        std::fs::write(p.join(format!("{n}.json")), json).unwrap();
    }

    const JSON: &str = r#"{"tipo":"carta","nome":"Carta Acme","idioma":"pt","variante_id":null,"vaga_id":7,"tags":[]}"#;

    #[tokio::test]
    async fn a_pending_letter_from_the_agent_becomes_a_document() {
        let dir = setup("pend");
        std::fs::remove_dir_all(dir.join("curriculos")).unwrap();
        std::fs::remove_dir_all(dir.join("cover_letters")).unwrap();
        {
            let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            c.execute(
                "INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em) VALUES (7, 'Dev', 'Acme', 'x', 'https://x.test/1', '2026-10-04')",
                [],
            )
            .unwrap();
        }
        pendente(&dir, "carta-1", JSON);
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (1, 0), "{:?}", r.falhas);
        assert_eq!(rows(&dir), vec![("carta".into(), "gerado".into(), "carta-1.pdf".into())]);
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let vaga: Option<i64> = c.query_row("SELECT vaga_id FROM documentos WHERE arquivo = 'carta-1.pdf'", [], |r| r.get(0)).unwrap();
        assert_eq!(vaga, Some(7));
        let left = std::fs::read_dir(crate::documentos::docs_dir(&dir).join("pendentes")).unwrap().count();
        assert_eq!(left, 0, "pendentes/ is emptied");
    }

    #[tokio::test]
    async fn a_failed_pending_letter_stays_for_the_next_start() {
        let dir = setup("pendfail");
        std::fs::remove_dir_all(dir.join("curriculos")).unwrap();
        std::fs::remove_dir_all(dir.join("cover_letters")).unwrap();
        pendente(&dir, "carta-2", JSON);
        let r = migrar_com(&dir, |_h: PathBuf, _p: PathBuf| async { Err::<(), String>("boom".into()) }).await;
        assert_eq!(r.falhas.len(), 1);
        let p = crate::documentos::docs_dir(&dir).join("pendentes");
        assert!(p.join("carta-2.json").exists() && p.join("carta-2.html").exists());
    }
}
