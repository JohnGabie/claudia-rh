// The ActionSink that feeds the Activity log. Best effort by design: a logging problem must never
// slow down or break what the agent is doing.
use std::path::PathBuf;

use super::{is_enabled, ActivityLog, NewEvent};
use crate::browser::nav::NavRecord;
use crate::mcp::tools::browser::{ActionRecord, ActionSink};

pub const MAX_AGE_DAYS: i64 = 30;
pub const MAX_CAPTURE_BYTES: u64 = 1_073_741_824; // 1 GB

pub struct LogSink {
    data_dir: PathBuf,
    local_dir: PathBuf,
}

impl LogSink {
    pub fn new(data_dir: PathBuf, local_dir: PathBuf) -> Self {
        Self { data_dir, local_dir }
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
        let _ = log.purge(MAX_AGE_DAYS, MAX_CAPTURE_BYTES);
    }
}

impl ActionSink for LogSink {
    fn enabled(&self) -> bool {
        is_enabled(&self.data_dir)
    }

    fn action(&self, rec: ActionRecord) {
        if !self.enabled() {
            return;
        }
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
        if !self.enabled() {
            return;
        }
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
    use crate::activity_log::{set_enabled, Filter};
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
    fn off_by_default_writes_nothing_and_creates_no_files() {
        let (data, local) = (temp_dir("sink-off-d"), temp_dir("sink-off-l"));
        let s = LogSink::new(data, local.clone());
        assert!(!s.enabled());
        s.action(rec("browser_click", "ok")); // even if called, it must not create the dev folder
        s.navigation(nav("https://x/"));
        assert!(!local.join("dev").exists());
    }

    #[test]
    fn when_on_it_records_actions_and_failures_with_screenshots() {
        let (data, local) = (temp_dir("sink-on-d"), temp_dir("sink-on-l"));
        set_enabled(&data, true).unwrap();
        let s = LogSink::new(data, local.clone());
        assert!(s.enabled());
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
        set_enabled(&data, true).unwrap();
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
        set_enabled(&data, true).unwrap();
        LogSink::new(data, local.clone()).action(rec("browser_open", "ok"));
        let row = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap().remove(0);
        assert_eq!(row.vaga_id, Some(1));
        assert!(row.session_id.is_some());
    }

    #[test]
    fn a_write_failure_never_panics_and_never_propagates() {
        let data = temp_dir("sink-fail-d");
        set_enabled(&data, true).unwrap();
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
        set_enabled(&data, true).unwrap();
        let s = LogSink::new(data, local.clone());
        s.action(rec("browser_click", "ok"));
        let log = ActivityLog::open(&local.join("dev")).unwrap();
        let id = log.list(&Filter::default()).unwrap()[0].id;
        log.conn.lock().unwrap().execute("UPDATE events SET ts = strftime('%Y-%m-%dT%H:%M:%SZ','now','-40 days') WHERE id = ?1", [id]).unwrap();
        s.action(rec("browser_click", "ok")); // the next write purges
        let rows = log.list(&Filter::default()).unwrap();
        assert_eq!(rows.len(), 1, "the 40-day-old row is gone");
        assert_ne!(rows[0].id, id);
    }
}
