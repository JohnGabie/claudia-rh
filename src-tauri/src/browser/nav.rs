// Navigation history: what pages the browser loaded, with redirect chains and who caused each one.
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::cdp::CdpClient;
use crate::mcp::tools::browser::ActionSink;

#[derive(Debug, Clone, PartialEq)]
pub struct NavRecord {
    pub url: String,
    pub status: Option<i64>,
    pub redirects: Vec<String>,
    /// "agent" while a browser_* tool is running; "outside-tool" otherwise (the site, or the user).
    pub initiator: String,
}

pub struct NavTracker {
    agent_active: Arc<AtomicBool>,
    redirects: Vec<String>,
}

impl NavTracker {
    pub fn new(agent_active: Arc<AtomicBool>) -> Self {
        Self { agent_active, redirects: Vec::new() }
    }

    fn initiator(&self) -> String {
        if self.agent_active.load(Ordering::SeqCst) { "agent" } else { "outside-tool" }.to_string()
    }

    /// Feeds one CDP event; returns a record when a navigation completed.
    pub fn on_event(&mut self, ev: &Value) -> Option<NavRecord> {
        let p = &ev["params"];
        match ev["method"].as_str()? {
            "Network.requestWillBeSent" if p["type"] == "Document" => {
                if let Some(from) = p["redirectResponse"]["url"].as_str() {
                    self.redirects.push(from.to_string());
                }
                None
            }
            "Network.responseReceived" if p["type"] == "Document" => Some(NavRecord {
                url: p["response"]["url"].as_str()?.to_string(),
                status: p["response"]["status"].as_i64(),
                redirects: std::mem::take(&mut self.redirects),
                initiator: self.initiator(),
            }),
            "Page.navigatedWithinDocument" => Some(NavRecord {
                url: p["url"].as_str()?.to_string(),
                status: None,
                redirects: vec![],
                initiator: self.initiator(),
            }),
            _ => None,
        }
    }
}

/// Feeds the sink from the CDP event stream. Runs inside the MCP process, so it only sees
/// navigations while an AI session is connected (documented limit).
pub fn spawn_tracker(client: &Arc<CdpClient>, sink: Arc<dyn ActionSink>, agent_active: Arc<AtomicBool>) {
    tokio::spawn(run_tracker(client.subscribe(), sink, agent_active));
}

async fn run_tracker(
    mut rx: tokio::sync::broadcast::Receiver<Value>,
    sink: Arc<dyn ActionSink>,
    agent_active: Arc<AtomicBool>,
) {
    {
        let mut tracker = NavTracker::new(agent_active);
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    if let Some(rec) = tracker.on_event(&ev) {
                        sink.navigation(rec);
                    }
                }
                // A burst of network events outran us: skip what was lost and keep tracking.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tracker() -> (NavTracker, Arc<AtomicBool>) {
        let a = Arc::new(AtomicBool::new(false));
        (NavTracker::new(a.clone()), a)
    }

    fn req(url: &str, redirect_from: Option<&str>) -> Value {
        req_kind(url, redirect_from, "Document")
    }

    fn req_kind(url: &str, redirect_from: Option<&str>, kind: &str) -> Value {
        let mut p = json!({ "type": kind, "request": { "url": url } });
        if let Some(r) = redirect_from {
            p["redirectResponse"] = json!({ "url": r, "status": 302 });
        }
        json!({ "method": "Network.requestWillBeSent", "params": p })
    }

    fn resp(url: &str, status: i64, kind: &str) -> Value {
        json!({ "method": "Network.responseReceived", "params": { "type": kind, "response": { "url": url, "status": status } } })
    }

    #[test]
    fn a_plain_navigation_is_one_record() {
        let (mut t, _) = tracker();
        assert!(t.on_event(&req("https://a/", None)).is_none());
        let r = t.on_event(&resp("https://a/", 200, "Document")).unwrap();
        assert_eq!((r.url.as_str(), r.status, r.redirects.len()), ("https://a/", Some(200), 0));
    }

    #[test]
    fn a_redirect_chain_is_collected_in_order() {
        let (mut t, _) = tracker();
        t.on_event(&req("https://a/", None));
        t.on_event(&req("https://b/", Some("https://a/")));
        t.on_event(&req("https://c/", Some("https://b/")));
        let r = t.on_event(&resp("https://c/", 200, "Document")).unwrap();
        assert_eq!(r.redirects, vec!["https://a/".to_string(), "https://b/".to_string()]);
        assert_eq!(r.url, "https://c/");
    }

    #[test]
    fn the_redirect_chain_does_not_leak_into_the_next_navigation() {
        let (mut t, _) = tracker();
        t.on_event(&req("https://b/", Some("https://a/")));
        t.on_event(&resp("https://b/", 200, "Document"));
        t.on_event(&req("https://z/", None));
        assert!(t.on_event(&resp("https://z/", 200, "Document")).unwrap().redirects.is_empty());
    }

    #[test]
    fn subresources_are_ignored() {
        let (mut t, _) = tracker();
        assert!(t.on_event(&resp("https://a/app.js", 200, "Script")).is_none());
        assert!(t.on_event(&resp("https://a/x.png", 200, "Image")).is_none());
        assert!(t.on_event(&req_kind("https://a/app.js", Some("https://a/old.js"), "Script")).is_none());
        // ...and a subresource redirect must not pollute the next document's chain.
        assert!(t.on_event(&resp("https://a/", 200, "Document")).unwrap().redirects.is_empty());
    }

    #[test]
    fn the_initiator_follows_whether_a_tool_is_running() {
        let (mut t, active) = tracker();
        active.store(true, Ordering::SeqCst);
        assert_eq!(t.on_event(&resp("https://a/", 200, "Document")).unwrap().initiator, "agent");
        active.store(false, Ordering::SeqCst);
        assert_eq!(t.on_event(&resp("https://b/", 200, "Document")).unwrap().initiator, "outside-tool");
    }

    #[test]
    fn same_document_navigation_is_recorded_without_a_status() {
        let (mut t, _) = tracker();
        let ev = json!({ "method": "Page.navigatedWithinDocument", "params": { "url": "https://a/#step2" } });
        let r = t.on_event(&ev).unwrap();
        assert_eq!((r.url.as_str(), r.status), ("https://a/#step2", None));
    }

    #[test]
    fn unrelated_and_malformed_events_are_ignored() {
        let (mut t, _) = tracker();
        assert!(t.on_event(&json!({ "method": "Page.loadEventFired", "params": {} })).is_none());
        assert!(t.on_event(&json!({})).is_none());
        assert!(t.on_event(&json!({ "method": "Network.responseReceived", "params": { "type": "Document" } })).is_none());
    }

    // ── the spawned loop ──────────────────────────────────────────────────

    struct Collect(std::sync::Mutex<Vec<NavRecord>>);
    impl ActionSink for Collect {
        fn enabled(&self) -> bool {
            true
        }
        fn action(&self, _: crate::mcp::tools::browser::ActionRecord) {}
        fn navigation(&self, r: NavRecord) {
            self.0.lock().unwrap().push(r);
        }
    }

    #[tokio::test]
    async fn the_loop_survives_a_burst_that_overflows_the_event_channel() {
        // A channel of 4 receives 50 events BEFORE the loop starts reading: the first recv is a guaranteed Lagged.
        let (tx, rx) = tokio::sync::broadcast::channel::<Value>(4);
        for i in 0..50 {
            tx.send(json!({ "method": "Network.dataReceived", "params": { "n": i } })).unwrap();
        }
        tx.send(resp("https://after-the-burst/", 200, "Document")).unwrap();
        drop(tx); // the loop ends by itself after draining
        let sink = Arc::new(Collect(Default::default()));
        run_tracker(rx, sink.clone(), Arc::new(AtomicBool::new(true))).await;
        let got = sink.0.lock().unwrap().clone();
        assert_eq!(got.len(), 1, "tracking must continue after a Lagged error: {got:?}");
        assert_eq!(got[0].url, "https://after-the-burst/");
    }

    #[tokio::test]
    async fn the_loop_ends_when_the_channel_closes() {
        let (tx, rx) = tokio::sync::broadcast::channel::<Value>(4);
        drop(tx);
        let sink = Arc::new(Collect(Default::default()));
        tokio::time::timeout(std::time::Duration::from_secs(2), run_tracker(rx, sink, Arc::new(AtomicBool::new(false))))
            .await
            .expect("a closed channel must end the loop, not hang it");
    }

    #[test]
    fn the_cdp_event_channel_is_big_enough_for_a_page_load() {
        assert!(std::hint::black_box(crate::browser::cdp::EVENT_CAPACITY) >= 2048);
    }
}
