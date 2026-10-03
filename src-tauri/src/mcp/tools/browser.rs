// The agent-facing browser actions. Each one: make sure the agent's own tab is visible, run the
// send guard (ONCE per action, AFTER the element is located and hit-tested), act, and (if the
// Activity log is on) report what happened.
//
// Security model: the thing being guarded against is a careless or manipulated MODEL, not a page
// that fights back. Still, every guard decision is built from what the BROWSER computes (the
// accessibility tree, DOM.getNodeForLocation) plus a small script that runs in an ISOLATED world,
// so a page that redefines `Element.prototype`, `JSON.stringify` or `document.activeElement`
// cannot blind the guard. Anything the guard cannot read is treated as a submit (fail closed).
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::browser::cdp::{self, CdpClient};
use crate::browser::guard::{self, GuardContext, Verdict};
use crate::browser::manager::{is_pinned, probe, read_state};
use crate::browser::nav::NavRecord;
use crate::browser::refs::{self, RefBook, RefNode};

/// Largest crop side, in CSS pixels: a 100000x100000 clip would crash the renderer and push a huge
/// image into the model's context.
const MAX_CROP_SIDE: f64 = 4000.0;
const MAX_TYPED_CHARS: usize = 50_000;
const MAX_READ_REFS: usize = 400;
const MAX_URL_BYTES: usize = 8 * 1024;

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
    /// When false, the tools skip screenshots entirely (no sink = zero cost; the app's LogSink is always on).
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

/// What the page's element says about itself, read from an ISOLATED world (see `INFO_FN`).
#[derive(Debug, Default, Deserialize, PartialEq)]
pub(crate) struct Info {
    #[serde(default)]
    name: String,
    /// A nameless `<button>` or `<input type=submit|image>` inside a form: its name cannot say what
    /// it does, so it counts as a submit by structure.
    #[serde(default, rename = "isSubmit")]
    is_submit: bool,
    /// An IFRAME/FRAME: what is inside (maybe another origin) is invisible to us.
    #[serde(default)]
    opaque: bool,
    /// Name of the form's default submit button, if the element is inside a form.
    #[serde(default, rename = "formDefault")]
    form_default: String,
    /// The info could not be read at all (script failed or returned garbage). Fail closed.
    #[serde(default)]
    unknown: bool,
}

impl Info {
    fn unreadable() -> Self {
        Info { unknown: true, ..Default::default() }
    }

    /// True when the element looks like a submit by structure alone, whatever its name says.
    fn structural_submit(&self) -> bool {
        self.is_submit || self.opaque || self.unknown
    }
}

/// Runs with `this` = the element, inside an isolated world. Returns a JSON string.
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
  const type = (el.type || '').toLowerCase();
  const form = el.form || (el.closest && el.closest('form'));
  const submitBtn = form ? form.querySelector('button[type=submit],input[type=submit],input[type=image],button:not([type])') : null;
  const empty = nameOf(el) === '';
  const submitLike = (tag === 'BUTTON' && type === 'submit') || (tag === 'INPUT' && (type === 'submit' || type === 'image'));
  return JSON.stringify({
    name: nameOf(el),
    isSubmit: empty && !!form && submitLike,
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

// ── URL policy ────────────────────────────────────────────────────────────

/// Host of an http(s) URL, lowercase, without port or userinfo; IPv6 without brackets.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = if let Some(inner) = authority.strip_prefix('[') {
        inner.split(']').next()?
    } else {
        authority.split(':').next()?
    };
    if host.is_empty() { None } else { Some(host.to_lowercase()) }
}

fn private_v4(ip: Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified() || ip.is_broadcast()
}

fn private_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return private_v4(v4);
    }
    let first = ip.segments()[0];
    ip.is_loopback() || ip.is_unspecified() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
}

/// True for hosts that are this machine or its network: the agent must not reach the browser's
/// own debug endpoint (`/json/list` lists the user's tabs) or the user's router.
fn is_local_host(host: &str) -> bool {
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".internal") {
        return true;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => private_v4(v4),
            IpAddr::V6(v6) => private_v6(v6),
        };
    }
    // "2130706433", "127.1" or "0x7f000001": Chrome reads those as 127.0.0.1. Refuse every numeric host
    // that is not a plain dotted quad (those were parsed above).
    let numeric = !host.is_empty() && host.chars().all(|c| c.is_ascii_digit() || c == '.');
    let hex = host.starts_with("0x") && host[2..].chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == 'x');
    numeric || hex
}

/// Why `url` may not be opened, or None if it may.
fn url_refusal(url: &str, allow_file: bool, allow_local: bool) -> Option<String> {
    if url.len() > MAX_URL_BYTES {
        return Some(format!("URL grande demais (máximo {MAX_URL_BYTES} bytes)"));
    }
    let lower = url.to_lowercase();
    if lower.starts_with("file://") && allow_file {
        return None;
    }
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Some("só URLs http ou https são permitidas".to_string());
    }
    match host_of(url) {
        None => Some("URL sem endereço válido".to_string()),
        Some(h) if is_local_host(&h) && !allow_local => {
            Some("endereços locais ou de rede interna não são permitidos".to_string())
        }
        Some(_) => None,
    }
}

// ── what the guard looks at ──────────────────────────────────────────────

/// Everything the guard knows about one action's target(s): the names to match and whether the
/// target is a submit by structure alone.
#[derive(Default)]
struct Probe {
    names: Vec<String>,
    structural: bool,
}

impl Probe {
    fn add_name(&mut self, n: &str) {
        if !n.trim().is_empty() {
            self.names.push(n.to_string());
        }
    }

    fn add_info(&mut self, info: &Info) {
        self.add_name(&info.name);
        self.structural |= info.structural_submit();
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
    allow_local_hosts: bool,
    tab: Mutex<Option<Tab>>,
    /// Stable refs per element within one document (see `RefBook`).
    refs: Mutex<RefBook>,
    agent_active: Arc<AtomicBool>,
}

impl BrowserTools {
    pub fn new(state_file: PathBuf, guard: Arc<dyn GuardContext>, sink: Arc<dyn ActionSink>) -> Self {
        Self {
            state_file,
            guard,
            sink,
            allow_file_urls: false,
            // Only a debug build, and only on request: a release build can never open local addresses.
            allow_local_hosts: cfg!(debug_assertions) && std::env::var_os("CLAUDIA_ALLOW_LOCAL_URLS").is_some(),
            tab: Mutex::new(None),
            refs: Mutex::new(RefBook::default()),
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
        let doc = Self::doc_id(c).await;
        let book = self.refs.lock().await;
        if !book.is_current(&doc) {
            return Err(STALE.into());
        }
        book.find(&doc, reference).cloned().ok_or_else(|| format!("ref {reference} não existe. Chame browser_read."))
    }

    // ── what the page says about an element: browser-computed, never page-trusted ──
    fn parse_info(v: &Value) -> Info {
        v.as_str().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_else(Info::unreadable)
    }

    /// An isolated JavaScript world in the main frame: its built-ins are pristine, whatever the
    /// page did to its own prototypes.
    async fn guard_world(c: &CdpClient) -> Result<i64, String> {
        let tree = c.call("Page.getFrameTree", json!({})).await?;
        let frame = tree["frameTree"]["frame"]["id"].as_str().ok_or("sem frame principal")?.to_string();
        let w = c.call("Page.createIsolatedWorld", json!({ "frameId": frame, "worldName": "claudia-guard" })).await?;
        w["executionContextId"].as_i64().ok_or_else(|| "sem contexto isolado".to_string())
    }

    /// Info about a DOM node. Any failure reads as "unreadable", which the guard treats as a submit.
    async fn info_for_node(c: &CdpClient, backend_id: i64) -> Info {
        let r = async {
            let world = Self::guard_world(c).await?;
            let o = c.call("DOM.resolveNode", json!({ "backendNodeId": backend_id, "executionContextId": world })).await?;
            let oid = o["object"]["objectId"].as_str().ok_or("sem objeto")?.to_string();
            let r = c
                .call("Runtime.callFunctionOn", json!({ "objectId": oid, "functionDeclaration": INFO_FN, "returnByValue": true }))
                .await?;
            Ok::<Info, String>(Self::parse_info(&r["result"]["value"]))
        }
        .await;
        r.unwrap_or_else(|_| Info::unreadable())
    }

    /// The accessible name RIGHT NOW (not the one cached at the last browser_read), computed by Chrome.
    async fn live_name(c: &CdpClient, backend_id: i64) -> String {
        c.call("Accessibility.getPartialAXTree", json!({ "backendNodeId": backend_id, "fetchRelatives": false }))
            .await
            .ok()
            .and_then(|r| r["nodes"][0]["name"]["value"].as_str().map(String::from))
            .unwrap_or_default()
    }

    /// Names and structure of a node, as the browser reports them.
    async fn probe_node(c: &CdpClient, backend_id: i64, into: &mut Probe) {
        into.add_name(&Self::live_name(c, backend_id).await);
        into.add_info(&Self::info_for_node(c, backend_id).await);
    }

    /// The node under a point, decided by Chrome (hit test), not by page script.
    async fn node_at_point(c: &CdpClient, x: f64, y: f64) -> Option<i64> {
        let r = c
            .call("DOM.getNodeForLocation", json!({ "x": x.round() as i64, "y": y.round() as i64, "includeUserAgentShadowDOM": false }))
            .await
            .ok()?;
        r["backendNodeId"].as_i64()
    }

    /// The node that has the keyboard focus, from the accessibility tree (`focused` is computed by
    /// Chrome; a page cannot fake it by redefining `document.activeElement`). None = nothing
    /// focusable has focus; Err = the tree could not be read.
    async fn focused_node(c: &CdpClient) -> Result<Option<i64>, String> {
        let ax = c.call("Accessibility.getFullAXTree", json!({})).await?;
        let nodes = ax["nodes"].as_array().cloned().unwrap_or_default();
        Ok(nodes
            .iter()
            .rev()
            .find(|n| {
                n["properties"]
                    .as_array()
                    .is_some_and(|ps| ps.iter().any(|p| p["name"] == "focused" && p["value"]["value"] == true))
            })
            .and_then(|n| n["backendDOMNodeId"].as_i64()))
    }

    /// Decides whether the action may proceed. Called exactly ONCE per action, after the target is
    /// located: a name that matches a submit pattern SPENDS the vaga's single allowed submit; a target
    /// that is only submit-like by structure (iframe, nameless button, unreadable) needs a valid
    /// confirmation but does not spend it.
    fn run_guard(&self, p: &Probe, page_url: &str, confirm: Option<i64>) -> Result<(), String> {
        let name_match = p.names.iter().any(|n| guard::looks_like_submit(n));
        let verdict = if name_match {
            // "submit" is a whole-word pattern: whatever the real label was, the guard sees a submit.
            guard::check_submit("submit", page_url, confirm, self.guard.as_ref())
        } else if p.structural {
            guard::check_confirmation(page_url, confirm, self.guard.as_ref())
        } else {
            Verdict::Allow
        };
        match verdict {
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
        if let Some(why) = url_refusal(url, self.allow_file_urls, self.allow_local_hosts) {
            return self.finish("browser_open", started, Err(why), None).await;
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
            let mut nodes = refs::build_refs(&ax, scope)?;
            let total = nodes.len();
            nodes.truncate(MAX_READ_REFS);
            let doc = Self::doc_id(&c).await;
            let nodes = self.refs.lock().await.update(&doc, nodes);
            let mut text = refs::render(&nodes);
            if total > MAX_READ_REFS {
                text.push_str(&format!("\n(mostrando {MAX_READ_REFS} de {total} elementos; use o parâmetro scope para ver o resto)"));
            }
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
            // 1. Locate FIRST (scroll, box): a failure here must not spend the single allowed submit.
            let (shot, x, y) = self.locate(&c, &n, true).await?;
            // 2. What is REALLY under the point, decided by Chrome, and what the target is called now.
            let mut probe = Probe::default();
            probe.add_name(&n.name); // cached: the model's own view
            Self::probe_node(&c, n.backend_node_id, &mut probe).await;
            match Self::node_at_point(&c, x, y).await {
                Some(hit) if hit != n.backend_node_id => Self::probe_node(&c, hit, &mut probe).await,
                Some(_) => {}
                None => probe.structural = true, // cannot tell what is under the point: fail closed
            }
            // 3. Only now the guard (once), then the click.
            let url = Self::page_url(&c).await;
            self.run_guard(&probe, &url, confirm)?;
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
        if text.chars().count() > MAX_TYPED_CHARS {
            return self
                .finish("browser_type", started, Err(format!("texto grande demais (máximo {MAX_TYPED_CHARS} caracteres)")), None)
                .await;
        }
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
                // The keyboard must not bypass the guard: look at what has the focus (as Chrome reports
                // it) and, for Enter, at the default submit button of its form (Enter in a text field
                // submits the form).
                let mut probe = Probe::default();
                match Self::focused_node(&c).await {
                    Ok(Some(id)) => {
                        Self::probe_node(&c, id, &mut probe).await;
                        if code == "Enter" {
                            let info = Self::info_for_node(&c, id).await;
                            probe.add_name(&info.form_default);
                        }
                    }
                    Ok(None) => {} // nothing focused: the key goes to the page body
                    Err(_) => probe.structural = true, // cannot tell what has the focus: fail closed
                }
                let url = Self::page_url(&c).await;
                self.run_guard(&probe, &url, confirm)?;
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
        let valid = [x, y, w, h].iter().all(|v| v.is_finite()) && w > 0.0 && h > 0.0 && w <= MAX_CROP_SIDE && h <= MAX_CROP_SIDE;
        if !valid {
            let why = format!("recorte inválido: largura e altura entre 1 e {MAX_CROP_SIDE} pixels");
            return self.finish("browser_crop", started, Err(why), None).await.map(ToolOutput::Text);
        }
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_crop", started, Err(e), None).await.map(ToolOutput::Text),
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
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return self.finish("browser_click_at", started, Err("coordenada inválida".into()), None).await;
        }
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_click_at", started, Err(e), None).await,
        };
        let r = async {
            // What is under the point is decided by Chrome's hit test, and described by Chrome.
            let mut probe = Probe::default();
            match Self::node_at_point(&c, x, y).await {
                Some(id) => Self::probe_node(&c, id, &mut probe).await,
                None => probe.structural = true, // cannot tell what is there: fail closed
            }
            let url = Self::page_url(&c).await;
            self.run_guard(&probe, &url, confirm)?;
            Self::mouse_click(&c, x, y).await?;
            Ok(Outcome { message: format!("Clicou em ({x}, {y})."), target: probe.names.first().cloned(), ..Default::default() })
        }
        .await;
        self.finish("browser_click_at", started, r, Some(&c)).await
    }
}

/// Guard context backed by the main database. site_allowed is true until the accounts cycle
/// (cycle 2) adds per-site permissions.
///
/// The single allowed submit per vaga lives in the DATABASE (table submit_claims), not in memory: an
/// MCP process that restarts, or two sessions running at once, must not be able to submit the
/// same vaga twice. Every failure to read or write it is a refusal (fail closed).
pub struct DbGuardContext {
    pub data_dir: PathBuf,
}

impl DbGuardContext {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn conn(&self) -> Option<rusqlite::Connection> {
        let c = super::open_db(&self.data_dir).ok()?;
        c.busy_timeout(Duration::from_secs(5)).ok()?;
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS submit_claims (
                 vaga_id INTEGER NOT NULL,
                 day TEXT NOT NULL,
                 claimed_at TEXT NOT NULL DEFAULT (datetime('now')),
                 PRIMARY KEY (vaga_id, day)
             );",
        )
        .ok()?;
        Some(c)
    }
}

impl GuardContext for DbGuardContext {
    fn vaga_status(&self, vaga_id: i64) -> Option<String> {
        let c = super::open_db(&self.data_dir).ok()?;
        c.query_row("SELECT status FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0)).ok()
    }

    /// Applications recorded today PLUS the ones this guard allowed today that the model never
    /// recorded: a model that forgets to record must not get an unlimited budget.
    fn applications_today(&self) -> i64 {
        let Some(conn) = self.conn() else { return i64::MAX / 2 }; // unreadable = budget exhausted
        let recorded = crate::db::queries::candidaturas_hoje(&conn).unwrap_or(i64::MAX / 2);
        let unrecorded: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM submit_claims c WHERE c.day = date('now','localtime')
                   AND NOT EXISTS (SELECT 1 FROM candidaturas k WHERE k.vaga_id = c.vaga_id)",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        recorded.saturating_add(unrecorded)
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

    /// The vaga being applied to: there must be EXACTLY ONE in 'candidatando'. Two means a crashed
    /// session left one behind and nobody can tell which one the model means, so none is active.
    fn active_vaga(&self) -> Option<i64> {
        let c = super::open_db(&self.data_dir).ok()?;
        let mut stmt = c.prepare("SELECT id FROM vagas WHERE status = 'candidatando' LIMIT 2").ok()?;
        let ids: Vec<i64> = stmt.query_map([], |r| r.get(0)).ok()?.filter_map(Result::ok).collect();
        if ids.len() == 1 { Some(ids[0]) } else { None }
    }

    fn claim_submit(&self, vaga_id: i64) -> bool {
        let Some(c) = self.conn() else { return false };
        matches!(
            c.execute(
                "INSERT OR IGNORE INTO submit_claims (vaga_id, day) VALUES (?1, date('now','localtime'))",
                [vaga_id],
            ),
            Ok(1)
        )
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
    fn the_active_vaga_is_the_one_in_candidatando_only_when_there_is_exactly_one() {
        let dir = temp_dir("guardctx-active");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir.clone());
        assert_eq!(ctx.active_vaga(), None, "nothing is in candidatando yet");
        db(&dir).execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
        assert_eq!(ctx.active_vaga(), Some(1));
        // A second one (left by a crashed session): nobody can tell which the model means.
        db(&dir).execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status) VALUES (2, 'x', 'y', 'linkedin', 'https://x/2', datetime('now'), 'candidatando')", []).unwrap();
        assert_eq!(ctx.active_vaga(), None, "two vagas in candidatando: no active one");
    }

    #[test]
    fn a_vaga_can_be_claimed_only_once_per_session() {
        let dir = temp_dir("guardctx-claim");
        seed_db(&dir);
        let ctx = DbGuardContext::new(dir.clone());
        assert!(ctx.claim_submit(1));
        assert!(!ctx.claim_submit(1), "the second submit for the same vaga is refused");
        assert!(ctx.claim_submit(2), "another vaga is independent");
        // A NEW process (a restart, or a second session) sees the same claims: they live in the database.
        let other = DbGuardContext::new(dir);
        assert!(!other.claim_submit(1), "a restarted MCP process must not be able to submit vaga 1 again");
        assert!(!other.claim_submit(2));
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

    #[test]
    fn unreadable_info_fails_closed_as_a_submit() {
        for bad in [json!(null), json!(42), json!("not json"), json!(""), json!("[1,2]")] {
            let i = BrowserTools::parse_info(&bad);
            assert!(i.unknown && i.structural_submit(), "garbage must read as an unreadable (hence submit-like) target: {bad}");
        }
        let ok = BrowserTools::parse_info(&json!(r#"{"name":"x"}"#));
        assert!(!ok.unknown && !ok.structural_submit(), "a well-formed ordinary element is not submit-like");
    }

    #[test]
    fn host_extraction() {
        assert_eq!(host_of("https://www.LinkedIn.com/jobs/1?x=1#y").as_deref(), Some("www.linkedin.com"));
        assert_eq!(host_of("http://user:pw@example.com:8080/a").as_deref(), Some("example.com"));
        assert_eq!(host_of("http://[::1]:9222/json").as_deref(), Some("::1"));
        assert_eq!(host_of("http:///nothing"), None);
        assert_eq!(host_of("not a url"), None);
    }

    #[test]
    fn local_and_private_hosts_are_recognised_in_every_disguise() {
        for h in [
            "localhost", "foo.localhost", "printer.local", "db.internal", "127.0.0.1", "127.1.2.3", "10.0.0.5",
            "192.168.1.1", "172.16.0.1", "172.31.255.1", "169.254.169.254", "0.0.0.0", "::1", "::", "fe80::1", "fd12:3456::1",
            "::ffff:127.0.0.1", "2130706433", "127.1", "0x7f000001", "0177.0.0.1",
        ] {
            assert!(is_local_host(h), "{h} must be treated as local");
        }
        for h in ["www.linkedin.com", "example.com", "8.8.8.8", "172.32.0.1", "172.15.0.1", "2606:4700::1111", "dk.indeed.com"] {
            assert!(!is_local_host(h), "{h} is public");
        }
    }

    #[test]
    fn the_url_policy() {
        assert_eq!(url_refusal("https://www.linkedin.com/jobs/view/1", false, false), None);
        assert!(url_refusal("http://127.0.0.1:9222/json/list", false, false).is_some(), "the browser's own debug endpoint");
        assert!(url_refusal("http://localhost:1420/", false, false).is_some());
        assert!(url_refusal("http://127.0.0.1:9222/", false, true).is_none(), "a debug build may opt in to local hosts");
        for bad in ["file:///c:/windows/win.ini", "javascript:alert(1)", "chrome://settings", "data:text/html,x", "ftp://x/", "about:blank"] {
            assert!(url_refusal(bad, false, false).is_some(), "{bad}");
        }
        assert!(url_refusal("file:///c:/x.html", true, false).is_none(), "file URLs only when explicitly allowed (tests)");
        let long = format!("https://example.com/{}", "a".repeat(MAX_URL_BYTES));
        assert!(url_refusal(&long, false, false).unwrap().contains("grande"));
        assert!(url_refusal("http://", false, false).is_some());
    }

    #[test]
    fn a_release_build_can_never_allow_local_hosts() {
        // The opt-in is gated by cfg!(debug_assertions); this test documents the expectation.
        let dir = temp_dir("local-gate");
        let t = BrowserTools::new(dir.join("s.json"), Arc::new(DbGuardContext::new(dir)), Arc::new(NoSink));
        if !cfg!(debug_assertions) {
            assert!(!t.allow_local_hosts);
        }
    }

    fn probe_of(names: &[&str], structural: bool) -> Probe {
        Probe { names: names.iter().map(|s| s.to_string()).collect(), structural }
    }

    fn tools_with(dir: &std::path::Path) -> BrowserTools {
        BrowserTools::new(dir.join("s.json"), Arc::new(DbGuardContext::new(dir.to_path_buf())), Arc::new(NoSink))
    }

    fn seed_candidatando(dir: &std::path::Path) {
        seed_db(dir);
        db(dir).execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
    }

    #[test]
    fn an_ordinary_target_is_never_stopped() {
        let dir = temp_dir("rg-ordinary");
        seed_candidatando(&dir);
        let t = tools_with(&dir);
        assert!(t.run_guard(&probe_of(&["Avançar", "Próximo"], false), "https://x", None).is_ok());
    }

    #[test]
    fn a_submit_name_anywhere_in_the_probe_needs_a_valid_confirmation_and_spends_it() {
        let dir = temp_dir("rg-name");
        seed_candidatando(&dir);
        let t = tools_with(&dir);
        // The cached name is harmless, the LIVE name is not.
        let p = probe_of(&["Revisar", "Enviar candidatura"], false);
        assert!(t.run_guard(&p, "https://x", None).unwrap_err().starts_with("RECUSADO"));
        assert!(t.run_guard(&p, "https://x", Some(1)).is_ok());
        let again = t.run_guard(&p, "https://x", Some(1)).unwrap_err();
        assert!(again.contains("já teve um envio autorizado hoje"), "the single submit was spent: {again}");
    }

    #[test]
    fn a_structural_submit_needs_a_valid_confirmation_but_does_not_spend_it() {
        let dir = temp_dir("rg-structural");
        seed_candidatando(&dir);
        let t = tools_with(&dir);
        let frame = probe_of(&[], true);
        assert!(t.run_guard(&frame, "https://x", None).unwrap_err().starts_with("RECUSADO"));
        assert!(t.run_guard(&frame, "https://x", Some(1)).is_ok());
        assert!(t.run_guard(&frame, "https://x", Some(1)).is_ok(), "an iframe click again: still not spent");
        // The real submit can still go through afterwards, exactly once.
        let submit = probe_of(&["Enviar candidatura"], false);
        assert!(t.run_guard(&submit, "https://x", Some(1)).is_ok());
        assert!(t.run_guard(&submit, "https://x", Some(1)).is_err());
    }

    #[test]
    fn two_processes_cannot_both_spend_the_same_vagas_submit() {
        // Same database, two BrowserTools (two MCP sessions): the claim lives in the database.
        let dir = temp_dir("rg-two");
        seed_candidatando(&dir);
        let (a, b) = (tools_with(&dir), tools_with(&dir));
        let submit = probe_of(&["Enviar candidatura"], false);
        assert!(a.run_guard(&submit, "https://x", Some(1)).is_ok());
        assert!(b.run_guard(&submit, "https://x", Some(1)).is_err(), "the second session must be refused");
    }

    #[test]
    fn two_vagas_in_candidatando_block_every_submit() {
        let dir = temp_dir("rg-two-active");
        seed_candidatando(&dir);
        db(&dir).execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status) VALUES (2, 'x', 'y', 'linkedin', 'https://x/2', datetime('now'), 'candidatando')", []).unwrap();
        let t = tools_with(&dir);
        let submit = probe_of(&["Enviar candidatura"], false);
        let e = t.run_guard(&submit, "https://x", Some(1)).unwrap_err();
        assert!(e.starts_with("RECUSADO") && !e.contains('2'), "{e}");
    }
}
