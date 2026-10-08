// Installs the browser in the background when the app opens, launches it only when the AI is called
// (a session starts) or the user presses the open button, keeps it alive while a session needs it,
// and stops it when the app exits. The UI follows along through the "browser-progress" event; the
// MCP processes only ever read state.json.
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::activity_log::sink::LogSink;
use crate::activity_log::BrowserEvent;

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

/// Install the pinned Chrome if it is not there yet (download, checksum, extract), without launching it.
/// Publishes `downloading`, or `error`. Returns the path of chrome.exe.
pub(crate) async fn install(
    local: &Path,
    pin: &Pin,
    publish: &(dyn Fn(Status) + Send + Sync),
    events: &(dyn Fn(BrowserEvent) + Send + Sync),
) -> Result<std::path::PathBuf, String> {
    let root = super::browser_root(local);
    if let Err(e) = std::fs::create_dir_all(&root) {
        let msg = format!("não foi possível criar a pasta do browser: {e}");
        publish(Status::error(msg.clone()));
        events(BrowserEvent::InstallFailed { version: pin.version.clone(), error: msg.clone() });
        return Err(msg);
    }

    publish(Status::at("downloading"));
    let throttle = Mutex::new(Throttle::new(Duration::from_millis(200)));
    // A real download is the only thing that sends progress: the first update marks its start.
    let downloaded = AtomicBool::new(false);
    let on_progress = |p: Progress| {
        if !downloaded.swap(true, Ordering::SeqCst) {
            events(BrowserEvent::DownloadStarted { version: pin.version.clone(), total_bytes: p.total });
        }
        forward_progress(&throttle, p, Instant::now(), publish)
    };
    match super::install::ensure_installed(&root, &pin.version, &pin.zip_url(), &pin.sha256, &on_progress).await {
        Ok(p) => {
            if downloaded.load(Ordering::SeqCst) {
                events(BrowserEvent::Installed { version: pin.version.clone() });
            }
            Ok(p)
        }
        Err(e) => {
            publish(Status::error(e.to_string()));
            events(BrowserEvent::InstallFailed { version: pin.version.clone(), error: e.to_string() });
            Err(e.to_string())
        }
    }
}

/// The Activity log writer for the app process. Each event is written on a blocking thread that nobody
/// waits for: the download, the launch and the watcher never slow down, and a failed write only reaches
/// diagnostics (inside `LogSink`). Without the data folders it silently does nothing.
fn event_sink(app: &AppHandle) -> impl Fn(BrowserEvent) + Send + Sync + 'static {
    let sink = match (app.path().app_data_dir(), app.path().app_local_data_dir()) {
        (Ok(data), Ok(local)) => Some(Arc::new(LogSink::new(data, local))),
        _ => None,
    };
    move |ev| {
        if let Some(sink) = &sink {
            let sink = sink.clone();
            drop(tauri::async_runtime::spawn_blocking(move || sink.browser_event(ev)));
        }
    }
}

/// One full preparation: install if needed, then launch (or reuse) and verify. Publishes the stages
/// `downloading`, `launching`, `ready` or `error`. The spawn function is injectable for tests.
pub(crate) async fn prepare(
    local: &Path,
    pin: &Pin,
    publish: &(dyn Fn(Status) + Send + Sync),
    events: &(dyn Fn(BrowserEvent) + Send + Sync),
    spawn: impl Fn(&LaunchOptions, u16) -> Result<u32, String>,
    kill: impl Fn(u32),
) -> Result<BrowserState, String> {
    let exe = install(local, pin, publish, events).await?;

    publish(Status::at("launching"));
    let opts = LaunchOptions::visible(exe, super::profile_dir(local));
    // The theme is written only on a real launch, never under a Chrome that is already running.
    let prepare_profile = |o: &LaunchOptions| {
        if let Err(e) = super::theme::apply_theme(&o.profile_dir) {
            log::warn!("browser theme not applied: {e}");
        }
    };
    match manager::ensure_running_full(&opts, &pin.version, &super::state_file(local), prepare_profile, spawn, kill, Duration::from_secs(20)).await {
        Ok(state) => {
            publish(Status::at("ready"));
            events(BrowserEvent::Launched { version: state.version.clone(), port: state.port });
            Ok(state)
        }
        Err(e) => {
            publish(Status::error(e.clone()));
            events(BrowserEvent::LaunchFailed { error: e.clone() });
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

/// Calls `apply` until it reports a window (the window appears a moment after the process starts),
/// at most `attempts` times. Returns whether a window was found.
pub(crate) async fn until_window(attempts: u32, every: Duration, mut apply: impl FnMut() -> usize) -> bool {
    for _ in 0..attempts {
        if apply() > 0 {
            return true;
        }
        tokio::time::sleep(every).await;
    }
    false
}

/// Someone asked for the browser while a preparation was still going (a session start, the open
/// button). The preparation that is running checks it when it ends and launches the browser then.
static LAUNCH_WANTED: AtomicBool = AtomicBool::new(false);

/// Launches the browser if it is not up (installing it first if needed). Safe to call any number of
/// times: only one preparation/watch task ever runs.
pub fn request_start(app: &AppHandle) {
    LAUNCH_WANTED.store(true, Ordering::SeqCst);
    let busy = app.try_state::<Running>().is_some_and(|r| r.0.load(Ordering::SeqCst));
    if !busy {
        tauri::async_runtime::spawn(start(app.clone()));
    }
}

/// Runs when the app opens. The browser is NOT launched here: it only runs when the AI is called (a
/// session starts) or the user presses the open button. This only makes sure the pinned Chrome is on
/// disk, so the first session does not wait for a 200 MB download, and ends in the `installed` stage.
/// A Chrome left running by an earlier run that did not exit cleanly is adopted and watched as before.
pub async fn install_in_background(app: AppHandle) {
    let publish = publisher(&app);
    let local = match app.path().app_local_data_dir() {
        Ok(d) => d,
        Err(e) => {
            publish(Status::error(e.to_string()));
            return;
        }
    };
    let pin = super::pin::pin();
    if is_alive(&super::state_file(&local), &pin.version).await {
        return start(app).await;
    }
    if let Some(r) = app.try_state::<Running>() {
        if r.0.swap(true, Ordering::SeqCst) {
            return;
        }
    }
    let events = event_sink(&app);
    if install(&local, &pin, &publish, &events).await.is_ok() {
        publish(Status::at("installed"));
    }
    finish_running(&app);
    if LAUNCH_WANTED.load(Ordering::SeqCst) {
        start(app).await;
    }
}

/// True while an agent session is running: only then is a browser that vanished worth relaunching.
fn agent_session_active(app: &AppHandle) -> bool {
    let Some(db) = app.try_state::<crate::DbState>() else { return false };
    let Ok(conn) = db.0.lock() else { return false };
    conn.query_row("SELECT COUNT(*) FROM sessoes WHERE terminada_em IS NULL", [], |r| r.get::<_, i64>(0))
        .map(|n| n > 0)
        .unwrap_or(false)
}

pub async fn start(app: AppHandle) {
    if let Some(r) = app.try_state::<Running>() {
        if r.0.swap(true, Ordering::SeqCst) {
            return; // already running
        }
    }
    LAUNCH_WANTED.store(false, Ordering::SeqCst);
    let publish = publisher(&app);
    let local = match app.path().app_local_data_dir() {
        Ok(d) => d,
        Err(e) => {
            publish(Status::error(e.to_string()));
            return finish_running(&app);
        }
    };
    let pin = super::pin::pin();
    let events = event_sink(&app);
    if let Ok(state) = prepare(&local, &pin, &publish, &events, manager::spawn_chrome, manager::kill_tree).await {
        // The orange logo on the window (taskbar, Alt+Tab): the window opens a moment after the launch.
        until_window(60, Duration::from_millis(250), || super::icon::apply_to_process(state.pid)).await;
        let active = || agent_session_active(&app);
        watch(&local, &pin, &publish, &events, &active).await;
    }
    finish_running(&app);
}

fn finish_running(app: &AppHandle) {
    if let Some(r) = app.try_state::<Running>() {
        r.0.store(false, Ordering::SeqCst);
    }
}

/// Is OUR browser answering right now? A missing or unreadable state.json counts as NOT alive: it
/// means another app instance stopped the browser and removed the file, and the watcher must relaunch
/// it instead of waiting forever for a file that will never come back.
pub(crate) async fn is_alive(state_file: &Path, version: &str) -> bool {
    match manager::read_state(state_file) {
        Some(state) => manager::probe(state.port).await.is_some_and(|b| manager::is_pinned(&b, version)),
        None => false,
    }
}

/// A browser that is gone is relaunched only while an agent session needs it.
pub(crate) fn relaunch_wanted(agent_active: bool) -> bool {
    agent_active
}

/// Health check every few seconds; relaunch a Chrome that died, give up if it keeps dying.
async fn watch(
    local: &Path,
    pin: &Pin,
    publish: &(dyn Fn(Status) + Send + Sync),
    events: &(dyn Fn(BrowserEvent) + Send + Sync),
    agent_active: &(dyn Fn() -> bool + Send + Sync),
) {
    let state_file = super::state_file(local);
    let mut watcher = Watcher::default();
    loop {
        tokio::time::sleep(WATCH_EVERY).await;
        let alive = is_alive(&state_file, &pin.version).await;
        if alive {
            // Chrome may reset its window icon (a new window, a theme change): put the orange one back.
            if let Some(state) = manager::read_state(&state_file) {
                // Win32 window calls block: keep them off the async worker.
                let _ = tokio::task::spawn_blocking(move || super::icon::apply_to_process(state.pid)).await;
            }
        }
        match watcher.observe(alive, Instant::now()) {
            WatchAction::Nothing => {}
            WatchAction::Relaunch => {
                events(BrowserEvent::Gone);
                if !relaunch_wanted(agent_active()) {
                    // Nobody is using it: a window the user closed stays closed until the next session or the open button.
                    events(BrowserEvent::NotRelaunched);
                    publish(Status::at("installed"));
                    return;
                }
                log::warn!("browser is gone, relaunching");
                match prepare(local, pin, publish, events, manager::spawn_chrome, manager::kill_tree).await {
                    Ok(state) => events(BrowserEvent::Relaunched { version: state.version, port: state.port }),
                    Err(_) => return,
                }
            }
            WatchAction::GiveUp => {
                events(BrowserEvent::GaveUp);
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
    request_start(&app);
}

/// The "open the browser" button: launches the Claudia browser (so the user can log into sites) and
/// does nothing if it is already up.
#[tauri::command]
pub fn browser_abrir(app: AppHandle) {
    request_start(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::path::PathBuf;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn noop_events(_: BrowserEvent) {}

    fn event_recorder() -> (Arc<Mutex<Vec<&'static str>>>, impl Fn(BrowserEvent) + Send + Sync) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let l2 = log.clone();
        (log, move |e: BrowserEvent| l2.lock().unwrap().push(e.name()))
    }

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

    #[test]
    fn a_vanished_browser_is_relaunched_only_while_an_agent_session_needs_it() {
        assert!(relaunch_wanted(true), "a session is running: bring the browser back");
        assert!(!relaunch_wanted(false), "nobody is using it: a window the user closed stays closed");
    }

    #[tokio::test]
    async fn until_window_stops_at_the_first_window_and_gives_up_after_the_attempts() {
        let mut calls = 0;
        let found = until_window(10, Duration::from_millis(1), || {
            calls += 1;
            if calls == 3 { 1 } else { 0 }
        })
        .await;
        assert!(found);
        assert_eq!(calls, 3, "it must stop as soon as the window is there");

        let mut tries = 0;
        let found = until_window(4, Duration::from_millis(1), || {
            tries += 1;
            0
        })
        .await;
        assert!(!found);
        assert_eq!(tries, 4, "and give up after exactly the allowed attempts, not wait forever");
    }

    #[tokio::test]
    async fn a_missing_state_file_counts_as_not_alive() {
        let dir = temp("alive-missing");
        assert!(!is_alive(&dir.join("state.json"), "154.0.1.2").await, "the watcher must see a vanished state.json as a dead browser");
    }

    #[tokio::test]
    async fn a_garbage_state_file_or_dead_port_counts_as_not_alive() {
        let dir = temp("alive-bad");
        let st = dir.join("state.json");
        std::fs::write(&st, "{ nope").unwrap();
        assert!(!is_alive(&st, "154.0.1.2").await);
        manager::write_state(&st, &BrowserState { port: 1, pid: 1, version: "154.0.1.2".into() }).unwrap();
        assert!(!is_alive(&st, "154.0.1.2").await);
    }

    #[tokio::test]
    async fn a_live_pinned_browser_is_alive_and_another_version_is_not() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("alive-live");
        let (_l, publish) = recorder();
        let st = prepare(&local, &pin, &publish, &noop_events, fake_spawn, |_| {}).await.unwrap();
        let file = super::super::state_file(&local);
        assert!(is_alive(&file, "154.0.1.2").await);
        assert!(!is_alive(&file, "999.0.0.1").await, "the port answers, but not as the pinned version");
        assert_eq!(st.version, "154.0.1.2");
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
        let st = prepare(&local, &pin, &publish, &noop_events, fake_spawn, |_| {}).await.unwrap();
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
        prepare(&local, &pin, &publish, &noop_events, move |o, port| {
            let prefs = std::fs::read_to_string(o.profile_dir.join("Default").join("Preferences")).unwrap_or_default();
            *seen.lock().unwrap() = prefs;
            fake_spawn(o, port)
        }, |_| {})
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
        let first = prepare(&local, &pin, &publish, &noop_events, fake_spawn, |_| {}).await.unwrap();
        let spawns = std::sync::atomic::AtomicUsize::new(0);
        let again = prepare(&local, &pin, &publish, &noop_events, |_, _| {
            spawns.fetch_add(1, Ordering::SeqCst);
            Err("must not spawn".into())
        }, |_| {})
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
        let e = prepare(&local, &pin, &publish, &noop_events, fake_spawn, |_| {}).await.unwrap_err();
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
        let e = prepare(&local, &pin, &publish, &noop_events, |_, _| Err("não abriu".to_string()), |_| {}).await.unwrap_err();
        assert!(e.contains("não abriu"));
        assert_eq!(log.lock().unwrap().last().unwrap().stage, "error");
    }

    // Pending Sandbox: the four tests below create temp folders (like the prepare() tests above).
    #[tokio::test]
    async fn a_first_run_records_download_install_and_launch_and_a_second_run_only_the_launch() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let local = temp("ev-first");
        let (_l, publish) = recorder();
        let (events, ev) = event_recorder();
        prepare(&local, &pin, &publish, &ev, fake_spawn, |_| {}).await.unwrap();
        assert_eq!(*events.lock().unwrap(), vec!["download_started", "installed", "launched"]);
        events.lock().unwrap().clear();
        prepare(&local, &pin, &publish, &ev, |_, _| Err("must not spawn".into()), |_| {}).await.unwrap();
        assert_eq!(*events.lock().unwrap(), vec!["launched"], "already installed: no download events");
    }

    #[tokio::test]
    async fn a_failed_download_records_install_failed_and_no_installed() {
        let zip = chrome_zip();
        let mut pin = pin_for(serve(zip.clone()).await, &zip);
        pin.sha256 = "0".repeat(64);
        let (_l, publish) = recorder();
        let (events, ev) = event_recorder();
        prepare(&temp("ev-hash"), &pin, &publish, &ev, fake_spawn, |_| {}).await.unwrap_err();
        assert_eq!(*events.lock().unwrap(), vec!["download_started", "install_failed"]);
    }

    #[tokio::test]
    async fn a_launch_failure_records_launch_failed() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let (_l, publish) = recorder();
        let (events, ev) = event_recorder();
        prepare(&temp("ev-launch"), &pin, &publish, &ev, |_, _| Err("não abriu".to_string()), |_| {}).await.unwrap_err();
        assert_eq!(events.lock().unwrap().last().copied(), Some("launch_failed"));
    }

    #[tokio::test]
    async fn install_alone_records_download_started_and_installed() {
        let zip = chrome_zip();
        let pin = pin_for(serve(zip.clone()).await, &zip);
        let (_l, publish) = recorder();
        let (events, ev) = event_recorder();
        install(&temp("ev-install"), &pin, &publish, &ev).await.unwrap();
        assert_eq!(*events.lock().unwrap(), vec!["download_started", "installed"]);
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
        prepare(&local, &pin, &publish, &noop_events, fake_spawn, |_| {}).await.unwrap();
        let downloading = log.lock().unwrap().iter().filter(|x| x.stage == "downloading").count();
        assert!(downloading <= 110, "at most ~1 update per percent, got {downloading}");
        assert!(downloading >= 2);
    }
}
