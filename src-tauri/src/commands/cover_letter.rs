use chrono::{Datelike, Local};
use std::io::BufRead;
use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager};
use crate::commands::perfil::{parse_candidato_base_interno, CandidatoBase};

// ── HTML helpers ──────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hex_to_rgb_cl(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 { return (0, 0, 0); }
    (
        u8::from_str_radix(&h[0..2], 16).unwrap_or(0),
        u8::from_str_radix(&h[2..4], 16).unwrap_or(0),
        u8::from_str_radix(&h[4..6], 16).unwrap_or(0),
    )
}

fn darken_cl(hex: &str, factor: f32) -> String {
    let (r, g, b) = hex_to_rgb_cl(hex);
    let blend = |c: u8| -> u8 { (c as f32 * (1.0 - factor)).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", blend(r), blend(g), blend(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_escapa_html_especial() {
        assert_eq!(esc("<script>alert(1)</script>"), "&lt;script&gt;alert(1)&lt;/script&gt;");
        assert_eq!(esc("a & b"), "a &amp; b");
        assert_eq!(esc(r#"say "hi""#), "say &quot;hi&quot;");
        assert_eq!(esc("clean"), "clean");
    }

    #[test]
    fn hex_to_rgb_cl_parseia_cores_validas() {
        assert_eq!(hex_to_rgb_cl("#d97757"), (0xd9, 0x77, 0x57));
        assert_eq!(hex_to_rgb_cl("#000000"), (0, 0, 0));
        assert_eq!(hex_to_rgb_cl("#ffffff"), (255, 255, 255));
        assert_eq!(hex_to_rgb_cl("aabbcc"), (0xaa, 0xbb, 0xcc)); // without #
    }

    #[test]
    fn hex_to_rgb_cl_hex_invalido_retorna_zero() {
        assert_eq!(hex_to_rgb_cl(""), (0, 0, 0));
        assert_eq!(hex_to_rgb_cl("#abc"), (0, 0, 0)); // too short (< 6 chars after #)
    }

    #[test]
    fn darken_cl_escurece_branco_cinquenta_porcento() {
        // (255 * (1 - 0.5)).round() = 127.5 → 128 = 0x80
        assert_eq!(darken_cl("#ffffff", 0.5), "#808080");
    }

    #[test]
    fn darken_cl_preto_permanece_preto() {
        assert_eq!(darken_cl("#000000", 0.9), "#000000");
    }
}

// ── Profile summary for Claude context ───────────────────────────────────────

fn build_profile_context(data: &CandidatoBase) -> String {
    let mut s = String::new();
    let dp = &data.dados_pessoais;

    s.push_str(&format!("Name: {}\n", dp.nome_completo));
    if !dp.localizacao_atual.is_empty() {
        s.push_str(&format!("Location: {}\n", dp.localizacao_atual));
    }
    for link in &dp.links {
        if !link.url.is_empty() {
            s.push_str(&format!("{}: {}\n", link.tipo, link.url));
        }
    }

    s.push_str("\n--- WORK EXPERIENCE ---\n");
    for exp in &data.experiencia {
        let fim = if exp.fim.is_empty() { "Present" } else { &exp.fim };
        s.push_str(&format!("\n{} at {} ({} – {})\n", exp.cargo, exp.empresa, exp.inicio, fim));
        if !exp.descricao.is_empty() {
            s.push_str(&format!("{}\n", exp.descricao));
        }
        if !exp.conquistas.is_empty() {
            for c in &exp.conquistas { s.push_str(&format!("• {}\n", c)); }
        }
        if !exp.tecnologias.is_empty() {
            s.push_str(&format!("Stack: {}\n", exp.tecnologias.join(", ")));
        }
    }

    if !data.projetos.is_empty() {
        s.push_str("\n--- KEY PROJECTS ---\n");
        for proj in data.projetos.iter().take(5) {
            s.push_str(&format!("\n{}", proj.nome));
            if !proj.url.is_empty() { s.push_str(&format!(" — {}", proj.url)); }
            s.push('\n');
            if !proj.descricao.is_empty() { s.push_str(&format!("{}\n", proj.descricao)); }
            if !proj.tecnologias.is_empty() {
                s.push_str(&format!("Tech: {}\n", proj.tecnologias.join(", ")));
            }
        }
    }

    if !data.competencias.is_empty() {
        s.push_str(&format!("\n--- SKILLS ---\n{}\n", data.competencias.join(", ")));
    }

    if !data.idiomas.is_empty() {
        s.push_str("\n--- LANGUAGES ---\n");
        for i in &data.idiomas {
            s.push_str(&format!("{}{}\n", i.idioma,
                if i.nivel.is_empty() { String::new() } else { format!(" ({})", i.nivel) }));
        }
    }

    s
}

// ── System prompt ─────────────────────────────────────────────────────────────

fn build_system_prompt(data: &CandidatoBase, idioma: &str, data_dir: &std::path::Path) -> String {
    let profile = build_profile_context(data);
    let prompt_id = if idioma == "en" { "cover_letter_en" } else { "cover_letter_pt" };
    crate::commands::prompts::read_prompt(data_dir, prompt_id)
        .replace("{{CANDIDATE_PROFILE}}", &profile)
}

// ── Pure rules (profile check, texts, title) ──────────────────────────────────

/// Letter fixed texts per language. `pt` is pt-BR (CLA-40).
struct TextosCarta {
    recipient: &'static str,
    subject_prefix: &'static str,
    salutation: &'static str,
    closing: &'static str,
    lang_attr: &'static str,
    title_prefix: &'static str,
}

fn textos_carta(idioma: &str) -> TextosCarta {
    if idioma == "en" {
        TextosCarta { recipient: "Hiring Manager", subject_prefix: "Re:", salutation: "Dear Hiring Manager,",
                      closing: "Best regards,", lang_attr: "en", title_prefix: "Cover Letter" }
    } else {
        TextosCarta { recipient: "Equipe de Recrutamento", subject_prefix: "Assunto:", salutation: "Prezada equipe de recrutamento,",
                      closing: "Atenciosamente,", lang_attr: "pt", title_prefix: "Carta de apresentação" }
    }
}

/// Joins the non-empty parts with " — ": a letter never shows "—  —".
fn juntar_partes(partes: &[&str]) -> String {
    partes.iter().map(|p| p.trim()).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" — ")
}

/// Checks the profile before writing a letter. No name: refuses (Err, nothing is written or asked
/// of the AI). No e-mail: Ok(Some(warning)), the letter is made without the contact line.
pub(crate) fn validar_perfil_para_carta(dp: &crate::commands::perfil::DadosPessoais, para: Destino) -> Result<Option<&'static str>, String> {
    if !texto_visivel(&dp.nome_completo) {
        return Err(match para {
            Destino::Agente => "a carta não foi salva: o perfil não tem nome (dados_pessoais.nome_completo). Registre uma pendência pedindo ao usuário para preencher o nome no Perfil.",
            Destino::Tela => "A carta não foi salva: o perfil não tem nome. Preencha o nome no Perfil.",
        }.into());
    }
    if !texto_visivel(&dp.email) {
        return Ok(Some(AVISO_SEM_EMAIL));
    }
    Ok(None)
}

/// Warning code (the screen translates it; the agent gets the text from `texto_aviso`).
pub(crate) const AVISO_SEM_EMAIL: &str = "sem_email";

pub(crate) fn texto_aviso(codigo: &str) -> &'static str {
    match codigo {
        AVISO_SEM_EMAIL => "o perfil não tem e-mail; a carta saiu sem contato.",
        _ => "aviso desconhecido.",
    }
}

/// Who reads the refusal: the agent (MCP) or the user (screen).
#[derive(Clone, Copy)]
pub(crate) enum Destino { Agente, Tela }

/// True when the text has at least one visible character: zero-width space, BOM, word joiner and
/// soft hyphen are not alphanumeric, so a value made only of them does not count.
fn texto_visivel(s: &str) -> bool {
    s.chars().any(char::is_alphanumeric)
}

// ── HTML builder ──────────────────────────────────────────────────────────────

/// The letter the agent wrote through the MCP (save_cover_letter), in the default colour.
pub(crate) fn render_carta(body_text: &str, data: &CandidatoBase, empresa: &str, cargo: &str, idioma: &str) -> String {
    build_cover_letter_html(body_text, data, empresa, cargo, idioma, "#D97757")
}

fn build_cover_letter_html(
    body_text: &str,
    data: &CandidatoBase,
    empresa: &str,
    cargo: &str,
    idioma: &str,
    color: &str,
) -> String {
    let accent = color;
    let accent_dark = darken_cl(color, 0.15);

    let dp = &data.dados_pessoais;
    let nome = esc(&dp.nome_completo);

    // Contact line — visible URLs for PDF ATS parsing
    let mut contact_parts: Vec<String> = Vec::new();
    if texto_visivel(&dp.email) {
        contact_parts.push(format!(r#"<a href="mailto:{0}">{0}</a>"#, esc(dp.email.trim())));
    }
    if !dp.localizacao_atual.is_empty() {
        contact_parts.push(esc(&dp.localizacao_atual));
    }
    for link in &dp.links {
        if !link.url.is_empty() {
            let visible = esc(
                link.url
                    .trim_start_matches("https://www.")
                    .trim_start_matches("https://")
                    .trim_start_matches("http://www.")
                    .trim_start_matches("http://")
                    .trim_end_matches('/')
            );
            contact_parts.push(format!(r#"<a href="{}">{}</a>"#, esc(&link.url), visible));
        }
    }
    let contact = contact_parts.join(" &middot; ");

    // Date
    let now = Local::now();
    let date = if idioma == "en" {
        let months = ["January","February","March","April","May","June",
                      "July","August","September","October","November","December"];
        format!("{} {}, {}", months[(now.month() - 1) as usize], now.day(), now.year())
    } else {
        let months = ["janeiro","fevereiro","março","abril","maio","junho",
                      "julho","agosto","setembro","outubro","novembro","dezembro"];
        format!("{} de {} de {}", now.day(), months[(now.month() - 1) as usize], now.year())
    };

    let tx = textos_carta(idioma);
    let (recipient, closing, lang_attr, salutation) = (tx.recipient, tx.closing, tx.lang_attr, tx.salutation);
    let subject = format!("{} {}", tx.subject_prefix, juntar_partes(&[&esc(cargo), &nome]));
    let titulo = juntar_partes(&[tx.title_prefix, &nome, &esc(cargo)]);
    let contact_div = if contact.is_empty() { String::new() } else { format!(r#"<div class="contact-line">{contact}</div>"#) };
    let empresa_br = if empresa.trim().is_empty() { String::new() } else { format!("<br>\n    {}", esc(empresa)) };

    // Body: convert plain paragraphs to <p> tags
    // Claude should output clean paragraphs separated by \n\n
    let body_html: String = body_text
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            // Collapse single newlines within a paragraph to a space
            let clean = p.trim().replace('\n', " ");
            format!("    <p>{}</p>", esc(&clean))
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(r#"<!DOCTYPE html>
<html lang="{lang_attr}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{titulo}</title>
<style>
  @page {{ size: A4; margin: 22mm 26mm; }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: Calibri, Arial, sans-serif; font-size: 11pt; color: #1a1a1a; line-height: 1.65; }}
  .header-block {{ border-bottom: 2.5px solid {accent}; padding-bottom: 14px; margin-bottom: 24px; }}
  .candidate-name {{ font-size: 19pt; font-weight: 700; color: {accent_dark}; margin-bottom: 5px; letter-spacing: -0.3px; }}
  .contact-line {{ font-size: 9.5pt; color: #555; }}
  .contact-line a {{ color: {accent}; text-decoration: none; }}
  .date-line {{ font-size: 10.5pt; color: #444; margin-bottom: 18px; }}
  .recipient-block {{ font-size: 10.5pt; margin-bottom: 18px; line-height: 1.5; }}
  .subject-line {{ font-weight: 700; font-size: 11pt; margin-bottom: 24px;
                   border-left: 3px solid {accent}; padding-left: 10px; color: #1a1a1a; }}
  .salutation {{ font-size: 11pt; margin-bottom: 16px; }}
  .body p {{ margin-bottom: 14px; font-size: 11pt; }}
  .body p:last-child {{ margin-bottom: 0; }}
  .closing-block {{ margin-top: 30px; font-size: 11pt; color: #333; }}
  .signature {{ margin-top: 22px; font-weight: 700; font-size: 11.5pt; }}
  a {{ color: {accent}; text-decoration: none; }}
</style>
</head>
<body>

  <div class="header-block">
    <div class="candidate-name">{nome}</div>
    {contact_div}
  </div>

  <div class="date-line">{date}</div>

  <div class="recipient-block">
    {recipient}{empresa_br}
  </div>

  <div class="subject-line">{subject}</div>

  <div class="salutation">{salutation}</div>

  <div class="body">
{body_html}
  </div>

  <div class="closing-block">{closing}</div>
  <div class="signature">{nome}</div>

</body>
</html>"#,
        lang_attr = lang_attr,
        nome = nome,
        titulo = titulo,
        empresa_br = empresa_br,
        accent = accent,
        accent_dark = accent_dark,
        contact_div = contact_div,
        date = date,
        recipient = recipient,
        subject = subject,
        salutation = salutation,
        body_html = body_html,
        closing = closing,
    )
}

// ── Core generation ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn spawn_cover_letter_claude(
    app: AppHandle,
    empresa: String,
    cargo: String,
    descricao_vaga: String,
    nota_extra: String,
    idioma: String,
    cor_primaria: String,
    variante_id: Option<String>,
) {
    std::thread::spawn(move || {
        let data_dir = app.path().app_data_dir().unwrap_or_default();
        let data = match parse_candidato_base_interno(&app) {
            Ok(d) => d,
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Erro ao ler perfil: {e}"));
                return;
            }
        };

        // CLA-40: no name, no letter; the AI is not even called.
        let aviso = match validar_perfil_para_carta(&data.dados_pessoais, Destino::Tela) {
            Ok(a) => a,
            Err(e) => {
                let _ = app.emit("cover-letter-error", e);
                return;
            }
        };

        let sys = build_system_prompt(&data, &idioma, &data_dir);

        // User message: job context
        let mut msg = format!("Write a cover letter for the following role:\n\nCompany: {empresa}\nRole: {cargo}");
        if !descricao_vaga.is_empty() {
            msg.push_str(&format!("\n\nJob description:\n{descricao_vaga}"));
        }
        if !nota_extra.is_empty() {
            msg.push_str(&format!("\n\nAdditional context:\n{nota_extra}"));
        }

        let mut child = match std::process::Command::new(crate::commands::claude_program())
            .args([
                "--dangerously-skip-permissions",
                "--print",
                "--output-format", "stream-json",
                "--verbose",
                "--include-partial-messages",
                &msg,
                "--system-prompt",
                &sys,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Erro ao iniciar Claude: {e}"));
                return;
            }
        };

        let stdout = child.stdout.take().expect("stdout piped");
        let reader = std::io::BufReader::new(stdout);
        let mut body_text = String::new();

        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if line.trim().is_empty() { continue; }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if val["type"] == "stream_event" {
                    if let Some(text) = val["event"]["delta"]["text"].as_str() {
                        body_text.push_str(text);
                        let _ = app.emit("cover-letter-stream", text.to_string());
                    }
                }
                if val["type"] == "result" && val["subtype"] == "success" {
                    if let Some(r) = val["result"].as_str() {
                        body_text = r.to_string();
                    }
                }
            }
        }
        let _ = child.wait();

        let body_text = body_text.trim().to_string();
        if body_text.is_empty() {
            let _ = app.emit("cover-letter-error", "Claude returned no content.".to_string());
            return;
        }

        let html = build_cover_letter_html(&body_text, &data, &empresa, &cargo, &idioma, &cor_primaria);

        let data_dir = match app.path().app_data_dir() {
            Ok(d) => d,
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Error getting data directory: {e}"));
                return;
            }
        };
        let empresa_slug: String = empresa
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .chars()
            .take(30)
            .collect();
        let empresa_slug = if empresa_slug.is_empty() { "empresa".to_string() } else { empresa_slug };
        let variante_id = variante_id.filter(|v| !v.is_empty());
        let mut tags = vec![
            crate::documentos::biblioteca::Tag { tipo: "cor".into(), valor: cor_primaria.clone() },
            crate::documentos::biblioteca::Tag { tipo: "idioma".into(), valor: idioma.clone() },
        ];
        if let Some(v) = variante_id.as_deref() {
            tags.insert(0, crate::documentos::biblioteca::Tag { tipo: "variante".into(), valor: v.to_string() });
        }
        let novo = crate::documentos::registrar::NovoGerado {
            tipo: "carta".into(),
            nome: if cargo.is_empty() { format!("Carta {empresa}") } else { format!("Carta {empresa} – {cargo}") },
            base: format!("cl_{empresa_slug}_{}", Local::now().format("%Y-%m-%d_%H-%M-%S")),
            html,
            idioma: Some(idioma.clone()),
            variante_id,
            vaga_id: None,
            tags,
        };
        let printer = app.clone();
        // This is a plain std thread (the CLI stream is blocking): drive the async print to completion here.
        let registered = tauri::async_runtime::block_on(crate::documentos::registrar::registrar_gerado(&data_dir, novo, move |h, p| async move {
            crate::documentos::pdf::html_to_pdf(&printer, &h, &p).await
        }));
        let view = registered.and_then(|id| crate::documentos::biblioteca::ver(&crate::documentos::biblioteca::open_conn(&data_dir)?, id));
        match view {
            Ok(doc) => {
                if let Some(a) = aviso {
                    let _ = app.emit("cover-letter-warning", a.to_string());
                }
                let _ = app.emit("cover-letter-done", doc);
            }
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Erro ao salvar a carta: {e}"));
            }
        }
    });
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn gerar_cover_letter(
    app: AppHandle,
    empresa: String,
    cargo: String,
    descricao_vaga: Option<String>,
    nota_extra: Option<String>,
    idioma: Option<String>,
    cor_primaria: Option<String>,
    variante_id: Option<String>,
) {
    spawn_cover_letter_claude(
        app,
        empresa,
        cargo,
        descricao_vaga.unwrap_or_default(),
        nota_extra.unwrap_or_default(),
        idioma.unwrap_or_else(|| "pt".to_string()),
        cor_primaria.unwrap_or_else(|| "#D97757".to_string()),
        variante_id,
    );
}

#[cfg(test)]
mod carta_tests {
    use super::*;
    use crate::commands::perfil::DadosPessoais;

    fn perfil(nome: &str, email: &str) -> CandidatoBase {
        CandidatoBase {
            dados_pessoais: DadosPessoais { nome_completo: nome.into(), email: email.into(), ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn sem_nome_recusa() {
        for nome in ["", "   ", "\u{200B}", "\u{FEFF}", "\u{2060}", "\u{00AD}", " \u{200B}\u{FEFF}\u{2060}\u{00AD} "] {
            let e = validar_perfil_para_carta(&perfil(nome, "ana@example.invalid").dados_pessoais, Destino::Tela).unwrap_err();
            assert!(e.contains("não tem nome"), "{e}");
        }
    }

    #[test]
    fn recusa_fala_com_cada_destinatario() {
        let dp = perfil("", "").dados_pessoais;
        let agente = validar_perfil_para_carta(&dp, Destino::Agente).unwrap_err();
        assert!(agente.contains("Registre uma pendência pedindo ao usuário para preencher o nome no Perfil"), "{agente}");
        let tela = validar_perfil_para_carta(&dp, Destino::Tela).unwrap_err();
        assert!(tela.contains("Preencha o nome no Perfil") && !tela.contains("pendência"), "{tela}");
    }

    #[test]
    fn sem_email_salva_e_avisa() {
        for email in ["", "   ", "\u{200B}"] {
            let a = validar_perfil_para_carta(&perfil("Ana Lima", email).dados_pessoais, Destino::Tela).unwrap();
            assert_eq!(a, Some("sem_email"));
        }
        assert!(texto_aviso("sem_email").contains("e-mail"));
    }

    #[test]
    fn email_so_de_espacos_nao_gera_mailto() {
        let h = render_carta("Corpo.", &perfil("Ana Lima", "   "), "ACME", "Dev", "pt");
        assert!(!h.contains("mailto:") && !h.contains(r#"<div class="contact-line">"#), "{h}");
    }

    #[test]
    fn perfil_completo_sem_aviso() {
        assert_eq!(validar_perfil_para_carta(&perfil("Ana Lima", "ana@example.invalid").dados_pessoais, Destino::Tela).unwrap(), None);
    }

    #[test]
    fn pt_usa_os_textos_pt_br() {
        let h = render_carta("Corpo.", &perfil("Ana Lima", "ana@example.invalid"), "ACME", "Dev", "pt");
        for t in ["Equipe de Recrutamento", "Prezada equipe de recrutamento,", "Atenciosamente,", "Assunto: Dev — Ana Lima", "<title>Carta de apresentação — Ana Lima — Dev</title>"] {
            assert!(h.contains(t), "falta {t}: {h}");
        }
        for t in ["Exmo.", "Responsável de Recrutamento", "melhores cumprimentos", "Cover Letter"] {
            assert!(!h.contains(t), "sobrou {t}");
        }
    }

    #[test]
    fn en_nao_muda() {
        let h = render_carta("Body.", &perfil("Ana Lima", "ana@example.invalid"), "ACME", "Dev", "en");
        for t in ["Dear Hiring Manager,", "Best regards,", "Hiring Manager", "Re: Dev — Ana Lima", "<title>Cover Letter — Ana Lima — Dev</title>", r#"lang="en""#] {
            assert!(h.contains(t), "falta {t}");
        }
    }

    #[test]
    fn titulo_e_assunto_sem_partes_vazias() {
        let h = render_carta("Corpo.", &perfil("Ana Lima", ""), "", "", "pt");
        assert!(h.contains("<title>Carta de apresentação — Ana Lima</title>"), "{h}");
        assert!(h.contains("Assunto: Ana Lima</div>"), "{h}");
        assert!(!h.contains("—  —") && !h.contains("— </") && !h.contains("— —"), "{h}");
        assert!(!h.contains(r#"<div class="contact-line">"#), "linha de contato vazia");
    }

    #[test]
    fn contato_aparece_com_email() {
        let h = render_carta("Corpo.", &perfil("Ana Lima", "ana@example.invalid"), "ACME", "Dev", "pt");
        assert!(h.contains("mailto:ana@example.invalid"));
    }

    #[test]
    fn juntar_partes_ignora_vazias() {
        assert_eq!(juntar_partes(&["a", "", "  ", "b"]), "a — b");
        assert_eq!(juntar_partes(&["", ""]), "");
    }
}
