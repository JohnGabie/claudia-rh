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

    /// B2: Helper for testing: return the number of pending calls
    #[cfg(test)]
    pub(crate) async fn pending_len(&self) -> usize {
        self.pending.lock().await.len()
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

/// Every plain-HTTP call to the browser gets this timeout: a Chrome that accepts
/// the connection but never answers must not hang a tool call forever.
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

fn http_with(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(timeout)
        .no_proxy() // this is localhost: never route it through a system proxy
        .build()
        .expect("static HTTP client configuration is valid")
}

fn http() -> reqwest::Client {
    http_with(HTTP_TIMEOUT)
}

pub async fn list_targets(port: u16) -> Result<Vec<Target>, String> {
    list_targets_with(port, HTTP_TIMEOUT).await
}

/// The real implementation of `list_targets`, with the timeout injectable so a
/// test can exercise the production path in milliseconds.
pub(crate) async fn list_targets_with(port: u16, timeout: Duration) -> Result<Vec<Target>, String> {
    let r = http_with(timeout)
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
            // service_worker FIRST and with a VALID localhost ws_url: only the kind filter can exclude it.
            let body = format!(r#"[{{"id":"S1","type":"service_worker","title":"sw","url":"u","webSocketDebuggerUrl":"ws://127.0.0.1:{0}/devtools/page/S1"}},{{"id":"T1","type":"page","title":"x","url":"about:blank","webSocketDebuggerUrl":"ws://127.0.0.1:{0}/devtools/page/T1"}}]"#, port);
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

    // B1: HTTP timeout test - TCP listener that accepts but never answers
    #[tokio::test]
    async fn http_timeout_on_unresponsive_server() {
        use tokio::io::AsyncReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                // Accept but never respond
                let _ = sock.read(&mut buf).await;
                // Just hang forever
                tokio::time::sleep(Duration::from_secs(100)).await;
            }
        });

        // list_targets_with should timeout within 2 seconds even though client has 300ms timeout
        let start = std::time::Instant::now();
        let result = list_targets_with(port, Duration::from_millis(300)).await;
        let elapsed = start.elapsed();

        match result {
            Err(e) => {
                assert!(elapsed < Duration::from_secs(2), "should timeout quickly, took {:.3}s", elapsed.as_secs_f64());
                assert!(e.contains("unreachable") || e.contains("timeout"), "should have connection error, got: {e}");
            }
            Ok(_) => panic!("should have timed out"),
        }
    }

    // B2: Pending cleanup after timeout
    #[tokio::test]
    async fn pending_entries_cleaned_up_after_timeout() {
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
                        if method == "Hang" {
                            // Don't respond to Hang
                        } else if method == "Fast" {
                            let _ = tx.send(Message::Text(json!({"id": id, "result": {"echo": method}}).to_string().into())).await;
                        }
                    }
                });
            }
        });

        let c = CdpClient::connect(&format!("ws://127.0.0.1:{port}")).await.unwrap().with_timeout(Duration::from_millis(150));

        // Send a call that will timeout
        let result = c.call("Hang", json!({})).await;
        assert!(result.is_err());

        // After timeout, pending should be empty
        tokio::time::sleep(Duration::from_millis(50)).await;
        let pending_count = c.pending_len().await;
        assert_eq!(pending_count, 0, "pending should be cleaned up after timeout");
    }

    // B3: Out-of-order matching with different response times
    #[tokio::test]
    async fn concurrent_calls_with_different_response_times_match_correctly() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((sock, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws = tokio_tungstenite::accept_async(sock).await.unwrap();
                    let (mut tx, mut rx) = ws.split();
                    // Every request is answered from its OWN task, so a slow one
                    // really is overtaken by a fast one that arrived after it.
                    let (reply_tx, mut reply_rx) = mpsc::unbounded_channel::<String>();
                    tokio::spawn(async move {
                        while let Some(m) = reply_rx.recv().await {
                            if tx.send(Message::Text(m.into())).await.is_err() {
                                break;
                            }
                        }
                    });
                    while let Some(Ok(Message::Text(t))) = rx.next().await {
                        let v: Value = serde_json::from_str(t.as_str()).unwrap();
                        let id = v["id"].clone();
                        let method = v["method"].as_str().unwrap().to_string();
                        let reply_tx = reply_tx.clone();
                        tokio::spawn(async move {
                            if method == "Slow" {
                                tokio::time::sleep(Duration::from_millis(300)).await;
                            }
                            let _ = reply_tx.send(json!({"id": id, "result": {"echo": method}}).to_string());
                        });
                    }
                });
            }
        });

        let c = std::sync::Arc::new(CdpClient::connect(&format!("ws://127.0.0.1:{port}")).await.unwrap());

        // Send A (Slow) and B (Fast) concurrently - they arrive at server in order A, B
        // but B should complete first, proving ID-based matching works
        // Each task reports (reply, the instant it finished).
        let a_handle = {
            let c = c.clone();
            tokio::spawn(async move {
                let r = c.call("Slow", json!({})).await;
                (r, std::time::Instant::now())
            })
        };

        let b_handle = {
            let c = c.clone();
            // Small delay to ensure A is sent first
            tokio::time::sleep(Duration::from_millis(30)).await;
            tokio::spawn(async move {
                let r = c.call("Fast", json!({})).await;
                (r, std::time::Instant::now())
            })
        };

        let (res_a, res_b) = tokio::join!(a_handle, b_handle);
        let ((a_val, a_done), (b_val, b_done)) = (res_a.unwrap(), res_b.unwrap());

        // Each call gets its OWN reply (matched by id)...
        assert_eq!(a_val.unwrap()["echo"], "Slow", "Call A should get the Slow reply");
        assert_eq!(b_val.unwrap()["echo"], "Fast", "Call B should get the Fast reply");
        // ...and the replies really did arrive out of order: B (sent second) finished first.
        assert!(b_done < a_done, "the fast call must finish before the slow one, or this test proves nothing");
    }
}
