//! Live tests for the minimum viewport: a real Chrome (headless=new has real window bounds) with a
//! temporary profile and no site. Run:
//! CLAUDIA_TEST_CHROME="C:/path/chrome.exe" cargo test --test viewport_live -- --ignored --test-threads=1
use claudia_rh_lib::browser::cdp;
use claudia_rh_lib::browser::manager::{self, LaunchOptions};
use claudia_rh_lib::browser::viewport::{ensure_min_viewport, MIN_H, MIN_W};
use serde_json::json;
use std::path::PathBuf;

fn chrome() -> PathBuf {
    PathBuf::from(std::env::var("CLAUDIA_TEST_CHROME").expect("set CLAUDIA_TEST_CHROME to chrome.exe"))
}

struct Live {
    client: cdp::CdpClient,
    pid: u32,
    profile: PathBuf,
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = std::process::Command::new("taskkill").args(["/PID", &self.pid.to_string(), "/T", "/F"]).output();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

async fn launch() -> Live {
    let profile = std::env::temp_dir().join(format!(
        "claudia-viewport-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let o = LaunchOptions::headless_for_tests(chrome(), profile.join("p"));
    let st = manager::ensure_running(&o, "", &profile.join("state.json")).await.expect("chrome launches");
    let page = cdp::first_page(st.port).await.unwrap();
    let client = cdp::CdpClient::connect(&page.ws_url).await.unwrap();
    client.call("Page.enable", json!({})).await.unwrap();
    Live { client, pid: st.pid, profile }
}

/// (viewport width, viewport height, outer width, outer height, window state)
async fn state(c: &cdp::CdpClient) -> (u32, u32, u32, u32, String) {
    let lm = c.call("Page.getLayoutMetrics", json!({})).await.unwrap();
    let w = c.call("Browser.getWindowForTarget", json!({})).await.unwrap();
    let b = &w["bounds"];
    (
        lm["cssLayoutViewport"]["clientWidth"].as_u64().unwrap() as u32,
        lm["cssLayoutViewport"]["clientHeight"].as_u64().unwrap() as u32,
        b["width"].as_u64().unwrap() as u32,
        b["height"].as_u64().unwrap() as u32,
        b["windowState"].as_str().unwrap().to_string(),
    )
}

async fn set_window(c: &cdp::CdpClient, bounds: serde_json::Value) {
    let w = c.call("Browser.getWindowForTarget", json!({})).await.unwrap();
    c.call("Browser.setWindowBounds", json!({ "windowId": w["windowId"], "bounds": bounds })).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
}

#[tokio::test]
#[ignore]
async fn a_shrunk_window_is_corrected_to_the_minimum() {
    let l = launch().await;
    set_window(&l.client, json!({ "windowState": "normal", "width": 700, "height": 400 })).await;
    let before = state(&l.client).await;
    assert!(before.0 < MIN_W && before.1 < MIN_H, "the setup really shrank the window: {before:?}");

    let report = ensure_min_viewport(&l.client).await.expect("ensure works");
    let after = state(&l.client).await;
    assert!(report.adjusted, "{report:?}");
    assert!(!report.below_minimum, "{report:?}");
    assert!(after.0 >= MIN_W && after.1 >= MIN_H, "the page area reaches the minimum: {after:?}");
    assert_eq!((report.after.0, report.after.1), (after.0, after.1), "the report says what the browser really has");
    assert!(report.note().unwrap().starts_with("viewport ajustado: "), "{:?}", report.note());

    // Asking again changes nothing: the second call must not touch a window that is already enough.
    let again = ensure_min_viewport(&l.client).await.unwrap();
    assert!(!again.adjusted && again.note().is_none(), "{again:?}");
}

#[tokio::test]
#[ignore]
async fn an_already_big_window_is_left_exactly_as_it_is() {
    let l = launch().await;
    set_window(&l.client, json!({ "windowState": "normal", "width": 1500, "height": 1000 })).await;
    let before = state(&l.client).await;
    let report = ensure_min_viewport(&l.client).await.unwrap();
    let after = state(&l.client).await;
    assert!(!report.adjusted && !report.below_minimum, "{report:?}");
    assert_eq!((before.2, before.3), (after.2, after.3), "the window size was not touched");
}

#[tokio::test]
#[ignore]
async fn a_wide_but_short_window_grows_in_height_only() {
    let l = launch().await;
    set_window(&l.client, json!({ "windowState": "normal", "width": 1600, "height": 500 })).await;
    let report = ensure_min_viewport(&l.client).await.unwrap();
    let after = state(&l.client).await;
    assert!(report.adjusted, "{report:?}");
    assert!(after.1 >= MIN_H, "the height reaches the minimum: {after:?}");
    assert_eq!(after.2, 1600, "the width that was already enough is not shrunk or changed: {after:?}");
}

#[tokio::test]
#[ignore]
async fn a_minimized_window_is_restored_and_sized() {
    let l = launch().await;
    set_window(&l.client, json!({ "windowState": "minimized" })).await;
    assert_eq!(state(&l.client).await.4, "minimized", "the setup really minimized it");
    let report = ensure_min_viewport(&l.client).await.unwrap();
    let after = state(&l.client).await;
    assert_eq!(after.4, "normal", "the window is back: {after:?} {report:?}");
    assert!(after.0 >= MIN_W && after.1 >= MIN_H, "{after:?}");
}

#[tokio::test]
#[ignore]
async fn resizing_never_fakes_the_device() {
    // The user sees the real window: no Emulation override may be left behind.
    let l = launch().await;
    set_window(&l.client, json!({ "windowState": "normal", "width": 700, "height": 400 })).await;
    ensure_min_viewport(&l.client).await.unwrap();
    let r = l
        .client
        .call("Runtime.evaluate", json!({ "expression": "JSON.stringify([innerWidth, outerWidth, screen.width])", "returnByValue": true }))
        .await
        .unwrap();
    let v: Vec<u32> = serde_json::from_str(r["result"]["value"].as_str().unwrap()).unwrap();
    // With a device override, innerWidth would equal the override while outerWidth stayed small.
    assert!(v[0] <= v[1], "innerWidth {} is within the real outer width {}: no emulation", v[0], v[1]);
}
