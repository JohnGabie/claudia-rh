// The agent-facing browser actions. Each one: make sure the agent's own tab is visible, run the
// send guard (ONCE per action), act, and (if the Activity log is on) report what happened.
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::browser::cdp::{self, CdpClient};
use crate::browser::guard::{self, GuardContext, Verdict};
use crate::browser::manager::{is_pinned, probe, read_state};
use crate::browser::nav::NavRecord;
use crate::browser::refs::{self, RefNode};

#[derive(Debug)]
pub enum ToolOutput {
    Text(String),
    Image { data_base64: String, mime: String },
}

pub struct ActionRecord {
    pub tool: String,
    pub target: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    /// "ok" | "error" | "refused"
    pub result: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub shot_before: Option<Vec<u8>>,
    pub shot_after: Option<Vec<u8>>,
}

pub trait ActionSink: Send + Sync {
    /// When false, the tools skip screenshots entirely (dev mode off = zero cost).
    fn enabled(&self) -> bool;
    fn action(&self, rec: ActionRecord);
    fn navigation(&self, rec: NavRecord);
}

pub struct NoSink;
impl ActionSink for NoSink {
    fn enabled(&self) -> bool {
        false
    }
    fn action(&self, _rec: ActionRecord) {}
    fn navigation(&self, _rec: NavRecord) {}
}

#[derive(Default)]
struct Outcome {
    message: String,
    target: Option<String>,
    role: Option<String>,
    text: Option<String>,
    shot_before: Option<Vec<u8>>,
}

/// What the page says about an element, computed IN the page (see `INFO_FN`).
#[derive(Debug, Default, Deserialize)]
struct Info {
    #[serde(default)]
    name: String,
    /// A nameless button/input inside a form (its name cannot say what it does).
    #[serde(default, rename = "isSubmit")]
    is_submit: bool,
    /// An IFRAME/FRAME: its inner content (maybe another origin) is invisible to us.
    #[serde(default)]
    opaque: bool,
    /// Name of the form's default submit button, if the element is inside a form.
    #[serde(default, rename = "formDefault")]
    form_default: String,
}

/// Runs with `this` = the element. Returns a JSON string.
const INFO_FN: &str = r#"function(){
  const el = this;
  const nameOf = (e) => {
    if (!e) return '';
    const lb = e.getAttribute && e.getAttribute('aria-labelledby');
    if (lb) {
      const t = lb.split(/\s+/).map((i) => { const x = document.getElementById(i); return x ? (x.innerText || x.textContent || '') : ''; }).join(' ').trim();
      if (t) return t;
    }
    return ((e.getAttribute && e.getAttribute('aria-label')) || e.innerText || e.value || e.title || e.alt || e.textContent || '').toString().trim();
  };
  const tag = (el.tagName || '').toUpperCase();
  const form = el.form || (el.closest && el.closest('form'));
  const submitBtn = form ? form.querySelector('button[type=submit],input[type=submit],button:not([type])') : null;
  const empty = nameOf(el) === '';
  return JSON.stringify({
    name: nameOf(el),
    // A <button> with no type attribute reports type==="submit" in the DOM, so the type alone proves nothing:
    // only a button/input with NO NAME inside a form is treated as a submit by its structure.
    isSubmit: empty && !!form && (tag === 'BUTTON' || tag === 'INPUT'),
    opaque: tag === 'IFRAME' || tag === 'FRAME',
    formDefault: submitBtn ? nameOf(submitBtn) : ''
  });
}"#;

const STALE: &str = "ref obsoleta: a página mudou. Chame browser_read de novo e use as refs novas.";

/// The model reads these messages: turn transport errors into something it can act on.
fn friendly(e: String) -> String {
    // A refusal ("RECUSADO: ...") matches none of the transport cases and passes through untouched.
    if e.contains("connection closed") {
        "a conexão com o browser fechou; chame browser_read de novo".to_string()
    } else if e.starts_with("timeout calling") {
        format!("o browser não respondeu a tempo ({e})")
    } else {
        e
    }
}

struct Tab {
    client: Arc<CdpClient>,
    target_id: String,
    port: u16,
}

pub struct BrowserTools {
    state_file: PathBuf,
    guard: Arc<dyn GuardContext>,
    sink: Arc<dyn ActionSink>,
    allow_file_urls: bool,
    tab: Mutex<Option<Tab>>,
    /// (document id at read time, refs). The id changes on every real navigation.
    refs: Mutex<(String, Vec<RefNode>)>,
    agent_active: Arc<AtomicBool>,
}

impl BrowserTools {
    pub fn new(state_file: PathBuf, guard: Arc<dyn GuardContext>, sink: Arc<dyn ActionSink>) -> Self {
        Self {
            state_file,
            guard,
            sink,
            allow_file_urls: false,
            tab: Mutex::new(None),
            refs: Mutex::new((String::new(), Vec::new())),
            agent_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Tests only: lets browser_open load the local fixture page.
    pub fn allow_file_urls(mut self) -> Self {
        self.allow_file_urls = true;
        self
    }

    // ── connection: the agent's OWN tab ───────────────────────────────────
    async fn tab(&self) -> Result<(Arc<CdpClient>, u16, String), String> {
        let mut g = self.tab.lock().await;
        if let Some(t) = g.as_ref() {
            if !t.client.is_closed() {
                cdp::bring_to_front(t.port, &t.target_id, &t.client).await?;
                return Ok((t.client.clone(), t.port, t.target_id.clone()));
            }
        }
        let state = read_state(&self.state_file)
            .ok_or("o browser da Claudia ainda não está pronto (preparando, ou o app não está aberto)")?;
        // Never talk to a port that now belongs to another program (the user's own Chrome, say).
        let ours = probe(state.port).await.is_some_and(|b| is_pinned(&b, &state.version));
        if !ours {
            return Err("o browser da Claudia não está respondendo ou foi reiniciado; aguarde alguns segundos e tente de novo".into());
        }
        // The tab the agent used before, if it still exists; otherwise a NEW one. Never the user's tab.
        let previous = g.as_ref().map(|t| t.target_id.clone());
        let target = match previous {
            Some(id) => {
                let prefix = format!("ws://127.0.0.1:{}/", state.port);
                match cdp::list_targets(state.port).await?.into_iter().find(|t| t.id == id && t.kind == "page" && t.ws_url.starts_with(&prefix)) {
                    Some(t) => t,
                    None => cdp::new_page(state.port).await?,
                }
            }
            None => cdp::new_page(state.port).await?,
        };
        let client = Arc::new(CdpClient::connect(&target.ws_url).await?);
        for m in ["Page.enable", "DOM.enable", "Accessibility.enable"] {
            client.call(m, json!({})).await?;
        }
        cdp::emulate_normal_languages(&client).await?;
        if self.sink.enabled() {
            client.call("Network.enable", json!({})).await?;
            crate::browser::nav::spawn_tracker(&client, self.sink.clone(), self.agent_active.clone());
        }
        cdp::bring_to_front(state.port, &target.id, &client).await?;
        *g = Some(Tab { client: client.clone(), target_id: target.id.clone(), port: state.port });
        Ok((client, state.port, target.id))
    }

    async fn eval(c: &CdpClient, expr: &str) -> Result<Value, String> {
        let r = c.call("Runtime.evaluate", json!({ "expression": expr, "returnByValue": true })).await?;
        Ok(r["result"]["value"].clone())
    }

    async fn page_url(c: &CdpClient) -> String {
        Self::eval(c, "location.href").await.ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    }

    async fn doc_id(c: &CdpClient) -> String {
        Self::eval(c, "String(performance.timeOrigin)").await.ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    }

    async fn resolve_ref(&self, c: &CdpClient, reference: &str) -> Result<RefNode, String> {
        let g = self.refs.lock().await;
        if g.0.is_empty() || g.0 != Self::doc_id(c).await {
            return Err(STALE.into());
        }
        refs::find(&g.1, reference).cloned().ok_or_else(|| format!("ref {reference} não existe. Chame browser_read."))
    }

    // ── what the page says about an element ──────────────────────────────
    fn parse_info(v: &Value) -> Info {
        v.as_str().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default()
    }

    async fn info_for_node(c: &CdpClient, backend_id: i64) -> Result<Info, String> {
        let o = c.call("DOM.resolveNode", json!({ "backendNodeId": backend_id })).await.map_err(|_| STALE.to_string())?;
        let oid = o["object"]["objectId"].as_str().ok_or(STALE)?;
        let r = c
            .call("Runtime.callFunctionOn", json!({ "objectId": oid, "functionDeclaration": INFO_FN, "returnByValue": true }))
            .await?;
        Ok(Self::parse_info(&r["result"]["value"]))
    }

    /// The accessible name RIGHT NOW (not the one cached at the last browser_read).
    async fn live_name(c: &CdpClient, backend_id: i64) -> String {
        c.call("Accessibility.getPartialAXTree", json!({ "backendNodeId": backend_id, "fetchRelatives": false }))
            .await
            .ok()
            .and_then(|r| r["nodes"][0]["name"]["value"].as_str().map(String::from))
            .unwrap_or_default()
    }

    async fn info_at_point(c: &CdpClient, x: f64, y: f64) -> Result<Info, String> {
        // Descends into OPEN shadow roots; a cross-origin iframe stays opaque (flagged by INFO_FN).
        let expr = format!(
            "(function(){{ let e = document.elementFromPoint({x},{y}); \
               while (e && e.shadowRoot) {{ const i = e.shadowRoot.elementFromPoint({x},{y}); if (!i || i === e) break; e = i; }} \
               if (!e) return null; \
               const t = e.closest('button,a,[role=button],input[type=submit],iframe,frame') || e; \
               return ({INFO_FN}).call(t); }})()"
        );
        Ok(Self::parse_info(&Self::eval(c, &expr).await?))
    }

    async fn info_of_focus(c: &CdpClient) -> Result<Info, String> {
        let expr = format!("({INFO_FN}).call(document.activeElement || document.body)");
        Ok(Self::parse_info(&Self::eval(c, &expr).await?))
    }

    /// The guard is consulted at most ONCE per action: `check_submit` can spend the vaga's single
    /// allowed submit, so asking twice about the same click would refuse a legitimate one.
    fn run_guard(&self, names: &[&str], info: &Info, page_url: &str, confirm: Option<i64>) -> Result<(), String> {
        let submit_like = info.is_submit || info.opaque || names.iter().any(|n| guard::looks_like_submit(n));
        if !submit_like {
            return Ok(());
        }
        // "submit" is an exact pattern: whatever made this look like a submit, the guard sees one.
        match guard::check_submit("submit", page_url, confirm, self.guard.as_ref()) {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(why),
        }
    }

    // ── screenshots (only when the Activity log is on) ───────────────────
    async fn capture(c: &CdpClient, clip: Option<Value>, scale: f64, quality: u32) -> Option<Vec<u8>> {
        let mut p = json!({ "format": "jpeg", "quality": quality });
        match clip {
            Some(mut cl) => {
                cl["scale"] = json!(scale);
                p["clip"] = cl;
            }
            None => {
                let m = c.call("Page.getLayoutMetrics", json!({})).await.ok()?;
                let v = &m["cssVisualViewport"];
                p["clip"] = json!({ "x": 0, "y": 0, "width": v["clientWidth"], "height": v["clientHeight"], "scale": scale });
            }
        }
        let r = c.call("Page.captureScreenshot", p).await.ok()?;
        base64::engine::general_purpose::STANDARD.decode(r["data"].as_str()?).ok()
    }

    /// Scroll into view (trap 4), read the box, and take the "before" crop.
    async fn locate(&self, c: &CdpClient, n: &RefNode, shoot: bool) -> Result<(Option<Vec<u8>>, f64, f64), String> {
        let id = n.backend_node_id;
        c.call("DOM.scrollIntoViewIfNeeded", json!({ "backendNodeId": id })).await.map_err(|_| STALE.to_string())?;
        let m = c.call("DOM.getBoxModel", json!({ "backendNodeId": id })).await.map_err(|_| STALE.to_string())?;
        let q: Vec<f64> = m["model"]["content"].as_array().ok_or(STALE)?.iter().filter_map(Value::as_f64).collect();
        if q.len() < 8 {
            return Err(STALE.into());
        }
        let xs = [q[0], q[2], q[4], q[6]];
        let ys = [q[1], q[3], q[5], q[7]];
        let (minx, maxx) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
        let (miny, maxy) = (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max));
        let shot = if shoot && self.sink.enabled() {
            let clip = json!({ "x": (minx - 40.0).max(0.0), "y": (miny - 40.0).max(0.0),
                               "width": (maxx - minx) + 80.0, "height": (maxy - miny) + 80.0 });
            Self::capture(c, Some(clip), 1.0, 70).await
        } else {
            None
        };
        Ok((shot, (minx + maxx) / 2.0, (miny + maxy) / 2.0))
    }

    async fn mouse_click(c: &CdpClient, x: f64, y: f64) -> Result<(), String> {
        c.call("Input.dispatchMouseEvent", json!({ "type": "mouseMoved", "x": x, "y": y })).await?;
        for t in ["mousePressed", "mouseReleased"] {
            c.call("Input.dispatchMouseEvent", json!({ "type": t, "x": x, "y": y, "button": "left", "clickCount": 1 })).await?;
        }
        Ok(())
    }

    /// Hidden radios/checkboxes sit under other elements (trap 3): click the label.
    async fn click_label_or_self(c: &CdpClient, backend_id: i64) -> Result<(), String> {
        let o = c.call("DOM.resolveNode", json!({ "backendNodeId": backend_id })).await.map_err(|_| STALE.to_string())?;
        let oid = o["object"]["objectId"].as_str().ok_or(STALE)?;
        c.call(
            "Runtime.callFunctionOn",
            json!({
                "objectId": oid, "returnByValue": true,
                "functionDeclaration": "function(){const l=this.id?document.querySelector('label[for=\"'+CSS.escape(this.id)+'\"]'):null;(l||this).click();return l?'label':'self'}"
            }),
        )
        .await?;
        Ok(())
    }

    // ── recording ─────────────────────────────────────────────────────────
    async fn finish(&self, tool: &str, started: Instant, r: Result<Outcome, String>, c: Option<&CdpClient>) -> Result<String, String> {
        self.agent_active.store(false, Ordering::SeqCst);
        let mut rec = ActionRecord {
            tool: tool.to_string(),
            target: None,
            role: None,
            text: None,
            url: None,
            result: "ok".into(),
            error: None,
            duration_ms: started.elapsed().as_millis() as u64,
            shot_before: None,
            shot_after: None,
        };
        let out = match r {
            Ok(o) => {
                rec.target = o.target;
                rec.role = o.role;
                rec.text = o.text;
                rec.shot_before = o.shot_before;
                if let (true, Some(c)) = (self.sink.enabled(), c) {
                    rec.shot_after = Self::capture(c, None, 0.5, 60).await;
                }
                Ok(o.message)
            }
            Err(e) => {
                let e = friendly(e);
                rec.result = if e.starts_with("RECUSADO") { "refused" } else { "error" }.into();
                rec.error = Some(e.clone());
                if let (true, Some(c)) = (self.sink.enabled(), c) {
                    rec.shot_after = Self::capture(c, None, 1.0, 80).await; // full size on failure
                }
                Err(e)
            }
        };
        if self.sink.enabled() {
            if let Some(c) = c {
                rec.url = Some(Self::page_url(c).await);
            }
            self.sink.action(rec);
        }
        out
    }

    // ── tools ─────────────────────────────────────────────────────────────
    pub async fn open(&self, url: &str) -> Result<String, String> {
        let started = Instant::now();
        let lower = url.to_lowercase();
        let ok_scheme = lower.starts_with("http://") || lower.starts_with("https://") || (self.allow_file_urls && lower.starts_with("file://"));
        if !ok_scheme {
            return self.finish("browser_open", started, Err("só URLs http ou https são permitidas".into()), None).await;
        }
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_open", started, Err(e), None).await,
        };
        let r = async {
            let nav = c.call("Page.navigate", json!({ "url": url })).await?;
            if let Some(err) = nav["errorText"].as_str() {
                return Err(format!("a navegação falhou: {err}"));
            }
            for _ in 0..100 {
                if Self::eval(&c, "document.readyState").await? == "complete" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            *self.refs.lock().await = (String::new(), Vec::new());
            Ok(Outcome { message: format!("Abriu {url}. Chame browser_read para ver a página."), target: Some(url.to_string()), ..Default::default() })
        }
        .await;
        self.finish("browser_open", started, r, Some(&c)).await
    }

    pub async fn read(&self, scope: Option<&str>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_read", started, Err(e), None).await,
        };
        let r = async {
            let ax = c.call("Accessibility.getFullAXTree", json!({})).await?;
            let nodes = refs::build_refs(&ax, scope)?;
            let text = refs::render(&nodes);
            *self.refs.lock().await = (Self::doc_id(&c).await, nodes);
            Ok(Outcome { message: text, ..Default::default() })
        }
        .await;
        self.finish("browser_read", started, r, Some(&c)).await
    }

    pub async fn click(&self, reference: &str, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_click", started, Err(e), None).await,
        };
        let r = async {
            let n = self.resolve_ref(&c, reference).await?;
            // Guard on the cached name AND the live one: the same node can turn from "Revisar" into "Enviar candidatura".
            let live = Self::live_name(&c, n.backend_node_id).await;
            let info = Self::info_for_node(&c, n.backend_node_id).await?;
            let url = Self::page_url(&c).await;
            self.run_guard(&[n.name.as_str(), live.as_str(), info.name.as_str()], &info, &url, confirm)?;
            let (shot, x, y) = self.locate(&c, &n, true).await?;
            if n.role == "radio" || n.role == "checkbox" {
                Self::click_label_or_self(&c, n.backend_node_id).await?;
            } else {
                Self::mouse_click(&c, x, y).await?;
            }
            Ok(Outcome { message: format!("Clicou em {} \"{}\".", n.role, n.name), target: Some(n.name), role: Some(n.role), shot_before: shot, ..Default::default() })
        }
        .await;
        self.finish("browser_click", started, r, Some(&c)).await
    }

    pub async fn type_text(&self, reference: &str, text: &str) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_type", started, Err(e), None).await,
        };
        let r = async {
            let n = self.resolve_ref(&c, reference).await?;
            let o = c.call("DOM.resolveNode", json!({ "backendNodeId": n.backend_node_id })).await.map_err(|_| STALE.to_string())?;
            let oid = o["object"]["objectId"].as_str().ok_or(STALE)?.to_string();
            let t = c
                .call("Runtime.callFunctionOn", json!({ "objectId": oid, "returnByValue": true, "functionDeclaration": "function(){return this.type||''}" }))
                .await?;
            let is_password = t["result"]["value"] == "password";
            let (shot, _, _) = self.locate(&c, &n, !is_password).await?;
            c.call("DOM.focus", json!({ "backendNodeId": n.backend_node_id })).await.map_err(|_| STALE.to_string())?;
            c.call("Input.insertText", json!({ "text": text })).await?;
            Ok(Outcome {
                message: format!("Escreveu em {} \"{}\".", n.role, n.name),
                target: Some(n.name),
                role: Some(n.role),
                // Passwords: neither the text nor the "before" crop are ever recorded.
                text: if is_password { None } else { Some(text.to_string()) },
                shot_before: if is_password { None } else { shot },
            })
        }
        .await;
        self.finish("browser_type", started, r, Some(&c)).await
    }

    pub async fn key(&self, key: &str, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_key", started, Err(e), None).await,
        };
        let r = async {
            let (code, vk, text) = match key {
                "Enter" => ("Enter", 13, "\r"),
                "Tab" => ("Tab", 9, ""),
                "Escape" => ("Escape", 27, ""),
                " " | "Space" => ("Space", 32, " "),
                other => return Err(format!("tecla não suportada: {other} (use Enter, Tab, Escape ou Space)")),
            };
            if code == "Enter" || code == "Space" {
                // The keyboard must not bypass the guard: check what has the focus, and for Enter also
                // the default submit button of its form (Enter in a text field submits the form).
                let info = Self::info_of_focus(&c).await?;
                let url = Self::page_url(&c).await;
                let mut names = vec![info.name.as_str()];
                if code == "Enter" {
                    names.push(info.form_default.as_str());
                }
                self.run_guard(&names, &info, &url, confirm)?;
            }
            let key_name = if code == "Space" { " " } else { code };
            for t in ["keyDown", "keyUp"] {
                let mut p = json!({ "type": t, "key": key_name, "code": code, "windowsVirtualKeyCode": vk });
                if t == "keyDown" && !text.is_empty() {
                    p["text"] = json!(text);
                }
                c.call("Input.dispatchKeyEvent", p).await?;
            }
            Ok(Outcome { message: format!("Pressionou {key}."), target: Some(key.to_string()), ..Default::default() })
        }
        .await;
        self.finish("browser_key", started, r, Some(&c)).await
    }

    pub async fn crop(&self, x: f64, y: f64, w: f64, h: f64) -> Result<ToolOutput, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => {
                let _ = self.finish("browser_crop", started, Err(e.clone()), None).await;
                return Err(friendly(e));
            }
        };
        let r = async {
            let s = c
                .call("Page.captureScreenshot", json!({ "format": "png", "clip": { "x": x, "y": y, "width": w, "height": h, "scale": 1 } }))
                .await?;
            s["data"].as_str().map(String::from).ok_or_else(|| "o browser não devolveu imagem".to_string())
        }
        .await;
        match r {
            Ok(data) => {
                let _ = self.finish("browser_crop", started, Ok(Outcome { message: "recorte".into(), ..Default::default() }), Some(&c)).await;
                Ok(ToolOutput::Image { data_base64: data, mime: "image/png".into() })
            }
            Err(e) => self.finish("browser_crop", started, Err(e), Some(&c)).await.map(ToolOutput::Text),
        }
    }

    pub async fn click_at(&self, x: f64, y: f64, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_click_at", started, Err(e), None).await,
        };
        let r = async {
            let info = Self::info_at_point(&c, x, y).await?;
            let url = Self::page_url(&c).await;
            self.run_guard(&[info.name.as_str()], &info, &url, confirm)?;
            Self::mouse_click(&c, x, y).await?;
            Ok(Outcome { message: format!("Clicou em ({x}, {y})."), target: Some(info.name), ..Default::default() })
        }
        .await;
        self.finish("browser_click_at", started, r, Some(&c)).await
    }
}

/// Guard context backed by the main database. `site_allowed` is `true` until the accounts cycle
/// (cycle 2) adds per-site permissions.
pub struct DbGuardContext {
    pub data_dir: PathBuf,
    claimed: StdMutex<HashSet<i64>>,
}

impl DbGuardContext {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir, claimed: StdMutex::new(HashSet::new()) }
    }
}

impl GuardContext for DbGuardContext {
    fn vaga_status(&self, vaga_id: i64) -> Option<String> {
        let c = super::open_db(&self.data_dir).ok()?;
        c.query_row("SELECT status FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0)).ok()
    }

    /// Applications recorded today PLUS the ones this session was allowed to send but the model
    /// never recorded: a model that forgets to record must not get an unlimited budget.
    fn applications_today(&self) -> i64 {
        let Ok(conn) = super::open_db(&self.data_dir) else { return 0 };
        let recorded = crate::db::queries::candidaturas_hoje(&conn).unwrap_or(0);
        let claimed: Vec<i64> = self.claimed.lock().map(|g| g.iter().copied().collect()).unwrap_or_default();
        let unrecorded = claimed
            .iter()
            .filter(|id| {
                conn.query_row("SELECT COUNT(*) FROM candidaturas WHERE vaga_id = ?1", [**id], |r| r.get::<_, i64>(0)).unwrap_or(0) == 0
            })
            .count() as i64;
        recorded + unrecorded
    }

    fn daily_limit(&self) -> i64 {
        std::fs::read_to_string(self.data_dir.join("disparo.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v["limite_diario"].as_i64())
            .unwrap_or(10)
    }

    fn site_allowed(&self, _url: &str) -> bool {
        true
    }

    fn active_vaga(&self) -> Option<i64> {
        let c = super::open_db(&self.data_dir).ok()?;
        c.query_row("SELECT id FROM vagas WHERE status = 'candidatando' ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).ok()
    }

    fn claim_submit(&self, vaga_id: i64) -> bool {
        self.claimed.lock().map(|mut g| g.insert(vaga_id)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    fn db(dir: &std::path::Path) -> rusqlite::Connection {
        rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()
    }

    fn record_application(dir: &std::path::Path, vaga_id: i64) {
        db(dir)
            .execute(
                "INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES (?1, datetime('now'), 'x', 'easy_apply')",
                [vaga_id],
            )
            .unwrap();
    }

    #[test]
    fn it_reads_the_status_of_a_vaga() {
        let dir = temp_dir("guardctx-status");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir.clone());
        assert_eq!(ctx.vaga_status(1).as_deref(), Some("descoberta"));
        assert_eq!(ctx.vaga_status(999), None);
        db(&dir).execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
        assert_eq!(ctx.vaga_status(1).as_deref(), Some("candidatando"));
    }

    #[test]
    fn the_daily_limit_defaults_to_ten_and_follows_disparo_json() {
        let dir = temp_dir("guardctx-limit");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir.clone());
        assert_eq!(ctx.daily_limit(), 10, "default when disparo.json is absent");
        std::fs::write(dir.join("disparo.json"), r#"{"limite_diario": 3}"#).unwrap();
        assert_eq!(ctx.daily_limit(), 3);
        std::fs::write(dir.join("disparo.json"), "{ not json").unwrap();
        assert_eq!(ctx.daily_limit(), 10, "a corrupt file falls back to the default, not to unlimited");
    }

    #[test]
    fn the_active_vaga_is_the_one_in_candidatando() {
        let dir = temp_dir("guardctx-active");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir.clone());
        assert_eq!(ctx.active_vaga(), None, "nothing is in candidatando yet");
        db(&dir).execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
        assert_eq!(ctx.active_vaga(), Some(1));
    }

    #[test]
    fn a_vaga_can_be_claimed_only_once_per_session() {
        let dir = temp_dir("guardctx-claim");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir);
        assert!(ctx.claim_submit(1));
        assert!(!ctx.claim_submit(1), "the second submit for the same vaga is refused");
        assert!(ctx.claim_submit(2), "another vaga is independent");
    }

    #[test]
    fn the_budget_counts_what_the_model_recorded_plus_what_it_was_allowed_but_never_recorded() {
        let dir = temp_dir("guardctx-budget");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir.clone());
        assert_eq!(ctx.applications_today(), 0);
        // The guard let a submit through, but the model never recorded the application: it still counts.
        assert!(ctx.claim_submit(1));
        assert_eq!(ctx.applications_today(), 1, "an unrecorded allowed submit spends the budget");
        // Once it IS recorded it must not be counted twice.
        record_application(&dir, 1);
        assert_eq!(ctx.applications_today(), 1, "recorded + claimed for the same vaga counts once");
        // Another application recorded by the model, unrelated to a claim.
        db(&dir)
            .execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status) VALUES (2, 'x', 'y', 'linkedin', 'https://x/2', datetime('now'), 'aplicada')", [])
            .unwrap();
        record_application(&dir, 2);
        assert_eq!(ctx.applications_today(), 2);
    }

    #[test]
    fn sites_are_allowed_until_the_accounts_cycle() {
        let ctx = DbGuardContext::new(std::env::temp_dir());
        assert!(ctx.site_allowed("https://www.linkedin.com/jobs/view/1"));
    }

    #[test]
    fn transport_errors_become_messages_the_model_can_act_on() {
        assert_eq!(friendly("RECUSADO: não.".into()), "RECUSADO: não.", "a refusal passes through untouched");
        assert!(friendly("connection closed".into()).contains("browser_read"));
        assert!(friendly("timeout calling DOM.getBoxModel".into()).contains("não respondeu"));
        assert_eq!(friendly("a navegação falhou: net::ERR_X".into()), "a navegação falhou: net::ERR_X");
    }

    #[test]
    fn info_parsing_tolerates_garbage_and_missing_fields() {
        let ok = BrowserTools::parse_info(&json!(r#"{"name":"Enviar","isSubmit":true,"opaque":false,"formDefault":"Ok"}"#));
        assert_eq!((ok.name.as_str(), ok.is_submit, ok.opaque, ok.form_default.as_str()), ("Enviar", true, false, "Ok"));
        let partial = BrowserTools::parse_info(&json!(r#"{"name":"x"}"#));
        assert!(!partial.is_submit && !partial.opaque && partial.form_default.is_empty());
        for bad in [json!(null), json!(42), json!("not json"), json!("")] {
            let i = BrowserTools::parse_info(&bad);
            assert!(i.name.is_empty() && !i.is_submit, "garbage must read as an empty, harmless Info");
        }
    }
}
