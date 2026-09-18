pub mod ansi;
pub mod redact;
pub mod watchdog;

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
    pub log_dir: Option<PathBuf>,
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

pub fn init_paths(app_data_dir: &Path, log_dir: Option<PathBuf>) -> DiagPaths {
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
    let paths = DiagPaths {
        dir,
        debug_dir,
        log_dir,
    };
    let _ = PATHS.set(paths.clone());
    paths
}

#[cfg(test)]
pub fn init_paths_for_test(dir: PathBuf) -> DiagPaths {
    std::fs::create_dir_all(&dir).ok();
    let p = DiagPaths {
        dir: dir.clone(),
        debug_dir: None,
        log_dir: None,
    };
    let _ = PATHS.set(p.clone());
    p
}

pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.to_string();
        log::error!("panic: {payload}");
        let line = format!("{} {payload}\n", chrono::Local::now().to_rfc3339());
        if let Some(p) = PATHS.get() {
            for root in std::iter::once(&p.dir).chain(p.debug_dir.as_ref()) {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(root.join("panic.log"))
                {
                    let _ = f.write_all(line.as_bytes());
                }
            }
        }
        emit_event("panic", "panic", None, &payload);
        prev(info);
    }));
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

pub fn previous_unclean(dir: &Path) -> bool {
    let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap_or_default();
    let last_app = text.lines().rev().find(|l| l.contains("\"cat\":\"app\""));
    match last_app {
        None => false,
        Some(l) => !l.contains("\"evt\":\"exit\""),
    }
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

pub fn export_zip(dest: &Path) -> Result<PathBuf, String> {
    pty_flush();
    let paths = PATHS
        .get()
        .ok_or_else(|| "diagnostics not initialized".to_string())?;
    export_zip_from(paths, dest)
}

fn export_zip_from(paths: &DiagPaths, dest: &Path) -> Result<PathBuf, String> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("manifest.txt", opts)
        .map_err(|e| e.to_string())?;
    zip.write_all(manifest_body().as_bytes())
        .map_err(|e| e.to_string())?;

    let mut seen: Vec<String> = Vec::new();
    for root in std::iter::once(&paths.dir).chain(paths.log_dir.as_ref()) {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || zip_path_forbidden(&path) || !zip_member_allowed(&path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if seen.iter().any(|s| s == name) {
                continue;
            }
            seen.push(name.to_string());
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let redacted = redact_utf8_lossy_lines(&bytes);
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            zip.write_all(redacted.as_bytes())
                .map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(dest.to_path_buf())
}

fn manifest_body() -> String {
    format!(
        "app=claudia-rh\nversion={}\nos={}\nidentifier=io.github.johngabie.claudia-rh\nexported_at={}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        chrono::Local::now().to_rfc3339(),
    )
}

fn zip_path_forbidden(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("candidate_base")
        || lower.contains("search_variants")
        || lower.contains("notif.json")
        || lower.contains(".yaml")
        || lower.contains(".yml")
}

fn zip_member_allowed(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(name, "events.jsonl" | "pty-tail.log" | "panic.log" | "claude-startup.log")
        || name.starts_with("claudia-rh.log")
}

fn redact_utf8_lossy_lines(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        out.push_str(&redact(line));
    }
    out
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

    #[test]
    fn export_zip_redacts_and_excludes_yaml() {
        use std::io::Read;

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "claudia-diag-zip-{}-{}",
            std::process::id(),
            stamp
        ));
        let _ = std::fs::remove_dir_all(&root);
        let diag = root.join("diagnostics");
        let log_dir = root.join("logs");
        std::fs::create_dir_all(&diag).unwrap();
        std::fs::create_dir_all(&log_dir).unwrap();

        std::fs::write(diag.join("events.jsonl"), "user a@b.com ran a job\n").unwrap();
        std::fs::write(diag.join("candidate_base.yaml"), "email: leak@x.com\n").unwrap();
        std::fs::write(diag.join("search_variants.yaml"), "q: secret\n").unwrap();
        std::fs::write(diag.join("notif.json"), "{\"token\":\"nope\"}\n").unwrap();
        std::fs::write(root.join("candidate_base.yaml"), "email: secret@x.com\n").unwrap();
        std::fs::write(log_dir.join("claudia-rh.log"), "mail a@b.com\n").unwrap();
        std::fs::write(log_dir.join("claudia-rh.log.1"), "rotated a@b.com\n").unwrap();

        let paths = DiagPaths {
            dir: diag,
            debug_dir: None,
            log_dir: Some(log_dir),
        };
        let dest = root.join("out.zip");
        export_zip_from(&paths, &dest).unwrap();

        let file = std::fs::File::open(&dest).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut names = Vec::new();
        for i in 0..archive.len() {
            names.push(archive.by_index(i).unwrap().name().to_string());
        }

        assert!(
            !names.iter().any(|n| n.contains("candidate_base")
                || n.contains("search_variants")
                || n.contains("notif.json")
                || n.ends_with(".yaml")
                || n.ends_with(".yml")),
            "{names:?}"
        );
        assert!(names.iter().any(|n| n == "manifest.txt"), "{names:?}");
        assert!(names.iter().any(|n| n == "events.jsonl"), "{names:?}");
        assert!(names.iter().any(|n| n == "claudia-rh.log"), "{names:?}");
        assert!(names.iter().any(|n| n == "claudia-rh.log.1"), "{names:?}");

        let mut events = String::new();
        archive
            .by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut events)
            .unwrap();
        assert!(events.contains("[email]"), "{events}");
        assert!(!events.contains("a@b.com"), "{events}");

        let mut log = String::new();
        archive
            .by_name("claudia-rh.log")
            .unwrap()
            .read_to_string(&mut log)
            .unwrap();
        assert!(log.contains("[email]"), "{log}");
        assert!(!log.contains("a@b.com"), "{log}");
    }

    #[test]
    fn previous_unclean_true_when_last_app_is_boot() {
        let dir = std::env::temp_dir().join(format!(
            "claudia-diag-unclean-boot-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("events.jsonl"),
            "{\"cat\":\"app\",\"evt\":\"boot\"}\n{\"cat\":\"session\",\"evt\":\"started\"}\n",
        )
        .unwrap();
        assert!(previous_unclean(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn previous_unclean_false_when_last_app_is_exit() {
        let dir = std::env::temp_dir().join(format!(
            "claudia-diag-unclean-exit-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("events.jsonl"),
            "{\"cat\":\"app\",\"evt\":\"boot\"}\n{\"cat\":\"app\",\"evt\":\"exit\"}\n",
        )
        .unwrap();
        assert!(!previous_unclean(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
