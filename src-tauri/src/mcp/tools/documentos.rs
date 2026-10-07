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

const MAX_CARTA: usize = 20_000;

/// Step 1 of save_cover_letter: renders the letter and leaves it in the app's queue,
/// documentos/pendentes/carta-<nanos>.{html,json} (the pair `migrar::processar_pendentes` consumes).
/// The only names written are built here, so nothing the agent sends can reach another folder.
/// Returns the queue's base name (internal: it never goes back to the agent) and, when the profile
/// has no e-mail, a warning for the agent's answer. A profile without a name is refused (CLA-40).
pub fn guardar_pendente(data_dir: &Path, texto: &str, vaga_id: Option<i64>, idioma: &str, variante_id: Option<&str>) -> Result<(String, Option<String>), String> {
    if idioma != "pt" && idioma != "en" {
        return Err(format!("idioma inválido: {idioma} (use pt ou en)"));
    }
    if texto.trim().is_empty() {
        return Err("o texto da carta está vazio".into());
    }
    if texto.chars().count() > MAX_CARTA {
        return Err(format!("o texto da carta passa de {MAX_CARTA} caracteres"));
    }
    if let Some(v) = variante_id {
        documentos::variantes::validar(data_dir, v)?;
    }
    let raw = std::fs::read_to_string(data_dir.join("candidate_base.yaml")).map_err(|e| format!("erro ao ler candidate_base.yaml: {e}"))?;
    let perfil = crate::commands::perfil::parse_candidato_base_str(&raw)?;
    let aviso = crate::commands::cover_letter::validar_perfil_para_carta(&perfil.dados_pessoais)?;
    let (empresa, cargo) = match vaga_id {
        Some(id) => super::open_db(data_dir)?
            .query_row("SELECT empresa, titulo FROM vagas WHERE id = ?1", [id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|_| format!("a vaga {id} não existe"))?,
        None => (String::new(), String::new()),
    };
    let nome = match (empresa.is_empty(), cargo.is_empty()) {
        (true, _) => "Carta".to_string(),
        (false, true) => format!("Carta {empresa}"),
        (false, false) => format!("Carta {empresa} – {cargo}"),
    };
    let html = crate::commands::cover_letter::render_carta(texto.trim(), &perfil, &empresa, &cargo, idioma);
    let mut tags = vec![
        biblioteca::Tag { tipo: "cor".into(), valor: "#D97757".into() },
        biblioteca::Tag { tipo: "idioma".into(), valor: idioma.to_string() },
    ];
    if let Some(v) = variante_id {
        tags.insert(0, biblioteca::Tag { tipo: "variante".into(), valor: v.to_string() });
    }
    let meta = serde_json::json!({
        "tipo": "carta", "nome": nome, "idioma": idioma, "variante_id": variante_id, "vaga_id": vaga_id, "tags": tags,
    });

    let pasta = documentos::docs_dir(data_dir).join("pendentes");
    std::fs::create_dir_all(&pasta).map_err(|e| format!("erro ao preparar a fila de cartas: {e}"))?;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let base = format!("carta-{nanos}");
    std::fs::write(pasta.join(format!("{base}.html")), html).map_err(|e| format!("erro ao guardar a carta: {e}"))?;
    // The app only looks at *.json: write it last, and atomically, so it never sees half a pair.
    let tmp = pasta.join(format!("{base}.json.tmp"));
    let json = serde_json::to_string(&meta).map_err(|e| e.to_string())?;
    let gravado = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, pasta.join(format!("{base}.json"))));
    if let Err(e) = gravado {
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(pasta.join(format!("{base}.html")));
        return Err(format!("erro ao guardar a carta: {e}"));
    }
    Ok((base, aviso))
}

/// The answer the agent reads: the wait's result, plus the warning when there is one.
pub fn juntar_aviso(resposta: String, aviso: Option<&str>) -> String {
    match aviso {
        Some(a) => format!("{resposta} Aviso: {a}"),
        None => resposta,
    }
}

/// Step 2: waits (polling the library) until the app has turned the queued letter into a document.
pub fn aguardar_documento(data_dir: &Path, base: &str, espera: std::time::Duration) -> String {
    let inicio = std::time::Instant::now();
    loop {
        let id: Option<i64> = biblioteca::open_conn(data_dir).ok().and_then(|c| {
            c.query_row("SELECT id FROM documentos WHERE arquivo = ?1", [format!("{base}.pdf")], |r| r.get(0)).ok()
        });
        if let Some(id) = id {
            return format!("Carta salva como documento {id} (PDF pronto).");
        }
        if inicio.elapsed() >= espera {
            return "Carta salva; o PDF fica pronto quando o app processar (abra o app).".into();
        }
        std::thread::sleep(std::time::Duration::from_millis(500).min(espera));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::temp_dir;

    const BASE_YAML: &str = "dados_pessoais:\n  nome_completo: Ana Lima\n  email: ana@example.com\n";

    fn dir_carta() -> std::path::PathBuf {
        let d = temp_dir("mcp-carta");
        std::fs::write(d.join("candidate_base.yaml"), BASE_YAML).unwrap();
        std::fs::write(d.join("search_variants.yaml"), "variantes:\n  - id: backend\n").unwrap();
        crate::db::apply(&biblioteca::open_conn(&d).unwrap()).unwrap();
        d
    }

    fn arvore(d: &Path) -> Vec<String> {
        fn anda(p: &Path, pre: &str, out: &mut Vec<String>) {
            for e in std::fs::read_dir(p).unwrap().flatten() {
                let n = format!("{pre}{}", e.file_name().to_string_lossy());
                if e.path().is_dir() {
                    anda(&e.path(), &format!("{n}/"), out);
                }
                out.push(n);
            }
        }
        let mut o = Vec::new();
        anda(d, "", &mut o);
        o.sort();
        o
    }

    #[test]
    fn save_cover_letter_leaves_a_pending_letter_and_no_path_in_the_answer() {
        let d = dir_carta();
        let antes = arvore(&d);
        let base = guardar_pendente(&d, "Prezados,\n\nQuero a vaga.", None, "pt", Some("backend")).unwrap().0;
        let resposta = aguardar_documento(&d, &base, std::time::Duration::ZERO);
        assert!(resposta.contains("fica pronto"), "{resposta}");
        for leak in [d.to_string_lossy().as_ref(), "pendentes", ".html", ".json", "carta-"] {
            assert!(!resposta.contains(leak) && !base.is_empty(), "{leak} leaked: {resposta}");
        }
        let novos: Vec<String> = arvore(&d).into_iter().filter(|n| !antes.contains(n)).collect();
        assert!(novos.iter().all(|n| n == "documentos" || n.starts_with("documentos/pendentes")), "only documentos/pendentes/ is written: {novos:?}");
        let pasta = documentos::docs_dir(&d).join("pendentes");
        let html = std::fs::read_to_string(pasta.join(format!("{base}.html"))).unwrap();
        assert!(html.contains("Quero a vaga.") && html.contains("Ana Lima"), "{html}");
        let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(pasta.join(format!("{base}.json"))).unwrap()).unwrap();
        assert_eq!(meta["tipo"], "carta");
        assert_eq!(meta["idioma"], "pt");
        assert_eq!(meta["variante_id"], "backend");
        assert!(meta["tags"].as_array().unwrap().iter().any(|t| t["tipo"] == "variante" && t["valor"] == "backend"));
        assert!(meta["nome"].as_str().unwrap().starts_with("Carta"));
    }

    #[test]
    fn juntar_aviso_so_acrescenta_quando_ha_aviso() {
        assert_eq!(juntar_aviso("Carta salva.".into(), None), "Carta salva.");
        let r = juntar_aviso("Carta salva.".into(), Some("sem e-mail."));
        assert_eq!(r, "Carta salva. Aviso: sem e-mail.");
    }

    // Pendente de Sandbox: grava em temp_dir.
    #[test]
    fn save_cover_letter_without_a_name_refuses_and_writes_nothing() {
        let d = temp_dir("mcp-carta-sem-nome");
        std::fs::write(d.join("candidate_base.yaml"), "dados_pessoais:\n  email: ana@example.invalid\n").unwrap();
        crate::db::apply(&biblioteca::open_conn(&d).unwrap()).unwrap();
        let antes = arvore(&d);
        let e = guardar_pendente(&d, "Texto.", None, "pt", None).unwrap_err();
        assert!(e.contains("não tem nome"), "{e}");
        assert_eq!(arvore(&d), antes, "nothing was written");
    }

    // Pendente de Sandbox: grava em temp_dir.
    #[test]
    fn save_cover_letter_without_an_email_saves_and_warns() {
        let d = temp_dir("mcp-carta-sem-email");
        std::fs::write(d.join("candidate_base.yaml"), "dados_pessoais:\n  nome_completo: Ana Lima\n").unwrap();
        crate::db::apply(&biblioteca::open_conn(&d).unwrap()).unwrap();
        let (base, aviso) = guardar_pendente(&d, "Texto.", None, "pt", None).unwrap();
        assert!(aviso.unwrap().contains("e-mail"));
        assert!(documentos::docs_dir(&d).join("pendentes").join(format!("{base}.json")).exists());
    }

    #[test]
    fn save_cover_letter_takes_company_and_title_from_the_vaga() {
        let d = dir_carta();
        biblioteca::open_conn(&d)
            .unwrap()
            .execute_batch("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status) VALUES (1, 'Dev Rust', 'ACME', 'linkedin', 'https://x/1', datetime('now'), 'descoberta');")
            .unwrap();
        let base = guardar_pendente(&d, "Texto.", Some(1), "en", None).unwrap().0;
        let pasta = documentos::docs_dir(&d).join("pendentes");
        let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(pasta.join(format!("{base}.json"))).unwrap()).unwrap();
        assert_eq!(meta["vaga_id"], 1);
        assert!(meta["nome"].as_str().unwrap().contains("ACME"), "{meta}");
        assert!(guardar_pendente(&d, "Texto.", Some(999), "en", None).is_err(), "unknown vaga");
    }

    #[test]
    fn save_cover_letter_refuses_bad_input_and_writes_nothing() {
        let d = dir_carta();
        let antes = arvore(&d);
        assert!(guardar_pendente(&d, "x", None, "pt", Some("inexistente")).is_err(), "unknown variant");
        assert!(guardar_pendente(&d, "x", None, "pt", Some("..\\..\\fora")).is_err(), "a path as a variant id");
        assert!(guardar_pendente(&d, "x", None, "../en", None).is_err(), "bad language");
        assert!(guardar_pendente(&d, "   ", None, "pt", None).is_err(), "empty text");
        assert!(guardar_pendente(&d, &"a".repeat(MAX_CARTA + 1), None, "pt", None).is_err(), "huge text");
        assert_eq!(arvore(&d), antes, "nothing was written");
    }

    /// The queue contract with documentos/migrar.rs: what this tool writes is what the app consumes.
    #[tokio::test]
    async fn the_app_consumes_the_letter_this_tool_queued() {
        let d = dir_carta();
        let base = guardar_pendente(&d, "Texto da carta.", None, "en", Some("backend")).unwrap().0;
        let r = crate::documentos::migrar::migrar_com(&d, |_h: std::path::PathBuf, p: std::path::PathBuf| async move {
            std::fs::write(p, b"%PDF-1.7 fake").map_err(|e| e.to_string())
        })
        .await;
        assert_eq!((r.migrados, r.falhas.len()), (1, 0), "{:?}", r.falhas);
        let lista = list_documentos(&d, "carta").unwrap();
        assert!(lista.contains("\"idioma\": \"en\"") && lista.contains("backend"), "{lista}");
        assert!(!documentos::docs_dir(&d).join("pendentes").join(format!("{base}.json")).exists(), "the queue is emptied");
        assert!(aguardar_documento(&d, &base, std::time::Duration::ZERO).contains("PDF pronto"));
    }

    #[test]
    fn a_queued_letter_that_the_app_registered_is_reported_as_a_document() {
        let d = dir_carta();
        let base = guardar_pendente(&d, "Texto.", None, "pt", None).unwrap().0;
        let c = biblioteca::open_conn(&d).unwrap();
        let docs = documentos::docs_dir(&d);
        std::fs::write(docs.join(format!("{base}.pdf")), b"%PDF-1.7").unwrap();
        let id = documentos::inserir(&c, &documentos::NovoDocumento { tipo: "carta", nome: "C", origem: "gerado", arquivo: &format!("{base}.pdf"), arquivo_html: None, formato: "pdf", idioma: Some("pt"), sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        let r = aguardar_documento(&d, &base, std::time::Duration::from_secs(2));
        assert!(r.contains(&format!("documento {id}")) && r.contains("PDF pronto"), "{r}");
        assert!(!r.contains(&base), "{r}");
    }

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
