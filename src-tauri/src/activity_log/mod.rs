// Activity log: what the AI did in the browser, with screenshots. ALWAYS ON (no switch, no setting): a
// database SEPARATE from the vagas DB, in a folder no MCP tool can read; only the app's own UI reads it
// (Settings, "View records"). Passwords are never recorded; rows and captures are purged by age and size.
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub mod shots;
pub mod sink;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    kind TEXT NOT NULL,
    session_id INTEGER, vaga_id INTEGER,
    tool TEXT, target TEXT, role TEXT, text TEXT, url TEXT,
    result TEXT, error TEXT, duration_ms INTEGER,
    shot_before TEXT, shot_after TEXT, extra TEXT
);
CREATE INDEX IF NOT EXISTS events_ts ON events(ts);
";

#[derive(Default, Clone, Debug)]
pub struct NewEvent {
    /// action | browser | failure | navigation
    pub kind: String,
    pub session_id: Option<i64>,
    pub vaga_id: Option<i64>,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub extra: Option<String>,
}

/// Longest text kept from an error or a page dialog message. The log is a debug aid, not an archive.
pub const MAX_EVENT_TEXT: usize = 200;

/// Cuts `s` to at most `max` characters (never in the middle of a character), adding an ellipsis when cut.
pub fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('\u{2026}');
    out
}

/// What the browser itself did (not the agent): the 11 events of the spec (D7). Written with
/// `LogSink::browser_event` as `kind = "browser"`, `tool` = [`BrowserEvent::name`].
/// Nothing here carries typed text or a password. The app process emits the first nine
/// (`service.rs`); the MCP process emits `TabCrashed` and `Dialog` (page events, wave 2).
/// Free text (errors, a dialog message) is clipped to [`MAX_EVENT_TEXT`] characters when converted.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserEvent {
    /// A real download of the browser began (not emitted when it was already installed).
    DownloadStarted { version: String, total_bytes: Option<u64> },
    /// The download finished and the browser is installed.
    Installed { version: String },
    InstallFailed { version: String, error: String },
    Launched { version: String, port: u16 },
    LaunchFailed { error: String },
    /// The browser stopped answering (two checks in a row failed).
    Gone,
    Relaunched { version: String, port: u16 },
    /// Gone, but no agent session needs it: a window the user closed stays closed.
    NotRelaunched,
    /// It closed several times in a row; the watcher stopped relaunching it.
    GaveUp,
    /// The agent's tab crashed (CDP `Inspector.targetCrashed`).
    TabCrashed { url: Option<String> },
    /// A JavaScript dialog opened (`alert`, `confirm`, `prompt`, `beforeunload`). The default value of a
    /// `prompt()` is deliberately not part of the event.
    Dialog { dialog_type: String, message: String, url: Option<String>, accepted: bool, during_tool: Option<String> },
}

impl BrowserEvent {
    /// The event name stored in `tool`.
    pub fn name(&self) -> &'static str {
        match self {
            Self::DownloadStarted { .. } => "download_started",
            Self::Installed { .. } => "installed",
            Self::InstallFailed { .. } => "install_failed",
            Self::Launched { .. } => "launched",
            Self::LaunchFailed { .. } => "launch_failed",
            Self::Gone => "gone",
            Self::Relaunched { .. } => "relaunched",
            Self::NotRelaunched => "not_relaunched",
            Self::GaveUp => "gave_up",
            Self::TabCrashed { .. } => "tab_crashed",
            Self::Dialog { .. } => "dialog",
        }
    }

    /// Pure conversion to a log row. Session and vaga are filled in by the sink.
    pub fn to_new_event(&self) -> NewEvent {
        let mut url = None;
        let mut error = None;
        let mut ok = true;
        let (target, extra) = match self {
            Self::DownloadStarted { version, total_bytes } => (Some(version.clone()), Some(serde_json::json!({ "version": version, "total_bytes": total_bytes }))),
            Self::Installed { version } => (Some(version.clone()), Some(serde_json::json!({ "version": version }))),
            Self::InstallFailed { version, error: e } => {
                ok = false;
                error = Some(clip(e, MAX_EVENT_TEXT));
                (Some(version.clone()), Some(serde_json::json!({ "version": version })))
            }
            Self::Launched { version, port } | Self::Relaunched { version, port } => {
                (Some(format!("{version} port {port}")), Some(serde_json::json!({ "version": version, "port": port })))
            }
            Self::LaunchFailed { error: e } => {
                ok = false;
                error = Some(clip(e, MAX_EVENT_TEXT));
                (None, None)
            }
            Self::Gone => {
                ok = false;
                (Some("stopped answering".to_string()), None)
            }
            Self::NotRelaunched => (Some("no active session".to_string()), None),
            Self::GaveUp => {
                ok = false;
                (Some("closed several times in a row".to_string()), None)
            }
            Self::TabCrashed { url: u } => {
                ok = false;
                url = u.clone();
                (Some("agent tab crashed".to_string()), None)
            }
            Self::Dialog { dialog_type, message, url: u, accepted, during_tool } => {
                url = u.clone();
                let message = clip(message, MAX_EVENT_TEXT);
                (
                    Some(format!("{dialog_type}: {message}")),
                    Some(serde_json::json!({ "dialog_type": dialog_type, "message": message, "accepted": accepted, "during_tool": during_tool })),
                )
            }
        };
        NewEvent {
            kind: "browser".into(),
            tool: Some(self.name().into()),
            target,
            url,
            result: Some(if ok { "ok" } else { "error" }.into()),
            error,
            extra: extra.map(|v| v.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct EventRow {
    pub id: i64,
    pub ts: String,
    pub kind: String,
    pub session_id: Option<i64>,
    pub vaga_id: Option<i64>,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub shot_before: Option<String>,
    pub shot_after: Option<String>,
    pub extra: Option<String>,
}

pub struct Filter {
    pub session_id: Option<i64>,
    pub vaga_id: Option<i64>,
    pub kind: Option<String>,
    /// Only rows older than this id: "load more" asks for the page after the last row it already has.
    pub before_id: Option<i64>,
    pub limit: i64,
}

impl Default for Filter {
    fn default() -> Self {
        Self { session_id: None, vaga_id: None, kind: None, before_id: None, limit: 200 }
    }
}

pub struct ActivityLog {
    conn: Mutex<Connection>,
    dir: PathBuf,
}

const COLS: &str = "id, ts, kind, session_id, vaga_id, tool, target, role, text, url, result, error, duration_ms, shot_before, shot_after, extra";

fn row(r: &rusqlite::Row) -> rusqlite::Result<EventRow> {
    Ok(EventRow {
        id: r.get(0)?,
        ts: r.get(1)?,
        kind: r.get(2)?,
        session_id: r.get(3)?,
        vaga_id: r.get(4)?,
        tool: r.get(5)?,
        target: r.get(6)?,
        role: r.get(7)?,
        text: r.get(8)?,
        url: r.get(9)?,
        result: r.get(10)?,
        error: r.get(11)?,
        duration_ms: r.get(12)?,
        shot_before: r.get(13)?,
        shot_after: r.get(14)?,
        extra: r.get(15)?,
    })
}

type RowIds = Vec<(i64, Option<String>, Option<String>)>;

impl ActivityLog {
    pub fn open(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let conn = Connection::open(dir.join("activity-log.db")).map_err(|e| e.to_string())?;
        // WAL + a busy timeout: the MCP process and the app write to the same file.
        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())?;
        conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(())).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { conn: Mutex::new(conn), dir: dir.to_path_buf() })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn record(&self, e: &NewEvent, shot_before: Option<&[u8]>, shot_after: Option<&[u8]>) -> Result<i64, String> {
        let id = {
            let c = self.conn.lock().map_err(|e| e.to_string())?;
            c.execute(
                "INSERT INTO events (kind, session_id, vaga_id, tool, target, role, text, url, result, error, duration_ms, extra)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![e.kind, e.session_id, e.vaga_id, e.tool, e.target, e.role, e.text, e.url, e.result, e.error, e.duration_ms, e.extra],
            )
            .map_err(|e| e.to_string())?;
            c.last_insert_rowid()
        };
        for (which, bytes) in [("before", shot_before), ("after", shot_after)] {
            if let Some(b) = bytes {
                // A failed image must never lose the event itself.
                if let Ok(rel) = shots::save(&self.dir, id, which, b) {
                    let col = if which == "before" { "shot_before" } else { "shot_after" };
                    let c = self.conn.lock().map_err(|e| e.to_string())?;
                    let _ = c.execute(&format!("UPDATE events SET {col} = ?1 WHERE id = ?2"), params![rel, id]);
                }
            }
        }
        Ok(id)
    }

    pub fn list(&self, f: &Filter) -> Result<Vec<EventRow>, String> {
        let c = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = c
            .prepare(&format!(
                "SELECT {COLS} FROM events
                 WHERE (?1 IS NULL OR session_id = ?1) AND (?2 IS NULL OR vaga_id = ?2) AND (?3 IS NULL OR kind = ?3)
                   AND (?4 IS NULL OR id < ?4)
                 ORDER BY id DESC LIMIT ?5"
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![f.session_id, f.vaga_id, f.kind, f.before_id, f.limit], row).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get(&self, id: i64) -> Result<Option<EventRow>, String> {
        let c = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = c.prepare(&format!("SELECT {COLS} FROM events WHERE id = ?1")).map_err(|e| e.to_string())?;
        let mut rows = stmt.query_map([id], row).map_err(|e| e.to_string())?;
        rows.next().transpose().map_err(|e| e.to_string())
    }

    fn rows_with_shots(&self, sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<RowIds, String> {
        let c = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = c.prepare(sql).map_err(|e| e.to_string())?;
        let r = stmt.query_map(args, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(|e| e.to_string())?;
        r.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    fn drop_row(&self, id: i64, before: Option<&str>, after: Option<&str>) -> Result<(), String> {
        for p in [before, after].into_iter().flatten() {
            shots::remove(&self.dir, p);
        }
        self.conn.lock().map_err(|e| e.to_string())?.execute("DELETE FROM events WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Drops rows older than `max_age_days`, then the oldest rows until the captures fit in
    /// `max_bytes`. Returns how many rows were removed.
    pub fn purge(&self, max_age_days: i64, max_bytes: u64) -> Result<usize, String> {
        let mut removed = 0;
        let cutoff = format!("-{max_age_days} days");
        let old = self.rows_with_shots("SELECT id, shot_before, shot_after FROM events WHERE ts < strftime('%Y-%m-%dT%H:%M:%SZ','now', ?1)", &[&cutoff])?;
        for (id, b, a) in old {
            self.drop_row(id, b.as_deref(), a.as_deref())?;
            removed += 1;
        }
        let all = self.rows_with_shots("SELECT id, shot_before, shot_after FROM events ORDER BY id ASC", &[])?;
        let sizes: Vec<u64> = all
            .iter()
            .map(|(_, b, a)| b.as_deref().map_or(0, |p| shots::size(&self.dir, p)) + a.as_deref().map_or(0, |p| shots::size(&self.dir, p)))
            .collect();
        let mut total: u64 = sizes.iter().sum();
        for ((id, b, a), size) in all.into_iter().zip(sizes) {
            if total <= max_bytes {
                break;
            }
            self.drop_row(id, b.as_deref(), a.as_deref())?;
            total -= size;
            removed += 1;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "claudia-al-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn ev(kind: &str, tool: &str) -> NewEvent {
        NewEvent { kind: kind.into(), tool: Some(tool.into()), ..Default::default() }
    }

    // ── BrowserEvent: pure conversion (touches no disk) ───────────────────

    fn all_events() -> Vec<BrowserEvent> {
        let v = || "154.0.1.2".to_string();
        vec![
            BrowserEvent::DownloadStarted { version: v(), total_bytes: Some(150) },
            BrowserEvent::Installed { version: v() },
            BrowserEvent::InstallFailed { version: v(), error: "sem rede".into() },
            BrowserEvent::Launched { version: v(), port: 9222 },
            BrowserEvent::LaunchFailed { error: "não abriu".into() },
            BrowserEvent::Gone,
            BrowserEvent::Relaunched { version: v(), port: 9223 },
            BrowserEvent::NotRelaunched,
            BrowserEvent::GaveUp,
            BrowserEvent::TabCrashed { url: Some("https://x/y".into()) },
            BrowserEvent::Dialog { dialog_type: "alert".into(), message: "oi".into(), url: Some("https://x/".into()), accepted: true, during_tool: None },
        ]
    }

    #[test]
    fn browser_events_map_to_the_eleven_names_and_the_browser_kind() {
        let names: Vec<&str> = all_events().iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            ["download_started", "installed", "install_failed", "launched", "launch_failed", "gone", "relaunched", "not_relaunched", "gave_up", "tab_crashed", "dialog"]
        );
        for e in all_events() {
            let row = e.to_new_event();
            assert_eq!(row.kind, "browser");
            assert_eq!(row.tool.as_deref(), Some(e.name()));
            assert!(row.session_id.is_none() && row.text.is_none(), "no typed text, and context is the sink's job");
        }
    }

    #[test]
    fn browser_event_results_say_ok_or_error_and_failures_carry_the_error() {
        let by_name = |n: &str| all_events().into_iter().find(|e| e.name() == n).unwrap().to_new_event();
        for ok in ["download_started", "installed", "launched", "relaunched", "not_relaunched", "dialog"] {
            assert_eq!(by_name(ok).result.as_deref(), Some("ok"), "{ok}");
        }
        for bad in ["install_failed", "launch_failed", "gone", "gave_up", "tab_crashed"] {
            assert_eq!(by_name(bad).result.as_deref(), Some("error"), "{bad}");
        }
        assert_eq!(by_name("install_failed").error.as_deref(), Some("sem rede"));
        assert_eq!(by_name("launch_failed").error.as_deref(), Some("não abriu"));
        assert_eq!(by_name("launched").target.as_deref(), Some("154.0.1.2 port 9222"));
        assert_eq!(by_name("tab_crashed").url.as_deref(), Some("https://x/y"));
        assert_eq!(by_name("dialog").target.as_deref(), Some("alert: oi"));
    }

    #[test]
    fn a_dialog_event_records_the_answer_and_the_tool_it_happened_in() {
        let ev = BrowserEvent::Dialog { dialog_type: "confirm".into(), message: "Enviar?".into(), url: None, accepted: false, during_tool: Some("browser_click".into()) };
        let extra: serde_json::Value = serde_json::from_str(ev.to_new_event().extra.as_deref().unwrap()).unwrap();
        assert_eq!(extra["accepted"], false);
        assert_eq!(extra["during_tool"], "browser_click");
        let outside = BrowserEvent::Dialog { dialog_type: "alert".into(), message: "x".into(), url: None, accepted: true, during_tool: None };
        let extra: serde_json::Value = serde_json::from_str(outside.to_new_event().extra.as_deref().unwrap()).unwrap();
        assert_eq!(extra["accepted"], true);
        assert!(extra["during_tool"].is_null());
    }

    #[test]
    fn browser_event_text_is_clipped_to_200_characters_without_splitting_one() {
        let long = "ã".repeat(500);
        let row = BrowserEvent::LaunchFailed { error: long.clone() }.to_new_event();
        assert_eq!(row.error.as_ref().unwrap().chars().count(), MAX_EVENT_TEXT + 1, "200 characters plus the ellipsis");
        let row = BrowserEvent::Dialog { dialog_type: "prompt".into(), message: long, url: None, accepted: false, during_tool: None }.to_new_event();
        let extra: serde_json::Value = serde_json::from_str(row.extra.as_deref().unwrap()).unwrap();
        assert_eq!(extra["message"].as_str().unwrap().chars().count(), MAX_EVENT_TEXT + 1);
        assert_eq!(clip("curto", 200), "curto", "short text is untouched");
    }

    #[test]
    fn records_and_lists_newest_first() {
        let log = ActivityLog::open(&temp("list")).unwrap();
        let a = log.record(&ev("action", "browser_open"), None, None).unwrap();
        let b = log.record(&ev("action", "browser_click"), None, None).unwrap();
        let rows = log.list(&Filter::default()).unwrap();
        assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![b, a]);
        assert_eq!(rows[0].tool.as_deref(), Some("browser_click"));
        assert!(!rows[0].ts.is_empty());
    }

    #[test]
    fn filters_by_kind_session_vaga_and_limit() {
        let log = ActivityLog::open(&temp("filter")).unwrap();
        let mut e1 = ev("action", "t");
        e1.session_id = Some(1);
        e1.vaga_id = Some(10);
        let mut e2 = ev("failure", "t");
        e2.session_id = Some(2);
        e2.vaga_id = Some(10);
        log.record(&e1, None, None).unwrap();
        log.record(&e2, None, None).unwrap();
        assert_eq!(log.list(&Filter { kind: Some("failure".into()), ..Default::default() }).unwrap().len(), 1);
        assert_eq!(log.list(&Filter { session_id: Some(1), ..Default::default() }).unwrap().len(), 1);
        assert_eq!(log.list(&Filter { vaga_id: Some(10), ..Default::default() }).unwrap().len(), 2);
        assert_eq!(log.list(&Filter { limit: 1, ..Default::default() }).unwrap().len(), 1);
    }

    #[test]
    fn get_returns_the_row_or_none() {
        let log = ActivityLog::open(&temp("get")).unwrap();
        let id = log.record(&ev("action", "t"), None, None).unwrap();
        assert!(log.get(id).unwrap().is_some());
        assert!(log.get(9999).unwrap().is_none());
    }

    #[test]
    fn many_connections_writing_and_reading_at_once_never_hit_a_lock_error() {
        // The MCP processes and the app all use the same file, some reading while others write.
        let d = temp("conc");
        ActivityLog::open(&d).unwrap(); // create the schema once
        let handles: Vec<_> = (0..6)
            .map(|n| {
                let d = d.clone();
                std::thread::spawn(move || {
                    let log = ActivityLog::open(&d).unwrap();
                    for i in 0..60 {
                        log.record(&ev("action", &format!("w{n}")), None, None).expect("a write must wait, not fail");
                        if i % 5 == 0 {
                            log.list(&Filter { limit: 10, ..Default::default() }).expect("a read must not fail");
                        }
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let log = ActivityLog::open(&d).unwrap();
        assert_eq!(log.list(&Filter { limit: 1000, ..Default::default() }).unwrap().len(), 360);
    }

    #[test]
    fn the_database_uses_write_ahead_logging() {
        let log = ActivityLog::open(&temp("wal")).unwrap();
        let mode: String = log.conn.lock().unwrap().query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn pages_walk_back_through_the_log_without_gaps_or_repeats() {
        let log = ActivityLog::open(&temp("pages")).unwrap();
        let ids: Vec<i64> = (0..7).map(|i| log.record(&ev("action", &format!("t{i}")), None, None).unwrap()).collect();
        let first = log.list(&Filter { limit: 3, ..Default::default() }).unwrap();
        assert_eq!(first.iter().map(|r| r.id).collect::<Vec<_>>(), vec![ids[6], ids[5], ids[4]]);
        let second = log.list(&Filter { limit: 3, before_id: Some(first[2].id), ..Default::default() }).unwrap();
        assert_eq!(second.iter().map(|r| r.id).collect::<Vec<_>>(), vec![ids[3], ids[2], ids[1]]);
        let last = log.list(&Filter { limit: 3, before_id: Some(second[2].id), ..Default::default() }).unwrap();
        assert_eq!(last.iter().map(|r| r.id).collect::<Vec<_>>(), vec![ids[0]], "the last page is short: nothing more to load");
        // The page cursor combines with the other filters.
        let mut e = ev("failure", "t");
        e.session_id = Some(5);
        let f1 = log.record(&e, None, None).unwrap();
        let f2 = log.record(&e, None, None).unwrap();
        let only = log.list(&Filter { kind: Some("failure".into()), before_id: Some(f2), ..Default::default() }).unwrap();
        assert_eq!(only.iter().map(|r| r.id).collect::<Vec<_>>(), vec![f1]);
    }

    #[test]
    fn screenshots_are_saved_and_referenced_by_relative_path() {
        let d = temp("shots");
        let log = ActivityLog::open(&d).unwrap();
        let id = log.record(&ev("action", "browser_click"), Some(b"BEFORE"), Some(b"AFTER")).unwrap();
        let row = log.get(id).unwrap().unwrap();
        let (b, a) = (row.shot_before.unwrap(), row.shot_after.unwrap());
        assert!(b.starts_with("capturas/") && b.ends_with("-before.jpg"), "{b}");
        assert_eq!(std::fs::read(d.join(&b)).unwrap(), b"BEFORE");
        assert_eq!(std::fs::read(d.join(&a)).unwrap(), b"AFTER");
    }

    #[test]
    fn an_event_without_screenshots_has_none() {
        let log = ActivityLog::open(&temp("noshots")).unwrap();
        let id = log.record(&ev("action", "t"), None, None).unwrap();
        let row = log.get(id).unwrap().unwrap();
        assert!(row.shot_before.is_none() && row.shot_after.is_none());
    }

    #[test]
    fn purge_by_age_removes_old_rows_and_their_files_but_keeps_fresh_ones() {
        let d = temp("purge-age");
        let log = ActivityLog::open(&d).unwrap();
        let old = log.record(&ev("action", "old"), Some(b"x"), None).unwrap();
        let fresh = log.record(&ev("action", "fresh"), Some(b"y"), None).unwrap();
        let old_file = d.join(log.get(old).unwrap().unwrap().shot_before.unwrap());
        let fresh_file = d.join(log.get(fresh).unwrap().unwrap().shot_before.unwrap());
        log.conn
            .lock()
            .unwrap()
            .execute("UPDATE events SET ts = strftime('%Y-%m-%dT%H:%M:%SZ','now','-40 days') WHERE id = ?1", [old])
            .unwrap();
        assert_eq!(log.purge(30, u64::MAX).unwrap(), 1);
        assert!(log.get(old).unwrap().is_none());
        assert!(!old_file.exists(), "the old row's capture is deleted too");
        assert!(log.get(fresh).unwrap().is_some() && fresh_file.exists());
    }

    #[test]
    fn purge_by_size_drops_the_oldest_first_and_stops_when_it_fits() {
        let d = temp("purge-size");
        let log = ActivityLog::open(&d).unwrap();
        let blob = vec![0u8; 10_000];
        let ids: Vec<i64> = (0..4).map(|_| log.record(&ev("action", "t"), Some(&blob), None).unwrap()).collect();
        assert_eq!(log.purge(30, 25_000).unwrap(), 2, "four images of 10 kB into a 25 kB budget: the two oldest go");
        assert!(log.get(ids[0]).unwrap().is_none() && log.get(ids[1]).unwrap().is_none());
        assert!(log.get(ids[2]).unwrap().is_some() && log.get(ids[3]).unwrap().is_some());
        assert_eq!(log.purge(30, 25_000).unwrap(), 0, "already within the budget: nothing more is removed");
    }

    #[test]
    fn resolve_rejects_traversal_and_foreign_paths() {
        let d = temp("resolve");
        let log = ActivityLog::open(&d).unwrap();
        let id = log.record(&ev("action", "t"), Some(b"x"), None).unwrap();
        let ok = log.get(id).unwrap().unwrap().shot_before.unwrap();
        assert!(shots::resolve(&d, &ok).is_ok());
        for bad in [
            "../secret.txt",
            "capturas/../../x",
            "Y:/inventado/arquivo.txt",
            "/etc/passwd",
            "capturas/../activity-log.db",
            "capturas\\..\\x",
            "activity-log.db",
            "",
        ] {
            assert!(shots::resolve(&d, bad).is_err(), "{bad}");
        }
    }
}
