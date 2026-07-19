// Embedded MCP server (see .claude/MCP-DESIGN.md).
//
// The same claudia-rh binary runs in three modes:
//   (default)                          → Tauri GUI
//   --mcp-serve --data-dir <dir> [...] → MCP stdio server, spawned by claude CLI
//   --mcp-call <tool> <json> [...]     → dev-only: invoke one tool directly
//
// All tool traffic (server or --mcp-call) goes through `dispatch`, which owns
// the debug log and the push notification to the running GUI.

pub mod tools;
mod server;

use std::io::Write;
use std::path::PathBuf;

pub struct McpConfig {
    pub data_dir: PathBuf,
    pub notify_port: Option<u16>,
    pub debug: bool,
}

/// Single entry point for every tool invocation. Owns cross-cutting concerns:
/// debug logging and the UI push notification.
pub fn dispatch(cfg: &McpConfig, tool: &str, args: &serde_json::Value) -> Result<String, String> {
    let result = match tool {
        "update_profile" => {
            let yaml = args["yaml"].as_str().ok_or("parâmetro 'yaml' em falta")?;
            tools::update_profile(&cfg.data_dir, yaml).inspect(|_| notify(cfg, "perfil"))
        }
        "close_pendencia" => {
            let id = args["id"].as_i64().ok_or("parâmetro 'id' em falta")?;
            let resolucao = args["resolucao"].as_str().unwrap_or("Resolvida via chat");
            tools::close_pendencia(&cfg.data_dir, id, resolucao).inspect(|_| notify(cfg, "db"))
        }
        "close_pendencias_vaga" => {
            let vaga_id = args["vaga_id"].as_i64().ok_or("parâmetro 'vaga_id' em falta")?;
            let resolucao = args["resolucao"].as_str().unwrap_or("Resolvida pelo agente");
            tools::close_pendencias_vaga(&cfg.data_dir, vaga_id, resolucao)
                .inspect(|_| notify(cfg, "db"))
        }
        "get_pendencia_vaga" => {
            let vaga_id = args["vaga_id"].as_i64().ok_or("parâmetro 'vaga_id' em falta")?;
            tools::get_pendencia_vaga(&cfg.data_dir, vaga_id)
        }
        "list_pendencias" => tools::list_pendencias(&cfg.data_dir),
        "register_vaga" => {
            let titulo = args["titulo"].as_str().ok_or("parâmetro 'titulo' em falta")?;
            let empresa = args["empresa"].as_str().ok_or("parâmetro 'empresa' em falta")?;
            let plataforma = args["plataforma"].as_str().unwrap_or("desconhecida");
            let url = args["url"].as_str().ok_or("parâmetro 'url' em falta")?;
            tools::register_vaga(
                &cfg.data_dir,
                titulo,
                empresa,
                plataforma,
                url,
                args["localizacao"].as_str(),
                args["modelo_trabalho"].as_str(),
                args["fonte_conexao"].as_str(),
            )
            .inspect(|_| notify(cfg, "db"))
        }
        "update_vaga_status" => {
            let vaga_id = args["vaga_id"].as_i64().ok_or("parâmetro 'vaga_id' em falta")?;
            let status = args["status"].as_str().ok_or("parâmetro 'status' em falta")?;
            tools::update_vaga_status(&cfg.data_dir, vaga_id, status, args["detalhe"].as_str())
                .inspect(|_| notify(cfg, "db"))
        }
        "register_candidatura" => {
            let vaga_id = args["vaga_id"].as_i64().ok_or("parâmetro 'vaga_id' em falta")?;
            let pasta = args["pasta_arquivos"].as_str().ok_or("parâmetro 'pasta_arquivos' em falta")?;
            let metodo = args["metodo"].as_str().unwrap_or("formulario");
            tools::register_candidatura(&cfg.data_dir, vaga_id, pasta, metodo)
                .inspect(|_| notify(cfg, "db"))
        }
        "create_pendencia" => {
            let vaga_id = args["vaga_id"].as_i64().ok_or("parâmetro 'vaga_id' em falta")?;
            let categoria = args["categoria"].as_str().ok_or("parâmetro 'categoria' em falta")?;
            let descricao = args["descricao"].as_str().ok_or("parâmetro 'descricao' em falta")?;
            tools::create_pendencia(&cfg.data_dir, vaga_id, categoria, descricao)
                .inspect(|_| notify(cfg, "db"))
        }
        other => Err(format!("ferramenta desconhecida: {other}")),
    };

    debug_log(cfg, tool, args, &result);
    result
}

/// Push notification to the GUI: connect to the localhost port the app opened
/// and send one line. Best-effort — the db_watcher poll is the safety net.
fn notify(cfg: &McpConfig, kind: &str) {
    if let Some(port) = cfg.notify_port {
        let _ = std::net::TcpStream::connect(("127.0.0.1", port))
            .and_then(|mut s| writeln!(s, "{kind}"));
    }
}

/// Dev-only tool-call log (--debug): one line per call in mcp-debug.log.
fn debug_log(cfg: &McpConfig, tool: &str, args: &serde_json::Value, result: &Result<String, String>) {
    if !cfg.debug {
        return;
    }
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = match result {
        Ok(ok) => format!("[{ts}] CALL {tool} args={args} → OK: {ok}\n"),
        Err(e) => format!("[{ts}] CALL {tool} args={args} → ERROR: {e}\n"),
    };
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cfg.data_dir.join("mcp-debug.log"))
        .and_then(|mut f| f.write_all(line.as_bytes()));
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

/// Called first thing from main(). Returns true when the process ran in an
/// MCP mode and the GUI must NOT start.
pub fn cli_main() -> bool {
    let args: Vec<String> = std::env::args().collect();
    let serve = args.iter().any(|a| a == "--mcp-serve");
    let call_at = args.iter().position(|a| a == "--mcp-call");
    if !serve && call_at.is_none() {
        return false;
    }

    let data_dir = parse_flag_value(&args, "--data-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let notify_port = parse_flag_value(&args, "--notify-port").and_then(|p| p.parse().ok());
    let cfg = McpConfig {
        data_dir,
        notify_port,
        debug: args.iter().any(|a| a == "--debug"),
    };

    if serve {
        if let Err(e) = server::run(cfg) {
            eprintln!("mcp-serve error: {e}");
            std::process::exit(1);
        }
        return true;
    }

    // --mcp-call <tool> '<json>' — dev shortcut, no MCP protocol in the loop.
    let i = call_at.unwrap();
    let tool = args.get(i + 1).cloned().unwrap_or_default();
    let json = args.get(i + 2).cloned().unwrap_or_else(|| "{}".to_string());
    let parsed: serde_json::Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("invalid JSON args: {e}");
            std::process::exit(2);
        }
    };
    match dispatch(&cfg, &tool, &parsed) {
        Ok(out) => println!("{out}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    true
}
