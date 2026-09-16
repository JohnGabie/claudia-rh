use crate::diagnostics;
use once_cell::sync::OnceCell;
use portable_pty::{native_pty_system, CommandBuilder, Child, MasterPty, PtySize};
use rusqlite::Connection;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

struct PtyState {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    _child: Box<dyn Child + Send + Sync>,
    session_id: Option<i64>,
}

static PTY: OnceCell<Mutex<Option<PtyState>>> = OnceCell::new();

fn pty_cell() -> &'static Mutex<Option<PtyState>> {
    PTY.get_or_init(|| Mutex::new(None))
}

/// Strip CSI/OSC ANSI so `contains` works on PTY output (escapes sit between letters).
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                for d in chars.by_ref() {
                    if d == '\u{7}' || d == '\u{9c}' {
                        break;
                    }
                    if d == '\u{1b}' {
                        let _ = chars.next(); // swallow trailing `\` of ST
                        break;
                    }
                }
            }
            Some(_) => {
                let _ = chars.next();
            }
            None => {}
        }
    }
    out
}

fn is_api_stall(haystack: &str) -> bool {
    let plain = strip_ansi(haystack);
    plain.contains("Waiting for API response")
        || plain.contains("stalled mid-stream")
}

fn send_continue(writer: &Arc<Mutex<Box<dyn Write + Send>>>) {
    if let Ok(mut w) = writer.lock() {
        let _ = w.write_all(b"continue");
        let _ = w.flush();
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    if let Ok(mut w) = writer.lock() {
        let _ = w.write_all(b"\r");
        let _ = w.flush();
    }
}

/// Spawn a generic PTY process, streaming output to the frontend via `pty-output` events.
pub fn iniciar(
    app: AppHandle,
    cmd: String,
    args: Vec<String>,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    *pty_cell().lock().unwrap() = None;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let mut builder = CommandBuilder::new(&cmd);
    for arg in &args {
        builder.arg(arg);
    }

    let child = pair.slave.spawn_command(builder).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let raw_writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let writer = Arc::new(Mutex::new(raw_writer as Box<dyn Write + Send>));

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = app.emit("pty-output", chunk.clone());
                    if chunk.contains("PERFIL_ATUALIZADO") {
                        let _ = app.emit("perfil-atualizado", ());
                    }
                }
                Err(_) => break,
            }
        }
    });

    *pty_cell().lock().unwrap() = Some(PtyState {
        master: pair.master,
        writer,
        _child: child,
        session_id: None,
    });
    Ok(())
}

/// Spawn the `claude` process for an execution session, with:
/// - SESSION_CHECKPOINT_REQUESTED detection → clean restart via frontend event
/// - Chrome extension disconnection detection → automatic /chrome reconnect attempt
/// - Session end → updates `sessoes.terminada_em` in the database
// TODO(refactor): group these args into a session config struct
#[allow(clippy::too_many_arguments)]
pub fn iniciar_claude(
    app: AppHandle,
    cmd: String,
    args: Vec<String>,
    rows: u16,
    cols: u16,
    session_id: i64,
    db: Arc<Mutex<Connection>>,
    cwd: String,
) -> Result<(), String> {
    *pty_cell().lock().unwrap() = None;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let mut builder = CommandBuilder::new(&cmd);
    for arg in &args {
        builder.arg(arg);
    }
    // CWD is a git-initialized directory: Claude Code skips the "trust this
    // folder?" prompt automatically inside git repositories.
    builder.cwd(&cwd);

    let child = pair.slave.spawn_command(builder).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let raw_writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let writer = Arc::new(Mutex::new(raw_writer as Box<dyn Write + Send>));
    let writer_for_thread = Arc::clone(&writer);

    let app_thread = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut line_buf = String::new();
        let mut checkpoint_requested = false;
        let mut reconnect_attempts: u32 = 0;
        let mut api_continue_attempts: u32 = 0;
        let mut last_api_continue: Option<Instant> = None;

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = app_thread.emit("pty-output", chunk.clone());
                    diagnostics::pty_push(&chunk);
                    diagnostics::watchdog::tick();

                    line_buf.push_str(&chunk);
                    // Keep line_buf bounded. Raw byte offset may land inside a
                    // multi-byte UTF-8 char, so advance to the next char boundary.
                    if line_buf.len() > 16384 {
                        let start = line_buf.len().saturating_sub(8192);
                        let start = (start..=line_buf.len())
                            .find(|&i| line_buf.is_char_boundary(i))
                            .unwrap_or(line_buf.len());
                        line_buf = line_buf[start..].to_string();
                    }

                    if line_buf.contains("SESSION_CHECKPOINT_REQUESTED") {
                        checkpoint_requested = true;
                        // Kill the current PTY (only if still ours)
                        let mut guard = pty_cell().lock().unwrap();
                        if guard.as_ref().and_then(|s| s.session_id) == Some(session_id) {
                            *guard = None;
                        }
                        drop(guard);
                        diagnostics::pty_flush();
                        break;
                    }

                    let chrome_error = line_buf.contains("Browser extension is not connected")
                        || line_buf.contains("Receiving end does not exist");

                    if chrome_error {
                        // Clear so we don't re-trigger on the same text
                        line_buf.clear();
                        if reconnect_attempts < 3 {
                            reconnect_attempts += 1;
                            diagnostics::emit_event(
                                "pty",
                                "chrome_reconnect",
                                Some(session_id),
                                "",
                            );
                            if let Ok(mut w) = writer_for_thread.lock() {
                                let _ = w.write_all(b"/chrome\r");
                                let _ = w.flush();
                            }
                        } else if reconnect_attempts == 3 {
                            reconnect_attempts += 1; // prevent repeat
                            let _ = app_thread.emit("chrome-reconnect-failed", ());
                            diagnostics::emit_event(
                                "pty",
                                "chrome_failed",
                                Some(session_id),
                                "",
                            );
                        }
                    }

                    // Network retry / stalled stream: skip the 2m wait and resume.
                    // Same "continue" the dashboard Resume button already sends.
                    if is_api_stall(&line_buf) {
                        line_buf.clear();
                        let cooled_down = last_api_continue
                            .map(|t| t.elapsed() >= Duration::from_secs(20))
                            .unwrap_or(true);
                        if cooled_down && api_continue_attempts < 5 {
                            api_continue_attempts += 1;
                            last_api_continue = Some(Instant::now());
                            let _ = app_thread.emit(
                                "pty-output",
                                format!(
                                    "\r\n\x1b[1;33m[Claudia RH]\x1b[0m API stall — sending continue ({api_continue_attempts}/5)\r\n"
                                ),
                            );
                            send_continue(&writer_for_thread);
                        } else if cooled_down && api_continue_attempts == 5 {
                            api_continue_attempts += 1;
                            let _ = app_thread.emit("api-continue-exhausted", ());
                        }
                    }
                }
                Err(_) => break,
            }
        }

        diagnostics::pty_flush();

        let motivo = if checkpoint_requested { "checkpoint" } else { "saiu" };
        diagnostics::emit_event("session", "ended", Some(session_id), motivo);

        // Update sessoes row, being careful not to overwrite a newer session
        if let Ok(conn) = db.lock() {
            let _ = conn.execute(
                "UPDATE sessoes SET terminada_em = datetime('now'), motivo_termino = ?1 WHERE id = ?2",
                rusqlite::params![motivo, session_id],
            );
        }

        // Clean up PTY cell if still pointing to our session
        {
            let mut guard = pty_cell().lock().unwrap();
            if guard.as_ref().and_then(|s| s.session_id) == Some(session_id) {
                *guard = None;
            }
        }

        if checkpoint_requested {
            let _ = app_thread.emit("session-checkpoint-requested", ());
        }
        let _ = app_thread.emit("session-ended", motivo.to_string());
    });

    *pty_cell().lock().unwrap() = Some(PtyState {
        master: pair.master,
        writer,
        _child: child,
        session_id: Some(session_id),
    });
    Ok(())
}

pub fn escrever(input: String) -> Result<(), String> {
    let writer_arc = {
        let guard = pty_cell().lock().unwrap();
        guard
            .as_ref()
            .map(|s| Arc::clone(&s.writer))
            .ok_or_else(|| "PTY not initialized".to_string())?
    };
    let mut w = writer_arc.lock().map_err(|e| e.to_string())?;
    w.write_all(input.as_bytes()).map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())
}

pub fn redimensionar(rows: u16, cols: u16) -> Result<(), String> {
    let guard = pty_cell().lock().unwrap();
    if let Some(state) = guard.as_ref() {
        state
            .master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())
    } else {
        Err("PTY not initialized".into())
    }
}

pub fn parar() {
    *pty_cell().lock().unwrap() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_joins_letters_split_by_csi() {
        let raw = "W\u{1b}[0maiting for API response";
        assert_eq!(strip_ansi(raw), "Waiting for API response");
    }

    #[test]
    fn detects_waiting_retry_status() {
        let msg = "Waiting for API response · will retry in 2m 33s · check your network";
        assert!(is_api_stall(msg));
        assert!(is_api_stall(&format!("\u{1b}[2m{msg}\u{1b}[0m")));
    }

    #[test]
    fn detects_stalled_mid_stream() {
        let msg = "API Error: Response stalled mid-stream. The response above may be incomplete";
        assert!(is_api_stall(msg));
        assert!(!is_api_stall("SESSION_CHECKPOINT_REQUESTED"));
        assert!(!is_api_stall("Browser extension is not connected"));
    }
}
