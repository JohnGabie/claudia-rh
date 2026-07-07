use once_cell::sync::OnceCell;
use portable_pty::{native_pty_system, CommandBuilder, Child, MasterPty, PtySize};
use rusqlite::Connection;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
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
pub fn iniciar_claude(
    app: AppHandle,
    cmd: String,
    args: Vec<String>,
    rows: u16,
    cols: u16,
    session_id: i64,
    db: Arc<Mutex<Connection>>,
    cwd: String,
    initial_task: String,
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

    // Send the initial task after a fixed delay. Pattern detection on raw PTY
    // output is unreliable because ANSI escape codes are interspersed between
    // characters, breaking any simple contains() check.
    let writer_for_timer = Arc::clone(&writer);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        // Type text into the readline buffer first, flush it...
        if let Ok(mut w) = writer_for_timer.lock() {
            let _ = w.write_all(initial_task.as_bytes());
            let _ = w.flush();
        }
        // ...then send Enter as a separate write after readline has processed
        // all typed characters. Sending text+\r in one atomic write causes \r
        // to arrive before readline finishes buffering the text, breaking the line.
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(mut w) = writer_for_timer.lock() {
            let _ = w.write_all(b"\r");
            let _ = w.flush();
        }
    });

    let app_thread = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut line_buf = String::new();
        let mut checkpoint_requested = false;
        let mut reconnect_attempts: u32 = 0;

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = app_thread.emit("pty-output", chunk.clone());

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
                        break;
                    }

                    let chrome_error = line_buf.contains("Browser extension is not connected")
                        || line_buf.contains("Receiving end does not exist");

                    if chrome_error {
                        // Clear so we don't re-trigger on the same text
                        line_buf.clear();
                        if reconnect_attempts < 3 {
                            reconnect_attempts += 1;
                            if let Ok(mut w) = writer_for_thread.lock() {
                                let _ = w.write_all(b"/chrome\r");
                                let _ = w.flush();
                            }
                        } else if reconnect_attempts == 3 {
                            reconnect_attempts += 1; // prevent repeat
                            let _ = app_thread.emit("chrome-reconnect-failed", ());
                        }
                    }
                }
                Err(_) => break,
            }
        }

        let motivo = if checkpoint_requested { "checkpoint" } else { "saiu" };

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
