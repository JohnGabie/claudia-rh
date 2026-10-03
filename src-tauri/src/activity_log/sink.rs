// The ActionSink that feeds the Activity log. Best effort by design: a logging problem must never
// slow down or break what the agent is doing.
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::{ActivityLog, NewEvent};
use crate::browser::nav::NavRecord;
use crate::mcp::tools::browser::{ActionRecord, ActionSink};

pub const MAX_AGE_DAYS: i64 = 30;
pub const MAX_CAPTURE_BYTES: u64 = 1_073_741_824; // 1 GB
/// The log is always on, so the purge (it scans every row and every capture file) must not run on every
/// action: once per process start, then at most this often.
const PURGE_EVERY: Duration = Duration::from_secs(10 * 60);

pub struct LogSink {
    data_dir: PathBuf,
    local_dir: PathBuf,
    last_purge: Mutex<Option<Instant>>,
}

impl LogSink {
    pub fn new(data_dir: PathBuf, local_dir: PathBuf) -> Self {
        Self { data_dir, local_dir, last_purge: Mutex::new(None) }
    }

    /// True when a purge should run now (never ran, or the last one was long enough ago); marks it as run.
    fn purge_due(&self, now: Instant) -> bool {
        let Ok(mut last) = self.last_purge.lock() else { return false };
        if last.is_none_or(|t| now.duration_since(t) >= PURGE_EVERY) {
            *last = Some(now);
            true
        } else {
            false
        }
    }

    fn open(&self) -> Option<ActivityLog> {
        match ActivityLog::open(&self.local_dir.join("dev")) {
            Ok(l) => Some(l),
            Err(e) => {
                crate::diagnostics::emit_event("activity_log", "write_failed", None, &e);
                None
            }
        }
    }

    /// Current session and vaga, from the main database. Missing is fine.
    fn context(&self) -> (Option<i64>, Option<i64>) {
        let Ok(c) = rusqlite::Connection::open(self.data_dir.join("claudia_rh.db")) else { return (None, None) };
        let s = c.query_row("SELECT id FROM sessoes WHERE terminada_em IS NULL ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).ok();
        let v = c.query_row("SELECT id FROM vagas WHERE status='candidatando' ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).ok();
        (s, v)
    }

    fn write(&self, ev: NewEvent, before: Option<&[u8]>, after: Option<&[u8]>) {
        let Some(log) = self.open() else { return };
        if let Err(e) = log.record(&ev, before, after) {
            crate::diagnostics::emit_event("activity_log", "write_failed", None, &e);
        }
        if self.purge_due(Instant::now()) {
            let _ = log.purge(MAX_AGE_DAYS, MAX_CAPTURE_BYTES);
        }
    }
}

impl ActionSink for LogSink {
    /// Always on: there is no setting. (A `dev-mode.json` left by an older version is ignored.)
    fn enabled(&self) -> bool {
        true
    }

    fn action(&self, rec: ActionRecord) {
        let (session_id, vaga_id) = self.context();
        let ev = NewEvent {
            kind: if rec.result == "ok" { "action".into() } else { "failure".into() },
            session_id,
            vaga_id,
            tool: Some(rec.tool),
            target: rec.target,
            role: rec.role,
            text: rec.text,
            url: rec.url,
            result: Some(rec.result),
            error: rec.error,
            duration_ms: Some(rec.duration_ms as i64),
            extra: None,
        };
        self.write(ev, rec.shot_before.as_deref(), rec.shot_after.as_deref());
    }

    fn navigation(&self, n: NavRecord) {
        let (session_id, vaga_id) = self.context();
        let extra = serde_json::json!({ "status": n.status, "redirects": n.redirects, "initiator": n.initiator }).to_string();
        let ev = NewEvent {
            kind: "navigation".into(),
            session_id,
            vaga_id,
            tool: Some("navigation".into()),
            url: Some(n.url),
            result: n.status.map(|s| s.to_string()),
            extra: Some(extra),
            ..Default::default()
        };
        self.write(ev, None, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity_log::Filter;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    fn rec(tool: &str, result: &str) -> ActionRecord {
        ActionRecord {
            tool: tool.into(),
            target: Some("Avançar".into()),
            role: Some("button".into()),
            text: None,
            url: Some("https://x/y".into()),
            result: result.into(),
            error: if result == "ok" { None } else { Some("boom".into()) },
            duration_ms: 12,
            shot_before: Some(b"B".to_vec()),
            shot_after: Some(b"A".to_vec()),
        }
    }

    fn nav(url: &str) -> NavRecord {
        NavRecord { url: url.into(), status: Some(200), redirects: vec!["https://a/".into()], initiator: "agent".into() }
    }

    #[test]
    fn it_records_with_no_configuration_at_all_and_ignores_an_old_dev_mode_file() {
        // A fresh data dir: no setting exists anywhere. The log must still record.
        let (data, local) = (temp_dir("sink-always-d"), temp_dir("sink-always-l"));
        let s = LogSink::new(data.clone(), local.clone());
        assert!(s.enabled(), "there is no switch");
        s.action(rec("browser_click", "ok"));
        s.navigation(nav("https://x/"));
        let rows = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap();
        assert_eq!(rows.len(), 2, "an action and a navigation were recorded without any setting");

        // A file from the old developer mode that says "off" must not turn anything off.
        std::fs::write(data.join("dev-mode.json"), r#"{"enabled":false}"#).unwrap();
        s.action(rec("browser_click", "ok"));
        let rows = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap();
        assert_eq!(rows.len(), 3, "the stale dev-mode.json is ignored");
    }

    #[test]
    fn the_purge_runs_at_first_and_then_at_most_every_ten_minutes() {
        let s = LogSink::new(temp_dir("sink-due-d"), temp_dir("sink-due-l"));
        let t0 = Instant::now();
        assert!(s.purge_due(t0), "first write of the process purges");
        assert!(!s.purge_due(t0 + Duration::from_secs(5)), "not on every action");
        assert!(!s.purge_due(t0 + Duration::from_secs(599)));
        assert!(s.purge_due(t0 + Duration::from_secs(600)), "and again after the interval");
    }

    #[test]
    fn it_records_actions_and_failures_with_screenshots() {
        let (data, local) = (temp_dir("sink-on-d"), temp_dir("sink-on-l"));
        let s = LogSink::new(data, local.clone());
        s.action(rec("browser_click", "ok"));
        s.action(rec("browser_click", "error"));
        s.action(rec("browser_click", "refused"));
        let rows = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap();
        assert_eq!(rows.len(), 3);
        let kinds: Vec<&str> = rows.iter().map(|r| r.kind.as_str()).collect();
        assert_eq!(kinds, vec!["failure", "failure", "action"]); // newest first
        assert!(rows[2].shot_before.is_some() && rows[2].shot_after.is_some());
        assert_eq!(rows[0].result.as_deref(), Some("refused"));
        assert_eq!(rows[2].target.as_deref(), Some("Avançar"));
    }

    #[test]
    fn navigation_is_recorded_with_its_redirects_and_initiator() {
        let (data, local) = (temp_dir("sink-nav-d"), temp_dir("sink-nav-l"));
        LogSink::new(data, local.clone()).navigation(nav("https://b/"));
        let row = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap().remove(0);
        assert_eq!(row.kind, "navigation");
        assert_eq!(row.url.as_deref(), Some("https://b/"));
        let extra: serde_json::Value = serde_json::from_str(row.extra.as_deref().unwrap()).unwrap();
        assert_eq!(extra["redirects"][0], "https://a/");
        assert_eq!(extra["initiator"], "agent");
        assert_eq!(row.result.as_deref(), Some("200"));
    }

    #[test]
    fn session_and_vaga_come_from_the_main_database() {
        let (data, local) = (temp_dir("sink-ctx-d"), temp_dir("sink-ctx-l"));
        seed_db(&data);
        let conn = rusqlite::Connection::open(data.join("claudia_rh.db")).unwrap();
        conn.execute("INSERT INTO sessoes (iniciada_em, motivo_disparo) VALUES (datetime('now'), 'manual')", []).unwrap();
        conn.execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
        LogSink::new(data, local.clone()).action(rec("browser_open", "ok"));
        let row = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap().remove(0);
        assert_eq!(row.vaga_id, Some(1));
        assert!(row.session_id.is_some());
    }

    #[test]
    fn a_write_failure_never_panics_and_never_propagates() {
        let data = temp_dir("sink-fail-d");
        // The local dir is a FILE: opening the log folder must fail.
        let local = temp_dir("sink-fail-l").join("im-a-file");
        std::fs::write(&local, b"x").unwrap();
        let s = LogSink::new(data, local);
        s.action(rec("browser_click", "ok")); // must simply return
        s.navigation(nav("https://x/"));
    }

    #[test]
    fn writing_purges_old_rows_so_the_log_never_grows_forever() {
        let (data, local) = (temp_dir("sink-purge-d"), temp_dir("sink-purge-l"));
        let s = LogSink::new(data.clone(), local.clone());
        s.action(rec("browser_click", "ok"));
        let log = ActivityLog::open(&local.join("dev")).unwrap();
        let id = log.list(&Filter::default()).unwrap()[0].id;
        log.conn.lock().unwrap().execute("UPDATE events SET ts = strftime('%Y-%m-%dT%H:%M:%SZ','now','-40 days') WHERE id = ?1", [id]).unwrap();
        // Within the interval the same sink does not purge again; a new process (a new sink) does.
        s.action(rec("browser_click", "ok"));
        assert_eq!(log.list(&Filter::default()).unwrap().len(), 2, "no purge on every action");
        LogSink::new(data, local.clone()).action(rec("browser_click", "ok"));
        let rows = log.list(&Filter::default()).unwrap();
        assert_eq!(rows.len(), 2, "the 40-day-old row is gone, the two recent ones stay");
        assert!(rows.iter().all(|r| r.id != id));
    }
}
