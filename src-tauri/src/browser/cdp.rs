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

        tokio::spawn(async move {
            while let Some(m) = rx.recv().await {
                if sink.send(Message::Text(m.into())).await.is_err() {
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
    let r = reqwest::get(format!("http://127.0.0.1:{port}/json/list"))
        .await
        .map_err(|e| format!("browser unreachable on port {port}: {e}"))?;
    r.json::<Vec<Target>>().await.map_err(|e| format!("bad /json/list reply: {e}"))
}

pub async fn first_page(port: u16) -> Result<Target, String> {
    list_targets(port)
        .await?
        .into_iter()
        .find(|t| t.kind == "page" && !t.ws_url.is_empty())
        .ok_or_else(|| "the browser has no open page".to_string())
}

/// A hidden tab silently swallows input events. Activate it and wait until the
/// page reports itself visible (trap 1 from the prototype).
pub async fn bring_to_front(port: u16, target_id: &str, client: &CdpClient) -> Result<(), String> {
    let _ = reqwest::get(format!("http://127.0.0.1:{port}/json/activate/{target_id}")).await;
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
            let body = r#"[{"id":"T1","type":"page","title":"x","url":"about:blank","webSocketDebuggerUrl":"ws://127.0.0.1:1/devtools/page/T1"},{"id":"S1","type":"service_worker","title":"sw","url":"u","webSocketDebuggerUrl":"ws://x"}]"#;
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
        });
        let t = first_page(port).await.unwrap();
        assert_eq!(t.id, "T1");
        assert_eq!(t.kind, "page");
    }
}
