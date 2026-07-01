use crate::DbState;
use rusqlite::Connection;
use serde::Serialize;
use std::io::BufRead;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DadosPorDia {
    pub data: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ParContagem {
    pub chave: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct MotivoPulado {
    pub categoria: String,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct PendenciaCategoria {
    pub categoria: String,
    pub total: i64,
    pub resolvidas: i64,
}

#[derive(Debug, Serialize)]
pub struct AgregadosFeedback {
    pub candidaturas_total: i64,
    pub candidaturas_semana: i64,
    pub candidaturas_por_dia: Vec<DadosPorDia>,
    pub por_variante: Vec<ParContagem>,
    pub por_resultado: Vec<ParContagem>,
    pub vagas_analisadas: i64,
    pub vagas_puladas: i64,
    pub vagas_pendentes: i64,
    pub dias_desde_ultimo_feedback: Option<i64>,
    pub ultimo_feedback_resumo: Option<String>,
    pub motivos_puladas: Vec<MotivoPulado>,
    pub pendencias_por_categoria: Vec<PendenciaCategoria>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RegistoFeedback {
    pub id: i64,
    pub gerado_em: String,
    pub gatilho: String,
    pub resumo: String,
    pub conteudo_completo: String,
    pub candidaturas_ate_aqui: i64,
}

#[derive(Debug, Serialize)]
pub struct SugestaoFeedback {
    pub sugerir: bool,
    pub motivo: String,
}

// ── Aggregation helpers ───────────────────────────────────────────────────

fn agregar(conn: &Connection) -> Result<AgregadosFeedback, rusqlite::Error> {
    let candidaturas_total: i64 = conn
        .query_row("SELECT COUNT(*) FROM candidaturas", [], |r| r.get(0))
        .unwrap_or(0);

    let candidaturas_semana: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM candidaturas WHERE enviada_em >= datetime('now', '-7 days')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let vagas_analisadas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vagas WHERE status IN ('analisada','candidatando','aplicada','pulada','pendente_revisao')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let vagas_puladas: i64 = conn
        .query_row("SELECT COUNT(*) FROM vagas WHERE status='pulada'", [], |r| r.get(0))
        .unwrap_or(0);

    let vagas_pendentes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vagas WHERE status='pendente_revisao'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Skip reasons grouped by category prefix (format: "categoria: motivo")
    let motivos_puladas = {
        let mut stmt = conn.prepare(
            "SELECT \
              CASE WHEN INSTR(COALESCE(motivo_status,''), ':') > 0 \
                   THEN TRIM(SUBSTR(motivo_status, 1, INSTR(motivo_status, ':')-1)) \
                   ELSE 'outro' END as cat, \
              COUNT(*) as total \
             FROM vagas WHERE status='pulada' \
             GROUP BY cat ORDER BY total DESC LIMIT 15",
        )?;
        let rows = stmt
            .query_map([], |r| Ok(MotivoPulado { categoria: r.get(0)?, total: r.get(1)? }))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // Pending issues grouped by category
    let pendencias_por_categoria = {
        let mut stmt = conn.prepare(
            "SELECT categoria, COUNT(*) as total, \
              SUM(CASE WHEN resolvida=1 THEN 1 ELSE 0 END) as resolvidas \
             FROM pendencias GROUP BY categoria ORDER BY total DESC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok(PendenciaCategoria {
                categoria: r.get(0)?,
                total: r.get(1)?,
                resolvidas: r.get(2)?,
            }))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // Last 30 days trend
    let candidaturas_por_dia = {
        let mut stmt = conn.prepare(
            "SELECT date(enviada_em), COUNT(*) \
             FROM candidaturas \
             WHERE enviada_em >= datetime('now', '-30 days') \
             GROUP BY date(enviada_em) \
             ORDER BY date(enviada_em)",
        )?;
        let rows = stmt
            .query_map([], |r| Ok(DadosPorDia { data: r.get(0)?, count: r.get(1)? }))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // By variant
    let por_variante = {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(v.variante_id, 'sem variante'), COUNT(c.id) \
             FROM candidaturas c JOIN vagas v ON c.vaga_id = v.id \
             GROUP BY v.variante_id \
             ORDER BY COUNT(c.id) DESC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok(ParContagem { chave: r.get(0)?, count: r.get(1)? }))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // By resultado
    let por_resultado = {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(resultado, 'sem_resposta'), COUNT(*) \
             FROM candidaturas \
             GROUP BY resultado \
             ORDER BY COUNT(*) DESC",
        )?;
        let rows = stmt
            .query_map([], |r| Ok(ParContagem { chave: r.get(0)?, count: r.get(1)? }))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // Last feedback
    let ultimo = conn.query_row(
        "SELECT resumo, CAST(julianday('now') - julianday(gerado_em) AS INTEGER) \
         FROM feedbacks ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
    );

    let (ultimo_feedback_resumo, dias_desde_ultimo_feedback) = match ultimo {
        Ok((resumo, dias)) => (Some(resumo), Some(dias)),
        Err(_) => (None, None),
    };

    Ok(AgregadosFeedback {
        candidaturas_total,
        candidaturas_semana,
        candidaturas_por_dia,
        por_variante,
        por_resultado,
        vagas_analisadas,
        vagas_puladas,
        vagas_pendentes,
        dias_desde_ultimo_feedback,
        ultimo_feedback_resumo,
        motivos_puladas,
        pendencias_por_categoria,
    })
}

fn formatar_prompt(dados: &AgregadosFeedback) -> String {
    let mut s = String::new();
    s.push_str("## Dados agregados das candidaturas\n\n");
    s.push_str(&format!("**Total de candidaturas enviadas:** {}\n", dados.candidaturas_total));
    s.push_str(&format!("**Candidaturas nos últimos 7 dias:** {}\n", dados.candidaturas_semana));
    if let Some(d) = dados.dias_desde_ultimo_feedback {
        s.push_str(&format!("**Dias desde o último feedback:** {}\n", d));
    } else {
        s.push_str("**Feedback anterior:** nenhum (primeira análise)\n");
    }
    s.push('\n');

    s.push_str(&format!(
        "**Vagas analisadas:** {}  |  **Puladas:** {}  |  **Pendentes:** {}\n\n",
        dados.vagas_analisadas, dados.vagas_puladas, dados.vagas_pendentes
    ));

    if !dados.por_resultado.is_empty() {
        s.push_str("**Resultados conhecidos (marcados manualmente):**\n");
        for p in &dados.por_resultado {
            s.push_str(&format!("- {}: {}\n", p.chave, p.count));
        }
        s.push('\n');
    }

    if dados.por_variante.len() > 1 {
        s.push_str("**Candidaturas por variante de busca:**\n");
        for p in &dados.por_variante {
            s.push_str(&format!("- {}: {} candidaturas\n", p.chave, p.count));
        }
        s.push('\n');
    }

    if !dados.candidaturas_por_dia.is_empty() {
        s.push_str("**Tendência (últimos 30 dias):**\n");
        for d in &dados.candidaturas_por_dia {
            s.push_str(&format!("- {}: {}\n", d.data, d.count));
        }
        s.push('\n');
    }

    if !dados.motivos_puladas.is_empty() {
        s.push_str("**Motivos de exclusão das vagas puladas:**\n");
        for m in &dados.motivos_puladas {
            s.push_str(&format!("- {}: {} vagas\n", m.categoria, m.total));
        }
        s.push('\n');
    }

    if !dados.pendencias_por_categoria.is_empty() {
        s.push_str("**Pendências por categoria:**\n");
        for p in &dados.pendencias_por_categoria {
            s.push_str(&format!("- {}: {} total ({} resolvidas)\n", p.categoria, p.total, p.resolvidas));
        }
        s.push('\n');
    }

    s.push_str("\nGera o feedback estruturado com base nestes dados.");
    s
}


fn extrair_resumo(texto: &str) -> String {
    let linhas: Vec<&str> = texto
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .take(3)
        .collect();
    let resumo = linhas.join(" ");
    if resumo.chars().count() > 200 {
        let end = resumo
            .char_indices()
            .map(|(i, _)| i)
            .nth(197)
            .unwrap_or(resumo.len());
        format!("{}…", &resumo[..end])
    } else {
        resumo
    }
}

fn spawn_feedback_claude(app: AppHandle, db: Arc<Mutex<Connection>>, gatilho: String) {
    std::thread::spawn(move || {
        let dados = {
            let conn = match db.lock() {
                Ok(c) => c,
                Err(_) => {
                    let _ = app.emit("feedback-output-done", "Erro: não foi possível aceder à base de dados.");
                    return;
                }
            };
            match agregar(&conn) {
                Ok(d) => d,
                Err(e) => {
                    let _ = app.emit("feedback-output-done", format!("Erro ao agregar dados: {e}"));
                    return;
                }
            }
        };

        let mensagem = formatar_prompt(&dados);
        let candidaturas_total = dados.candidaturas_total;
        let data_dir = app.path().app_data_dir().unwrap_or_default();
        let system_prompt = crate::commands::prompts::read_prompt(&data_dir, "feedback");

        let mut child = match std::process::Command::new(crate::commands::claude_program())
            .args([
                "--dangerously-skip-permissions",
                "--print",
                "--output-format", "stream-json",
                "--verbose",
                "--include-partial-messages",
                &mensagem,
                "--system-prompt",
                &system_prompt,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("feedback-output", format!("Erro ao invocar Claude: {e}"));
                let _ = app.emit("feedback-output-done", ());
                return;
            }
        };

        let stdout = child.stdout.take().expect("stdout piped");
        let reader = std::io::BufReader::new(stdout);
        let mut conteudo = String::new();

        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if line.trim().is_empty() { continue; }

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                if val["type"] == "stream_event" {
                    if let Some(text) = val["event"]["delta"]["text"].as_str() {
                        conteudo.push_str(text);
                        let _ = app.emit("feedback-output", text.to_string());
                    }
                }
                if val["type"] == "result" && val["subtype"] == "success" {
                    if let Some(r) = val["result"].as_str() {
                        conteudo = r.to_string();
                    }
                }
            }
        }

        let _ = child.wait();

        let conteudo = conteudo.trim().to_string();
        let resumo = extrair_resumo(&conteudo);

        {
            if let Ok(conn) = db.lock() {
                let _ = conn.execute(
                    "INSERT INTO feedbacks (gerado_em, gatilho, resumo, conteudo_completo, candidaturas_ate_aqui) \
                     VALUES (datetime('now'), ?1, ?2, ?3, ?4)",
                    rusqlite::params![gatilho, resumo, conteudo, candidaturas_total],
                );
            }
        }

        let _ = app.emit("feedback-output-done", ());
    });
}

// ── Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn agregar_dados_feedback(state: State<'_, DbState>) -> Result<AgregadosFeedback, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    agregar(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn gerar_feedback(
    app: AppHandle,
    state: State<'_, DbState>,
    gatilho: String,
) -> Result<(), String> {
    let db = Arc::clone(&state.0);
    spawn_feedback_claude(app, db, gatilho);
    Ok(())
}

#[tauri::command]
pub fn listar_feedbacks(state: State<'_, DbState>) -> Result<Vec<RegistoFeedback>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, gerado_em, gatilho, resumo, conteudo_completo, candidaturas_ate_aqui \
             FROM feedbacks ORDER BY id DESC LIMIT 50",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(RegistoFeedback {
                id: r.get(0)?,
                gerado_em: r.get(1)?,
                gatilho: r.get(2)?,
                resumo: r.get(3)?,
                conteudo_completo: r.get(4)?,
                candidaturas_ate_aqui: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

#[tauri::command]
pub fn marcar_resultado_candidatura(
    state: State<'_, DbState>,
    id: i64,
    resultado: Option<String>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE candidaturas SET resultado = ?1 WHERE id = ?2",
        rusqlite::params![resultado, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn sugerir_feedback(state: State<'_, DbState>) -> Result<SugestaoFeedback, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM candidaturas", [], |r| r.get(0))
        .unwrap_or(0);

    if total == 0 {
        return Ok(SugestaoFeedback { sugerir: false, motivo: String::new() });
    }

    let ultimo = conn.query_row(
        "SELECT candidaturas_ate_aqui, CAST(julianday('now') - julianday(gerado_em) AS INTEGER) \
         FROM feedbacks ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    );

    match ultimo {
        Err(_) => {
            // No feedback yet
            if total >= 5 {
                Ok(SugestaoFeedback {
                    sugerir: true,
                    motivo: format!("{} candidaturas enviadas — considera gerar o primeiro feedback.", total),
                })
            } else {
                Ok(SugestaoFeedback { sugerir: false, motivo: String::new() })
            }
        }
        Ok((cands_na_altura, dias)) => {
            let novas = total - cands_na_altura;
            if novas >= 10 {
                Ok(SugestaoFeedback {
                    sugerir: true,
                    motivo: format!("{} novas candidaturas desde o último feedback.", novas),
                })
            } else if dias >= 14 {
                Ok(SugestaoFeedback {
                    sugerir: true,
                    motivo: format!("{} dias desde o último feedback.", dias),
                })
            } else {
                Ok(SugestaoFeedback { sugerir: false, motivo: String::new() })
            }
        }
    }
}
