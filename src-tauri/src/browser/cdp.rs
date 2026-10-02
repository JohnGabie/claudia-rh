// Minimal CDP client: one WebSocket to one page target. Zero magic: JSON in,
// JSON out, ids matched to oneshot channels, events broadcast.
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

pub struct CdpClient {
    tx: mpsc::UnboundedSender<String>,
    pending: Pending,
    next_id: AtomicU64,
    events: broadcast::Sender<Value>,
    closed: Arc<AtomicBool>,
    timeout: Duration,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        let (ws, _) = connect_async(ws_url).await.map_err(|e| format!("cdp connect: {e}"))?;
        let (mut sink, mut stream) = ws.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (events, _) = broadcast::channel(512);
        let closed = Arc::new(AtomicBool::new(false));

        // Give the writer task copies of closed and pending so it can mark them on error.
        let (closed_w, pending_w) = (closed.clone(), pending.clone());
        tokio::spawn(async move {
            while let Some(m) = rx.recv().await {
                if sink.send(Message::Text(m.into())).await.is_err() {
                    // Mark closed and drain pending calls with error.
                    closed_w.store(true, Ordering::SeqCst);
                    for (_, s) in pending_w.lock().await.drain() {
                        let _ = s.send(Err("connection closed".into()));
                    }
                    break;
                }
            }
        });

        let (p2, ev2, c2) = (pending.clone(), events.clone(), closed.clone());
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let Message::Text(t) = msg else { continue };
                let Ok(v) = serde_json::from_str::<Value>(t.as_str()) else { continue };
                if let Some(id) = v.get("id").and_then(Value::as_u64) {
                    if let Some(s) = p2.lock().await.remove(&id) {
                        let r = match v.get("error") {
                            Some(e) => Err(e.to_string()),
                            None => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
                        };
                        let _ = s.send(r);
                    }
                } else {
                    let _ = ev2.send(v);
                }
            }
            c2.store(true, Ordering::SeqCst);
            for (_, s) in p2.lock().await.drain() {
                let _ = s.send(Err("connection closed".into()));
            }
        });

        Ok(Self { tx, pending, next_id: AtomicU64::new(1), events, closed, timeout: Duration::from_secs(30) })
    }

    pub fn with_timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        if self.is_closed() {
            return Err("connection closed".into());
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (s, r) = oneshot::channel();
        self.pending.lock().await.insert(id, s);

        // Re-check is_closed right after inserting: the reader might have closed between the first check and insert.
        if self.is_closed() {
            self.pending.lock().await.remove(&id);
            return Err("connection closed".into());
        }

        let msg = json!({ "id": id, "method": method, "params": params }).to_string();
        if self.tx.send(msg).is_err() {
            self.pending.lock().await.remove(&id);
            return Err("connection closed".into());
        }
        match tokio::time::timeout(self.timeout, r).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err("connection closed".into()),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(format!("timeout calling {method}"))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Target {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl", default)]
    pub ws_url: String,
}

pub async fn list_targets(port: u16) -> Result<Vec<Target>, String> {
    let client = http();
    let r = client
        .get(format!("http://127.0.0.1:{port}/json/list"))
        .send()
        .await
        .map_err(|e| format!("browser unreachable on port {port}: {e}"))?;
    r.json::<Vec<Target>>().await.map_err(|e| format!("bad /json/list reply: {e}"))
}

pub async fn first_page(port: u16) -> Result<Target, String> {
    let targets = list_targets(port).await?;
    targets
        .into_iter()
        .find(|t| {
            t.kind == "page"
                && !t.ws_url.is_empty()
                && t.ws_url.starts_with(&format!("ws://127.0.0.1:{}/", port))
        })
        .ok_or_else(|| "the browser has no open page".to_string())
}

/// Helper for testing with a custom timeout.
#[cfg(test)]
pub(crate) async fn list_targets_with(port: u16, timeout: Duration) -> Result<Vec<Target>, String> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .no_proxy()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let r = client
        .get(format!("http://127.0.0.1:{port}/json/list"))
        .send()
        .await
        .map_err(|e| format!("browser unreachable on port {port}: {e}"))?;
    r.json::<Vec<Target>>().await.map_err(|e| format!("bad /json/list reply: {e}"))
}

/// HTTP client with default timeout and no proxy.
fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .no_proxy()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// A hidden tab silently swallows input events. Activate it and wait until the
/// page reports itself visible (trap 1 from the prototype).
pub async fn bring_to_front(port: u16, target_id: &str, client: &CdpClient) -> Result<(), String> {
    let http_client = http();
    let _ = http_client.get(format!("http://127.0.0.1:{port}/json/activate/{target_id}")).send().await;
    for _ in 0..25 {
        let r = client
            .call("Runtime.evaluate", json!({ "expression": "document.visibilityState", "returnByValue": true }))
            .await?;
        if r["result"]["value"] == "visible" {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err("the tab stayed hidden; input would be swallowed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message;

    /// Fake CDP endpoint. Methods: "Boom" -> error reply, "Emit" -> event then
    /// reply, "Close" -> drops the socket without replying, "Hang" -> never
    /// replies, anything else -> {"echo": method}.
    async fn fake_server() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((sock, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws = tokio_tungstenite::accept_async(sock).await.unwrap();
                    let (mut tx, mut rx) = ws.split();
                    while let Some(Ok(Message::Text(t))) = rx.next().await {
                        let v: Value = serde_json::from_str(t.as_str()).unwrap();
                        let id = v["id"].clone();
                        match v["method"].as_str().unwrap() {
                            "Boom" => { let _ = tx.send(Message::Text(json!({"id": id, "error": {"message": "nope"}}).to_string().into())).await; }
                            "Emit" => {
                                let _ = tx.send(Message::Text(json!({"method": "Test.event", "params": {"n": 1}}).to_string().into())).await;
                                let _ = tx.send(Message::Text(json!({"id": id, "result": {}}).to_string().into())).await;
                            }
                            "Close" => break,
                            "Hang" => {}
                            m => { let _ = tx.send(Message::Text(json!({"id": id, "result": {"echo": m}}).to_string().into())).await; }
                        }
                    }
                });
            }
        });
        format!("ws://127.0.0.1:{port}")
    }

    #[tokio::test]
    async fn call_returns_the_result() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let r = c.call("Page.navigate", json!({})).await.unwrap();
        assert_eq!(r["echo"], "Page.navigate");
    }

    #[tokio::test]
    async fn concurrent_calls_get_their_own_replies() {
        let c = std::sync::Arc::new(CdpClient::connect(&fake_server().await).await.unwrap());
        let (a, b) = tokio::join!(c.call("A", json!({})), c.call("B", json!({})));
        assert_eq!(a.unwrap()["echo"], "A");
        assert_eq!(b.unwrap()["echo"], "B");
    }

    #[tokio::test]
    async fn error_reply_becomes_err() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let e = c.call("Boom", json!({})).await.unwrap_err();
        assert!(e.contains("nope"), "{e}");
    }

    #[tokio::test]
    async fn events_are_broadcast() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let mut rx = c.subscribe();
        c.call("Emit", json!({})).await.unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        assert_eq!(ev["method"], "Test.event");
    }

    #[tokio::test]
    async fn closed_connection_fails_the_pending_call_and_is_flagged() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let e = c.call("Close", json!({})).await.unwrap_err();
        assert!(e.contains("closed"), "{e}");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(c.is_closed());
        assert!(c.call("Again", json!({})).await.unwrap_err().contains("closed"));
    }

    #[tokio::test]
    async fn a_call_that_never_gets_a_reply_times_out() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap().with_timeout(Duration::from_millis(150));
        let e = c.call("Hang", json!({})).await.unwrap_err();
        assert!(e.contains("timeout"), "{e}");
    }

    #[tokio::test]
    async fn list_targets_parses_the_json_endpoint() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut b = [0u8; 1024];
            let _ = s.read(&mut b).await;
            let body = format!(r#"[{{"id":"T1","type":"page","title":"x","url":"about:blank","webSocketDebuggerUrl":"ws://127.0.0.1:{}/devtools/page/T1"}},{{"id":"S1","type":"service_worker","title":"sw","url":"u","webSocketDebuggerUrl":"ws://x"}}]"#, port);
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
        });
        let t = first_page(port).await.unwrap();
        assert_eq!(t.id, "T1");
        assert_eq!(t.kind, "page");
    }

    #[tokio::test]
    async fn hang_after_a_drop_all_calls_fail_quickly() {
        // Fake server that accepts the WebSocket and immediately drops it.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((sock, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let _ = tokio_tungstenite::accept_async(sock).await;
                    // Immediately drop the connection without replying.
                });
            }
        });

        let url = format!("ws://127.0.0.1:{port}");
        let c = std::sync::Arc::new(CdpClient::connect(&url).await.unwrap().with_timeout(Duration::from_secs(5)));

        // Wait for the reader to close.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Fire 30 concurrent calls and measure time for each.
        let start = std::time::Instant::now();
        let handles: Vec<_> = (0..30)
            .map(|i| {
                let c = c.clone();
                tokio::spawn(async move {
                    let call_start = std::time::Instant::now();
                    let result = c.call(&format!("Call{i}"), json!({})).await;
                    let elapsed = call_start.elapsed();
                    (result, elapsed)
                })
            })
            .collect();

        for h in handles {
            let (result, elapsed) = h.await.unwrap();
            assert!(result.is_err(), "expected error");
            assert!(result.unwrap_err().contains("closed"), "expected 'closed' message");
            assert!(
                elapsed.as_secs_f64() < 1.0,
                "call took {:.3}s, should be under 1s",
                elapsed.as_secs_f64()
            );
        }

        let total = start.elapsed();
        assert!(
            total.as_secs_f64() < 2.0,
            "all 30 calls took {:.3}s, should be under 2s",
            total.as_secs_f64()
        );
    }

    #[tokio::test]
    async fn concurrent_calls_get_correct_responses() {
        // Test that concurrent calls with different response speeds still get matched correctly.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((sock, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws = tokio_tungstenite::accept_async(sock).await.unwrap();
                    let (mut tx, mut rx) = ws.split();

                    while let Some(Ok(Message::Text(t))) = rx.next().await {
                        let v: Value = serde_json::from_str(t.as_str()).unwrap();
                        let id = v["id"].clone();
                        let method = v["method"].as_str().unwrap();

                        // Send response immediately for all methods.
                        let _ = tx.send(Message::Text(json!({"id": id, "result": {"echo": method}}).to_string().into())).await;
                    }
                });
            }
        });

        let c = std::sync::Arc::new(CdpClient::connect(&format!("ws://127.0.0.1:{port}")).await.unwrap());

        // Send multiple concurrent calls
        let (res1, res2, res3) = tokio::join!(
            c.call("Method1", json!({})),
            c.call("Method2", json!({})),
            c.call("Method3", json!({}))
        );

        // All should get their correct responses
        assert_eq!(res1.unwrap()["echo"], "Method1");
        assert_eq!(res2.unwrap()["echo"], "Method2");
        assert_eq!(res3.unwrap()["echo"], "Method3");
    }

    #[tokio::test]
    async fn list_targets_filters_by_kind() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut b = [0u8; 1024];
            let _ = s.read(&mut b).await;
            // Put service_worker FIRST to catch if the filter is removed.
            let body = format!(r#"[{{"id":"S1","type":"service_worker","title":"sw","url":"u","webSocketDebuggerUrl":"ws://x"}},{{"id":"T1","type":"page","title":"x","url":"about:blank","webSocketDebuggerUrl":"ws://127.0.0.1:{}/devtools/page/T1"}}]"#, port);
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
        });
        let t = first_page(port).await.unwrap();
        // Must be the page target, not the service_worker.
        assert_eq!(t.id, "T1");
        assert_eq!(t.kind, "page");
    }

    #[tokio::test]
    async fn ws_url_validation_in_first_page() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut b = [0u8; 1024];
            let _ = s.read(&mut b).await;
            // Return a target with invalid ws_url pointing to evil.example
            let body = r#"[{"id":"T1","type":"page","title":"x","url":"about:blank","webSocketDebuggerUrl":"ws://evil.example:9/devtools/page/X"}]"#;
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
        });

        let result = first_page(port).await;
        assert!(result.is_err(), "should reject target with invalid ws_url");
        assert!(result.unwrap_err().contains("no open page"), "error should mention no open page");
    }
}
