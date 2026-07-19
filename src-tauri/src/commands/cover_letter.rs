use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use crate::commands::perfil::{parse_candidato_base_interno, CandidatoBase};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoverLetterInfo {
    pub path: String,
    pub file_name: String,
    pub empresa: String,
    pub cargo: String,
    pub idioma: String,
    pub gerado_em: String,
}

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

// ── HTML builder ──────────────────────────────────────────────────────────────

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
    if !dp.email.is_empty() {
        contact_parts.push(format!(r#"<a href="mailto:{0}">{0}</a>"#, esc(&dp.email)));
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

    let (recipient, subject_prefix, closing, lang_attr, salutation) = if idioma == "en" {
        ("Hiring Manager", "Re:", "Best regards,", "en",
         "Dear Hiring Manager,".to_string())
    } else {
        ("Responsável de Recrutamento", "Assunto:", "Com os melhores cumprimentos,", "pt",
         "Exmo./Exma. Responsável de Recrutamento,".to_string())
    };

    let subject = format!("{} {} — {}", subject_prefix, esc(cargo), nome);

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
<title>Cover Letter — {nome} — {cargo_esc}</title>
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
    <div class="contact-line">{contact}</div>
  </div>

  <div class="date-line">{date}</div>

  <div class="recipient-block">
    {recipient}<br>
    {empresa_esc}
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
        cargo_esc = esc(cargo),
        empresa_esc = esc(empresa),
        accent = accent,
        accent_dark = accent_dark,
        contact = contact,
        date = date,
        recipient = recipient,
        subject = subject,
        salutation = salutation,
        body_html = body_html,
        closing = closing,
    )
}

// ── Core generation ───────────────────────────────────────────────────────────

fn spawn_cover_letter_claude(
    app: AppHandle,
    empresa: String,
    cargo: String,
    descricao_vaga: String,
    nota_extra: String,
    idioma: String,
    cor_primaria: String,
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

        // Save file
        let dir = match app.path().app_data_dir() {
            Ok(d) => d,
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Error getting data directory: {e}"));
                return;
            }
        };
        let cl_dir = dir.join("cover_letters");
        if let Err(e) = std::fs::create_dir_all(&cl_dir) {
            let _ = app.emit("cover-letter-error", format!("Error creating directory: {e}"));
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d_%H-%M").to_string();
        // Sanitize empresa name for filename
        let empresa_slug: String = empresa.chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let empresa_slug = if empresa_slug.is_empty() { "empresa".to_string() } else { empresa_slug };
        let empresa_slug: String = empresa_slug.chars().take(30).collect();

        let file_name = format!("cl_{empresa_slug}_{timestamp}.html");
        let path = cl_dir.join(&file_name);

        if let Err(e) = std::fs::write(&path, &html) {
            let _ = app.emit("cover-letter-error", format!("Erro ao salvar arquivo: {e}"));
            return;
        }

        let info = CoverLetterInfo {
            path: path.to_string_lossy().to_string(),
            file_name,
            empresa,
            cargo,
            idioma,
            gerado_em: timestamp,
        };

        let _ = app.emit("cover-letter-done", info);
    });
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn gerar_cover_letter(
    app: AppHandle,
    empresa: String,
    cargo: String,
    descricao_vaga: Option<String>,
    nota_extra: Option<String>,
    idioma: Option<String>,
    cor_primaria: Option<String>,
) {
    spawn_cover_letter_claude(
        app,
        empresa,
        cargo,
        descricao_vaga.unwrap_or_default(),
        nota_extra.unwrap_or_default(),
        idioma.unwrap_or_else(|| "pt".to_string()),
        cor_primaria.unwrap_or_else(|| "#D97757".to_string()),
    );
}

#[tauri::command]
pub fn listar_cover_letters(app: AppHandle) -> Result<Vec<CoverLetterInfo>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cl_dir = dir.join("cover_letters");
    if !cl_dir.exists() { return Ok(vec![]); }

    let mut infos: Vec<CoverLetterInfo> = std::fs::read_dir(&cl_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "html").unwrap_or(false))
        .map(|e| {
            let path = e.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            // filename: cl_{empresa_slug}_{YYYY-MM-DD_HH-MM}.html
            let stem = file_name
                .strip_prefix("cl_").unwrap_or(&file_name)
                .strip_suffix(".html").unwrap_or(&file_name);
            let (empresa_slug, gerado_em) = if stem.len() > 17 {
                let date_part = &stem[stem.len()-16..];
                let slug = &stem[..stem.len()-17];
                (slug.replace('-', " "), date_part.replace('_', " "))
            } else {
                (stem.to_string(), String::new())
            };
            CoverLetterInfo {
                path: path.to_string_lossy().to_string(),
                file_name,
                empresa: empresa_slug,
                cargo: String::new(),
                idioma: String::new(),
                gerado_em,
            }
        })
        .collect();

    infos.sort_by(|a, b| b.gerado_em.cmp(&a.gerado_em));
    Ok(infos)
}

#[tauri::command]
pub fn abrir_cover_letter(app: AppHandle, path: String) -> Result<(), String> {
    app.opener().open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apagar_cover_letter(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}
