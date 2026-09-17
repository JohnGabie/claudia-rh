pub mod ansi;
pub mod redact;

pub use ansi::strip_ansi;
pub use redact::redact;

use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct DiagPaths {
    pub dir: PathBuf,
    pub debug_dir: Option<PathBuf>,
}

pub static PATHS: once_cell::sync::OnceCell<DiagPaths> = once_cell::sync::OnceCell::new();

const PTY_RING_MAX: usize = 100 * 1024;
const PTY_FLUSH_EVERY: Duration = Duration::from_secs(2);

struct PtyRing {
    buf: String,
    max: usize,
}

impl PtyRing {
    fn new() -> Self {
        Self {
            buf: String::new(),
            max: PTY_RING_MAX,
        }
    }

    fn push(&mut self, s: &str) {
        self.append(s);
    }

    fn append(&mut self, s: &str) {
        self.buf.push_str(s);
        if self.buf.len() > self.max {
            let mut start = self.buf.len() - self.max;
            while start < self.buf.len() && !self.buf.is_char_boundary(start) {
                start += 1;
            }
            self.buf.replace_range(..start, "");
        }
    }

    fn snapshot(&self) -> String {
        self.buf.clone()
    }
}

static PTY_RING: Mutex<PtyRing> = Mutex::new(PtyRing {
    buf: String::new(),
    max: PTY_RING_MAX,
});
static LAST_FLUSH: Mutex<Option<Instant>> = Mutex::new(None);

pub fn init_paths(app_data_dir: &Path) -> DiagPaths {
    let dir = app_data_dir.join("diagnostics");
    std::fs::create_dir_all(&dir).ok();
    let debug_dir = if cfg!(debug_assertions) {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("debug-logs");
        std::fs::create_dir_all(&p).ok();
        Some(p)
    } else {
        None
    };
    let paths = DiagPaths { dir, debug_dir };
    let _ = PATHS.set(paths.clone());
    paths
}

#[cfg(test)]
pub fn init_paths_for_test(dir: PathBuf) -> DiagPaths {
    std::fs::create_dir_all(&dir).ok();
    let p = DiagPaths {
        dir: dir.clone(),
        debug_dir: None,
    };
    let _ = PATHS.set(p.clone());
    p
}

#[derive(Serialize)]
struct EventLine<'a> {
    ts: String,
    lvl: &'static str,
    cat: &'a str,
    evt: &'a str,
    sid: Option<i64>,
    msg: String,
}

pub fn emit_event_to(
    dir: &Path,
    debug: Option<&Path>,
    cat: &str,
    evt: &str,
    sid: Option<i64>,
    msg: &str,
) {
    let line = EventLine {
        ts: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        lvl: "info",
        cat,
        evt,
        sid,
        msg: redact(msg),
    };
    let mut json = match serde_json::to_string(&line) {
        Ok(s) => s,
        Err(_) => return,
    };
    json.push('\n');
    append_file(dir, "events.jsonl", &json);
    if let Some(debug) = debug {
        append_file(debug, "events.jsonl", &json);
    }
}

pub fn emit_event(cat: &str, evt: &str, sid: Option<i64>, msg: &str) {
    let Some(p) = PATHS.get() else { return };
    emit_event_to(&p.dir, p.debug_dir.as_deref(), cat, evt, sid, msg);
}

fn append_file(root: &Path, name: &str, append_line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(name))
    {
        let _ = f.write_all(append_line.as_bytes());
    }
}

fn write_both(name: &str, append_line: &str) {
    let Some(p) = PATHS.get() else { return };
    for root in std::iter::once(&p.dir).chain(p.debug_dir.as_ref()) {
        append_file(root, name, append_line);
    }
}

fn overwrite_both(name: &str, contents: &str) {
    let Some(p) = PATHS.get() else { return };
    for root in std::iter::once(&p.dir).chain(p.debug_dir.as_ref()) {
        let _ = std::fs::write(root.join(name), contents);
    }
}

pub fn pty_push(chunk: &str) {
    let cleaned = redact(&strip_ansi(chunk));
    if let Ok(mut ring) = PTY_RING.lock() {
        ring.push(&cleaned);
    }
    let should_flush = match LAST_FLUSH.lock() {
        Ok(last) => last.map(|t| t.elapsed() >= PTY_FLUSH_EVERY).unwrap_or(true),
        Err(_) => false,
    };
    if should_flush {
        pty_flush();
    }
}

pub fn pty_flush() {
    let snap = match PTY_RING.lock() {
        Ok(ring) => ring.snapshot(),
        Err(_) => return,
    };
    overwrite_both("pty-tail.log", &snap);
    if let Ok(mut last) = LAST_FLUSH.lock() {
        *last = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_event_writes_jsonl_and_redacts() {
        let dir = std::env::temp_dir().join(format!("claudia-diag-evt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        emit_event_to(&dir, None, "session", "started", Some(7), "user a@b.com");
        let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(text.contains("\"cat\":\"session\""));
        assert!(text.contains("\"evt\":\"started\""));
        assert!(text.contains("[email]"));
        assert!(!text.contains("a@b.com"));
        assert!(text.contains("\"sid\":7"));
        assert!(text.contains("\"lvl\":\"info\""));
    }

    #[test]
    fn emit_event_reads_paths_and_writes_null_sid() {
        let dir =
            std::env::temp_dir().join(format!("claudia-diag-evt-null-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        init_paths_for_test(dir.clone());
        emit_event("app", "boot", None, "ok");
        let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(text.contains("\"sid\":null"), "{text}");
        assert!(text.contains("\"cat\":\"app\""));
        assert!(text.contains("\"evt\":\"boot\""));
    }

    #[test]
    fn pty_ring_caps_at_100kib() {
        let mut r = PtyRing::new();
        r.push(&"x".repeat(80_000));
        r.push(&"y".repeat(80_000));
        assert!(r.snapshot().len() <= 100 * 1024);
        assert!(r.snapshot().contains('y'));
    }

    #[test]
    fn pty_push_strips_ansi_and_redacts() {
        pty_push("W\u{1b}[0maiting a@b.com");
        let snap = PTY_RING.lock().unwrap().snapshot();
        assert!(snap.contains("Waiting"), "{snap}");
        assert!(snap.contains("[email]"), "{snap}");
        assert!(!snap.contains("a@b.com"));
        assert!(!snap.contains('\u{1b}'));
    }
}
