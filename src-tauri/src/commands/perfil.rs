use once_cell::sync::OnceCell;
use rusqlite;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

// ── candidate_base.yaml structs ───────────────────────────────────────────────

// Deserializes a YAML number (int or float) as f64.
fn deser_f64_or_int<'de, D>(d: D) -> Result<f64, D::Error>
where D: serde::Deserializer<'de> {
    let val: serde_yaml::Value = serde::Deserialize::deserialize(d)?;
    match &val {
        serde_yaml::Value::Number(n) => n.as_f64().ok_or_else(|| serde::de::Error::custom("invalid number")),
        _ => Err(serde::de::Error::custom(format!("expected number, got {:?}", val))),
    }
}

// Deserializes a YAML null or missing value as an empty String.
fn deser_null_str<'de, D>(d: D) -> Result<String, D::Error>
where D: serde::Deserializer<'de> {
    let opt: Option<String> = serde::Deserialize::deserialize(d)?;
    Ok(opt.unwrap_or_default())
}

// Flattens a competencias field that may be either a list of strings or a
// map of category → [string] (as Claude sometimes generates).
fn deserialize_competencias<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where D: serde::Deserializer<'de> {
    let val: serde_yaml::Value = serde::Deserialize::deserialize(d)?;
    Ok(match val {
        serde_yaml::Value::Sequence(seq) => seq
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        serde_yaml::Value::Mapping(map) => map
            .into_iter()
            .flat_map(|(_, v)| match v {
                serde_yaml::Value::Sequence(s) => s
                    .into_iter()
                    .filter_map(|i| i.as_str().map(String::from))
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect(),
        _ => vec![],
    })
}

fn default_competencias() -> Vec<String> { vec![] }

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Link {
    #[serde(default)] pub tipo: String,
    #[serde(default)] pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DadosPessoais {
    #[serde(default, alias = "nome")] pub nome_completo: String,
    #[serde(default)] pub email: String,
    #[serde(default)] pub telefone: String,
    #[serde(default, alias = "localizacao")] pub localizacao_atual: String,
    #[serde(default)] pub endereco: String,
    #[serde(default)] pub nacionalidade: String,
    #[serde(default)] pub data_nascimento: String,
    #[serde(default)] pub cpf: String,
    #[serde(default)] pub links: Vec<Link>,
    // Legacy flat fields written by Claude — synthesized into `links` after load
    #[serde(default, skip_serializing)] pub linkedin: String,
    #[serde(default, skip_serializing)] pub github: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Experiencia {
    #[serde(default)] pub empresa: String,
    #[serde(default)] pub cargo: String,
    #[serde(default)] pub inicio: String,
    #[serde(default, deserialize_with = "deser_null_str")] pub fim: String,
    #[serde(default)] pub descricao: String,
    #[serde(default)] pub conquistas: Vec<String>,
    #[serde(default)] pub tecnologias: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Projeto {
    #[serde(default)] pub nome: String,
    #[serde(default)] pub descricao: String,
    #[serde(default)] pub tecnologias: Vec<String>,
    #[serde(default)] pub url: String,
    #[serde(default)] pub origem: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Formacao {
    #[serde(default)] pub instituicao: String,
    #[serde(default)] pub curso: String,
    #[serde(default)] pub inicio: String,
    #[serde(default, alias = "fim_previsto")] pub fim: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Idioma {
    #[serde(default, alias = "lingua")] pub idioma: String,
    #[serde(default)] pub nivel: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GapConhecido {
    #[serde(default)] pub competencia: String,
    #[serde(default)] pub contexto: String,
    #[serde(default)] pub como_abordar: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RespostasModelo {
    #[serde(default)] pub porque_esta_vaga: String,
    #[serde(default)] pub pretensao_salarial_texto: String,
    #[serde(default)] pub notice_period: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FonteUsada {
    #[serde(default)] pub tipo: String,
    #[serde(default)] pub referencia: String,
    #[serde(default)] pub consultado_em: String,
}

// Aceita tanto [{tipo,referencia,consultado_em}] como ["url1","url2"] — Claude por vezes
// escreve strings simples em vez de objectos.
fn deserialize_fontes<'de, D>(d: D) -> Result<Vec<FonteUsada>, D::Error>
where D: serde::Deserializer<'de> {
    let val: serde_yaml::Value = serde::Deserialize::deserialize(d)?;
    let seq = match val {
        serde_yaml::Value::Sequence(s) => s,
        _ => return Ok(vec![]),
    };
    Ok(seq.into_iter().map(|v| match v {
        serde_yaml::Value::String(s) => FonteUsada { referencia: s, ..Default::default() },
        other => serde_yaml::from_value(other).unwrap_or_default(),
    }).collect())
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CandidatoBase {
    #[serde(default)] pub dados_pessoais: DadosPessoais,
    #[serde(default)] pub experiencia: Vec<Experiencia>,
    #[serde(default)] pub projetos: Vec<Projeto>,
    #[serde(default)] pub formacao: Vec<Formacao>,
    #[serde(default = "default_competencias", deserialize_with = "deserialize_competencias")]
    pub competencias: Vec<String>,
    #[serde(default)] pub idiomas: Vec<Idioma>,
    #[serde(default)] pub gaps_conhecidos: Vec<GapConhecido>,
    #[serde(default)] pub respostas_modelo: RespostasModelo,
    #[serde(default)] pub ultima_atualizacao: String,
    #[serde(default, deserialize_with = "deserialize_fontes")] pub fontes_usadas: Vec<FonteUsada>,
}

// ── search_variants.yaml structs ──────────────────────────────────────────────

fn default_peso() -> f64 { 50.0 }
fn default_ativa() -> bool { true }

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SearchVariant {
    #[serde(default)] pub id: String,
    #[serde(default, alias = "nome")] pub nome_exibicao: String,
    #[serde(default = "default_peso", deserialize_with = "deser_f64_or_int")] pub peso: f64,
    #[serde(default = "default_ativa")] pub ativa: bool,
    #[serde(default)] pub foco_competencias: Vec<String>,
    #[serde(default)] pub foco_experiencia: Vec<String>,
    #[serde(default)] pub regioes_aceitas: Vec<String>,
    #[serde(default)] pub modelos_trabalho: Vec<String>,
    #[serde(default)] pub idiomas_aplicacao: Vec<String>,
    #[serde(default)] pub cv_gerado_path: String,
    #[serde(default)] pub cv_gerado_em: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FaixaSalarial {
    pub minimo: Option<f64>,
    #[serde(default)] pub moeda: String,
    #[serde(default)] pub flexivel: bool,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PreferenciasGlobais {
    #[serde(default)] pub faixa_salarial: FaixaSalarial,
    #[serde(default)] pub setores_evitar: Vec<String>,
    #[serde(default)] pub empresas_evitar: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PerguntaPendente {
    #[serde(default)] pub pergunta: String,
    #[serde(default)] pub origem_vaga: String,
    #[serde(default)] pub variante_relacionada: String,
    #[serde(default)] pub data: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SearchVariants {
    #[serde(default)] pub variantes: Vec<SearchVariant>,
    #[serde(default)] pub preferencias_globais: PreferenciasGlobais,
    #[serde(default)] pub red_lines: Vec<String>,
    #[serde(default)] pub perguntas_pendentes: Vec<PerguntaPendente>,
}

// ── File paths ────────────────────────────────────────────────────────────────

fn candidato_base_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("candidate_base.yaml"))
}

fn search_variants_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("search_variants.yaml"))
}

fn estrategia_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("strategy.md"))
}

// ── YAML commands ─────────────────────────────────────────────────────────────

/// Shared parsing logic for candidate_base.yaml — no eprintln debug output.
pub fn parse_candidato_base_interno(app: &AppHandle) -> Result<CandidatoBase, String> {
    let path = candidato_base_path(app)?;
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(CandidatoBase::default()),
        Err(e) => return Err(e.to_string()),
    };

    // Normalize `key: null` to `key: ""` so legacy Claude-generated YAMLs don't fail
    let content: String = raw
        .lines()
        .map(|line| {
            if let Some(pos) = line.find(": null") {
                let after = line[pos + 6..].trim_start();
                if after.is_empty() || after.starts_with('#') {
                    return line[..pos].to_string() + ": \"\"";
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut candidato: CandidatoBase = serde_yaml::from_str::<CandidatoBase>(&content)
        .map_err(|e| format!("YAML parse error: {e}"))?;

    // Synthesize links from legacy flat fields written by Claude
    if candidato.dados_pessoais.links.is_empty() {
        let dp = &candidato.dados_pessoais;
        let mut links: Vec<Link> = vec![];
        if !dp.linkedin.is_empty() {
            links.push(Link { tipo: "LinkedIn".into(), url: dp.linkedin.clone() });
        }
        if !dp.github.is_empty() {
            links.push(Link { tipo: "GitHub".into(), url: dp.github.clone() });
        }
        candidato.dados_pessoais.links = links;
    }
    Ok(candidato)
}

#[tauri::command]
pub fn ler_candidato_base(app: AppHandle) -> Result<CandidatoBase, String> {
    eprintln!("[ler_candidato_base] parsing via parse_candidato_base_interno");
    match parse_candidato_base_interno(&app) {
        Ok(c) => {
            eprintln!("[ler_candidato_base] parse OK, nome={:?} email={:?} exp_count={}",
                c.dados_pessoais.nome_completo, c.dados_pessoais.email, c.experiencia.len());
            Ok(c)
        }
        Err(e) => {
            eprintln!("[ler_candidato_base] PARSE ERROR: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn guardar_candidato_base(app: AppHandle, dados: CandidatoBase) -> Result<(), String> {
    let path = candidato_base_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_yaml::to_string(&dados).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ler_search_variants(app: AppHandle) -> Result<Vec<SearchVariant>, String> {
    let path = search_variants_path(&app)?;
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };
    let sv: SearchVariants = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
    Ok(sv.variantes)
}

#[tauri::command]
pub fn guardar_search_variants(app: AppHandle, variantes: SearchVariants) -> Result<(), String> {
    let path = search_variants_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_yaml::to_string(&variantes).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ler_estrategia(app: AppHandle) -> Result<String, String> {
    let path = estrategia_path(&app)?;
    match std::fs::read_to_string(&path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn guardar_estrategia(app: AppHandle, conteudo: String) -> Result<(), String> {
    let path = estrategia_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, conteudo).map_err(|e| e.to_string())
}

// ── Profile chat session ──────────────────────────────────────────────────────

// Conversation history: vec of (role, content) where role is "user" or "assistant"
static PERFIL_CONV: OnceCell<Mutex<Vec<(String, String)>>> = OnceCell::new();

fn perfil_conv() -> &'static Mutex<Vec<(String, String)>> {
    PERFIL_CONV.get_or_init(|| Mutex::new(vec![]))
}

// Running claude process for the profile chat, kept here so the user can
// interrupt it. Taken (set to None) on natural completion or on interrupt.
static PERFIL_CHILD: OnceCell<Mutex<Option<std::process::Child>>> = OnceCell::new();

fn perfil_child() -> &'static Mutex<Option<std::process::Child>> {
    PERFIL_CHILD.get_or_init(|| Mutex::new(None))
}

fn read_open_pendencias(db_path: &std::path::Path) -> String {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return "(não foi possível ler pendências)".to_string(),
    };
    let mut stmt = match conn.prepare(
        "SELECT p.id, p.categoria, p.descricao, v.titulo, v.empresa \
         FROM pendencias p LEFT JOIN vagas v ON p.vaga_id = v.id \
         WHERE p.resolvida = 0 ORDER BY p.criada_em DESC",
    ) {
        Ok(s) => s,
        Err(_) => return "(sem pendências abertas)".to_string(),
    };
    let rows: Vec<String> = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let cat: String = row.get(1)?;
            let desc: String = row.get(2)?;
            let titulo: Option<String> = row.get(3)?;
            let empresa: Option<String> = row.get(4)?;
            Ok(format!(
                "- ID {id}: [{cat}] {desc} (vaga: {} @ {})",
                titulo.as_deref().unwrap_or("?"),
                empresa.as_deref().unwrap_or("?"),
            ))
        })
        .map(|r| r.filter_map(|x| x.ok()).collect())
        .unwrap_or_default();

    if rows.is_empty() {
        "(sem pendências abertas)".to_string()
    } else {
        rows.join("\n")
    }
}

fn build_system_prompt(app: &AppHandle, conv: &[(String, String)]) -> String {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    let base_yaml = std::fs::read_to_string(data_dir.join("candidate_base.yaml"))
        .unwrap_or_else(|_| "(ainda vazio)".to_string());
    let variants_yaml = std::fs::read_to_string(data_dir.join("search_variants.yaml"))
        .unwrap_or_else(|_| "(ainda vazio)".to_string());
    let data_dir_str = data_dir.to_string_lossy().to_string();
    let db_path = data_dir.join("claudia_rh.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    let pendencias_str = read_open_pendencias(&db_path);

    let mut history = String::new();
    if !conv.is_empty() {
        history.push_str("\n\n## Conversa anterior (contexto)\n");
        for (role, content) in conv {
            let label = if role == "user" { "Utilizador" } else { "Claudia" };
            history.push_str(&format!("\n**{}**: {}\n", label, content));
        }
    }

    let mut prompt = crate::commands::prompts::read_prompt(&data_dir, "perfil")
        .replace("{{DATA_DIR}}", &data_dir_str)
        .replace("{{CANDIDATE_BASE_YAML}}", &base_yaml)
        .replace("{{SEARCH_VARIANTS_YAML}}", &variants_yaml)
        .replace("{{CONVERSA_ANTERIOR}}", &history);

    // Always append pendências block so the model can resolve them when asked,
    // regardless of the on-disk prompt version.
    // NOTE: sqlite3 CLI is NOT installed by default on Windows — use Python instead,
    // which has sqlite3 built-in.
    prompt.push_str(&format!(
        "\n\n## Pendências abertas do sistema\n\n\
         {pendencias_str}\n\n\
         Se o utilizador pedir para marcar uma ou mais pendências como resolvidas \
         (ex: \"marca como ok\", \"já resolvemos o salário\", \"fecha tudo\"), \
         usa Python (tem sqlite3 embutido — o CLI sqlite3 NÃO está instalado no Windows):\n\
         ```python\n\
         import sqlite3\n\
         c = sqlite3.connect(r\"{db_path_str}\")\n\
         # fechar uma pendência específica (substitui ID e MOTIVO):\n\
         c.execute(\"UPDATE pendencias SET resolvida=1, resolvida_em=datetime('now'), resolucao=? WHERE id=?\", [\"MOTIVO\", ID])\n\
         # OU fechar TODAS de uma vez:\n\
         # c.execute(\"UPDATE pendencias SET resolvida=1, resolvida_em=datetime('now'), resolucao='Marcado pelo utilizador' WHERE resolvida=0\")\n\
         c.commit(); c.close()\n\
         ```\n\
         Guarda o script num ficheiro .py temporário e corre com `python` (ou `python3`). \
         Após executar, confirma ao utilizador quais foram fechadas. \
         A interface actualiza-se automaticamente.",
    ));

    prompt
}

fn spawn_perfil_claude(app: AppHandle, message: String) {
    std::thread::spawn(move || {
        let conv = perfil_conv().lock().unwrap().clone();
        let sys = build_system_prompt(&app, &conv);

        let mut child = match Command::new(crate::commands::claude_program())
            .args([
                "--dangerously-skip-permissions",
                "--print",
                "--output-format", "stream-json",
                "--verbose",
                "--include-partial-messages",
                &message,
                "--system-prompt",
                &sys,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("perfil-output", format!("Erro ao iniciar Claude: {e}"));
                let _ = app.emit("perfil-output-done", ());
                return;
            }
        };

        let stdout = child.stdout.take().expect("stdout piped");
        *perfil_child().lock().unwrap() = Some(child);
        let reader = std::io::BufReader::new(stdout);
        let mut full_response = String::new();

        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if line.trim().is_empty() { continue; }

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if val["type"] == "stream_event" {
                    if let Some(text) = val["event"]["delta"]["text"].as_str() {
                        full_response.push_str(text);
                        let emit_text = text.replace("PERFIL_ATUALIZADO", "");
                        if !emit_text.is_empty() {
                            let _ = app.emit("perfil-output", emit_text);
                        }
                    }
                }
                if val["type"] == "result" && val["subtype"] == "success" {
                    if let Some(r) = val["result"].as_str() {
                        full_response = r.to_string();
                    }
                }
            }
        }

        // None here means the user interrupted (interromper_perfil took and killed it).
        if let Some(mut c) = perfil_child().lock().unwrap().take() {
            let _ = c.wait();
        }

        // Notify frontend that the agent may have resolved pendências or updated the DB.
        let _ = app.emit("db-atualizada", ());
        let _ = app.emit("pendencia-resolvida", ());

        if full_response.contains("PERFIL_ATUALIZADO") {
            let _ = app.emit("perfil-atualizado", ());
        }
        let clean = full_response.replace("PERFIL_ATUALIZADO", "").trim().to_string();

        {
            let mut c = perfil_conv().lock().unwrap();
            c.push(("user".to_string(), message));
            c.push(("assistant".to_string(), clean));
            if c.len() > 40 {
                let drain = c.len() - 40;
                c.drain(0..drain);
            }
        }

        let _ = app.emit("perfil-output-done", ());
    });
}

#[tauri::command]
pub fn interromper_perfil() -> Result<(), String> {
    if let Some(mut child) = perfil_child().lock().map_err(|e| e.to_string())?.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

// Drops the last user↔assistant exchange from the in-memory history so the
// user can edit and resend their last message without duplicating context.
#[tauri::command]
pub fn remover_ultima_troca_perfil() -> Result<(), String> {
    let mut c = perfil_conv().lock().map_err(|e| e.to_string())?;
    if let Some(idx) = c.iter().rposition(|(role, _)| role == "user") {
        c.truncate(idx);
    }
    Ok(())
}

#[tauri::command]
pub fn iniciar_sessao_perfil(
    app: AppHandle,
    contexto: String,
    primeira_message: String,
) -> Result<(), String> {
    *perfil_conv().lock().map_err(|e| e.to_string())? = vec![];

    let msg = if !contexto.is_empty() && contexto != "geral" {
        format!("[Foco: {}]\n\n{}", contexto, primeira_message)
    } else {
        primeira_message
    };

    spawn_perfil_claude(app, msg);
    Ok(())
}

#[tauri::command]
pub fn enviar_mensagem_perfil(app: AppHandle, mensagem: String) -> Result<(), String> {
    spawn_perfil_claude(app, mensagem);
    Ok(())
}

// ── Profile Chrome session (--print --chrome) ─────────────────────────────
// Uses the same --print streaming approach as the regular chat but adds
// --chrome so Claude has access to the authenticated browser session.
// This lets it navigate LinkedIn (even with login) and private GitHub repos.

fn build_chrome_system_prompt(app: &AppHandle, conv: &[(String, String)]) -> String {
    let data_dir = app.path().app_data_dir().unwrap_or_default();
    let base_yaml = std::fs::read_to_string(data_dir.join("candidate_base.yaml"))
        .unwrap_or_else(|_| "(ainda vazio)".to_string());
    let variants_yaml = std::fs::read_to_string(data_dir.join("search_variants.yaml"))
        .unwrap_or_else(|_| "(ainda vazio)".to_string());
    let data_dir_str = data_dir.to_string_lossy().to_string();

    let mut history = String::new();
    if !conv.is_empty() {
        history.push_str("\n\n## Conversa anterior (contexto)\n");
        for (role, content) in conv {
            let label = if role == "user" { "Utilizador" } else { "Claudia" };
            history.push_str(&format!("\n**{}**: {}\n", label, content));
        }
    }

    format!(
        r#"És a Claudia, assistente de construção de perfil profissional. Tens acesso ao browser Chrome com a sessão autenticada do utilizador.

Ficheiros alvo:
- `{dir}/candidate_base.yaml` — dados pessoais, experiência, projetos, formação, competências, idiomas, gaps, respostas modelo
- `{dir}/search_variants.yaml` — variantes de busca/CV

## Estado atual do perfil

### candidate_base.yaml
```yaml
{base}
```

### search_variants.yaml
```yaml
{variants}
```

## Processo
O utilizador já indicou quais as plataformas a importar (ver primeira mensagem). Não perguntes URLs — navega directamente:

1. **LinkedIn** (se pedido): abre `https://www.linkedin.com/in/` e a sessão autenticada redireciona para o perfil do utilizador; ou clica no avatar → "Ver o meu perfil". Extrai: nome, localização, headline, experiência, formação, competências, idiomas, links.
2. **GitHub** (se pedido): abre `https://github.com` e clica no avatar → "Your profile". Extrai: nome, bio, localização, repositórios públicos e privados visíveis (nome, descrição, linguagens principais).
3. Combina tudo no YAML e mostra ao utilizador para confirmação.
4. Após confirmação explícita, grava o ficheiro e escreve na linha seguinte exatamente: `PERFIL_ATUALIZADO`
5. Imediatamente após escrever `PERFIL_ATUALIZADO`, fecha os tabs do LinkedIn e/ou GitHub que abriste. Não abras tabs novos depois disso.

## Regras
- Não perguntes URLs — vai directamente às plataformas com a sessão autenticada
- Nunca inventas informação — só incluis o que está explicitamente visível
- Comunicas em português europeu, de forma concisa e direta{history}"#,
        dir = data_dir_str,
        base = base_yaml,
        variants = variants_yaml,
        history = history,
    )
}

fn spawn_chrome_session(app: AppHandle, message: String) {
    std::thread::spawn(move || {
        let conv = perfil_conv().lock().unwrap().clone();
        let sys = build_chrome_system_prompt(&app, &conv);

        let mut child = match Command::new(crate::commands::claude_program())
            .args([
                "--dangerously-skip-permissions",
                "--print",
                "--chrome",
                "--output-format", "stream-json",
                "--verbose",
                "--include-partial-messages",
                &message,
                "--system-prompt",
                &sys,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("perfil-output", format!("Erro ao iniciar sessão Chrome: {e}"));
                let _ = app.emit("perfil-output-done", ());
                return;
            }
        };

        let stdout = child.stdout.take().expect("stdout piped");
        *perfil_child().lock().unwrap() = Some(child);
        let reader = std::io::BufReader::new(stdout);
        let mut full_response = String::new();

        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if line.trim().is_empty() { continue; }

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if val["type"] == "stream_event" {
                    if let Some(text) = val["event"]["delta"]["text"].as_str() {
                        full_response.push_str(text);
                        let emit_text = text.replace("PERFIL_ATUALIZADO", "");
                        if !emit_text.is_empty() {
                            let _ = app.emit("perfil-output", emit_text);
                        }
                    }
                }
                if val["type"] == "result" && val["subtype"] == "success" {
                    if let Some(r) = val["result"].as_str() {
                        full_response = r.to_string();
                    }
                }
            }
        }

        // None here means the user interrupted (interromper_perfil took and killed it).
        if let Some(mut c) = perfil_child().lock().unwrap().take() {
            let _ = c.wait();
        }

        // Notify frontend that the agent may have resolved pendências or updated the DB.
        let _ = app.emit("db-atualizada", ());
        let _ = app.emit("pendencia-resolvida", ());

        if full_response.contains("PERFIL_ATUALIZADO") {
            let _ = app.emit("perfil-atualizado", ());
        }
        let clean = full_response.replace("PERFIL_ATUALIZADO", "").trim().to_string();

        {
            let mut c = perfil_conv().lock().unwrap();
            c.push(("user".to_string(), message));
            c.push(("assistant".to_string(), clean));
            if c.len() > 40 {
                let drain = c.len() - 40;
                c.drain(0..drain);
            }
        }

        let _ = app.emit("perfil-output-done", ());
    });
}

#[tauri::command]
pub fn iniciar_sessao_perfil_chrome(app: AppHandle, primeira_mensagem: String) -> Result<(), String> {
    *perfil_conv().lock().map_err(|e| e.to_string())? = vec![];
    spawn_chrome_session(app, primeira_mensagem);
    Ok(())
}

#[tauri::command]
pub fn escrever_para_perfil_chrome(app: AppHandle, input: String) -> Result<(), String> {
    spawn_chrome_session(app, input);
    Ok(())
}

#[tauri::command]
pub fn guardar_pesos_variantes(app: AppHandle, pesos: std::collections::HashMap<String, f64>) -> Result<(), String> {
    let path = search_variants_path(&app)?;
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    let mut sv: SearchVariants = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
    for v in sv.variantes.iter_mut() {
        if let Some(&peso) = pesos.get(&v.id) {
            v.peso = (peso * 10.0).round() / 10.0; // 1 decimal place
        }
    }
    let out = serde_yaml::to_string(&sv).map_err(|e| e.to_string())?;
    std::fs::write(&path, out).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn guardar_variante_unica(app: AppHandle, variante: SearchVariant) -> Result<(), String> {
    let path = search_variants_path(&app)?;
    let mut sv = match std::fs::read_to_string(&path) {
        Ok(content) => serde_yaml::from_str::<SearchVariants>(&content).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SearchVariants::default(),
        Err(e) => return Err(e.to_string()),
    };
    if let Some(pos) = sv.variantes.iter().position(|v| v.id == variante.id) {
        sv.variantes[pos] = variante;
    } else {
        sv.variantes.push(variante);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_yaml::to_string(&sv).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidato_base_roundtrip() {
        let yaml = r#"
dados_pessoais:
  nome_completo: "João Silva"
  email: "joao@exemplo.com"
  telefone: "+351 912 345 678"
  localizacao_atual: "Copenhagen, Denmark"
  links:
    - tipo: "github"
      url: "https://github.com/joao"
experiencia:
  - empresa: "Acme Corp"
    cargo: "Backend Developer"
    inicio: "2020-01"
    fim: ""
    descricao: "Desenvolvimento de APIs REST"
    conquistas:
      - "Reduzi latência em 40%"
    tecnologias: ["Rust", "Python"]
competencias:
  - "Rust"
  - "Python"
  - "SQL"
idiomas:
  - idioma: "Português"
    nivel: "Nativo"
  - idioma: "Inglês"
    nivel: "C1"
ultima_atualizacao: "2025-01-01"
"#;
        let parsed: CandidatoBase = serde_yaml::from_str(yaml).expect("parse failed");
        assert_eq!(parsed.dados_pessoais.nome_completo, "João Silva");
        assert_eq!(parsed.dados_pessoais.links.len(), 1);
        assert_eq!(parsed.experiencia.len(), 1);
        assert_eq!(parsed.experiencia[0].empresa, "Acme Corp");
        assert_eq!(parsed.competencias, vec!["Rust", "Python", "SQL"]);
        assert_eq!(parsed.idiomas[1].nivel, "C1");

        // Serialize back and re-parse
        let out = serde_yaml::to_string(&parsed).expect("serialize failed");
        let reparsed: CandidatoBase = serde_yaml::from_str(&out).expect("re-parse failed");
        assert_eq!(reparsed.dados_pessoais.nome_completo, "João Silva");
    }

    #[test]
    fn search_variants_roundtrip() {
        let yaml = r#"
variantes:
  - id: "backend"
    nome_exibicao: "Backend"
    peso: 60
    ativa: true
    foco_competencias: ["Rust", "Python"]
    foco_experiencia: []
    regioes_aceitas: ["Dinamarca", "remoto-global"]
    modelos_trabalho: ["remoto", "hibrido"]
    idiomas_aplicacao: ["en", "da"]
    cv_gerado_path: ""
    cv_gerado_em: ""
  - id: "fullstack"
    nome_exibicao: "Full Stack"
    peso: 40
    ativa: true
    foco_competencias: []
    foco_experiencia: []
    regioes_aceitas: []
    modelos_trabalho: ["hibrido"]
    idiomas_aplicacao: ["en"]
    cv_gerado_path: ""
    cv_gerado_em: ""
preferencias_globais:
  faixa_salarial:
    minimo: 45000
    moeda: "DKK"
    flexivel: false
  setores_evitar: []
  empresas_evitar: []
red_lines:
  - "pedido de salário fora da faixa"
perguntas_pendentes: []
"#;
        let parsed: SearchVariants = serde_yaml::from_str(yaml).expect("parse failed");
        assert_eq!(parsed.variantes.len(), 2);
        assert_eq!(parsed.variantes[0].id, "backend");
        assert_eq!(parsed.variantes[0].peso, 60.0);
        assert!(parsed.variantes[0].ativa);
        assert_eq!(parsed.variantes[1].nome_exibicao, "Full Stack");
        assert_eq!(parsed.preferencias_globais.faixa_salarial.minimo, Some(45000.0));
        assert_eq!(parsed.preferencias_globais.faixa_salarial.moeda, "DKK");
        assert_eq!(parsed.red_lines.len(), 1);

        // Serialize and re-parse
        let out = serde_yaml::to_string(&parsed).expect("serialize failed");
        let reparsed: SearchVariants = serde_yaml::from_str(&out).expect("re-parse failed");
        assert_eq!(reparsed.variantes[0].peso, 60.0);
    }

    #[test]
    fn candidato_base_defaults_on_empty() {
        let yaml = "{}";
        let parsed: CandidatoBase = serde_yaml::from_str(yaml).expect("parse failed");
        assert_eq!(parsed.dados_pessoais.nome_completo, "");
        assert!(parsed.experiencia.is_empty());
        assert!(parsed.competencias.is_empty());
    }

    #[test]
    fn search_variants_empty_file() {
        // Simulates what ler_search_variants does when file is empty/default
        let yaml = "{}";
        let sv: SearchVariants = serde_yaml::from_str(yaml).expect("parse failed");
        assert!(sv.variantes.is_empty());
        assert!(sv.red_lines.is_empty());
    }
}
