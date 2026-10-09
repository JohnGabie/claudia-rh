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
use crate::browser::guard::{self, GuardContext, SubmitDecision, Verdict};
use crate::browser::manager::{is_pinned, probe, read_state};
use crate::browser::loopguard::LoopGuard;
use crate::browser::nav::NavRecord;
use crate::browser::page_events::{self, PageState};
use crate::browser::pagestate;
use crate::browser::refs::{self, RefBook, RefNode};
use crate::browser::text;
use std::sync::Mutex as StdMutex;

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
    /// What the browser itself did (a dialog, a tab crash). Default: nothing, for `NoSink` and test sinks.
    fn browser_event(&self, _ev: crate::activity_log::BrowserEvent) {}
}

pub struct NoSink;
impl ActionSink for NoSink {
    fn enabled(&self) -> bool {
        false
    }
    fn action(&self, _rec: ActionRecord) {}
    fn navigation(&self, _rec: NavRecord) {}
}

/// What browser_upload attaches: always resolved by the app from the library, never a path from the agent.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: i64,
    pub nome: String,
    pub formato: String,
    pub tamanho: u64,
    pub sha256: String,
    pub path: std::path::PathBuf,
    pub regra: String,
}

pub trait DocumentSource: Send + Sync {
    fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String>;
    /// Called once per successful upload, so the app remembers which document went to which vaga.
    fn attached(&self, _vaga_id: i64, _tipo: &str, _documento_id: i64) {}
}

/// The Activity log line of an upload: id and name of the document, never its content or path.
pub(crate) fn upload_log_text(att: &Attachment) -> String {
    format!("documento {} | {} | {} | {} bytes | sha256 {}", att.id, att.nome, att.formato, att.tamanho, &att.sha256[..att.sha256.len().min(12)])
}

pub struct NoDocuments;
impl DocumentSource for NoDocuments {
    fn for_upload(&self, _: i64, _: &str, _: Option<i64>) -> Result<Attachment, String> {
        Err("a biblioteca de documentos não está disponível: importe um currículo no app e marque-o como preferido".into())
    }
}

/// Each step of an upload gives up after this: a page that never answers must not hold the tab.
const UPLOAD_STEP: Duration = Duration::from_secs(5);

/// Whether a file field's `accept` attribute takes a document of this format, read the way the
/// browser's chooser reads it (extensions and MIME types, case-insensitive). Empty accept = anything.
pub(crate) fn accept_allows(accept: &str, formato: &str) -> bool {
    let mime = match formato {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "",
    };
    let tokens: Vec<String> = accept.split(',').map(|t| t.trim().to_ascii_lowercase()).filter(|t| !t.is_empty()).collect();
    tokens.is_empty()
        || tokens.iter().any(|t| t == "*/*" || *t == format!(".{formato}") || t == mime || (t == "application/*" && !mime.is_empty()))
}

#[derive(Default)]
struct Outcome {
    message: String,
    target: Option<String>,
    role: Option<String>,
    text: Option<String>,
    shot_before: Option<Vec<u8>>,
    /// What makes two calls "the same call" for the repeated-call hint, when `target` is not it
    /// (a read has no target; a crop is identified by its rectangle).
    loop_key: Option<String>,
}

/// A crop's loop key is its rectangle: moving the crop around the page is exploring, not looping.
fn crop_loop_key(x: f64, y: f64, w: f64, h: f64) -> String {
    format!("{x:.0},{y:.0},{w:.0},{h:.0}")
}

/// A scroll's loop key includes where it left the page (position and height): scrolling that keeps
/// moving the page, or makes a list load more, is exploring. Only a scroll that leaves the page exactly
/// where it was (the end of the page, a container whose element did not move) adds up to the hint.
fn scroll_loop_key(direction: &str, reference: Option<&str>, pos: &ScrollPos, element_top: Option<f64>) -> String {
    format!("{direction}|{}|{:.0}|{:.0}|{:.0}", reference.unwrap_or(""), pos.y, pos.content_h, element_top.unwrap_or(0.0))
}

/// The clip for an Activity log screenshot. `clip` is in viewport coordinates (a box from
/// DOM.getBoxModel) and None means the whole visible viewport. Page.captureScreenshot takes its clip
/// in DOCUMENT coordinates, so the scroll offset is added: without it a scrolled page came out as the
/// top of the document, blank where it is off screen.
fn shot_clip(metrics: &Value, clip: Option<Value>, scale: f64) -> Value {
    let v = &metrics["cssVisualViewport"];
    let mut cl = clip.unwrap_or_else(|| json!({ "x": 0.0, "y": 0.0, "width": v["clientWidth"], "height": v["clientHeight"] }));
    let (page_x, page_y) = (v["pageX"].as_f64().unwrap_or(0.0), v["pageY"].as_f64().unwrap_or(0.0));
    cl["x"] = json!(cl["x"].as_f64().unwrap_or(0.0) + page_x);
    cl["y"] = json!(cl["y"].as_f64().unwrap_or(0.0) + page_y);
    cl["scale"] = json!(scale);
    cl
}

fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Where the page is scrolled, from Chrome's own layout metrics (no JavaScript in the page).
#[derive(Debug, Clone, Copy)]
struct ScrollPos {
    y: f64,
    client_w: f64,
    client_h: f64,
    content_h: f64,
}

impl ScrollPos {
    fn max_y(&self) -> f64 {
        (self.content_h - self.client_h).max(0.0)
    }

    fn percent(&self) -> i64 {
        let m = self.max_y();
        if m <= 0.0 { 100 } else { ((self.y / m) * 100.0).round().clamp(0.0, 100.0) as i64 }
    }

    fn at_end(&self) -> bool {
        self.y >= self.max_y() - 2.0
    }
}

/// Polls (60 ms apart) a wheel is given to start moving the document before it is taken as not moving it.
const SCROLL_START_POLLS: u32 = 8;

/// Pure: did the document move (or change height) between two readings?
fn scroll_moved(before: &ScrollPos, now: &ScrollPos) -> bool {
    (now.y - before.y).abs() >= 1.0 || (now.content_h - before.content_h).abs() >= 1.0
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
    /// The form has a default submit button (what Enter in a field presses), named or not.
    #[serde(default, rename = "formDefaultExists")]
    form_default_exists: bool,
    /// The element submits a `<form>` when clicked: a `<button>` (no type or type=submit) or an
    /// `<input type=submit|image>` that belongs to a form, WHATEVER its name.
    #[serde(default, rename = "formSubmit")]
    form_submit: bool,
    /// Raw facts about the element that submits (see `INFO_FN`); `submit_facts` turns them into the two flags above.
    #[serde(default)]
    tag: String,
    #[serde(default)]
    btype: String,
    #[serde(default, rename = "hasForm")]
    has_form: bool,
    #[serde(default, rename = "nameEmpty")]
    name_empty: bool,
    /// The name of the element that was really hit (a span inside the button), for the send-name check only.
    #[serde(default, rename = "hitName")]
    hit_name: String,
    /// The info could not be read at all (script failed or returned garbage). Fail closed.
    #[serde(default)]
    unknown: bool,
}

/// Pure: is the element a submit button, and does it submit a form? `tag` and `btype` come from the element the
/// page script resolved (the button, not a span inside it). Returns (nameless submit in a form, submits a form).
pub(crate) fn submit_facts(tag: &str, btype: &str, has_form: bool, name_empty: bool) -> (bool, bool) {
    let tag = tag.to_ascii_uppercase();
    let ty = btype.to_ascii_lowercase();
    let submit_like = (tag == "BUTTON" && ty == "submit") || (tag == "INPUT" && (ty == "submit" || ty == "image"));
    (name_empty && has_form && submit_like, has_form && submit_like)
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
  // The element the user would really press: a click on a child of a button (<button><span>Concluir</span></button>)
  // lands on the span, but it is the button that submits. Everything below is read from that button.
  const SEL = 'button,input[type=submit],input[type=image]';
  const isSubmitEl = (e) => {
    const t = (e.tagName || '').toUpperCase();
    const ty = (e.type || '').toLowerCase();
    return (t === 'BUTTON' && ty === 'submit') || (t === 'INPUT' && (ty === 'submit' || ty === 'image'));
  };
  // Up the COMPOSED tree: out of a shadow root through its host too.
  const up = (e) => e.parentElement || (e.getRootNode && e.getRootNode().host) || null;
  let b = null;
  for (let e = el, n = 0; e && n < 100; e = up(e), n++) {
    if (e.matches && e.matches(SEL)) { b = e; break; }
  }
  if (!b) {
    // A click on <label for=...> presses its control.
    const lab = el.closest && el.closest('label');
    if (lab && lab.control && isSubmitEl(lab.control)) b = lab.control;
  }
  if (!b) b = el;
  const tag = (b.tagName || '').toUpperCase();
  const type = (b.type || '').toLowerCase();
  let form = b.form || null;
  for (let e = b, n = 0; !form && e && n < 100; e = up(e), n++) {
    if ((e.tagName || '').toUpperCase() === 'FORM') form = e;
  }
  // The default button is the first submit button in DOCUMENT order, wherever its tag sits (form="id" included).
  // form.elements leaves out <input type=image>: those of THIS form (also form="id" ones) are added, then sorted.
  let submitBtn = null;
  if (form) {
    const imgs = Array.from(document.querySelectorAll('input[type=image]')).filter((i) => i.form === form);
    const all = Array.from(form.elements).filter(isSubmitEl).concat(imgs.filter((i) => !Array.from(form.elements).includes(i)));
    all.sort((x, y) => (x.compareDocumentPosition(y) & Node.DOCUMENT_POSITION_FOLLOWING) ? -1 : 1);
    submitBtn = all[0] || null;
  }
  return JSON.stringify({
    name: nameOf(b),
    hitName: nameOf(el),
    tag: tag,
    btype: type,
    hasForm: !!form,
    nameEmpty: nameOf(b) === '',
    opaque: tag === 'IFRAME' || tag === 'FRAME',
    formDefault: submitBtn ? nameOf(submitBtn) : '',
    formDefaultExists: !!submitBtn
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
    /// Every name collected, for the send-name check (context included: a field's own name).
    names: Vec<String>,
    /// The names of the element that would act (the button): the navigation check looks only at these.
    /// On Enter, the focused text field's name is context, not part of the button.
    target_names: Vec<String>,
    structural: bool,
    /// Some examined target submits a `<form>` (see `Info::form_submit`), whatever its name.
    form_submit: bool,
}

impl Probe {
    fn add_name(&mut self, n: &str) {
        if !n.trim().is_empty() {
            self.names.push(n.to_string());
            self.target_names.push(n.to_string());
        }
    }

    /// Enter on a focused element: if it is itself a form-submit button it is the target; otherwise
    /// (a text field) its names only count for the send-name check and its structure stays structural.
    fn add_focused(&mut self, focused: Probe) {
        if focused.form_submit {
            self.names.extend(focused.names);
            self.target_names.extend(focused.target_names);
        } else {
            self.names.extend(focused.names);
        }
        self.structural |= focused.structural;
        self.form_submit |= focused.form_submit;
    }

    fn add_info(&mut self, info: &Info) {
        self.add_name(&info.name);
        // What the hit element says ("Submit application" in a span inside a type=button) counts for the send-name
        // check only; it is not the button's own name.
        if !info.hit_name.trim().is_empty() {
            self.names.push(info.hit_name.clone());
        }
        self.structural |= info.structural_submit();
        self.form_submit |= info.form_submit;
    }

    /// Enter in a field presses its form's default submit button: judge that button too. A form
    /// without one submits nothing by Enter (documented limit); a nameless one is structural.
    fn add_form_default(&mut self, info: &Info) {
        if !info.form_default_exists {
            return;
        }
        self.form_submit = true;
        if info.form_default.trim().is_empty() {
            self.structural = true;
        } else {
            self.add_name(&info.form_default);
        }
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
    /// Dialog and crash handling shared with the page-event task (see `browser::page_events`).
    page_state: Arc<PageState>,
    /// Repeated-call hint (see `loopguard`). A plain mutex: never held across an await.
    loopguard: StdMutex<LoopGuard>,
    /// Hash of what the last browser_read / browser_text returned: part of the page signature the
    /// repeated-call hint compares.
    last_content: StdMutex<u64>,
    /// (State per tools instance, which serves one agent session whose calls are sequential: it cannot cross results.)
    /// A line for the NEXT result (the viewport was adjusted ...), set where the tab is obtained and
    /// delivered by `finish`.
    pending_note: StdMutex<Option<String>>,
    /// Extra lines for a tool whose result is an image (browser_crop): the server reads them with
    /// `take_extra` and adds them as text next to the image.
    deferred_extra: StdMutex<Option<String>>,
    /// `viewport_abaixo_do_minimo` is said once per session, not on every result.
    below_warned: AtomicBool,
    /// Where the viewport guarantee last gave up (a screen smaller than the minimum): not retried at that size.
    gave_up_at: StdMutex<Option<(u32, u32)>>,
    /// True when the window was resized by the call that just obtained the tab: coordinates measured before
    /// it (a crop) are no longer valid.
    viewport_adjusted: AtomicBool,
    /// Where browser_upload gets its file from (the document library in the app).
    docs: Arc<dyn DocumentSource>,
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
            page_state: Arc::new(PageState::default()),
            loopguard: StdMutex::new(LoopGuard::default()),
            last_content: StdMutex::new(0),
            pending_note: StdMutex::new(None),
            deferred_extra: StdMutex::new(None),
            below_warned: AtomicBool::new(false),
            gave_up_at: StdMutex::new(None),
            viewport_adjusted: AtomicBool::new(false),
            docs: Arc::new(NoDocuments),
        }
    }

    /// Lines the last browser_crop could not carry in its image result (a viewport note, a repeated-call
    /// hint). Taken once.
    pub fn take_extra(&self) -> Option<String> {
        self.deferred_extra.lock().ok()?.take()
    }

    /// The document library browser_upload attaches from.
    pub fn with_documents(mut self, docs: Arc<dyn DocumentSource>) -> Self {
        self.docs = docs;
        self
    }

    /// Tests only: lets browser_open load the local fixture page.
    pub fn allow_file_urls(mut self) -> Self {
        self.allow_file_urls = true;
        self
    }

    // ── connection: the agent's OWN tab ───────────────────────────────────
    /// Every tool gets its tab here, so this is also the one place where the minimum viewport is
    /// enforced before the tool acts. The note it returns (if any) rides on that tool's result.
    async fn tab(&self) -> Result<(Arc<CdpClient>, u16, String), String> {
        let t = self.tab_inner().await?;
        self.viewport_adjusted.store(false, Ordering::SeqCst);
        let gave_up_at = self.gave_up_at.lock().ok().and_then(|g| *g);
        // An error here (window not adjustable, no layout yet) never fails the tool: just no note.
        if let Ok(report) = crate::browser::viewport::ensure_min_viewport_with(&t.0, gave_up_at).await {
            let (note, adjusted) = viewport_decision(&report, self.below_warned.load(Ordering::SeqCst));
            if report.below_minimum {
                self.below_warned.store(true, Ordering::SeqCst);
            }
            if let Ok(mut g) = self.gave_up_at.lock() {
                *g = if report.below_minimum { Some(report.after) } else { None };
            }
            self.viewport_adjusted.store(adjusted, Ordering::SeqCst);
            if let (Some(note), Ok(mut g)) = (note, self.pending_note.lock()) {
                *g = Some(note);
            }
        }
        Ok(t)
    }

    async fn tab_inner(&self) -> Result<(Arc<CdpClient>, u16, String), String> {
        let mut g = self.tab.lock().await;
        if self.page_state.take_crashed() {
            // The renderer died: drop the tab (best effort to close it) so the next browser_open starts a new one.
            if let Some(t) = g.take() {
                let _ = tokio::time::timeout(Duration::from_secs(1), t.client.call("Page.close", json!({}))).await;
            }
            // browser_open goes on and opens a new tab; any other tool stops with the way forward.
            if let Some(why) = page_events::crash_stops_tool(self.page_state.opening()) {
                return Err(why.to_string());
            }
        }
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
        // Before anything else can open a dialog: from here on a native dialog never freezes the tab.
        page_events::spawn_handler(&client, self.sink.clone(), self.page_state.clone());
        for m in ["Page.enable", "DOM.enable", "Accessibility.enable"] {
            client.call(m, json!({})).await?;
        }
        // Without it Chrome may not report `Inspector.targetCrashed`. Best effort.
        let _ = client.call("Inspector.enable", json!({})).await;
        cdp::emulate_normal_languages(&client).await?;
        if self.sink.enabled() {
            client.call("Network.enable", json!({})).await?;
            crate::browser::nav::spawn_tracker(&client, self.sink.clone(), self.agent_active.clone());
        }
        cdp::bring_to_front(state.port, &target.id, &client).await?;
        *g = Some(Tab { client: client.clone(), target_id: target.id.clone(), port: state.port });
        Ok((client, state.port, target.id))
    }

    /// Evaluates a fixed, read-only expression in the ISOLATED world (not the page's own): a page cannot
    /// redefine what it sees (`document.readyState`, `location.href`).
    async fn eval(c: &CdpClient, expr: &str) -> Result<Value, String> {
        let world = Self::guard_world(c).await?;
        let r = c.call("Runtime.evaluate", json!({ "expression": expr, "returnByValue": true, "contextId": world })).await?;
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
        book.find(&doc, reference).cloned().ok_or_else(|| {
            format!("ref \"{reference}\" não existe. Use a ref eN que browser_read mostra (ex.: e1), não id nem name do HTML; campos de arquivo aparecem como button com o nome do rótulo. Chame browser_read se a página mudou.")
        })
    }

    // ── what the page says about an element: browser-computed, never page-trusted ──
    fn parse_info(v: &Value) -> Info {
        let mut info: Info = v.as_str().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_else(Info::unreadable);
        // The raw facts, when the page script sent them, decide the two submit flags.
        if !info.tag.is_empty() {
            (info.is_submit, info.form_submit) = submit_facts(&info.tag, &info.btype, info.has_form, info.name_empty);
        }
        info
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
        let decision = guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit);
        let verdict = match decision {
            // "submit" is a whole-word pattern: whatever the real label was, the guard sees a submit.
            SubmitDecision::ConfirmAndSpend => guard::check_submit("submit", page_url, confirm, self.guard.as_ref()),
            SubmitDecision::ConfirmStructural => guard::check_confirmation(page_url, confirm, self.guard.as_ref()),
            SubmitDecision::ConfirmForm if confirm.is_none() => Verdict::Refuse(
                "RECUSADO: este botão envia um formulário. Se for o envio da candidatura, repita com confirm_submit = <id da vaga>."
                    .into(),
            ),
            SubmitDecision::ConfirmForm => guard::check_confirmation(page_url, confirm, self.guard.as_ref()),
            SubmitDecision::Allow => Verdict::Allow,
        };
        match verdict {
            Verdict::Allow => {
                // A send the guard let through: the site's own "are you sure?" may now be accepted.
                if approves_dialogs(decision) {
                    self.page_state.approve_click(page_url);
                }
                Ok(())
            }
            Verdict::Refuse(why) => Err(why),
        }
    }

    // ── screenshots (only when the Activity log is on) ───────────────────
    async fn capture(c: &CdpClient, clip: Option<Value>, scale: f64, quality: u32) -> Option<Vec<u8>> {
        let m = c.call("Page.getLayoutMetrics", json!({})).await.ok()?;
        let p = json!({ "format": "jpeg", "quality": quality, "clip": shot_clip(&m, clip, scale) });
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

    // ── what ends a result: the viewport note and the repeated-call hint ──
    /// The state of the page the model last saw: where it is and what it last read. Two calls with the
    /// same signature are "the page did not change".
    async fn page_signature(&self, c: Option<&CdpClient>) -> String {
        let content = self.last_content.lock().map(|g| *g).unwrap_or(0);
        match c {
            Some(c) => format!("{}|{}|{content}", Self::page_url(c).await, Self::doc_id(c).await),
            None => format!("-|-|{content}"),
        }
    }

    /// Extra lines for a result: a pending viewport note, then the repeated-call hint. Each starts
    /// with "\n"; empty when there is nothing to say. A hint never blocks anything.
    async fn extra_lines(&self, tool: &str, key: &str, c: Option<&CdpClient>) -> String {
        // Dialogs the app answered (during this tool or while none was running) come first.
        let mut s = self.page_state.take_notes();
        if let Ok(mut g) = self.pending_note.lock() {
            if let Some(n) = g.take() {
                s.push('\n');
                s.push_str(&n);
            }
        }
        let signature = self.page_signature(c).await;
        let hint = self.loopguard.lock().ok().and_then(|mut g| g.observe(tool, key, &signature));
        if let Some(h) = hint {
            s.push('\n');
            s.push_str(&h);
        }
        s
    }

    /// What the page showed the model on its last read: part of the "did the page change" signature.
    fn remember_content(&self, shown: &str) {
        if let Ok(mut g) = self.last_content.lock() {
            *g = hash_str(shown);
        }
    }

    /// The `estado:` line for the page the tab shows now.
    async fn state_line(c: &CdpClient, ax: &Value) -> String {
        let url = Self::page_url(c).await;
        let ready = Self::eval(c, "document.readyState").await.ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
        pagestate::detect(&url, &ready, ax).line()
    }

    // ── recording ─────────────────────────────────────────────────────────
    async fn finish_with(
        &self,
        tool: &str,
        started: Instant,
        r: Result<Outcome, String>,
        c: Option<&CdpClient>,
        alvo: Option<(String, String)>,
    ) -> Result<String, String> {
        self.agent_active.store(false, Ordering::SeqCst);
        // Every exit passes here: whatever the guard approved, or `browser_open` allowed, ends with the call.
        self.page_state.end();
        let loop_key = match &r {
            Ok(o) => o.loop_key.clone().or_else(|| o.target.clone()),
            Err(_) => None,
        }
        .unwrap_or_default();
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
                if let Some((t, r)) = alvo {
                    rec.target = Some(t);
                    rec.role = Some(r);
                }
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
        let extra = self.extra_lines(tool, &loop_key, c).await;
        if extra.is_empty() {
            return out;
        }
        if tool == "browser_crop" {
            // An image result has no text to carry the lines: the server adds them next to the image.
            if let Ok(mut g) = self.deferred_extra.lock() {
                *g = Some(extra.trim_start_matches('\n').to_string());
            }
            return out;
        }
        out.map(|m| attach_extra(&m, &extra)).map_err(|e| e + &extra)
    }

    async fn finish(&self, tool: &str, started: Instant, r: Result<Outcome, String>, c: Option<&CdpClient>) -> Result<String, String> {
        self.finish_with(tool, started, r, c, None).await
    }

    // ── tools ─────────────────────────────────────────────────────────────
    pub async fn open(&self, url: &str) -> Result<String, String> {
        let started = Instant::now();
        if let Some(why) = url_refusal(url, self.allow_file_urls, self.allow_local_hosts) {
            return self.finish("browser_open", started, Err(why), None).await;
        }
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_open");
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
                if Self::eval(&c, "document.readyState").await.map(|v| v == "complete").unwrap_or(false) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            let mut message = format!("Abriu {url}. Chame browser_read para ver a página.");
            // The state of the page we landed on (a redirect may have taken us to a login wall). If the
            // tree cannot be read the line is simply left out: opening still worked.
            if let Ok(ax) = c.call("Accessibility.getFullAXTree", json!({})).await {
                message.push('\n');
                message.push_str(&Self::state_line(&c, &ax).await);
            }
            Ok(Outcome { message, target: Some(url.to_string()), ..Default::default() })
        }
        .await;
        self.finish("browser_open", started, r, Some(&c)).await
    }

    pub async fn read(&self, scope: Option<&str>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_read");
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
            self.remember_content(&text);
            text.push('\n');
            text.push_str(&Self::state_line(&c, &ax).await);
            Ok(Outcome { message: text, loop_key: Some(scope.unwrap_or("").to_string()), ..Default::default() })
        }
        .await;
        self.finish("browser_read", started, r, Some(&c)).await
    }

    /// The text of the page (spec 3.2), from the accessibility tree, paginated by characters. Independent
    /// of the viewport and of scrolling; runs no JavaScript in the page and never reads what was typed
    /// into a field.
    pub async fn text(&self, scope: Option<&str>, offset: Option<usize>, max_chars: Option<usize>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_text");
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_text", started, Err(e), None).await,
        };
        let r = async {
            let ax = c.call("Accessibility.getFullAXTree", json!({})).await?;
            let sc = match text::ScopeArg::parse(scope) {
                text::ScopeArg::Page => text::Scope::Page,
                text::ScopeArg::Role(s) => text::Scope::Role(s),
                text::ScopeArg::Ref(r) => text::Scope::Backend(self.resolve_ref(&c, &r).await?.backend_node_id),
            };
            let full = text::extract(&ax, &sc)?;
            self.remember_content(&full);
            let offset = offset.unwrap_or(0);
            let page = text::paginate(&full, offset, max_chars.unwrap_or(0));
            let mut message = text::render(&page);
            message.push('\n');
            message.push_str(&Self::state_line(&c, &ax).await);
            Ok(Outcome { message, loop_key: Some(format!("{}|{offset}", scope.unwrap_or(""))), ..Default::default() })
        }
        .await;
        self.finish("browser_text", started, r, Some(&c)).await
    }

    // ── scrolling (spec 3.5): the mouse wheel, no JavaScript in the page ──
    async fn scroll_pos(c: &CdpClient) -> Result<ScrollPos, String> {
        let m = c.call("Page.getLayoutMetrics", json!({})).await?;
        let v = &m["cssVisualViewport"];
        Ok(ScrollPos {
            y: v["pageY"].as_f64().unwrap_or(0.0),
            client_w: v["clientWidth"].as_f64().unwrap_or(0.0),
            client_h: v["clientHeight"].as_f64().unwrap_or(0.0),
            content_h: m["cssContentSize"]["height"].as_f64().unwrap_or(0.0),
        })
    }

    /// Where the wheel turns when no element was asked for: the middle of the page, unless a form field is
    /// there. A wheel over a focused number field changes its value ("5 anos" becomes 4), so the first
    /// candidate point that Chrome says is not a field is used (the middle if none is found).
    async fn wheel_point(c: &CdpClient, w: f64, h: f64) -> (f64, f64) {
        for (x, y) in wheel_candidates(w, h) {
            let on_field = match Self::node_at_point(c, x, y).await {
                Some(id) => c
                    .call("DOM.describeNode", json!({ "backendNodeId": id }))
                    .await
                    .ok()
                    .and_then(|d| d["node"]["nodeName"].as_str().map(is_field_node))
                    .unwrap_or(false),
                None => false,
            };
            if !on_field {
                return (x, y);
            }
        }
        (w / 2.0, h / 2.0)
    }

    async fn wheel(c: &CdpClient, x: f64, y: f64, delta_y: f64) -> Result<(), String> {
        c.call("Input.dispatchMouseEvent", json!({ "type": "mouseMoved", "x": x, "y": y })).await?;
        c.call("Input.dispatchMouseEvent", json!({ "type": "mouseWheel", "x": x, "y": y, "deltaX": 0, "deltaY": delta_y })).await?;
        Ok(())
    }

    /// Waits until the scroll position and the page height stop changing (smooth scrolling animates).
    /// `before` is the position when the wheel was sent: the wheel is handled asynchronously by the
    /// browser (and on a loaded machine the animation can start late), so a position that has not moved
    /// yet is first given up to ~0.5 s to START moving. Without this the "still" check below read the
    /// not-yet-started scroll as a finished one and the tool reported the page unmoved (CLA-38).
    async fn settle(c: &CdpClient, before: &ScrollPos) -> Result<ScrollPos, String> {
        let mut last = Self::scroll_pos(c).await?;
        for _ in 0..SCROLL_START_POLLS {
            if scroll_moved(before, &last) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(60)).await;
            last = Self::scroll_pos(c).await?;
        }
        for _ in 0..12 {
            tokio::time::sleep(Duration::from_millis(80)).await;
            let now = Self::scroll_pos(c).await?;
            let still = (now.y - last.y).abs() < 0.5 && (now.content_h - last.content_h).abs() < 0.5;
            last = now;
            if still {
                break;
            }
        }
        Ok(last)
    }

    /// The top edge of an element's content box, in viewport coordinates (None when it cannot be measured).
    async fn box_top(c: &CdpClient, backend_id: i64) -> Option<f64> {
        let m = c.call("DOM.getBoxModel", json!({ "backendNodeId": backend_id })).await.ok()?;
        m["model"]["content"].as_array()?.get(1)?.as_f64()
    }

    pub async fn scroll(&self, direction: &str, pages: Option<f64>, reference: Option<&str>) -> Result<String, String> {
        let started = Instant::now();
        if !matches!(direction, "down" | "up" | "top" | "bottom") {
            let why = format!("direção inválida \"{direction}\": use down, up, top ou bottom");
            return self.finish("browser_scroll", started, Err(why), None).await;
        }
        let pages = pages.unwrap_or(0.8);
        if !pages.is_finite() || pages <= 0.0 {
            return self.finish("browser_scroll", started, Err("pages deve ser um número maior que 0".into()), None).await;
        }
        let pages = pages.min(5.0);
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_scroll");
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_scroll", started, Err(e), None).await,
        };
        let r = async {
            // Where the wheel turns: over the element (its scrollable container moves) or the middle of the page.
            let element = match reference {
                Some(r) => Some(self.resolve_ref(&c, r).await?),
                None => None,
            };
            let (x, y) = match &element {
                Some(n) => {
                    let (_, x, y) = self.locate(&c, n, false).await?; // scrolls it into view, then measures
                    (x, y)
                }
                None => {
                    let p = Self::scroll_pos(&c).await?;
                    Self::wheel_point(&c, p.client_w, p.client_h).await
                }
            };
            let start = Self::scroll_pos(&c).await?;
            let start_top = match &element {
                Some(n) => Self::box_top(&c, n.backend_node_id).await,
                None => None,
            };
            let sign = if matches!(direction, "down" | "bottom") { 1.0 } else { -1.0 };
            let one_step = start.client_h.max(200.0) * pages;
            let to_the_end = matches!(direction, "top" | "bottom");
            let begun = Instant::now();
            let mut pos = start;
            let mut top = start_top;
            for _ in 0..30 {
                let (before, before_top) = (pos, top);
                let delta = if to_the_end { start.client_h.max(200.0) * 3.0 } else { one_step };
                Self::wheel(&c, x, y, sign * delta).await?;
                pos = Self::settle(&c, &before).await?;
                top = match &element {
                    Some(n) => Self::box_top(&c, n.backend_node_id).await,
                    None => None,
                };
                let page_moved = (pos.y - before.y).abs() >= 1.0 || (pos.content_h - before.content_h).abs() >= 1.0;
                let element_moved = matches!((before_top, top), (Some(a), Some(b)) if (a - b).abs() >= 1.0);
                if !to_the_end || !(page_moved || element_moved) || begun.elapsed() > Duration::from_secs(8) {
                    break;
                }
            }
            let message = match (&element, reference) {
                (Some(_), Some(r)) => {
                    let moved = match (start_top, top) {
                        (Some(a), Some(b)) => (a - b).abs() >= 1.0,
                        _ => (pos.y - start.y).abs() >= 1.0,
                    };
                    format!(
                        "Rolou o contêiner de {r} {direction}. moveu: {moved}{}. Chame browser_read para ver o que apareceu.",
                        if moved { "" } else { " (já está no limite nessa direção)" }
                    )
                }
                _ => {
                    let amount = if to_the_end { String::new() } else { format!(" {pages} tela(s)") };
                    // The document's position is all that is measured here: a panel inside the page may have scrolled.
                    let fits = if (pos.y - start.y).abs() < 1.0 { " A posição do documento não mudou; um painel interno pode ter rolado." } else { "" };
                    format!(
                        "Rolou {direction}{amount}. Posição: {}% (y={} de {}). fim_da_pagina: {}.{fits} Chame browser_read para ver o que apareceu.",
                        pos.percent(),
                        pos.y.round(),
                        pos.max_y().round(),
                        pos.at_end()
                    )
                }
            };
            Ok(Outcome { message, target: Some(direction.to_string()), loop_key: Some(scroll_loop_key(direction, reference, &pos, top)), ..Default::default() })
        }
        .await;
        self.finish("browser_scroll", started, r, Some(&c)).await
    }

    /// Attaches a library document to a file field (or to the button that opens the file chooser).
    /// The agent gives a ref and at most a document id; the file is chosen and checked by the app.
    /// Every failure is a tool error: the chooser interception is always released, nothing reloads or
    /// relaunches the browser, and a button that would submit the application is never clicked.
    pub async fn upload(&self, reference: &str, tipo: Option<&str>, documento: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_upload");
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_upload", started, Err(e), None).await,
        };
        let mut alvo: Option<(String, String)> = None;
        let r = async {
            let tipo = tipo.unwrap_or("curriculo");
            if tipo != "curriculo" && tipo != "carta" {
                return Err(format!("tipo inválido: {tipo} (use curriculo ou carta)"));
            }
            let vaga = self
                .guard
                .active_vaga()
                .ok_or_else(|| "RECUSADO: anexar exige exatamente uma vaga em 'candidatando' (a vaga sendo candidatada).".to_string())?;
            let att = self.docs.for_upload(vaga, tipo, documento)?;
            let n = self.resolve_ref(&c, reference).await?;
            alvo = Some((n.name.clone(), n.role.clone()));
            let file = att.path.to_string_lossy().to_string();
            let (shot, x, y) = self.locate(&c, &n, true).await?;
            let field = if Self::is_file_input(&c, n.backend_node_id).await {
                tokio::time::timeout(UPLOAD_STEP, c.call("DOM.setFileInputFiles", json!({ "files": [file], "backendNodeId": n.backend_node_id })))
                    .await
                    .map_err(|_| "o campo de arquivo não respondeu a tempo".to_string())??;
                n.backend_node_id
            } else {
                // A button that opens the chooser: it must not be a submit (an upload never submits).
                let mut probe = Probe::default();
                probe.add_name(&n.name);
                Self::probe_node(&c, n.backend_node_id, &mut probe).await;
                match Self::node_at_point(&c, x, y).await {
                    Some(hit) if hit != n.backend_node_id => Self::probe_node(&c, hit, &mut probe).await,
                    Some(_) => {}
                    None => probe.structural = true,
                }
                let url = Self::page_url(&c).await;
                if self.run_guard(&probe, &url, None).is_err() {
                    return Err("RECUSADO: esse elemento envia a candidatura ou um formulário; browser_upload só anexa arquivos. Se o campo de arquivo aparece no browser_read, use a ref dele em vez da do botão.".into());
                }
                Self::upload_via_chooser(&c, x, y, &file).await?
            };
            tokio::time::sleep(Duration::from_millis(300)).await;
            let file_name = att.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            // The native control keeps the chosen file to itself (it never reaches innerText): ask the field.
            let (held, accept) = Self::file_field_state(&c, field).await;
            let probe_js = format!("document.body ? document.body.innerText.includes({}) : false", serde_json::to_string(&file_name).unwrap_or_default());
            let on_page = Self::eval(&c, &probe_js).await.ok().and_then(|v| v.as_bool()).unwrap_or(false);
            let mut message = format!("Anexou \"{}\" (arquivo {}, {}, {} bytes) pela regra: {}.", att.nome, file_name, att.formato, att.tamanho, att.regra);
            if !accept_allows(&accept, &att.formato) {
                message.push_str(&format!(" Atenção: o campo aceita só {accept} e o arquivo é {}; o site provavelmente vai recusá-lo. Confira com browser_text e, se recusado, registre uma pendência.", att.formato));
            } else if held != file_name && !on_page {
                message.push_str(" Atenção: o campo não ficou com o arquivo e a página não mostrou o nome dele; confira com browser_read ou browser_text antes de seguir.");
            }
            // Remember which document went to this vaga, unless the field's own accept rule refuses its format.
            if accept_allows(&accept, &att.formato) {
                self.docs.attached(vaga, tipo, att.id);
            }
            // Add the state line after constructing the message.
            if let Ok(ax) = c.call("Accessibility.getFullAXTree", json!({})).await {
                message.push('\n');
                message.push_str(&Self::state_line(&c, &ax).await);
            }
            Ok(Outcome {
                message,
                target: Some(n.name),
                role: Some(n.role),
                // Never the content: name, format, size and a short hash only.
                text: Some(upload_log_text(&att)),
                shot_before: shot,
                ..Default::default()
            })
        }
        .await;
        self.finish_with("browser_upload", started, r, Some(&c), alvo).await
    }

    /// (name of the file the field holds, its accept attribute), read in the isolated world. Empty when unreadable.
    async fn file_field_state(c: &CdpClient, backend: i64) -> (String, String) {
        let r = async {
            let world = Self::guard_world(c).await?;
            let o = c.call("DOM.resolveNode", json!({ "backendNodeId": backend, "executionContextId": world })).await?;
            let oid = o["object"]["objectId"].as_str().ok_or("sem objeto")?.to_string();
            let v = c
                .call(
                    "Runtime.callFunctionOn",
                    json!({ "objectId": oid, "returnByValue": true, "functionDeclaration": "function(){return {name: (this.files && this.files[0]) ? this.files[0].name : '', accept: this.accept || ''}}" }),
                )
                .await?;
            let val = &v["result"]["value"];
            Ok::<_, String>((val["name"].as_str().unwrap_or("").to_string(), val["accept"].as_str().unwrap_or("").to_string()))
        }
        .await;
        r.unwrap_or_default()
    }

    async fn is_file_input(c: &CdpClient, backend: i64) -> bool {
        let Ok(d) = c.call("DOM.describeNode", json!({ "backendNodeId": backend })).await else { return false };
        let node = &d["node"];
        if node["nodeName"].as_str() != Some("INPUT") {
            return false;
        }
        let attrs = node["attributes"].as_array().cloned().unwrap_or_default();
        attrs.chunks(2).any(|kv| {
            kv.first().and_then(|k| k.as_str()) == Some("type") && kv.get(1).and_then(|v| v.as_str()).is_some_and(|v| v.eq_ignore_ascii_case("file"))
        })
    }

    /// Clicks a button that opens the file chooser, intercepting the chooser so no native dialog ever
    /// appears. The interception is ALWAYS turned off again, success or not: the tab must stay usable.
    async fn upload_via_chooser(c: &CdpClient, x: f64, y: f64, file: &str) -> Result<i64, String> {
        let mut events = c.subscribe();
        let _ = c.call("Page.enable", json!({})).await;
        c.call("Page.setInterceptFileChooserDialog", json!({ "enabled": true })).await?;
        let result = async {
            Self::mouse_click(c, x, y).await?;
            let backend = tokio::time::timeout(UPLOAD_STEP, async {
                loop {
                    match events.recv().await {
                        Ok(ev) if ev["method"] == "Page.fileChooserOpened" => return ev["params"]["backendNodeId"].as_i64(),
                        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => return None,
                    }
                }
            })
            .await
            .map_err(|_| "o elemento não abriu o seletor de arquivo (tempo esgotado)".to_string())?
            .ok_or_else(|| "o seletor de arquivo abriu sem um campo de arquivo associado".to_string())?;
            tokio::time::timeout(UPLOAD_STEP, c.call("DOM.setFileInputFiles", json!({ "files": [file], "backendNodeId": backend })))
                .await
                .map_err(|_| "o campo de arquivo não respondeu a tempo".to_string())??;
            Ok(backend)
        }
        .await;
        let _ = c.call("Page.setInterceptFileChooserDialog", json!({ "enabled": false })).await;
        result
    }

    pub async fn click(&self, reference: &str, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_click");
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
        self.page_state.begin("browser_type");
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
                ..Default::default()
            })
        }
        .await;
        self.finish("browser_type", started, r, Some(&c)).await
    }

    pub async fn key(&self, key: &str, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        self.page_state.begin("browser_key");
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
                        let mut focused = Probe::default();
                        Self::probe_node(&c, id, &mut focused).await;
                        probe.add_focused(focused);
                        if code == "Enter" {
                            let info = Self::info_for_node(&c, id).await;
                            probe.add_form_default(&info);
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
        self.page_state.begin("browser_crop");
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
                let key = crop_loop_key(x, y, w, h);
                let _ = self.finish("browser_crop", started, Ok(Outcome { message: "recorte".into(), loop_key: Some(key), ..Default::default() }), Some(&c)).await;
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
        self.page_state.begin("browser_click_at");
        let (c, _, _) = match self.tab().await {
            Ok(t) => t,
            Err(e) => return self.finish("browser_click_at", started, Err(e), None).await,
        };
        if let Some(why) = click_at_after_resize_refusal(self.viewport_adjusted.load(Ordering::SeqCst)) {
            return self.finish("browser_click_at", started, Err(why), Some(&c)).await;
        }
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

/// Whether a guard decision that ended in `Allow` was a SEND the guard let through (a valid `confirm_submit`),
/// as opposed to an ordinary action. Only the former may accept the site's own `confirm()` (policy P2).
pub(crate) fn approves_dialogs(decision: SubmitDecision) -> bool {
    !matches!(decision, SubmitDecision::Allow)
}

/// Points to try for the wheel, middle first, then the sides and the quarters of the page.
pub(crate) fn wheel_candidates(w: f64, h: f64) -> Vec<(f64, f64)> {
    vec![(w / 2.0, h / 2.0), (w * 0.25, h / 2.0), (w * 0.75, h / 2.0), (w / 2.0, h * 0.25), (w / 2.0, h * 0.75), (w * 0.1, h * 0.1), (w * 0.9, h * 0.9)]
}

/// Form fields whose value a wheel can change.
pub(crate) fn is_field_node(node_name: &str) -> bool {
    matches!(node_name.to_ascii_uppercase().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
}

/// What a viewport report means for this call: the note to show (the below-minimum warning only the first
/// time) and whether the window was resized right now.
pub(crate) fn viewport_decision(r: &crate::browser::viewport::ViewportReport, already_warned: bool) -> (Option<String>, bool) {
    let note = if r.below_minimum && already_warned { None } else { r.note() };
    (note, r.adjusted)
}

/// A `browser_click_at` right after the window was resized: the coordinates came from a crop of the old
/// layout, so the click could land on something else. Refused with a way forward.
pub(crate) fn click_at_after_resize_refusal(adjusted: bool) -> Option<String> {
    adjusted.then(|| "A janela foi redimensionada agora (veja a nota de viewport): as coordenadas que você mediu eram do layout antigo. Chame browser_crop de novo e use as novas coordenadas.".to_string())
}

/// Appends the viewport note / repeated-call hint (`extra`, each line starting with "\n") to a result
/// BEFORE its closing `estado:` line, so `estado:` is always the last line. Page text can never put a
/// line like it there: `text::neutralize` prefixes those, and the real one is the last line of `msg`.
pub(crate) fn attach_extra(msg: &str, extra: &str) -> String {
    match msg.rfind('\n') {
        Some(i) if msg[i + 1..].starts_with("estado:") => format!("{}{}\n{}", &msg[..i], extra, &msg[i + 1..]),
        _ => format!("{msg}{extra}"),
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
    #[test]
    fn the_upload_line_in_the_activity_log_names_the_document_and_its_id() {
        let att = super::Attachment {
            id: 42,
            nome: "CV Backend".into(),
            formato: "pdf".into(),
            tamanho: 1234,
            sha256: "0123456789abcdef0123".into(),
            path: "x.pdf".into(),
            regra: "r".into(),
        };
        let t = super::upload_log_text(&att);
        assert!(t.starts_with("documento 42 | CV Backend | pdf | 1234 bytes"), "{t}");
        assert!(!t.contains("x.pdf"), "never a path");
    }

    #[test]
    fn a_file_field_accept_rule_is_read_like_the_browser_does() {
        use super::accept_allows;
        assert!(accept_allows("", "pdf"), "no accept: anything");
        assert!(accept_allows(".pdf", "pdf"));
        assert!(accept_allows(".PDF, .doc", "pdf"), "case and spaces");
        assert!(accept_allows("application/pdf", "pdf"));
        assert!(accept_allows("*/*", "docx"));
        assert!(accept_allows(".docx", "docx"));
        assert!(accept_allows("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "docx"));
        assert!(!accept_allows(".pdf", "docx"));
        assert!(!accept_allows("image/*", "pdf"));
    }

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
        // PENDING SANDBOX: creates a folder (test_support::temp_dir), so it does not run on the user's PC.
        let dir = crate::mcp::tools::test_support::temp_dir("sites-allowed");
        let ctx = DbGuardContext::new(dir);
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
    fn submit_facts_are_read_from_the_button_and_a_child_of_it_is_not_a_way_around() {
        // Pure. A click_at on <span> inside <button> resolves to the BUTTON before this is called.
        assert_eq!(submit_facts("BUTTON", "submit", true, false), (false, true));
        assert_eq!(submit_facts("BUTTON", "submit", true, true), (true, true), "nameless: structural");
        assert_eq!(submit_facts("INPUT", "image", true, false), (false, true));
        assert_eq!(submit_facts("BUTTON", "button", true, false), (false, false));
        assert_eq!(submit_facts("BUTTON", "submit", false, false), (false, false), "no form: nothing to submit");
        // The span itself is never a submit: this is what the old script saw and why it let the click through.
        assert_eq!(submit_facts("SPAN", "", true, false), (false, false));
        let info = BrowserTools::parse_info(&json!(r#"{"name":"Concluir","tag":"BUTTON","btype":"submit","hasForm":true}"#));
        let mut p = Probe::default();
        p.add_name(&info.name);
        p.add_info(&info);
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::ConfirmForm);
    }

    #[test]
    fn a_send_name_on_the_hit_child_of_a_plain_button_still_counts_as_a_send() {
        // <button type=button aria-label="x"><span>Submit application</span></button>: the button is called "x",
        // the span says what it does. Pure: strings and structs.
        let info = BrowserTools::parse_info(&json!(r#"{"name":"x","hitName":"Submit application","tag":"BUTTON","btype":"button","hasForm":false}"#));
        let mut p = Probe::default();
        p.add_info(&info);
        assert_eq!(p.target_names, vec!["x".to_string()], "the hit name is not the button's name");
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::ConfirmAndSpend);
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
    fn form_submit_facts_reach_the_probe_and_the_pure_decision() {
        // Pure: only strings and structs, no page, no disk.
        let upload = BrowserTools::parse_info(&json!(r#"{"name":"Upload","isSubmit":false,"formSubmit":true,"formDefaultExists":true,"formDefault":"Upload"}"#));
        let mut p = Probe::default();
        p.add_info(&upload);
        assert!(p.form_submit && !p.structural);
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::ConfirmForm);

        let next = BrowserTools::parse_info(&json!(r#"{"name":"Next","formSubmit":true}"#));
        let mut p = Probe::default();
        p.add_name("Next");
        p.add_info(&next);
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::Allow);

        // Enter: the form's default button is judged as well; a nameless one is structural.
        let field = BrowserTools::parse_info(&json!(r#"{"name":"Email","formDefaultExists":true,"formDefault":"Concluir"}"#));
        let mut p = Probe::default();
        p.add_info(&field);
        p.add_form_default(&field);
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::ConfirmForm);
        let nameless = BrowserTools::parse_info(&json!(r#"{"name":"Email","formDefaultExists":true,"formDefault":""}"#));
        let mut p = Probe::default();
        p.add_form_default(&nameless);
        assert!(p.structural);
        // The focused text field's own name is context: a form whose default button is "Próximo" passes.
        let campo = BrowserTools::parse_info(&json!(r#"{"name":"Campo B","formDefaultExists":true,"formDefault":"Próximo"}"#));
        let mut focused = Probe::default();
        focused.add_info(&campo);
        let mut p = Probe::default();
        p.add_focused(focused);
        p.add_form_default(&campo);
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::Allow);
        // No default button: nothing to judge (documented limit).
        let none = BrowserTools::parse_info(&json!(r#"{"name":"Search"}"#));
        let mut p = Probe::default();
        p.add_info(&none);
        p.add_form_default(&none);
        assert_eq!(guard::classify_submit(&p.names, &p.target_names, p.structural, p.form_submit), SubmitDecision::Allow);
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
        for bad in ["file:///y:/inventado/arquivo.txt", "javascript:alert(1)", "chrome://settings", "data:text/html,x", "ftp://x/", "about:blank"] {
            assert!(url_refusal(bad, false, false).is_some(), "{bad}");
        }
        assert!(url_refusal("file:///y:/inventado/x.html", true, false).is_none(), "file URLs only when explicitly allowed (tests)");
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

    #[test]
    fn only_a_send_the_guard_let_through_may_accept_a_confirm_dialog() {
        assert!(!approves_dialogs(SubmitDecision::Allow), "an ordinary click never approves a confirm");
        for d in [SubmitDecision::ConfirmAndSpend, SubmitDecision::ConfirmStructural, SubmitDecision::ConfirmForm] {
            assert!(approves_dialogs(d), "{d:?}");
        }
    }

    fn probe_of(names: &[&str], structural: bool) -> Probe {
        let names: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        Probe { target_names: names.clone(), names, structural, ..Default::default() }
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

    fn ok(message: &str) -> Result<Outcome, String> {
        Ok(Outcome { message: message.into(), ..Default::default() })
    }

    fn ok_target(message: &str, target: &str) -> Result<Outcome, String> {
        Ok(Outcome { message: message.into(), target: Some(target.into()), ..Default::default() })
    }

    #[test]
    fn the_wheel_avoids_form_fields_and_starts_in_the_middle() {
        let c = wheel_candidates(1200.0, 800.0);
        assert_eq!(c[0], (600.0, 400.0), "the middle comes first");
        assert!(c.len() >= 5, "there are other points to fall back to");
        assert!(c.iter().all(|(x, y)| (0.0..=1200.0).contains(x) && (0.0..=800.0).contains(y)));
        for n in ["input", "INPUT", "textarea", "Select"] {
            assert!(is_field_node(n), "{n}");
        }
        for n in ["DIV", "BODY", "BUTTON", "A"] {
            assert!(!is_field_node(n), "{n}");
        }
    }

    #[test]
    fn the_below_minimum_warning_is_said_once_and_a_resize_is_known() {
        use crate::browser::viewport::ViewportReport;
        let below = ViewportReport { before: (1100, 700), after: (1100, 700), adjusted: false, below_minimum: true };
        assert_eq!(viewport_decision(&below, false), (Some("viewport_abaixo_do_minimo: 1100x700".into()), false));
        assert_eq!(viewport_decision(&below, true), (None, false), "second time: silence");
        let grown = ViewportReport { before: (929, 861), after: (1200, 861), adjusted: true, below_minimum: false };
        assert_eq!(viewport_decision(&grown, true), (Some("viewport ajustado: 929x861 -> 1200x861".into()), true), "a resize is always said");
        let fine = ViewportReport { before: (1900, 900), after: (1900, 900), adjusted: false, below_minimum: false };
        assert_eq!(viewport_decision(&fine, false), (None, false));
    }

    #[test]
    fn a_click_at_right_after_a_resize_is_refused_with_a_way_forward() {
        let why = click_at_after_resize_refusal(true).expect("refused");
        assert!(why.contains("browser_crop") && why.contains("coordenadas"), "{why}");
        assert_eq!(click_at_after_resize_refusal(false), None);
    }

    #[test]
    fn the_estado_line_stays_last_and_notes_go_before_it() {
        assert_eq!(attach_extra("lista\nestado: ok", "\nviewport ajustado: 1x1 -> 2x2"), "lista\nviewport ajustado: 1x1 -> 2x2\nestado: ok");
        assert_eq!(attach_extra("lista", "\nnota"), "lista\nnota", "no estado line: appended as before");
        let two = attach_extra("a\nestado: ok", "\nn1\nn2");
        assert_eq!(two.lines().last(), Some("estado: ok"), "{two}");
    }

    #[tokio::test]
    async fn a_pending_viewport_note_rides_on_the_next_result_once() {
        let dir = temp_dir("note-once");
        let t = tools_with(&dir);
        *t.pending_note.lock().unwrap() = Some("viewport ajustado: 929x861 -> 1200x800".into());
        let first = t.finish("browser_read", Instant::now(), ok("lista"), None).await.unwrap();
        assert_eq!(first, "lista\nviewport ajustado: 929x861 -> 1200x800");
        let second = t.finish("browser_read", Instant::now(), ok("lista"), None).await.unwrap();
        assert_eq!(second, "lista", "the note is delivered once, not on every later result");
    }

    #[tokio::test]
    async fn the_fourth_identical_call_carries_the_hint_and_still_succeeds() {
        let dir = temp_dir("loop-hint");
        let t = tools_with(&dir);
        for i in 1..=3 {
            let m = t.finish("browser_key", Instant::now(), ok_target("Pressionou Tab.", "Tab"), None).await.unwrap();
            assert_eq!(m, "Pressionou Tab.", "call {i} has no hint");
        }
        let m = t.finish("browser_key", Instant::now(), ok_target("Pressionou Tab.", "Tab"), None).await.unwrap();
        assert!(m.starts_with("Pressionou Tab.") && m.contains("repetiu browser_key 4 vezes") && m.contains("browser_text"), "{m}");
    }

    #[tokio::test]
    async fn a_hint_on_a_refusal_keeps_the_refusal_first_and_it_is_still_an_error() {
        let dir = temp_dir("loop-hint-err");
        let t = tools_with(&dir);
        let mut last = Ok(String::new());
        for _ in 0..4 {
            last = t.finish("browser_click", Instant::now(), Err("RECUSADO: este botão envia a candidatura.".into()), None).await;
        }
        let e = last.unwrap_err();
        assert!(e.starts_with("RECUSADO"), "the refusal text stays first: {e}");
        assert!(e.contains("repetiu browser_click 4 vezes"), "{e}");
    }

    #[tokio::test]
    async fn the_hint_resets_when_what_the_page_shows_changes() {
        let dir = temp_dir("loop-reset");
        let t = tools_with(&dir);
        for _ in 0..3 {
            t.finish("browser_key", Instant::now(), ok_target("x", "Tab"), None).await.unwrap();
        }
        t.remember_content("a different page content"); // what browser_read / browser_text do after reading
        let m = t.finish("browser_key", Instant::now(), ok_target("x", "Tab"), None).await.unwrap();
        assert_eq!(m, "x", "the page changed: the count starts again");
    }

    #[tokio::test]
    async fn a_crop_defers_its_extra_lines_to_the_server_because_an_image_cannot_carry_text() {
        let dir = temp_dir("crop-extra");
        let t = tools_with(&dir);
        *t.pending_note.lock().unwrap() = Some("viewport ajustado".into());
        let m = t.finish("browser_crop", Instant::now(), ok("recorte"), None).await.unwrap();
        assert_eq!(m, "recorte", "the crop's own result is untouched");
        assert_eq!(t.take_extra().as_deref(), Some("viewport ajustado"));
        assert_eq!(t.take_extra(), None, "taken once");
    }

    #[test]
    fn an_identical_crop_is_a_repeat_and_a_different_rectangle_is_not() {
        // The crop's loop key is its rectangle: moving the crop around the page is exploring, not looping.
        let mut g = LoopGuard::default();
        for i in 0..6 {
            assert_eq!(g.observe("browser_crop", &crop_loop_key(0.0, i as f64 * 100.0, 300.0, 200.0), "s"), None);
        }
        let mut g = LoopGuard::default();
        let same = crop_loop_key(0.0, 100.0, 300.0, 200.0);
        let hints = (0..4).filter(|_| g.observe("browser_crop", &same, "s").is_some()).count();
        assert_eq!(hints, 1, "the same rectangle four times is a repeat");
    }

    #[test]
    fn a_log_screenshot_of_a_scrolled_page_shows_what_is_on_screen_not_the_top_of_the_document() {
        // Page.captureScreenshot's clip is in DOCUMENT coordinates. The Haiku round's shot after a scroll
        // (y=420) came out with its top 420 px blank: it clipped the top of the document, not the screen.
        let m = json!({ "cssVisualViewport": { "pageX": 0.0, "pageY": 420.0, "clientWidth": 1185.0, "clientHeight": 800.0 } });
        let whole = shot_clip(&m, None, 0.5);
        assert_eq!((whole["x"].as_f64(), whole["y"].as_f64()), (Some(0.0), Some(420.0)), "{whole}");
        assert_eq!((whole["width"].as_f64(), whole["height"].as_f64(), whole["scale"].as_f64()), (Some(1185.0), Some(800.0), Some(0.5)));
        // A "before" crop comes from DOM.getBoxModel, which is in VIEWPORT coordinates.
        let boxed = shot_clip(&m, Some(json!({ "x": 0.0, "y": 89.0, "width": 1249.0, "height": 140.0 })), 1.0);
        assert_eq!((boxed["x"].as_f64(), boxed["y"].as_f64()), (Some(0.0), Some(509.0)), "{boxed}");
        assert_eq!((boxed["width"].as_f64(), boxed["height"].as_f64()), (Some(1249.0), Some(140.0)));
        let unscrolled = json!({ "cssVisualViewport": { "pageX": 0.0, "pageY": 0.0, "clientWidth": 1200.0, "clientHeight": 800.0 } });
        assert_eq!(shot_clip(&unscrolled, None, 1.0)["y"].as_f64(), Some(0.0));
    }

    #[test]
    fn scrolls_that_move_the_page_are_not_a_loop_and_scrolls_stuck_in_place_are() {
        let at = |y: f64, content_h: f64| ScrollPos { y, client_w: 1200.0, client_h: 800.0, content_h };
        let mut g = LoopGuard::default();
        for (y, h) in [(420.0, 1640.0), (1060.0, 1640.0), (1640.0, 2860.0), (2280.0, 2860.0), (2860.0, 4080.0)] {
            assert_eq!(g.observe("browser_scroll", &scroll_loop_key("down", None, &at(y, h), None), "s"), None, "y={y}");
        }
        let mut g = LoopGuard::default();
        let stuck = scroll_loop_key("down", None, &at(3280.0, 4080.0), None);
        for _ in 0..3 {
            assert_eq!(g.observe("browser_scroll", &stuck, "s"), None);
        }
        assert!(g.observe("browser_scroll", &stuck, "s").is_some(), "four scrolls that did not move the page");
    }

    #[tokio::test]
    async fn scroll_rejects_bad_arguments_before_touching_the_browser() {
        let dir = temp_dir("scroll-args");
        let t = tools_with(&dir);
        let e = t.scroll("sideways", None, None).await.unwrap_err();
        assert!(e.contains("direção inválida") && e.contains("down"), "{e}");
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let e = t.scroll("down", Some(bad), None).await.unwrap_err();
            assert!(e.contains("maior que 0"), "{bad}: {e}");
        }
    }

    #[test]
    fn scroll_position_math() {
        let top = ScrollPos { y: 0.0, client_w: 1200.0, client_h: 800.0, content_h: 3000.0 };
        assert_eq!((top.max_y(), top.percent(), top.at_end()), (2200.0, 0, false));
        let mid = ScrollPos { y: 1100.0, ..top };
        assert_eq!((mid.percent(), mid.at_end()), (50, false));
        let end = ScrollPos { y: 2199.0, ..top };
        assert!(end.at_end(), "within 2px of the end counts as the end");
        let fits = ScrollPos { y: 0.0, client_w: 1200.0, client_h: 800.0, content_h: 500.0 };
        assert_eq!((fits.max_y(), fits.percent(), fits.at_end()), (0.0, 100, true), "a page that fits has nothing to scroll");
    }

    #[test]
    fn a_scroll_counts_as_started_once_the_document_moved_or_changed_height() {
        let at = ScrollPos { y: 0.0, client_w: 1200.0, client_h: 800.0, content_h: 3000.0 };
        assert!(!scroll_moved(&at, &at), "the same reading: the wheel has not taken effect yet");
        assert!(!scroll_moved(&at, &ScrollPos { y: 0.4, ..at }), "sub-pixel noise is not movement");
        assert!(scroll_moved(&at, &ScrollPos { y: 120.0, ..at }), "the page moved");
        assert!(scroll_moved(&at, &ScrollPos { content_h: 3400.0, ..at }), "a page that grew (infinite scroll) counts");
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
