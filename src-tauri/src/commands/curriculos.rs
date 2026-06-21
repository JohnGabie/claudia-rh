use chrono::{Local, Datelike};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use crate::commands::perfil::{parse_candidato_base_interno, CandidatoBase};

fn traduzir_conteudo_en(data: &CandidatoBase) -> CandidatoBase {
    let input = serde_json::json!({
        "experiencia": data.experiencia.iter().enumerate().map(|(i, e)| serde_json::json!({
            "index": i,
            "descricao": e.descricao,
            "conquistas": e.conquistas,
        })).collect::<Vec<_>>(),
        "projetos": data.projetos.iter().enumerate().map(|(i, p)| serde_json::json!({
            "index": i,
            "descricao": p.descricao,
        })).collect::<Vec<_>>(),
    });

    let prompt = format!(
        "Translate the following CV content from Portuguese to English. \
        Keep technical terms, tool names, company names, and proper nouns unchanged. \
        Return ONLY a valid JSON object with the exact same structure — no markdown, no explanation.\n\n{}",
        input
    );

    let mut translated = data.clone();

    let output = std::process::Command::new("claude")
        .args(["--dangerously-skip-permissions", "--print", &prompt])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let text = text.trim();
            let json_text = if let (Some(s), Some(e)) = (text.find('{'), text.rfind('}')) {
                &text[s..=e]
            } else {
                text
            };
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_text) {
                if let Some(arr) = val["experiencia"].as_array() {
                    for item in arr {
                        if let Some(idx) = item["index"].as_u64().map(|n| n as usize) {
                            if idx < translated.experiencia.len() {
                                if let Some(d) = item["descricao"].as_str() {
                                    translated.experiencia[idx].descricao = d.to_string();
                                }
                                if let Some(c) = item["conquistas"].as_array() {
                                    translated.experiencia[idx].conquistas = c
                                        .iter()
                                        .filter_map(|v| v.as_str().map(str::to_string))
                                        .collect();
                                }
                            }
                        }
                    }
                }
                if let Some(arr) = val["projetos"].as_array() {
                    for item in arr {
                        if let Some(idx) = item["index"].as_u64().map(|n| n as usize) {
                            if idx < translated.projetos.len() {
                                if let Some(d) = item["descricao"].as_str() {
                                    translated.projetos[idx].descricao = d.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    translated
}

// ── Struct ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurriculoInfo {
    pub path: String,
    pub file_name: String,
    pub template_id: String,
    pub template_nome: String,
    pub gerado_em: String,
}

// ── Color helpers ─────────────────────────────────────────────────────────────

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 { return (0, 0, 0); }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    (r, g, b)
}

fn tint(hex: &str, factor: f32) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let blend = |c: u8| -> u8 { (c as f32 + factor * (255.0 - c as f32)).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", blend(r), blend(g), blend(b))
}

fn darken(hex: &str, factor: f32) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    let blend = |c: u8| -> u8 { (c as f32 * (1.0 - factor)).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", blend(r), blend(g), blend(b))
}

// ── i18n ─────────────────────────────────────────────────────────────────────

struct Lang {
    present: &'static str,
    professional_summary: &'static str,
    technical_skills: &'static str,
    skills: &'static str,
    skills_stack: &'static str,
    professional_experience: &'static str,
    projects: &'static str,
    selected_projects: &'static str,
    education_full: &'static str,
    education_short: &'static str,
    languages: &'static str,
    with_years: &'static str,
    years_word: &'static str,
    specialized_in: &'static str,
    specialized_start: &'static str,
}

const PT: Lang = Lang {
    present: "Presente",
    professional_summary: "Resumo Profissional",
    technical_skills: "Competências Técnicas",
    skills: "Competências",
    skills_stack: "Competências & Stack",
    professional_experience: "Experiência Profissional",
    projects: "Projetos",
    selected_projects: "Projetos Selecionados",
    education_full: "Formação Académica",
    education_short: "Formação",
    languages: "Idiomas",
    with_years: "com",
    years_word: "anos de experiência",
    specialized_in: "especializado em",
    specialized_start: "Especializado em",
};

const EN: Lang = Lang {
    present: "Present",
    professional_summary: "Professional Summary",
    technical_skills: "Technical Skills",
    skills: "Skills",
    skills_stack: "Skills & Stack",
    professional_experience: "Professional Experience",
    projects: "Projects",
    selected_projects: "Selected Projects",
    education_full: "Academic Background",
    education_short: "Education",
    languages: "Languages",
    with_years: "with",
    years_word: "years of experience",
    specialized_in: "specialized in",
    specialized_start: "Specialized in",
};

fn pick_lang(idioma: &str) -> &'static Lang {
    if idioma == "en" { &EN } else { &PT }
}

// ── HTML helpers ──────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn build_contact_line(data: &CandidatoBase) -> String {
    let dp = &data.dados_pessoais;
    let mut parts: Vec<String> = vec![];

    if !dp.email.is_empty() {
        parts.push(format!(r#"<a href="mailto:{}">{}</a>"#, esc(&dp.email), esc(&dp.email)));
    }
    if !dp.localizacao_atual.is_empty() {
        parts.push(esc(&dp.localizacao_atual));
    }
    for link in &dp.links {
        if !link.url.is_empty() {
            // ATS reads the PDF as plain text — href is never extracted.
            // The visible label must BE the URL so ATS sees it when parsing the PDF.
            let visible = esc(
                link.url
                    .trim_start_matches("https://www.")
                    .trim_start_matches("https://")
                    .trim_start_matches("http://www.")
                    .trim_start_matches("http://")
                    .trim_end_matches('/')
            );
            parts.push(format!(r#"<a href="{}">{}</a>"#, esc(&link.url), visible));
        }
    }

    parts.join(" &middot; ")
}

fn build_experiencia_html(data: &CandidatoBase, lang: &Lang) -> String {
    let mut html = String::new();
    for exp in &data.experiencia {
        let fim = if exp.fim.is_empty() { lang.present.to_string() } else { esc(&exp.fim) };
        html.push_str(&format!(
            r#"<div class="exp-item">
  <div class="exp-row">
    <span class="exp-title">{} &mdash; {}</span>
    <span class="exp-date">{} &ndash; {}</span>
  </div>"#,
            esc(&exp.cargo),
            esc(&exp.empresa),
            esc(&exp.inicio),
            fim,
        ));
        if !exp.descricao.is_empty() {
            html.push_str(&format!("<p class=\"exp-desc\">{}</p>", esc(&exp.descricao)));
        }
        if !exp.conquistas.is_empty() {
            html.push_str("<ul>");
            for c in &exp.conquistas {
                html.push_str(&format!("<li>{}</li>", esc(c)));
            }
            html.push_str("</ul>");
        }
        if !exp.tecnologias.is_empty() {
            html.push_str("<div class=\"tags\">");
            for t in &exp.tecnologias {
                html.push_str(&format!("<span class=\"tag\">{}</span>", esc(t)));
            }
            html.push_str("</div>");
        }
        html.push_str("</div>");
    }
    html
}

fn build_projetos_html(data: &CandidatoBase, limit: usize) -> String {
    let mut html = String::new();
    for proj in data.projetos.iter().take(limit) {
        let nome_link = if !proj.url.is_empty() {
            format!(r#"<a href="{}">{}</a>"#, esc(&proj.url), esc(&proj.nome))
        } else {
            esc(&proj.nome)
        };
        html.push_str(&format!("<div class=\"proj-item\"><p class=\"proj-title\"><strong>{nome_link}</strong>"));
        // URL inline after name when no description (gives ATS and human readers context)
        if proj.descricao.is_empty() && !proj.url.is_empty() {
            html.push_str(&format!(r#" &mdash; <span class="proj-url"><a href="{}">{}</a></span>"#,
                esc(&proj.url), esc(&proj.url)));
        }
        html.push_str("</p>");
        if !proj.descricao.is_empty() {
            html.push_str(&format!("<p class=\"proj-desc\">{}</p>", esc(&proj.descricao)));
        }
        if !proj.tecnologias.is_empty() {
            html.push_str("<div class=\"tags\">");
            for t in &proj.tecnologias {
                html.push_str(&format!("<span class=\"tag\">{}</span>", esc(t)));
            }
            html.push_str("</div>");
        }
        html.push_str("</div>");
    }
    html
}

fn build_competencias_tags(data: &CandidatoBase) -> String {
    data.competencias
        .iter()
        .map(|c| format!("<span class=\"tag\">{}</span>", esc(c)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_competencias_pills(data: &CandidatoBase) -> String {
    data.competencias
        .iter()
        .map(|c| format!("<span class=\"skill-tag\">{}</span>", esc(c)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_competencias_csv(data: &CandidatoBase) -> String {
    data.competencias.iter().map(|c| esc(c)).collect::<Vec<_>>().join(", ")
}

// ATS helpers ──────────────────────────────────────────────────────────────────

fn compute_anos_experiencia(data: &CandidatoBase) -> String {
    let current_year = Local::now().year() as u32;
    let earliest = data.experiencia.iter()
        .filter_map(|e| e.inicio.trim().get(..4).and_then(|y| y.parse::<u32>().ok()))
        .min();
    match earliest {
        Some(y) => {
            let anos = current_year.saturating_sub(y);
            if anos == 0 { String::new() } else { format!("{anos}") }
        }
        None => String::new(),
    }
}

fn extract_all_keywords(data: &CandidatoBase) -> Vec<String> {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut result = Vec::new();
    let mut push = |s: &str| {
        if !s.is_empty() && seen.insert(s.to_lowercase()) {
            result.push(s.to_string());
        }
    };
    for c in &data.competencias { push(c); }
    for exp in &data.experiencia { for t in &exp.tecnologias { push(t); } }
    for proj in &data.projetos { for t in &proj.tecnologias { push(t); } }
    result
}

fn build_resumo_automatico(data: &CandidatoBase, lang: &Lang) -> String {
    let cargo = data.experiencia.first().map(|e| e.cargo.as_str()).unwrap_or("");
    let anos = compute_anos_experiencia(data);
    let keywords = extract_all_keywords(data);
    let top: Vec<_> = keywords.iter().take(5).map(|s| esc(s)).collect();

    let mut out = String::new();
    if !cargo.is_empty() {
        if !anos.is_empty() {
            out.push_str(&format!("{} {} {}+ {}", esc(cargo), lang.with_years, anos, lang.years_word));
        } else {
            out.push_str(&esc(cargo));
        }
    }
    if !top.is_empty() {
        if !out.is_empty() {
            out.push_str(&format!(", {} ", lang.specialized_in));
        } else {
            out.push_str(&format!("{} ", lang.specialized_start));
        }
        out.push_str(&top.join(", "));
        out.push('.');
    }
    out
}

fn build_formacao_html(data: &CandidatoBase, lang: &Lang) -> String {
    let mut html = String::new();
    for f in &data.formacao {
        let fim = if f.fim.is_empty() { lang.present.to_string() } else { esc(&f.fim) };
        html.push_str(&format!(
            "<div class=\"form-item\"><strong>{}</strong> &mdash; {} <span class=\"exp-date\">{} &ndash; {}</span></div>",
            esc(&f.curso),
            esc(&f.instituicao),
            esc(&f.inicio),
            fim,
        ));
    }
    html
}

fn build_idiomas_line(data: &CandidatoBase) -> String {
    data.idiomas
        .iter()
        .map(|i| {
            if i.nivel.is_empty() {
                esc(&i.idioma)
            } else {
                format!("{} ({})", esc(&i.idioma), esc(&i.nivel))
            }
        })
        .collect::<Vec<_>>()
        .join(" &middot; ")
}

// ── Templates ─────────────────────────────────────────────────────────────────

fn template_classic_ats(data: &CandidatoBase, color: &str, lang: &Lang) -> String {
    let nome = esc(&data.dados_pessoais.nome_completo);
    let contact = build_contact_line(data);
    let resumo = build_resumo_automatico(data, lang);
    let competencias_csv = build_competencias_csv(data);
    let competencias_tags = build_competencias_tags(data);
    let experiencia = build_experiencia_html(data, lang);
    let projetos = build_projetos_html(data, usize::MAX);
    let formacao = build_formacao_html(data, lang);
    let s_resumo = lang.professional_summary;
    let s_skills = lang.technical_skills;
    let s_exp = lang.professional_experience;
    let s_proj = lang.projects;
    let s_edu = lang.education_full;
    let s_lang = lang.languages;
    let idiomas = build_idiomas_line(data);
    let accent = color;
    let tag_bg = tint(color, 0.92);
    let tag_border = tint(color, 0.70);
    let tag_text = darken(color, 0.25);
    let html_lang = if lang.with_years == "with" { "en" } else { "pt" };
    let resumo_block = if !resumo.is_empty() {
        format!(r#"<section>
  <h2>{s_resumo}</h2>
  <p style="font-size:10.5pt;color:#222;line-height:1.55">{resumo}</p>
</section>"#)
    } else { String::new() };

    format!(r#"<!DOCTYPE html>
<html lang="{html_lang}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{nome}</title>
<style>
  @page {{ size: A4; margin: 18mm 20mm; }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: Calibri, Arial, sans-serif; font-size: 11pt; color: #1a1a1a; line-height: 1.45; }}
  h1 {{ font-size: 20pt; font-weight: 700; margin-bottom: 4px; }}
  .contact {{ font-size: 10pt; color: #333; margin-bottom: 14px; }}
  .contact a {{ color: {accent}; text-decoration: underline; }}
  section {{ margin-bottom: 4px; }}
  h2 {{ font-size: 11pt; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em;
        border-bottom: 1.5px solid {accent}; padding-bottom: 3px; margin: 14px 0 7px; }}
  .skills-csv {{ font-size: 10pt; color: #333; margin-bottom: 5px; }}
  .exp-item {{ margin-bottom: 10px; }}
  .exp-row {{ display: flex; justify-content: space-between; align-items: baseline; }}
  .exp-title {{ font-weight: 700; font-size: 10.5pt; }}
  .exp-date {{ font-size: 10pt; color: #555; white-space: nowrap; margin-left: 8px; }}
  .exp-desc {{ font-size: 10pt; margin-top: 3px; color: #333; }}
  ul {{ margin: 4px 0 4px 18px; }}
  li {{ font-size: 10pt; margin-bottom: 2px; }}
  .tags {{ display: flex; flex-wrap: wrap; gap: 4px; margin-top: 5px; }}
  .tag {{ background: {tag_bg}; border: 1px solid {tag_border}; padding: 1px 6px; border-radius: 3px;
           font-size: 9pt; color: {tag_text}; }}
  .proj-item {{ margin-bottom: 9px; }}
  .proj-title {{ font-size: 10.5pt; }}
  .proj-url {{ font-size: 9.5pt; }}
  .proj-desc {{ font-size: 10pt; color: #333; margin-top: 2px; }}
  .form-item {{ margin-bottom: 6px; font-size: 10pt; }}
  a {{ color: {accent}; text-decoration: none; }}
</style>
</head>
<body>
  <h1>{nome}</h1>
  <div class="contact">{contact}</div>

  {resumo_block}

  <section>
    <h2>{s_skills}</h2>
    <p class="skills-csv">{competencias_csv}</p>
    <div class="tags">{competencias_tags}</div>
  </section>

  <section>
    <h2>{s_exp}</h2>
    {experiencia}
  </section>

  <section>
    <h2>{s_proj}</h2>
    {projetos}
  </section>

  <section>
    <h2>{s_edu}</h2>
    {formacao}
  </section>

  <section>
    <h2>{s_lang}</h2>
    <p style="font-size:10pt">{idiomas}</p>
  </section>
</body>
</html>"#)
}

fn template_hybrid_skills(data: &CandidatoBase, color: &str, lang: &Lang) -> String {
    let nome = esc(&data.dados_pessoais.nome_completo);
    let contact = build_contact_line(data);
    let resumo = build_resumo_automatico(data, lang);
    let competencias_pills = build_competencias_pills(data);
    let competencias_csv = build_competencias_csv(data);
    let experiencia = build_experiencia_html(data, lang);
    let projetos = build_projetos_html(data, usize::MAX);
    let formacao = build_formacao_html(data, lang);
    let idiomas = build_idiomas_line(data);
    let s_skills = lang.skills;
    let s_exp = lang.professional_experience;
    let s_proj = lang.projects;
    let s_edu = lang.education_full;
    let s_lang = lang.languages;
    let accent = color;
    let accent_dark = darken(color, 0.20);
    let skill_bg = tint(color, 0.90);
    let skill_border = color;
    let skill_text = darken(color, 0.30);
    let tag_bg = tint(color, 0.88);
    let tag_border = tint(color, 0.60);
    let tag_text = darken(color, 0.25);
    let html_lang = if lang.with_years == "with" { "en" } else { "pt" };
    let resumo_block = if !resumo.is_empty() {
        format!(r#"<p class="tagline">{resumo}</p>"#)
    } else { String::new() };

    format!(r#"<!DOCTYPE html>
<html lang="{html_lang}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{nome}</title>
<style>
  @page {{ size: A4; margin: 18mm 20mm; }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: Calibri, Arial, sans-serif; font-size: 11pt; color: #1a1a1a; line-height: 1.45; }}
  h1 {{ font-size: 20pt; font-weight: 700; margin-bottom: 3px; }}
  .tagline {{ font-size: 10pt; color: #555; margin-bottom: 6px; font-style: italic; }}
  .contact {{ font-size: 10pt; color: #444; margin-bottom: 12px; }}
  .contact a {{ color: {accent}; text-decoration: underline; }}
  section {{ margin-bottom: 4px; }}
  h2 {{ font-size: 11pt; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em;
        border-bottom: 2px solid {accent}; padding-bottom: 3px; margin: 14px 0 8px; color: {accent_dark}; }}
  .skills-cloud {{ display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 5px; }}
  .skill-tag {{ background: {skill_bg}; border: 1px solid {skill_border}; padding: 3px 10px; border-radius: 14px;
                font-size: 10pt; color: {skill_text}; font-weight: 500; }}
  .skills-csv {{ font-size: 9pt; color: #888; margin-top: 3px; }}
  .exp-item {{ margin-bottom: 10px; }}
  .exp-row {{ display: flex; justify-content: space-between; align-items: baseline; }}
  .exp-title {{ font-weight: 700; font-size: 10.5pt; }}
  .exp-date {{ font-size: 10pt; color: #777; white-space: nowrap; margin-left: 8px; }}
  .exp-desc {{ font-size: 10pt; margin-top: 3px; color: #444; }}
  ul {{ margin: 4px 0 4px 18px; }}
  li {{ font-size: 10pt; margin-bottom: 2px; }}
  .tags {{ display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }}
  .tag {{ background: {tag_bg}; border: 1px solid {tag_border}; padding: 1px 6px; border-radius: 3px;
           font-size: 9pt; color: {tag_text}; }}
  .proj-item {{ margin-bottom: 9px; }}
  .proj-title {{ font-size: 10.5pt; }}
  .proj-url {{ font-size: 9.5pt; }}
  .proj-desc {{ font-size: 10pt; color: #444; margin-top: 2px; }}
  .form-item {{ margin-bottom: 6px; font-size: 10pt; }}
  a {{ color: {accent}; text-decoration: none; }}
</style>
</head>
<body>
  <h1>{nome}</h1>
  {resumo_block}
  <div class="contact">{contact}</div>

  <section>
    <h2>{s_skills}</h2>
    <div class="skills-cloud">{competencias_pills}</div>
    <p class="skills-csv">{competencias_csv}</p>
  </section>

  <section>
    <h2>{s_exp}</h2>
    {experiencia}
  </section>

  <section>
    <h2>{s_proj}</h2>
    {projetos}
  </section>

  <section>
    <h2>{s_edu}</h2>
    {formacao}
  </section>

  <section>
    <h2>{s_lang}</h2>
    <p style="font-size:10pt">{idiomas}</p>
  </section>
</body>
</html>"#)
}

fn template_dev_compact(data: &CandidatoBase, color: &str, lang: &Lang) -> String {
    let nome = esc(&data.dados_pessoais.nome_completo);
    let contact = build_contact_line(data);
    let anos = compute_anos_experiencia(data);
    let experiencia = build_experiencia_html(data, lang);
    let projetos = build_projetos_html(data, 4);
    let formacao = build_formacao_html(data, lang);
    let idiomas = build_idiomas_line(data);
    let s_skills = lang.skills_stack;
    let s_exp = lang.professional_experience;
    let s_proj = lang.selected_projects;
    let s_edu = lang.education_short;
    let s_lang = lang.languages;
    let anos_label = if lang.with_years == "with" {
        format!("{anos}+ years of experience")
    } else {
        format!("{anos}+ anos de experiência")
    };
    let accent = color;
    let tag_bg = tint(color, 0.90);
    let tag_border = tint(color, 0.60);
    let tag_text = darken(color, 0.30);
    let all_keywords = extract_all_keywords(data);
    let competencias_plain = all_keywords.join(", ");

    let github_url = data.dados_pessoais.links.iter()
        .find(|l| l.tipo.to_lowercase().contains("github"))
        .map(|l| l.url.clone())
        .unwrap_or_default();

    // Show bare URL as visible text — ATS reads the PDF as plain text, not href attributes
    let github_visible = github_url
        .trim_start_matches("https://www.")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string();

    let github_display = if !github_url.is_empty() {
        format!(r#"<div class="github-url"><a href="{}">{}</a></div>"#, esc(&github_url), esc(&github_visible))
    } else {
        String::new()
    };

    let anos_display = if !anos.is_empty() {
        format!(r#"<div class="anos-exp">{anos_label}</div>"#)
    } else { String::new() };

    let html_lang = if lang.with_years == "with" { "en" } else { "pt" };
    format!(r#"<!DOCTYPE html>
<html lang="{html_lang}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{nome}</title>
<style>
  @page {{ size: A4; margin: 18mm 20mm; }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: Calibri, Arial, sans-serif; font-size: 10pt; color: #1a1a1a; line-height: 1.35; }}
  h1 {{ font-size: 17pt; font-weight: 700; margin-bottom: 1px; }}
  .anos-exp {{ font-size: 9.5pt; color: {accent}; font-weight: 600; margin-bottom: 1px; }}
  .github-url {{ font-size: 9.5pt; color: {accent}; margin-bottom: 2px; }}
  .github-url a {{ color: {accent}; text-decoration: underline; }}
  .contact {{ font-size: 9.5pt; color: #444; margin-bottom: 10px; }}
  .contact a {{ color: {accent}; text-decoration: underline; }}
  section {{ margin-bottom: 2px; }}
  h2 {{ font-size: 10pt; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;
        border-left: 3px solid {accent}; padding-left: 8px; margin: 11px 0 5px; color: #1a1a1a; }}
  .skills-text {{ font-size: 9.5pt; color: #333; line-height: 1.5; }}
  .exp-item {{ margin-bottom: 8px; }}
  .exp-row {{ display: flex; justify-content: space-between; align-items: baseline; }}
  .exp-title {{ font-weight: 700; font-size: 10pt; }}
  .exp-date {{ font-size: 9pt; color: #666; white-space: nowrap; margin-left: 8px; }}
  .exp-desc {{ font-size: 9.5pt; margin-top: 2px; color: #444; }}
  ul {{ margin: 3px 0 3px 16px; }}
  li {{ font-size: 9.5pt; margin-bottom: 1px; }}
  .tags {{ display: flex; flex-wrap: wrap; gap: 3px; margin-top: 3px; }}
  .tag {{ background: {tag_bg}; border: 1px solid {tag_border}; padding: 1px 5px; border-radius: 3px;
           font-size: 8.5pt; color: {tag_text}; }}
  .proj-item {{ margin-bottom: 6px; }}
  .proj-title {{ font-size: 10pt; }}
  .proj-url {{ font-size: 9pt; }}
  .proj-desc {{ font-size: 9.5pt; color: #444; margin-top: 1px; }}
  .form-item {{ margin-bottom: 5px; font-size: 9.5pt; }}
  a {{ color: {accent}; text-decoration: none; }}
</style>
</head>
<body>
  <h1>{nome}</h1>
  {anos_display}
  {github_display}
  <div class="contact">{contact}</div>

  <section>
    <h2>{s_skills}</h2>
    <p class="skills-text">{competencias_plain}</p>
  </section>

  <section>
    <h2>{s_exp}</h2>
    {experiencia}
  </section>

  <section>
    <h2>{s_proj}</h2>
    {projetos}
  </section>

  <section>
    <h2>{s_edu}</h2>
    {formacao}
  </section>

  <section>
    <h2>{s_lang}</h2>
    <p style="font-size:9.5pt">{idiomas}</p>
  </section>
</body>
</html>"#)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn gerar_curriculo(app: AppHandle, template_id: String, cor_primaria: Option<String>, idioma: Option<String>) -> Result<CurriculoInfo, String> {
    let raw = parse_candidato_base_interno(&app)?;
    let idioma_str = idioma.as_deref().unwrap_or("pt");
    let color = cor_primaria.as_deref().unwrap_or("#D97757");
    let lang = pick_lang(idioma_str);
    let data = if idioma_str == "en" { traduzir_conteudo_en(&raw) } else { raw };
    let html = match template_id.as_str() {
        "classic-ats" => template_classic_ats(&data, color, lang),
        "hybrid-skills" => template_hybrid_skills(&data, color, lang),
        "dev-compact" => template_dev_compact(&data, color, lang),
        _ => return Err(format!("Template desconhecido: {template_id}")),
    };
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cv_dir = dir.join("curriculos");
    std::fs::create_dir_all(&cv_dir).map_err(|e| e.to_string())?;
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M").to_string();
    let file_name = format!("cv_{template_id}_{timestamp}.html");
    let path = cv_dir.join(&file_name);
    std::fs::write(&path, &html).map_err(|e| e.to_string())?;
    let template_nome = match template_id.as_str() {
        "classic-ats" => "Clássico ATS",
        "hybrid-skills" => "Híbrido Competências",
        _ => "Dev Compacto",
    }.to_string();
    Ok(CurriculoInfo {
        path: path.to_string_lossy().to_string(),
        file_name,
        template_id,
        template_nome,
        gerado_em: timestamp,
    })
}

#[tauri::command]
pub fn listar_curriculos(app: AppHandle) -> Result<Vec<CurriculoInfo>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cv_dir = dir.join("curriculos");
    if !cv_dir.exists() { return Ok(vec![]); }
    let mut infos: Vec<CurriculoInfo> = std::fs::read_dir(&cv_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "html").unwrap_or(false))
        .map(|e| {
            let path = e.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            // filename: cv_{template_id}_{YYYY-MM-DD_HH-MM}.html
            // parse: strip "cv_" prefix and ".html" suffix, then last 16 chars are date
            let stem = file_name.strip_prefix("cv_").unwrap_or(&file_name)
                .strip_suffix(".html").unwrap_or(&file_name);
            let (template_id, gerado_em) = if stem.len() > 17 {
                let date_part = &stem[stem.len()-16..]; // "YYYY-MM-DD_HH-MM"
                let id_part = &stem[..stem.len()-17];   // strip trailing "_" + date
                (id_part.to_string(), date_part.replace('_', " "))
            } else {
                (stem.to_string(), String::new())
            };
            let template_nome = match template_id.as_str() {
                "classic-ats" => "Clássico ATS",
                "hybrid-skills" => "Híbrido Competências",
                "dev-compact" => "Dev Compacto",
                _ => "Desconhecido",
            }.to_string();
            CurriculoInfo {
                path: path.to_string_lossy().to_string(),
                file_name,
                template_id,
                template_nome,
                gerado_em,
            }
        })
        .collect();
    infos.sort_by(|a, b| b.gerado_em.cmp(&a.gerado_em));
    Ok(infos)
}

#[tauri::command]
pub fn abrir_curriculo(app: AppHandle, path: String) -> Result<(), String> {
    app.opener().open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apagar_curriculo(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}
