// Prepares the browser in the background right after the app opens (install, theme, launch), keeps
// it alive, and stops it when the app exits. The UI follows along through the "browser-progress"
// event; the MCP processes only ever read state.json.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use super::install::Progress;
use super::manager::{self, BrowserState, LaunchOptions, WatchAction, Watcher};
use super::pin::Pin;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Status {
    pub stage: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

impl Status {
    fn at(stage: &str) -> Self {
        Self { stage: stage.into(), downloaded: 0, total: None, message: None }
    }

    pub fn idle() -> Self {
        Self::at("idle")
    }

    fn error(message: impl Into<String>) -> Self {
        Self { message: Some(message.into()), ..Self::at("error") }
    }
}

pub fn percent(s: &Status) -> Option<u8> {
    match s.total {
        Some(t) if t > 0 => Some(((s.downloaded.min(t) * 100) / t) as u8),
        _ => None,
    }
}

/// Download progress arrives on every network chunk (tens of thousands of times for 150 MB).
/// The UI gets an update only when the percentage moved or enough time passed.
pub struct Throttle {
    last_pct: Option<u8>,
    last_at: Option<Instant>,
    every: Duration,
}

impl Throttle {
    pub fn new(every: Duration) -> Self {
        Self { last_pct: None, last_at: None, every }
    }

    pub fn should_emit(&mut self, p: Progress, now: Instant) -> bool {
        let pct = percent(&Status { stage: String::new(), downloaded: p.downloaded, total: p.total, message: None });
        let moved = pct != self.last_pct;
        let waited = self.last_at.is_none_or(|t| now.duration_since(t) >= self.every);
        if self.last_at.is_none() || moved || waited {
            self.last_pct = pct;
            self.last_at = Some(now);
            true
        } else {
            false
        }
    }
}

/// Sends a download progress update to the UI only when the throttle lets it through.
fn forward_progress(throttle: &Mutex<Throttle>, p: Progress, now: Instant, publish: &(dyn Fn(Status) + Send + Sync)) {
    let emit = throttle.lock().map(|mut t| t.should_emit(p, now)).unwrap_or(true);
    if emit {
        publish(Status { stage: "downloading".into(), downloaded: p.downloaded, total: p.total, message: None });
    }
}

/// One full preparation: install if needed, then launch (or reuse) and verify. Publishes the stages
/// `downloading`, `launching`, `ready` or `error`. The spawn function is injectable for tests.
pub(crate) async fn prepare(
    local: &Path,
    pin: &Pin,
    publish: &(dyn Fn(Status) + Send + Sync),
    spawn: impl Fn(&LaunchOptions, u16) -> Result<u32, String>,
) -> Result<BrowserState, String> {
    let root = super::browser_root(local);
    if let Err(e) = std::fs::create_dir_all(&root) {
        let msg = format!("não foi possível criar a pasta do browser: {e}");
        publish(Status::error(msg.clone()));
        return Err(msg);
    }

    publish(Status::at("downloading"));
    let throttle = Mutex::new(Throttle::new(Duration::from_millis(200)));
    let on_progress = |p: Progress| forward_progress(&throttle, p, Instant::now(), publish);
    let exe = match super::install::ensure_installed(&root, &pin.version, &pin.zip_url(), &pin.sha256, &on_progress).await {
        Ok(p) => p,
        Err(e) => {
            publish(Status::error(e.to_string()));
            return Err(e.to_string());
        }
    };

    publish(Status::at("launching"));
    let opts = LaunchOptions::visible(exe, super::profile_dir(local));
    // The theme is written only on a real launch, never under a Chrome that is already running.
    let prepare_profile = |o: &LaunchOptions| {
        if let Err(e) = super::theme::apply_theme(&o.profile_dir) {
            log::warn!("browser theme not applied: {e}");
        }
    };
    match manager::ensure_running_full(&opts, &pin.version, &super::state_file(local), prepare_profile, spawn, manager::kill_tree, Duration::from_secs(20)).await {
        Ok(state) => {
            publish(Status::at("ready"));
            Ok(state)
        }
        Err(e) => {
            publish(Status::error(e.clone()));
            Err(e)
        }
    }
}

pub struct StatusState(pub Arc<Mutex<Status>>);

/// True while a preparation/watch task is alive, so "retry" cannot start a second one.
pub struct Running(pub AtomicBool);

fn publisher(app: &AppHandle) -> impl Fn(Status) + Send + Sync + 'static {
    let app = app.clone();
    move |st: Status| {
        if let Some(state) = app.try_state::<StatusState>() {
            if let Ok(mut g) = state.0.lock() {
                *g = st.clone();
            }
        }
        let _ = app.emit("browser-progress", st);
    }
}

const WATCH_EVERY: Duration = Duration::from_secs(3);

pub async fn start(app: AppHandle) {
    if let Some(r) = app.try_state::<Running>() {
        if r.0.swap(true, Ordering::SeqCst) {
            return; // already running
        }
    }
    let publish = publisher(&app);
    let local = match app.path().app_local_data_dir() {
        Ok(d) => d,
        Err(e) => {
            publish(Status::error(e.to_string()));
            return finish_running(&app);
        }
    };
    let pin = super::pin::pin();
    if prepare(&local, &pin, &publish, manager::spawn_chrome).await.is_ok() {
        watch(&local, &pin, &publish).await;
    }
    finish_running(&app);
}

fn finish_running(app: &AppHandle) {
    if let Some(r) = app.try_state::<Running>() {
        r.0.store(false, Ordering::SeqCst);
    }
}

/// Health check every few seconds; relaunch a Chrome that died, give up if it keeps dying.
async fn watch(local: &Path, pin: &Pin, publish: &(dyn Fn(Status) + Send + Sync)) {
    let state_file = super::state_file(local);
    let mut watcher = Watcher::default();
    loop {
        tokio::time::sleep(WATCH_EVERY).await;
        let Some(state) = manager::read_state(&state_file) else { continue }; // shutting down
        let alive = manager::probe(state.port).await.is_some_and(|b| manager::is_pinned(&b, &pin.version));
        match watcher.observe(alive, Instant::now()) {
            WatchAction::Nothing => {}
            WatchAction::Relaunch => {
                log::warn!("browser is gone, relaunching");
                if prepare(local, pin, publish, manager::spawn_chrome).await.is_err() {
                    return;
                }
            }
            WatchAction::GiveUp => {
                publish(Status::error("o browser fechou várias vezes seguidas; abra o app de novo ou reinicie o browser pelas configurações"));
                return;
            }
        }
    }
}

/// Called when the app exits: a Chrome with a debug port open and the user's accounts logged in,
/// with nobody watching it, is exposure for no benefit.
pub fn stop(app: &AppHandle) {
    if let Ok(local) = app.path().app_local_data_dir() {
        tauri::async_runtime::block_on(manager::shutdown(&super::state_file(&local)));
    }
}

#[tauri::command]
pub fn browser_status(state: tauri::State<'_, StatusState>) -> Status {
    state.0.lock().map(|g| g.clone()).unwrap_or_else(|_| Status::error("estado do browser indisponível"))
}

/// "Try again" after an error (no network, antivirus, ...).
#[tauri::command]
pub fn browser_retry(app: AppHandle) {
    tauri::async_runtime::spawn(start(app));
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::path::PathBuf;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn s(downloaded: u64, total: Option<u64>) -> Status {
        Status { stage: "downloading".into(), downloaded, total, message: None }
    }

    #[test]
    fn percent_is_none_without_a_total_and_clamped_to_100() {
        assert_eq!(percent(&s(10, None)), None);
        assert_eq!(percent(&s(0, Some(200))), Some(0));
        assert_eq!(percent(&s(50, Some(200))), Some(25));
        assert_eq!(percent(&s(999, Some(200))), Some(100));
        assert_eq!(percent(&s(1, Some(0))), None);
    }

    fn p(downloaded: u64, total: Option<u64>) -> Progress {
        Progress { downloaded, total }
    }

    #[test]
    fn progress_is_throttled_to_whole_percent_steps_or_the_interval() {
        let mut t = Throttle::new(Duration::from_millis(200));
        let t0 = Instant::now();
        assert!(t.should_emit(p(0, Some(1000)), t0), "the first update always goes out");
        // 999 more chunks inside the same percent and the same instant: nothing.
        let mut sent = 0;
        for d in 1..10 {
            if t.should_emit(p(d, Some(1000)), t0) {
                sent += 1;
            }
        }
        assert_eq!(sent, 0, "same percent, same instant: suppressed");
        assert!(t.should_emit(p(10, Some(1000)), t0), "the percentage moved: goes out");
        assert!(!t.should_emit(p(11, Some(1000)), t0 + Duration::from_millis(50)));
        assert!(t.should_emit(p(11, Some(1000)), t0 + Duration::from_millis(250)), "enough time passed");
        assert!(t.should_emit(p(1000, Some(1000)), t0 + Duration::from_millis(252)), "reaching 100% goes out");
    }

    #[test]
    fn an_unknown_total_is_throttled_by_time_only() {
        let mut t = Throttle::new(Duration::from_millis(200));
        let t0 = Instant::now();
        assert!(t.should_emit(p(1, None), t0));
        assert!(!t.should_emit(p(2, None), t0 + Duration::from_millis(10)));
        assert!(t.should_emit(p(3, None), t0 + Duration::from_millis(300)));
    }

    #[test]
    fn forward_progress_sends_a_small_fraction_of_a_flood_of_chunks() {
        let throttle = Mutex::new(Throttle::new(Duration::from_millis(200)));
        let (log, publish) = recorder();
        let t0 = Instant::now();
        for d in 0..1000u64 {
            forward_progress(&throttle, Progress { downloaded: d * 10, total: Some(10_000) }, t0, &publish);
        }
        let sent = log.lock().unwrap().len();
        assert!(sent <= 101, "1000 chunks over 100 percent steps must send at most ~100 updates, sent {sent}");
        assert!(sent >= 50, "but the UI still follows the download, sent {sent}");
        assert!(log.lock().unwrap().iter().all(|x| x.stage == "downloading"));
    }

    // ── prepare(): the whole pipeline against local servers ───────────────

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "claudia-svc-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn chrome_zip() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            zw.start_file("chrome-win64/chrome.exe", zip::write::SimpleFileOptions::default()).unwrap();
            zw.write_all(b"MZfake").unwrap();
            zw.finish().unwrap();
        }
        buf.into_inner()
    }

    async fn serve(body: Vec<u8>) -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = l.accept().await {
                let body = body.clone();
                tokio::spawn(async move {
                    let mut b = [0u8; 2048];
                    let _ = sock.read(&mut b).await;
                    let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(&body).await;
                });
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    fn pin_for(base: String, zip: &[u8]) -> Pin {
        Pin { version: "154.0.1.2".into(), sha256: format!("{:x}", Sha256::digest(zip)), base_url: base }
    }

    /// A spawn that "starts" a fake Chrome answering /json/version on the chosen port.
    fn fake_spawn(o: &LaunchOptions, port: u16) -> Result<u32, String> {
        assert!(o.exe.exists(), "the browser must be installed BEFORE it is launched");
        let std_l = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        std_l.set_nonblocking(true).unwrap();
        let l = tokio::net::TcpListener::from_std(std_l).unwrap();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = l.accept().await {
                let mut b = [0u8; 1024];
                let _ = sock.read(&mut b).await;
                let body = r#"{"Browser":"Chrome/154.0.1.2"}"#;
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = sock.write_all(head.as_bytes()).await;
            }
        });
        Ok(555)
    }

    fn recorder() -> (Arc<Mutex<Vec<Status>>>, impl Fn(Status) + Send + Sync) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let l2 = log.clone();
        (log, move |st| l2.lock().unwrap().push(st))
    }

    #[tokio::test]
    async fn a_first_run_downloads_then_launches_then_is_ready_and_remembers_the_port() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("first");
        let (log, publish) = recorder();
        let st = prepare(&local, &pin, &publish, fake_spawn).await.unwrap();
        let stages: Vec<String> = {
            let mut v: Vec<String> = log.lock().unwrap().iter().map(|x| x.stage.clone()).collect();
            v.dedup();
            v
        };
        assert_eq!(stages, vec!["downloading", "launching", "ready"]);
        assert_eq!(manager::read_state(&super::super::state_file(&local)).map(|x| x.port), Some(st.port));
        assert!(local.join("browser").join("154.0.1.2").join("chrome-win64").join("chrome.exe").exists());
    }

    #[tokio::test]
    async fn the_theme_is_applied_to_the_profile_before_the_first_launch() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("theme");
        let (_log, publish) = recorder();
        let seen_at_spawn = Arc::new(Mutex::new(String::new()));
        let seen = seen_at_spawn.clone();
        prepare(&local, &pin, &publish, move |o, port| {
            let prefs = std::fs::read_to_string(o.profile_dir.join("Default").join("Preferences")).unwrap_or_default();
            *seen.lock().unwrap() = prefs;
            fake_spawn(o, port)
        })
        .await
        .unwrap();
        assert!(seen_at_spawn.lock().unwrap().contains("-2525353"), "the orange theme must already be in Preferences when Chrome starts");
    }

    #[tokio::test]
    async fn a_second_run_reuses_the_install_and_the_live_browser() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("second");
        let (_l, publish) = recorder();
        let first = prepare(&local, &pin, &publish, fake_spawn).await.unwrap();
        let spawns = std::sync::atomic::AtomicUsize::new(0);
        let again = prepare(&local, &pin, &publish, |_, _| {
            spawns.fetch_add(1, Ordering::SeqCst);
            Err("must not spawn".into())
        })
        .await
        .unwrap();
        assert_eq!(first.port, again.port);
        assert_eq!(spawns.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_failed_download_ends_in_the_error_stage_with_a_message() {
        let zip = chrome_zip();
        let mut pin = pin_for(serve(zip.clone()).await, &zip);
        pin.sha256 = "0".repeat(64); // the download will not match
        let local = temp("bad-hash");
        let (log, publish) = recorder();
        let e = prepare(&local, &pin, &publish, fake_spawn).await.unwrap_err();
        assert!(e.contains("confere"), "{e}");
        let last = log.lock().unwrap().last().cloned().unwrap();
        assert_eq!(last.stage, "error");
        assert!(last.message.unwrap().contains("confere"));
    }

    #[tokio::test]
    async fn a_launch_failure_ends_in_the_error_stage() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("bad-launch");
        let (log, publish) = recorder();
        let e = prepare(&local, &pin, &publish, |_, _| Err("não abriu".to_string())).await.unwrap_err();
        assert!(e.contains("não abriu"));
        assert_eq!(log.lock().unwrap().last().unwrap().stage, "error");
    }

    #[tokio::test]
    async fn the_download_progress_that_reaches_the_ui_is_throttled() {
        // 12 MB served in one go produces many chunks: the UI must see far fewer updates.
        let zip = {
            let mut buf = std::io::Cursor::new(Vec::new());
            {
                let mut zw = zip::ZipWriter::new(&mut buf);
                let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
                zw.start_file("chrome-win64/chrome.exe", opts).unwrap();
                zw.write_all(&vec![7u8; 12_000_000]).unwrap();
                zw.finish().unwrap();
            }
            buf.into_inner()
        };
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("throttle");
        let (log, publish) = recorder();
        prepare(&local, &pin, &publish, fake_spawn).await.unwrap();
        let downloading = log.lock().unwrap().iter().filter(|x| x.stage == "downloading").count();
        assert!(downloading <= 110, "at most ~1 update per percent, got {downloading}");
        assert!(downloading >= 2);
    }
}
