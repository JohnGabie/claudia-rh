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
    let e = without_viewport_note(&t.click(&submit, Some(9)).await.unwrap_err());
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

// ───────────── Task 3.3 / 3.4: what the Activity log receives from the real tools ─────────────
use claudia_rh_lib::browser::nav::NavRecord;
use claudia_rh_lib::mcp::tools::browser::{ActionRecord, ActionSink};

struct Mem {
    actions: Mutex<Vec<ActionRecord>>,
    navs: Mutex<Vec<NavRecord>>,
}

impl Mem {
    fn new() -> Arc<Mem> {
        Arc::new(Mem { actions: Mutex::new(vec![]), navs: Mutex::new(vec![]) })
    }
}

impl ActionSink for Mem {
    fn enabled(&self) -> bool {
        true
    }
    fn action(&self, r: ActionRecord) {
        self.actions.lock().unwrap().push(r);
    }
    fn navigation(&self, r: NavRecord) {
        self.navs.lock().unwrap().push(r);
    }
}

fn tools_with_sink(l: &Live, sink: Arc<Mem>) -> BrowserTools {
    BrowserTools::new(l.profile.join("state.json"), Ctx::candidatando(7), sink).allow_file_urls()
}

#[tokio::test]
#[ignore]
async fn recorded_actions_carry_the_target_a_before_and_an_after_screenshot() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone());
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Nome"), "Ana").await.unwrap();
    t.click(&ref_of(&list, "button", "Avançar"), None).await.unwrap();
    let recs = mem.actions.lock().unwrap();
    let typed = recs.iter().find(|r| r.tool == "browser_type").unwrap();
    assert_eq!(typed.target.as_deref(), Some("Nome"));
    assert_eq!(typed.text.as_deref(), Some("Ana"));
    assert!(typed.shot_before.as_ref().is_some_and(|b| b.len() > 100), "a mutating action keeps a 'before' crop");
    assert!(typed.shot_after.as_ref().is_some_and(|b| b.len() > 100), "and an 'after' image");
    let click = recs.iter().find(|r| r.tool == "browser_click").unwrap();
    assert_eq!(click.result, "ok");
    assert!(click.shot_before.as_ref().is_some_and(|b| b.len() > 100), "a click keeps a crop of its target");
    assert!(click.url.as_deref().is_some_and(|u| u.contains("apply-form")));
    let read = recs.iter().find(|r| r.tool == "browser_read").unwrap();
    assert!(read.shot_before.is_none(), "a read-only action takes no 'before' crop");
}

#[tokio::test]
#[ignore]
async fn a_password_field_is_never_recorded_neither_text_nor_before_crop() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone());
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Senha"), "segredo-123").await.unwrap();
    let recs = mem.actions.lock().unwrap();
    let pw = recs.iter().find(|r| r.tool == "browser_type").unwrap();
    assert_eq!(pw.target.as_deref(), Some("Senha"));
    assert!(pw.text.is_none(), "the password text must never be recorded");
    assert!(pw.shot_before.is_none(), "the crop 'before' of a password field must never be recorded");
    // The secret must not appear anywhere else in the record either.
    assert!(!format!("{:?} {:?}", pw.error, pw.target).contains("segredo-123"));
}

#[tokio::test]
#[ignore]
async fn a_refusal_is_recorded_as_refused_with_a_full_screenshot() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone());
    t.open(&fixture_url()).await.unwrap();
    for _ in 0..2 {
        let list = t.read(None).await.unwrap();
        let name = if list.contains("\"Avaliar\"") { "Avaliar" } else { "Avançar" };
        t.click(&ref_of(&list, "button", name), None).await.unwrap();
    }
    let list = t.read(None).await.unwrap();
    let submit = ref_of(&list, "button", "Enviar candidatura");
    let _ = t.click(&submit, None).await.unwrap_err();
    let recs = mem.actions.lock().unwrap();
    let refused = recs.iter().rev().find(|r| r.result == "refused").expect("the refusal was recorded");
    assert!(refused.error.as_deref().is_some_and(|e| e.starts_with("RECUSADO")));
    assert!(refused.shot_after.as_ref().is_some_and(|b| b.len() > 100), "a failure keeps a screenshot");
}

#[tokio::test]
#[ignore]
async fn an_error_is_recorded_as_error_not_refused() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone());
    t.open(&fixture_url()).await.unwrap();
    let _ = t.click("e99", None).await.unwrap_err();
    let recs = mem.actions.lock().unwrap();
    let r = recs.iter().rev().find(|r| r.tool == "browser_click").unwrap();
    assert_eq!(r.result, "error");
}

#[tokio::test]
#[ignore]
async fn the_agents_navigation_is_recorded_with_its_initiator() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone());
    t.open(&fixture_url()).await.unwrap();
    // The tracker runs in its own task: give it a moment.
    for _ in 0..30 {
        if !mem.navs.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let navs = mem.navs.lock().unwrap();
    assert!(!navs.is_empty(), "the navigation must be recorded");
    let n = navs.iter().find(|n| n.url.contains("apply-form")).expect("the fixture page was recorded");
    assert_eq!(n.initiator, "agent", "it happened while a browser_open was running");
}

// ───────────── The orange logo on a REAL, visible Chrome window ─────────────
use claudia_rh_lib::browser::icon;

/// A visible (not headless) Chrome the way the app launches it. A window appears for a few seconds.
async fn launch_visible() -> Live {
    let profile = std::env::temp_dir().join(format!(
        "claudia-icon-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let state_path = profile.join("state.json");
    let o = LaunchOptions::visible(chrome(), profile.join("p"));
    let st = manager::ensure_running(&o, "", &state_path).await.expect("a visible chrome launches");
    let page = cdp::first_page(st.port).await.unwrap();
    let client = cdp::CdpClient::connect(&page.ws_url).await.unwrap();
    Live { client, tab_id: page.id.clone(), port: st.port, pid: st.pid, profile }
}

async fn wait_for_window(pid: u32) -> usize {
    for _ in 0..60 {
        let n = icon::apply_to_process(pid);
        if n > 0 {
            return n;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    0
}

#[tokio::test]
#[ignore]
async fn the_orange_icon_lands_on_the_real_chrome_window_and_chrome_keeps_working() {
    let l = launch_visible().await;
    let windows = wait_for_window(l.pid).await;
    assert!(windows >= 1, "the Chrome window was found");
    let orange = icon::orange_big_handle().expect("the orange icon exists");
    let now = icon::current_big_icons(l.pid);
    assert!(!now.is_empty() && now.iter().all(|h| *h == orange), "every Chrome window of the process wears the orange icon: {now:?} vs {orange}");
    let orange_small = icon::orange_small_handle().expect("the small orange icon exists");
    let small_now = icon::current_small_icons(l.pid);
    assert!(!small_now.is_empty() && small_now.iter().all(|h| *h == orange_small), "the small slot (title bar / tray) wears it too: {small_now:?}");
    // Chrome is not broken by it: it still answers CDP, still navigates, still reports itself visible.
    l.client.call("Page.navigate", json!({ "url": fixture_url() })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(eval(&l.client, "document.title").await, "fixture");
    // (Window visibility is not asserted here: whether Windows lets a test process raise a window varies.)
    let again = cdp::list_targets(l.port).await.unwrap();
    assert!(again.iter().any(|t| t.kind == "page"));
}

#[tokio::test]
#[ignore]
async fn applying_the_icon_twice_changes_nothing_the_second_time() {
    let l = launch_visible().await;
    wait_for_window(l.pid).await;
    assert_eq!(icon::apply_to_process_counting_changes(l.pid), 0, "already orange: nothing to change");
}

#[tokio::test]
#[ignore]
async fn the_icon_is_put_back_if_chrome_resets_it() {
    // Chrome may reset its window icon (new window, theme change). The watcher calls apply again.
    let l = launch_visible().await;
    wait_for_window(l.pid).await;
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SendMessageW, ICON_BIG, WM_SETICON};
        struct Reset(u32);
        unsafe extern "system" fn clear(hwnd: windows_sys::Win32::Foundation::HWND, lp: windows_sys::Win32::Foundation::LPARAM) -> windows_sys::Win32::Foundation::BOOL {
            let r = &*(lp as *const Reset);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == r.0 && IsWindowVisible(hwnd) != 0 {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, 0);
            }
            1
        }
        let r = Reset(l.pid);
        unsafe { EnumWindows(Some(clear), &r as *const Reset as isize) };
    }
    let orange = icon::orange_big_handle().unwrap();
    assert!(icon::current_big_icons(l.pid).iter().any(|h| *h != orange), "the reset worked (precondition)");
    assert!(icon::apply_to_process_counting_changes(l.pid) >= 1, "apply puts it back");
    assert!(icon::current_big_icons(l.pid).iter().all(|h| *h == orange));
}

#[tokio::test]
#[ignore]
async fn a_process_that_is_not_ours_is_never_touched() {
    // The test process itself has no Chrome window: nothing to change, and nothing is claimed.
    assert_eq!(icon::apply_to_process(std::process::id()), 0);
}

// ───────────── Robust reading (spec 2026-10-03): browser_text, estado:, repeated-call hint, browser_scroll ─────────────
// Fixtures only: nothing here touches a job site.

fn fixture_file(name: &str) -> String {
    let p = std::fs::canonicalize(format!("tests/fixtures/{name}")).unwrap();
    let path_str = p.display().to_string();
    format!("file:///{}", path_str.trim_start_matches(r"\\?\").replace('\\', "/"))
}

/// Resizes the (shared) browser window from the harness's own tab: the agent's tab lives in the same window.
async fn set_window(l: &Live, w: i64, h: i64) {
    let win = l.client.call("Browser.getWindowForTarget", json!({})).await.unwrap();
    let id = win["windowId"].as_i64().unwrap();
    l.client.call("Browser.setWindowBounds", json!({ "windowId": id, "bounds": { "windowState": "normal" } })).await.unwrap();
    l.client.call("Browser.setWindowBounds", json!({ "windowId": id, "bounds": { "width": w, "height": h } })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
}

/// (body, next_offset) of a browser_text result.
fn text_page(out: &str) -> (String, Option<usize>) {
    assert!(out.starts_with("[conteúdo da página"), "the untrusted-content line comes first: {out}");
    let rest = out.split_once('\n').unwrap().1;
    let (body, tail) = rest.split_once("\n[fim do conteúdo ").unwrap_or_else(|| panic!("no end marker in: {out}"));
    let next = tail
        .lines()
        .find_map(|l| l.strip_prefix("next_offset: "))
        .and_then(|r| r.split_whitespace().next())
        .and_then(|n| n.parse().ok());
    (body.to_string(), next)
}

async fn read_all_text(t: &BrowserTools, page_chars: usize) -> String {
    let mut all = String::new();
    let mut offset = 0usize;
    for _ in 0..100 {
        let out = t.text(None, Some(offset), Some(page_chars)).await.unwrap();
        let (body, next) = text_page(&out);
        all.push_str(&body);
        all.push('\n');
        match next {
            Some(n) => offset = n,
            None => return all,
        }
    }
    panic!("browser_text never reached the end of the page");
}

/// The `estado:` line of a tool result (a viewport note may come after it).
fn estado_line(s: &str) -> &str {
    s.lines().find(|l| l.starts_with("estado:")).unwrap_or("")
}

/// The result without the viewport note line (the headless test window is smaller than the minimum, so it shows up).
fn without_viewport_note(s: &str) -> String {
    s.lines().filter(|l| !l.starts_with("viewport")).collect::<Vec<_>>().join("
")
}

#[tokio::test]
#[ignore]
async fn browser_text_reads_a_long_narrow_page_completely_and_the_same_at_any_window_size() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    let mut texts = vec![];
    let mut inner_widths = vec![];
    for (w, h) in [(1200, 800), (1800, 1000)] {
        set_window(&l, w, h).await;
        t.open(&fixture_file("long-text.html")).await.unwrap();
        let a = agent_client(&l).await;
        inner_widths.push(eval(&a, "innerWidth").await.as_i64().unwrap());
        // With the defaults the page is longer than one page of text: the result says where to continue.
        let first = t.text(None, None, None).await.unwrap();
        assert!(first.contains("next_offset: ") && first.contains("para continuar"), "{first}");
        texts.push(read_all_text(&t, 3000).await);
    }
    assert!(inner_widths[1] > inner_widths[0] + 300, "the window really changed size: {inner_widths:?}");
    assert_eq!(texts[0], texts[1], "the text does not depend on the window size ({inner_widths:?})");
    let full = &texts[0];
    assert!(full.chars().count() > 6000, "the fixture is longer than the default page");
    for must in ["# Engenheiro de Software", "## Sobre a vaga", "Texto de abertura com um link inline Python no meio da frase", "• Requisito um: Python", "Linha solta em div A", "[botão: Candidatura simplificada]", "Parágrafo 1:", "Parágrafo 40:", "FIM-DO-TEXTO-XYZ"] {
        assert!(full.contains(must), "missing {must:?}");
    }
    // What must never be there: hidden content, and what sits inside a field (a typed value, a password).
    for never in ["SEGREDO-DISPLAY-NONE", "SEGREDO-ARIA-HIDDEN", "Ana-Digitada", "segredo-123", "texto-dentro-da-textarea"] {
        assert!(!full.contains(never), "{never:?} must not come out of browser_text");
    }
}

#[tokio::test]
#[ignore]
async fn browser_text_never_returns_what_the_agent_typed_and_scopes_to_the_dialog_or_a_ref() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Nome"), "Ana-Digitada-XYZ").await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Senha"), "segredo-123").await.unwrap();
    let page = t.text(None, None, None).await.unwrap();
    assert!(page.contains("[botão: Fora do modal]") && page.contains("[botão: Avançar]"), "{page}");
    assert!(!page.contains("Ana-Digitada-XYZ") && !page.contains("segredo-123"), "typed values never come back: {page}");
    let dialog = t.text(Some("dialog"), None, None).await.unwrap();
    assert!(dialog.contains("[diálogo: Candidatar-se]") && dialog.contains("Avançar"), "{dialog}");
    assert!(!dialog.contains("Fora do modal"), "the dialog scope excludes what is outside it: {dialog}");
    // By ref: just that element.
    let one = t.text(Some(&ref_of(&list, "button", "Avançar")), None, None).await.unwrap();
    assert!(one.contains("[botão: Avançar]") && !one.contains("Fora do modal"), "{one}");
    assert!(t.text(Some("tabpanel"), None, None).await.unwrap_err().contains("não encontrado"));
}

#[tokio::test]
#[ignore]
async fn the_estado_line_tells_an_ordinary_page_a_login_dialog_a_login_url_a_captcha_and_an_empty_page_apart() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    for (file, expected) in [
        ("long-text.html", "estado: ok"),
        ("login-wall.html", "estado: login_wall"),
        ("authwall.html", "estado: login_wall"),
        ("captcha.html", "estado: captcha"),
        ("empty.html", "estado: vazio"),
    ] {
        let opened = t.open(&fixture_file(file)).await.unwrap();
        assert_eq!(estado_line(&opened), expected, "browser_open on {file}: {opened}");
        let read = t.read(None).await.unwrap();
        assert_eq!(estado_line(&read), expected, "browser_read on {file}: {read}");
        let text = t.text(None, None, None).await.unwrap();
        assert_eq!(estado_line(&text), expected, "browser_text on {file}: {text}");
    }
}

#[tokio::test]
#[ignore]
async fn the_fourth_identical_call_on_an_unchanged_page_carries_a_hint_and_still_works() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    for i in 1..=4 {
        let r = t.key("Tab", None).await.unwrap();
        if i < 4 {
            assert!(!r.contains("repetiu"), "press {i}: {r}");
        } else {
            assert!(r.starts_with("Pressionou Tab.") && r.contains("repetiu browser_key 4 vezes") && r.contains("browser_text"), "{r}");
        }
    }
    // A new page: the count starts again.
    t.open(&fixture_file("authwall.html")).await.unwrap();
    let r = t.key("Tab", None).await.unwrap();
    assert!(!r.contains("repetiu"), "{r}");
    // Reading the same unchanged page over and over is a loop too.
    t.open(&fixture_url()).await.unwrap();
    let mut last = String::new();
    for _ in 0..4 {
        last = t.read(None).await.unwrap();
    }
    assert!(last.contains("repetiu browser_read 4 vezes"), "{last}");
}

#[tokio::test]
#[ignore]
async fn a_page_that_changed_between_reads_is_not_a_loop() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.read(None).await.unwrap();
    t.read(None).await.unwrap();
    // The page changes under the model's feet (the button renames itself), then it reads again.
    t.click(&ref_of(&list, "button", "Avançar"), None).await.unwrap();
    let fourth = t.read(None).await.unwrap();
    assert!(fourth.contains("button \"Avaliar\""), "the content really changed: {fourth}");
    assert!(!fourth.contains("repetiu"), "what the page shows changed, so this is not a loop: {fourth}");
}

#[tokio::test]
#[ignore]
async fn scroll_loads_more_of_an_infinite_list_and_reports_where_the_page_is() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("infinite-list.html")).await.unwrap();
    let before = t.read(None).await.unwrap();
    let count = |s: &str| s.lines().filter(|x| x.contains("link \"Vaga ")).count();
    assert_eq!(count(&before), 20, "{before}");
    assert!(!before.contains("\"Vaga 21\""));
    let r1 = t.scroll("down", Some(3.0), None).await.unwrap();
    assert!(r1.contains("Rolou down 3 tela(s)") && r1.contains("Posição:") && r1.contains("fim_da_pagina:"), "{r1}");
    tokio::time::sleep(Duration::from_millis(500)).await; // the page loads 20 more near the end
    t.scroll("down", Some(3.0), None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let after = t.read(None).await.unwrap();
    assert!(after.contains("link \"Vaga 21\""), "scrolling made the list load more: {after}");
    assert!(count(&after) > 20);
    let top = t.scroll("top", None, None).await.unwrap();
    assert!(top.contains("Posição: 0%"), "{top}");
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "scrollY").await.as_f64().unwrap(), 0.0);
    assert!(t.scroll("sideways", None, None).await.unwrap_err().contains("direção inválida"));
}

#[tokio::test]
#[ignore]
async fn scrolling_down_a_list_that_keeps_moving_is_not_called_a_loop_but_scrolling_at_the_end_is() {
    // Haiku round: four scrolls down an infinite list, each one moving the page and loading more, got
    // "Dica: você repetiu browser_scroll 4 vezes sem a página mudar". The page did change.
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("infinite-list.html")).await.unwrap();
    for i in 1..=5 {
        let r = t.scroll("down", None, None).await.unwrap();
        assert!(!r.contains("Dica:"), "scroll {i} moved the page, it is not a loop: {r}");
        tokio::time::sleep(Duration::from_millis(400)).await; // the page loads 20 more near the end
    }
    // A page that cannot move any more: the same scroll over and over IS a loop.
    t.open(&fixture_file("empty.html")).await.unwrap();
    let mut last = String::new();
    for _ in 0..4 {
        last = t.scroll("down", None, None).await.unwrap();
    }
    assert!(last.contains("repetiu browser_scroll 4 vezes"), "{last}");
}

#[tokio::test]
#[ignore]
async fn scroll_with_a_ref_moves_that_elements_own_container_and_says_when_it_is_at_the_limit() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("scroll-box.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let a = agent_client(&l).await;
    let scroll_top = || async { eval(&a, "document.getElementById('box').scrollTop").await.as_f64().unwrap() };
    assert_eq!(scroll_top().await, 0.0);
    let first = ref_of(&list, "link", "Item 1");
    let r = t.scroll("down", None, Some(&first)).await.unwrap();
    assert!(r.contains("moveu: true"), "{r}");
    assert!(scroll_top().await > 0.0, "the list's own scroll moved, the page did not");
    assert_eq!(eval(&a, "scrollY").await.as_f64().unwrap(), 0.0);
    let b = t.scroll("bottom", None, Some(&first)).await.unwrap();
    assert!(b.contains("moveu: true"), "{b}");
    let max = eval(&a, "document.getElementById('box').scrollHeight - document.getElementById('box').clientHeight").await.as_f64().unwrap();
    assert!((scroll_top().await - max).abs() <= 2.0, "bottom reaches the end of the container");
    let last = ref_of(&list, "link", "Item 40");
    let again = t.scroll("down", None, Some(&last)).await.unwrap();
    assert!(again.contains("moveu: false") && again.contains("já está no limite"), "{again}");
}

// ───────────── Review fixes (2026-10-03): viewport through a real tool, click_at after a resize, forged text, wheel vs fields ─────────────

#[tokio::test]
#[ignore]
async fn a_tool_call_restores_the_minimum_viewport_and_says_so() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("long-text.html")).await.unwrap();
    set_window(&l, 700, 400).await;
    let a = agent_client(&l).await;
    assert!(eval(&a, "innerWidth").await.as_i64().unwrap() < 1100, "the window really is small now");
    // A TOOL (not ensure_min_viewport directly) must fix it before acting and say so.
    let out = t.text(None, None, None).await.unwrap();
    assert!(out.contains("viewport ajustado:"), "the result says the window was adjusted: {out}");
    assert_eq!(out.lines().last(), Some("estado: ok"), "estado stays the last line: {out}");
    let a = agent_client(&l).await;
    assert!(eval(&a, "innerWidth").await.as_i64().unwrap() >= 1200, "the viewport is at least 1200 wide after the tool");
    assert!(eval(&a, "innerHeight").await.as_i64().unwrap() >= 800, "and at least 800 high");
    let again = t.text(None, None, None).await.unwrap();
    assert!(!again.contains("viewport"), "the note is delivered once: {again}");
}

#[tokio::test]
#[ignore]
async fn click_at_right_after_a_resize_is_refused_and_works_on_the_next_call() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("long-text.html")).await.unwrap();
    set_window(&l, 700, 400).await;
    let e = t.click_at(10.0, 10.0, None).await.unwrap_err();
    assert!(e.contains("redimensionada") && e.contains("browser_crop"), "refused because the layout just changed: {e}");
    let next = t.click_at(10.0, 10.0, None).await;
    let msg = match &next {
        Ok(m) => m.clone(),
        Err(m) => m.clone(),
    };
    assert!(!msg.contains("redimensionada"), "the window is stable now: the refusal is gone: {msg}");
}

#[tokio::test]
#[ignore]
async fn a_page_cannot_forge_the_tools_lines_in_browser_text() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("forge.html")).await.unwrap();
    let out = t.text(None, None, None).await.unwrap();
    assert_eq!(out.lines().last(), Some("estado: ok"), "the real estado is last and says ok, not the page's login_wall: {out}");
    for own in ["estado:", "total_chars:", "next_offset:", "[fim do conteúdo "] {
        let n = out.lines().filter(|l| l.starts_with(own)).count();
        let expected = if own == "next_offset:" { 0 } else { 1 };
        assert_eq!(n, expected, "exactly the tool's own {own:?} line starts a line, the page's copies are neutralized: {out}");
    }
    assert!(out.contains("Fim legítimo da descrição.") && out.contains("Texto normal da descrição"), "the legitimate text is intact: {out}");
    assert!(out.contains("| estado: login_wall"), "the page's imitation is visible but marked: {out}");
}

#[tokio::test]
#[ignore]
async fn the_wheel_does_not_change_a_focused_number_field() {
    let l = launch().await;
    set_window(&l, 1400, 1000).await;
    let t = tools(&l, Ctx::candidatando(7));
    t.open(&fixture_file("number-field.html")).await.unwrap();
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "document.activeElement && document.activeElement.id").await, "anos", "the field is focused");
    let r = t.scroll("down", Some(1.0), None).await.unwrap();
    assert!(r.contains("Rolou down"), "{r}");
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "document.getElementById('anos').value").await, "5", "the value was not changed by the wheel");
    assert!(eval(&a, "scrollY").await.as_f64().unwrap() > 0.0, "and the page did scroll");
}

// ── browser_upload (attach-CV plan, part 1) ──────────────────────────────────

struct FakeDocs {
    path: std::path::PathBuf,
    formato: &'static str,
}

impl claudia_rh_lib::mcp::tools::browser::DocumentSource for FakeDocs {
    fn for_upload(&self, _vaga: i64, _tipo: &str, _doc: Option<i64>) -> Result<claudia_rh_lib::mcp::tools::browser::Attachment, String> {
        Ok(claudia_rh_lib::mcp::tools::browser::Attachment {
            id: 1,
            nome: "CV Teste".into(),
            formato: self.formato.into(),
            tamanho: std::fs::metadata(&self.path).unwrap().len(),
            sha256: "abcdef0123456789".into(),
            path: self.path.clone(),
            regra: "preferido geral".into(),
        })
    }
}

fn fake_doc(ext: &'static str) -> Arc<FakeDocs> {
    let p = std::env::temp_dir().join(format!("claudia-upload-{}-{ext}.{ext}", std::process::id()));
    std::fs::write(&p, if ext == "pdf" { &b"%PDF-1.7 teste"[..] } else { &b"PK\x03\x04 teste"[..] }).unwrap();
    Arc::new(FakeDocs { path: p, formato: ext })
}

async fn text_of(l: &Live, id: &str) -> String {
    let a = agent_client(l).await;
    eval(&a, &format!("document.getElementById('{id}').textContent")).await.as_str().unwrap_or("").to_string()
}

#[tokio::test]
#[ignore]
async fn upload_into_a_visible_file_input_and_the_page_sees_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap();
    assert!(out.contains("CV Teste") && out.contains("preferido geral"), "{out}");
    assert!(text_of(&l, "nome-cv").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_through_a_button_that_opens_the_chooser() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Carregar currículo"), None, None).await.unwrap();
    assert!(text_of(&l, "nome-esc").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_refuses_a_submit_button_and_never_clicks_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Enviar candidatura"), None, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn upload_without_an_active_vaga_is_refused() {
    let l = launch().await;
    let none = Arc::new(Ctx { status: "candidatando", active: None, claimed: Mutex::new(HashSet::new()) });
    let t = tools(&l, none).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap_err();
    assert!(e.contains("candidatando"), "{e}");
}

#[tokio::test]
#[ignore]
async fn upload_reports_when_the_page_did_not_take_the_file() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("docx"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Só PDF"), None, None).await.unwrap();
    assert!(out.contains("a página não mostrou o nome do arquivo"), "{out}");
}

#[tokio::test]
#[ignore]
async fn a_chooser_that_never_opens_times_out_and_the_tab_still_works() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Botão que não abre nada"), None, None).await.unwrap_err();
    assert!(e.contains("seletor de arquivo"), "{e}");
    // The tab is still usable, and the chooser is no longer intercepted: a later real chooser works.
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Carregar currículo"), None, None).await.unwrap();
    assert!(text_of(&l, "nome-esc").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_with_an_empty_library_is_refused_with_a_hint() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)); // NoDocuments
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap_err();
    assert!(e.contains("documentos"), "{e}");
    t.read(None).await.unwrap(); // tab still fine
}

#[tokio::test]
#[ignore]
async fn after_an_upload_the_file_chooser_is_no_longer_intercepted() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let _ = t.upload(&ref_of(&list, "button", "Botão que não abre nada"), None, None).await.unwrap_err();
    // A plain click (not an upload) on the chooser button: with the interception released the browser
    // handles the chooser itself (headless closes it: 'cancel'); still intercepted, nothing ever happens.
    let list = t.read(None).await.unwrap();
    t.click(&ref_of(&list, "button", "Carregar currículo"), None).await.unwrap();
    let mut seen = String::new();
    for _ in 0..20 {
        seen = text_of(&l, "nome-esc").await;
        if !seen.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    assert_eq!(seen, "cancelado", "the chooser must reach the browser again after an upload");
}
