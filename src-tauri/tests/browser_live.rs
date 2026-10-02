//! Live tests: a real Chrome (headless) against tests/fixtures/apply-form.html.
//! Run: CLAUDIA_TEST_CHROME="C:/path/chrome.exe" cargo test --test browser_live -- --ignored --test-threads=1
use claudia_rh_lib::browser::cdp;
use claudia_rh_lib::browser::manager::{self, LaunchOptions};
use claudia_rh_lib::browser::refs;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn chrome() -> PathBuf {
    PathBuf::from(std::env::var("CLAUDIA_TEST_CHROME").expect("set CLAUDIA_TEST_CHROME to chrome.exe"))
}

fn fixture_url() -> String {
    let p = std::fs::canonicalize("tests/fixtures/apply-form.html").unwrap();
    let path_str = p.display().to_string();
    let path_str = path_str.trim_start_matches(r"\\?\").replace('\\', "/");
    format!("file:///{}", path_str)
}

struct Live {
    client: cdp::CdpClient,
    /// Target id of the harness's own (the "user's") tab.
    tab_id: String,
    port: u16,
    pid: u32,
    profile: PathBuf,
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &self.pid.to_string(), "/T", "/F"])
            .output();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

async fn launch() -> Live {
    let profile = std::env::temp_dir().join(format!(
        "claudia-live-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state_path = profile.join("state.json");
    let o = LaunchOptions::headless_for_tests(chrome(), profile.join("p"));
    let st = manager::ensure_running(&o, "", &state_path).await.expect("chrome launches");
    let page = cdp::first_page(st.port).await.unwrap();
    let client = cdp::CdpClient::connect(&page.ws_url).await.unwrap();
    for m in &["Page.enable", "DOM.enable", "Accessibility.enable"] {
        client.call(m, json!({})).await.unwrap();
    }
    client.call("Page.navigate", json!({ "url": fixture_url() })).await.unwrap();
    for _ in 0..40 {
        let r = client
            .call("Runtime.evaluate", json!({ "expression": "document.readyState", "returnByValue": true }))
            .await
            .unwrap();
        if r["result"]["value"] == "complete" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Live {
        client,
        tab_id: page.id.clone(),
        port: st.port,
        pid: st.pid,
        profile,
    }
}

async fn eval(c: &cdp::CdpClient, expr: &str) -> serde_json::Value {
    c.call("Runtime.evaluate", json!({ "expression": expr, "returnByValue": true }))
        .await
        .unwrap()["result"]["value"]
        .clone()
}

#[tokio::test]
#[ignore]
async fn real_chrome_answers_cdp_and_we_can_read_the_accessibility_tree() {
    let l = launch().await;
    let ax = l
        .client
        .call("Accessibility.getFullAXTree", json!({}))
        .await
        .unwrap();
    let nodes = refs::build_refs(&ax, None).unwrap();
    let names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"Nome"), "missing 'Nome' in {names:?}");
    assert!(names.contains(&"Avançar"), "missing 'Avançar' in {names:?}");
    assert!(names.contains(&"Sim"), "missing 'Sim' in {names:?}");

    // Check that the modal appears as a dialog in the tree (by scoping to it)
    // build_refs only returns INTERACTIVE nodes, but scoping proves the dialog is in the tree
    let dialog_nodes = refs::build_refs(&ax, Some("dialog")).unwrap();
    assert!(
        !dialog_nodes.is_empty(),
        "build_refs with dialog scope returned empty list; dialog not found in tree"
    );
    // The dialog should contain Nome and Avançar (which are inside the modal div)
    let dialog_names: Vec<&str> = dialog_nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(dialog_names.contains(&"Nome"), "Nome not in dialog scope");
    assert!(dialog_names.contains(&"Avançar"), "Avançar not in dialog scope");
    // And the scope really EXCLUDES what is outside the modal (the trap: the modal has no role=dialog in the DOM).
    assert!(names.contains(&"Fora do modal"), "the button outside the modal is in the full tree");
    assert!(!dialog_names.contains(&"Fora do modal"), "dialog scope must exclude it: {dialog_names:?}");
}

#[tokio::test]
#[ignore]
async fn a_second_ensure_running_reuses_the_same_chrome() {
    let l = launch().await;
    // Same state file as launch(): a second call must not start another Chrome.
    let state_path = l.profile.join("state.json");
    let o = LaunchOptions::headless_for_tests(chrome(), l.profile.join("p"));
    let again = manager::ensure_running(&o, "", &state_path).await.unwrap();
    assert_eq!(again.port, l.port);
    assert_eq!(again.pid, l.pid);
}

#[tokio::test]
#[ignore]
async fn the_chrome_we_launched_reports_itself_visible() {
    let l = launch().await;
    let page = cdp::first_page(l.port).await.unwrap();
    cdp::bring_to_front(l.port, &page.id, &l.client).await.unwrap();
    assert_eq!(eval(&l.client, "document.visibilityState").await, "visible");
}

#[tokio::test]
#[ignore]
async fn emulated_languages_make_navigator_languages_look_normal() {
    // Task 0.6: a one-language navigator.languages is a weak automation signal. Writing
    // intl.accept_languages into Preferences does NOT change it (verified on Chrome 154); the CDP
    // emulation does, from the next navigation on.
    let l = launch().await;
    cdp::emulate_normal_languages(&l.client).await.unwrap();
    l.client.call("Page.navigate", json!({ "url": format!("{}?again", fixture_url()) })).await.unwrap();
    for _ in 0..40 {
        let r = l
            .client
            .call("Runtime.evaluate", json!({ "expression": "document.readyState", "returnByValue": true }))
            .await
            .unwrap();
        if r["result"]["value"] == "complete" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let langs = eval(&l.client, "navigator.languages").await;
    let langs: Vec<String> = langs.as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    let primary = eval(&l.client, "navigator.language").await;
    assert!(langs.len() >= 2, "still a single language: {langs:?}");
    assert_eq!(langs[0], primary.as_str().unwrap(), "the first language stays the browser's own");
    assert!(langs.iter().any(|x| x == "en"), "English is in the list: {langs:?}");
    let ua = eval(&l.client, "navigator.userAgent").await;
    assert!(ua.as_str().unwrap().contains("Chrome/"), "the user agent is untouched: {ua}");
}

#[tokio::test]
#[ignore]
async fn new_page_gives_the_agent_its_own_tab() {
    let l = launch().await;
    // After launch, there should be one tab (the one we navigated to)
    let targets_before = cdp::list_targets(l.port).await.unwrap();
    let pages_before: Vec<_> = targets_before.iter().filter(|t| t.kind == "page").collect();
    assert_eq!(pages_before.len(), 1, "expected 1 page tab after launch");

    // Create a new page
    let new_target = cdp::new_page(l.port).await.unwrap();
    assert_ne!(new_target.id, pages_before[0].id, "new page should have a different id");

    // List targets again; should now have 2 page targets
    let targets_after = cdp::list_targets(l.port).await.unwrap();
    let pages_after: Vec<_> = targets_after.iter().filter(|t| t.kind == "page").collect();
    assert_eq!(pages_after.len(), 2, "expected 2 page tabs after new_page");

    // first_page should still work (it returns the most recently active page)
    let first = cdp::first_page(l.port).await.unwrap();
    assert!(!first.ws_url.is_empty(), "first_page should return a valid target");
}

// ───────────────────────── Task 2.2: the agent-facing tools, against a real Chrome ─────────────────────────
use claudia_rh_lib::browser::guard::GuardContext;
use claudia_rh_lib::mcp::tools::browser::{BrowserTools, NoSink, ToolOutput};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// A guard context that really remembers claims, like the production one.
struct Ctx {
    status: &'static str,
    active: Option<i64>,
    claimed: Mutex<HashSet<i64>>,
}

impl Ctx {
    fn candidatando(active: i64) -> Arc<Ctx> {
        Arc::new(Ctx { status: "candidatando", active: Some(active), claimed: Mutex::new(HashSet::new()) })
    }
}

impl GuardContext for Ctx {
    fn vaga_status(&self, _: i64) -> Option<String> {
        Some(self.status.to_string())
    }
    fn applications_today(&self) -> i64 {
        0
    }
    fn daily_limit(&self) -> i64 {
        10
    }
    fn site_allowed(&self, _: &str) -> bool {
        true
    }
    fn active_vaga(&self) -> Option<i64> {
        self.active
    }
    fn claim_submit(&self, vaga_id: i64) -> bool {
        self.claimed.lock().unwrap().insert(vaga_id)
    }
}

fn tools(l: &Live, g: Arc<dyn GuardContext>) -> BrowserTools {
    BrowserTools::new(l.profile.join("state.json"), g, Arc::new(NoSink)).allow_file_urls()
}

/// A client on the AGENT's tab (the one the tools opened), not the harness's original tab.
async fn agent_client(l: &Live) -> cdp::CdpClient {
    let page = cdp::list_targets(l.port)
        .await
        .unwrap()
        .into_iter()
        .find(|t| t.kind == "page" && t.id != l.tab_id)
        .expect("the tools opened a tab of their own");
    cdp::CdpClient::connect(&page.ws_url).await.unwrap()
}

fn ref_of(listing: &str, role: &str, name: &str) -> String {
    listing
        .lines()
        .find(|l| l.contains(&format!("{role} \"{name}\"")))
        .unwrap_or_else(|| panic!("no {role} \"{name}\" in:\n{listing}"))
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

async fn center_of(c: &cdp::CdpClient, selector: &str) -> (f64, f64) {
    let v = eval(
        c,
        &format!("(()=>{{const r=document.querySelector('{selector}').getBoundingClientRect();return [r.x+r.width/2, r.y+r.height/2]}})()"),
    )
    .await;
    (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

#[tokio::test]
#[ignore]
async fn the_agent_works_in_its_own_tab_and_leaves_the_users_alone() {
    let l = launch().await;
    let before = eval(&l.client, "location.href").await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&format!("{}?agent", fixture_url())).await.unwrap();
    let pages: Vec<_> = cdp::list_targets(l.port).await.unwrap().into_iter().filter(|x| x.kind == "page").collect();
    assert_eq!(pages.len(), 2, "the user's tab plus the agent's own");
    assert_eq!(eval(&l.client, "location.href").await, before, "the user's tab was not navigated");
    // Reading and clicking work on the agent's tab.
    let list = t.read(None).await.unwrap();
    assert!(list.contains("button \"Avançar\""), "{list}");
}

#[tokio::test]
#[ignore]
async fn click_by_ref_advances_the_button_and_it_renames_itself() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.click(&ref_of(&list, "button", "Avançar"), None).await.unwrap();
    let list2 = t.read(None).await.unwrap();
    assert!(list2.contains("button \"Avaliar\""), "{list2}");
}

#[tokio::test]
#[ignore]
async fn the_hidden_radio_is_checked_through_its_label() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.click(&ref_of(&list, "radio", "Sim"), None).await.unwrap();
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "document.getElementById('r1').checked").await, true);
}

#[tokio::test]
#[ignore]
async fn typing_fills_the_field() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Nome"), "Ana").await.unwrap();
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "document.getElementById('name').value").await, "Ana");
}

/// Walks the fixture's advance button to its final "Enviar candidatura" label.
async fn walk_to_submit(t: &BrowserTools) -> String {
    for _ in 0..2 {
        let list = t.read(None).await.unwrap();
        let adv = ref_of(&list, "button", if list.contains("\"Avaliar\"") { "Avaliar" } else { "Avançar" });
        t.click(&adv, None).await.unwrap();
    }
    let list = t.read(None).await.unwrap();
    // The modal's own button (the fixture also has a form with its own submit button).
    list.lines()
        .filter(|l| l.contains("button \"Enviar candidatura\""))
        .map(|l| l.split_whitespace().next().unwrap().to_string())
        .next()
        .expect("a submit button")
}

#[tokio::test]
#[ignore]
async fn submit_is_refused_without_confirmation_and_allowed_once_with_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let submit = walk_to_submit(&t).await;
    let a = agent_client(&l).await;

    let e = t.click(&submit, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO") && e.contains("confirm_submit"), "{e}");
    assert_eq!(eval(&a, "window.__submitted").await, false);

    t.click(&submit, Some(7)).await.unwrap();
    assert_eq!(eval(&a, "window.__submitted").await, true, "with the right confirmation it goes through");

    // ONE submit per vaga per session: a second confirmed click is refused.
    eval(&a, "window.__submitted = false; true").await;
    let list = t.read(None).await.unwrap();
    let again = list.lines().filter(|l| l.contains("button \"Enviar candidatura\"")).map(|l| l.split_whitespace().next().unwrap().to_string()).next().unwrap();
    let e2 = t.click(&again, Some(7)).await.unwrap_err();
    assert!(e2.starts_with("RECUSADO"), "{e2}");
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn a_confirmation_for_another_vaga_than_the_active_one_is_refused() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let submit = walk_to_submit(&t).await;
    let e = t.click(&submit, Some(9)).await.unwrap_err();
    assert!(e.starts_with("RECUSADO") && e.contains('9'), "{e}");
    assert!(!e.contains('7'), "the refusal must not reveal which vaga is active: {e}");
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn a_refused_attempt_does_not_spend_the_vagas_single_submit() {
    let l = launch().await;
    let ctx = Ctx::candidatando(7);
    let t = tools(&l, ctx.clone());
    t.open(&fixture_url()).await.unwrap();
    let submit = walk_to_submit(&t).await;
    assert!(t.click(&submit, None).await.is_err());
    assert!(t.click(&submit, Some(9)).await.is_err());
    assert!(ctx.claimed.lock().unwrap().is_empty(), "refusals must not claim the vaga");
    t.click(&submit, Some(7)).await.unwrap();
    assert_eq!(ctx.claimed.lock().unwrap().len(), 1);
}

#[tokio::test]
#[ignore]
async fn submit_is_refused_when_the_vaga_is_not_candidatando() {
    let l = launch().await;
    let ctx = Arc::new(Ctx { status: "analisada", active: Some(7), claimed: Mutex::new(HashSet::new()) });
    let t = tools(&l, ctx);
    t.open(&fixture_url()).await.unwrap();
    let submit = walk_to_submit(&t).await;
    let e = t.click(&submit, Some(7)).await.unwrap_err();
    assert!(e.starts_with("RECUSADO") && e.contains("candidatando"), "{e}");
}

#[tokio::test]
#[ignore]
async fn the_live_name_is_guarded_not_the_stale_cached_one() {
    // The same node turns from "Revisar" into "Enviar candidatura" AFTER the last browser_read.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    let adv = ref_of(&list, "button", "Avançar"); // cached name: harmless
    let a = agent_client(&l).await;
    eval(&a, "document.getElementById('adv').textContent = 'Enviar candidatura'; true").await;
    let e = t.click(&adv, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "the cached name 'Avançar' must not hide the live one: {e}");
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn pressing_enter_on_a_focused_submit_button_is_refused_too() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    eval(&a, "(()=>{const b=document.getElementById('adv'); b.textContent='Enviar candidatura'; b.focus(); return true})()").await;
    let e = t.key("Enter", None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn enter_in_a_text_field_guards_the_forms_default_submit_button() {
    // Enter inside a text field submits its form: the field's own name ("Busca") is harmless,
    // the form's submit button ("Enviar candidatura") is not.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    eval(&a, "document.getElementById('q').focus(); true").await;
    let e = t.key("Enter", None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&a, "window.__formSubmitted").await, false);
    // With the right confirmation the keyboard path works as well.
    t.key("Enter", Some(7)).await.unwrap();
    assert_eq!(eval(&a, "window.__formSubmitted").await, true);
}

#[tokio::test]
#[ignore]
async fn a_submit_button_with_no_name_is_guarded() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    let (x, y) = center_of(&a, "#anon").await;
    let e = t.click_at(x, y, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "a nameless type=submit must count as a submit: {e}");
    assert_eq!(eval(&a, "window.__anonSubmitted").await, false);
}

#[tokio::test]
#[ignore]
async fn a_submit_button_inside_an_iframe_is_guarded_by_what_is_inside() {
    // Chrome's hit test looks INTO a same-origin iframe: the button there is what gets described.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    let (x, y) = center_of(&a, "#fr2").await;
    let e = t.click_at(x, y, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&a, "window.__frameSubmitted").await, serde_json::Value::Null);
    // A harmless button inside an iframe is not stopped.
    let (hx, hy) = center_of(&a, "#fr").await;
    t.click_at(hx, hy, None).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn click_at_a_coordinate_is_guarded_by_the_element_under_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    eval(&a, "document.getElementById('adv').textContent='Enviar candidatura'; true").await;
    let (x, y) = center_of(&a, "#adv").await;
    let e = t.click_at(x, y, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn an_ordinary_click_at_a_coordinate_goes_through() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    let (x, y) = center_of(&a, "#adv").await;
    t.click_at(x, y, None).await.unwrap();
    assert_eq!(eval(&a, "document.getElementById('adv').textContent").await, "Avaliar");
}

#[tokio::test]
#[ignore]
async fn a_stale_ref_after_navigation_is_an_error_not_a_wrong_click() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    let adv = ref_of(&list, "button", "Avançar");
    t.open(&format!("{}?second", fixture_url())).await.unwrap(); // a new document: the refs are gone
    let e = t.click(&adv, None).await.unwrap_err();
    assert!(e.contains("obsoleta"), "{e}");
}

#[tokio::test]
#[ignore]
async fn crop_returns_a_png_image() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    match t.crop(0.0, 0.0, 200.0, 100.0).await.unwrap() {
        ToolOutput::Image { data_base64, mime } => {
            assert_eq!(mime, "image/png");
            assert!(data_base64.len() > 100);
        }
        other => panic!("expected an image, got {other:?}"),
    }
}

#[tokio::test]
#[ignore]
async fn urls_that_are_not_http_are_refused_before_touching_the_browser() {
    let l = launch().await;
    // file URLs are NOT allowed here (no allow_file_urls)
    let t = BrowserTools::new(l.profile.join("state.json"), Ctx::candidatando(7), Arc::new(NoSink));
    for bad in ["file:///c:/windows/win.ini", "javascript:alert(1)", "chrome://settings", "data:text/html,<b>x</b>", "ftp://x/"] {
        let e = t.open(bad).await.unwrap_err();
        assert!(e.contains("http"), "{bad}: {e}");
    }
}

#[tokio::test]
#[ignore]
async fn opening_a_dead_address_reports_the_browser_error() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    let e = t.open("https://this-host-does-not-exist.invalid/").await.unwrap_err();
    assert!(e.contains("falhou") && e.to_lowercase().contains("err_"), "{e}");
}

#[tokio::test]
#[ignore]
async fn a_port_that_is_not_our_pinned_browser_is_not_used() {
    // state.json says version 154.9.9.9 but the Chrome on that port is another build.
    let l = launch().await;
    let state_path = l.profile.join("state.json");
    let mut st = manager::read_state(&state_path).unwrap();
    st.version = "154.9.9.9".into();
    manager::write_state(&state_path, &st).unwrap();
    let t = tools(&l, Ctx::candidatando(7));
    let e = t.open(&fixture_url()).await.unwrap_err();
    assert!(e.contains("reiniciado") || e.contains("não está respondendo"), "{e}");
}

#[tokio::test]
#[ignore]
async fn without_a_state_file_the_error_tells_the_agent_what_is_happening() {
    let l = launch().await;
    let t = BrowserTools::new(l.profile.join("no-such-state.json"), Ctx::candidatando(7), Arc::new(NoSink)).allow_file_urls();
    let e = t.open(&fixture_url()).await.unwrap_err();
    assert!(e.contains("ainda não está pronto"), "{e}");
}

#[tokio::test]
#[ignore]
async fn the_agents_tab_reports_a_normal_language_list() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    // The tools applied the emulation when they connected; a fresh navigation makes it visible.
    t.open(&format!("{}?languages", fixture_url())).await.unwrap();
    let n = eval(&a, "navigator.languages.length").await;
    assert!(n.as_i64().unwrap() >= 2, "navigator.languages has {n} entries");
}

#[tokio::test]
#[ignore]
async fn a_named_submit_type_button_in_a_form_is_not_blocked_just_for_its_type() {
    // A <button type=submit> named "Próximo" is an ordinary step, not the application submit: blocking it
    // would make every LinkedIn-style form unusable.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.click(&ref_of(&list, "button", "Próximo"), None).await.unwrap();
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "window.__nextClicked").await, true);
}

// ───────────── Task 2.2 security fixes: what the guard must survive ─────────────

#[tokio::test]
#[ignore]
async fn a_page_that_poisons_its_own_javascript_cannot_blind_the_guard() {
    // The page redefines the things a naive guard would ask: JSON.stringify, closest, activeElement.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    eval(
        &a,
        "(()=>{ JSON.stringify = () => '{}'; Element.prototype.closest = () => null; \
           Object.defineProperty(document, 'activeElement', { get: () => null }); \
           const b = document.getElementById('adv'); b.textContent = 'Enviar candidatura'; b.focus(); return true })()",
    )
    .await;
    let e = t.key("Enter", None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "the keyboard path must not trust document.activeElement: {e}");
    assert_eq!(eval(&a, "window.__submitted").await, false);
    // And the pointer path.
    let (x, y) = center_of(&a, "#adv").await;
    let e2 = t.click_at(x, y, None).await.unwrap_err();
    assert!(e2.starts_with("RECUSADO"), "{e2}");
    assert_eq!(eval(&a, "window.__submitted").await, false);
    // A NAMELESS submit is recognised only by structure, which is read in an isolated world: the
    // poisoned page script (JSON.stringify now returns "{}") must not be able to hide it.
    let (ax, ay) = center_of(&a, "#anon").await;
    let e3 = t.click_at(ax, ay, None).await.unwrap_err();
    assert!(e3.starts_with("RECUSADO"), "structure must be read from an isolated world: {e3}");
    assert_eq!(eval(&a, "window.__anonSubmitted").await, false);
}

#[tokio::test]
#[ignore]
async fn a_submit_button_inside_a_shadow_root_is_guarded_on_the_keyboard_path() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    eval(
        &a,
        "(()=>{ const host = document.createElement('div'); document.body.append(host); \
           const root = host.attachShadow({mode:'open'}); const b = document.createElement('button'); \
           b.textContent = 'Enviar candidatura'; b.onclick = () => { window.__shadowSubmitted = true }; \
           root.append(b); b.focus(); window.__shadowSubmitted = false; return true })()",
    )
    .await;
    let e = t.key("Enter", None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&a, "window.__shadowSubmitted").await, false);
}

#[tokio::test]
#[ignore]
async fn an_overlay_that_appears_over_a_harmless_button_is_caught_by_the_hit_test() {
    // The model's cached view says "Avançar"; by click time a submit button covers it.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    let adv = ref_of(&list, "button", "Avançar");
    let a = agent_client(&l).await;
    eval(
        &a,
        "(()=>{ const r = document.getElementById('adv').getBoundingClientRect(); const o = document.createElement('button'); \
           o.textContent = 'Enviar candidatura'; o.style.cssText = `position:fixed;z-index:99999;left:${r.x}px;top:${r.y}px;width:${r.width}px;height:${r.height}px`; \
           o.onclick = () => { window.__overlayClicked = true }; document.body.append(o); window.__overlayClicked = false; return true })()",
    )
    .await;
    let e = t.click(&adv, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "what is really under the click point must be guarded: {e}");
    assert_eq!(eval(&a, "window.__overlayClicked").await, false);
}

#[tokio::test]
#[ignore]
async fn a_nameless_submit_with_a_valid_confirmation_does_not_spend_the_single_submit() {
    let l = launch().await;
    let ctx = Ctx::candidatando(7);
    let t = tools(&l, ctx.clone());
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    let (x, y) = center_of(&a, "#anon").await;
    t.click_at(x, y, Some(7)).await.unwrap();
    t.click_at(x, y, Some(7)).await.unwrap();
    assert!(ctx.claimed.lock().unwrap().is_empty(), "a structural (nameless) submit must not burn the vaga's submit");
    let submit = walk_to_submit(&t).await;
    t.click(&submit, Some(7)).await.unwrap();
    assert_eq!(eval(&a, "window.__submitted").await, true, "the real submit still works afterwards");
}

#[tokio::test]
#[ignore]
async fn a_nameless_text_field_is_not_mistaken_for_a_submit() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let a = agent_client(&l).await;
    let (x, y) = center_of(&a, "#anonText").await;
    t.click_at(x, y, None).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn a_failed_locate_does_not_spend_the_single_submit() {
    let l = launch().await;
    let ctx = Ctx::candidatando(7);
    let t = tools(&l, ctx.clone());
    t.open(&fixture_url()).await.unwrap();
    let submit = walk_to_submit(&t).await;
    let a = agent_client(&l).await;
    // The submit button leaves the DOM: locate fails with a stale ref, BEFORE the guard runs.
    eval(&a, "[...document.querySelectorAll('button')].filter(b => b.textContent === 'Enviar candidatura').forEach(b => b.remove()); true").await;
    let e = t.click(&submit, Some(7)).await.unwrap_err();
    assert!(e.contains("obsoleta"), "{e}");
    assert!(ctx.claimed.lock().unwrap().is_empty(), "a click that never happened must not claim the vaga");
}

#[tokio::test]
#[ignore]
async fn refs_stay_stable_across_a_scoped_read() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let full = t.read(None).await.unwrap();
    let adv = ref_of(&full, "button", "Avançar");
    let scoped = t.read(Some("dialog")).await.unwrap();
    assert_eq!(ref_of(&scoped, "button", "Avançar"), adv, "a narrower read must not renumber the element");
    t.click(&adv, None).await.unwrap();
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "document.getElementById('adv').textContent").await, "Avaliar");
}

#[tokio::test]
#[ignore]
async fn oversized_inputs_are_refused_before_touching_the_browser() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.crop(0.0, 0.0, 100000.0, 100000.0).await.unwrap_err();
    assert!(e.contains("recorte inválido"), "{e}");
    assert!(t.crop(0.0, 0.0, 0.0, 10.0).await.is_err());
    assert!(t.crop(0.0, 0.0, f64::NAN, 10.0).await.is_err());
    let big = "x".repeat(60_000);
    let e2 = t.type_text(&ref_of(&list, "textbox", "Nome"), &big).await.unwrap_err();
    assert!(e2.contains("grande demais"), "{e2}");
    let long_url = format!("https://example.com/{}", "a".repeat(9000));
    assert!(t.open(&long_url).await.unwrap_err().contains("grande"));
    assert!(t.click_at(-5.0, 10.0, None).await.is_err());
}

#[tokio::test]
#[ignore]
async fn the_browsers_own_debug_endpoint_cannot_be_opened() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    for url in [format!("http://127.0.0.1:{}/json/list", l.port), format!("http://localhost:{}/json/version", l.port), "http://169.254.169.254/".to_string()] {
        let e = t.open(&url).await.unwrap_err();
        assert!(e.contains("locais"), "{url}: {e}");
    }
}
