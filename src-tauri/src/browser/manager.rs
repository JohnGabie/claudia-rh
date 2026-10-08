// Launches and reuses the one Chrome the app owns. The app (main process) owns
// the browser; MCP processes only read state.json to find the port.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub exe: PathBuf,
    pub profile_dir: PathBuf,
    headless: bool,
}

impl LaunchOptions {
    /// The only constructor production code may use: the user must always see the window.
    pub fn visible(exe: PathBuf, profile_dir: PathBuf) -> Self {
        Self { exe, profile_dir, headless: false }
    }

    /// Tests only. Never call this from app code.
    #[doc(hidden)]
    pub fn headless_for_tests(exe: PathBuf, profile_dir: PathBuf) -> Self {
        Self { exe, profile_dir, headless: true }
    }
}

pub fn build_args(o: &LaunchOptions, port: u16) -> Vec<String> {
    let mut a = vec![
        format!("--remote-debugging-port={port}"),
        format!("--user-data-dir={}", o.profile_dir.display()),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
    ];
    if o.headless {
        a.push("--headless=new".to_string());
    } else {
        // The user sees this window and the agent reads pages in it: open it as large as the screen allows.
        // (viewport::ensure_min_viewport keeps it from being too small later; Chrome keeps the last size otherwise.)
        a.push("--start-maximized".to_string());
    }
    a
}

pub fn free_port() -> Result<u16, String> {
    let l = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    l.local_addr().map(|a| a.port()).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserState {
    pub port: u16,
    pub pid: u32,
    pub version: String,
}

pub fn write_state(path: &Path, s: &BrowserState) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("state path has no parent")?).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn read_state(path: &Path) -> Option<BrowserState> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub async fn probe(port: u16) -> Option<String> {
    let r = reqwest::Client::builder()
        .no_proxy() // localhost: never through a system proxy
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?
        .get(format!("http://127.0.0.1:{port}/json/version"))
        .send()
        .await
        .ok()?;
    let v: serde_json::Value = r.json().await.ok()?;
    v["Browser"].as_str().map(String::from)
}

pub async fn wait_ready(port: u16, timeout: Duration) -> Result<String, String> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(b) = probe(port).await {
            return Ok(b);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(format!("the browser did not answer on port {port} within {timeout:?}"))
}

/// One launch at a time in this process: two callers (the startup task and a tool
/// call, say) must never start two Chromes on the same profile.
static LAUNCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Chrome itself writes DevToolsActivePort into the profile folder: its first line
/// is the debug port. It lets us find a Chrome that is still alive even when
/// state.json was lost or is stale.
fn devtools_active_port(profile_dir: &Path) -> Option<u16> {
    std::fs::read_to_string(profile_dir.join("DevToolsActivePort")).ok()?.lines().next()?.trim().parse().ok()
}

/// True when a browser string ("Chrome/154.0.8037.92", "HeadlessChrome/...") is EXACTLY the pinned
/// version: a prefix match would accept 154.0.8037.9 for 154.0.8037.92. An empty version accepts
/// any build (live tests only).
pub fn is_pinned(browser: &str, version: &str) -> bool {
    version.is_empty() || browser.rsplit('/').next() == Some(version)
}

/// A browser of the pinned version already answering on one of the known ports
/// (state.json first, then the profile's DevToolsActivePort). Anything else
/// answering there (the user's own Chrome, say) is NOT ours and is ignored.
async fn reusable(o: &LaunchOptions, version: &str, state_path: &Path) -> Option<BrowserState> {
    let known = read_state(state_path);
    let mut candidates: Vec<(u16, u32)> = Vec::new();
    if let Some(s) = &known {
        candidates.push((s.port, s.pid));
    }
    if let Some(p) = devtools_active_port(&o.profile_dir) {
        if !candidates.iter().any(|(port, _)| *port == p) {
            candidates.push((p, 0));
        }
    }
    for (port, pid) in candidates {
        if probe(port).await.is_some_and(|b| is_pinned(&b, version)) {
            return Some(BrowserState { port, pid, version: version.to_string() });
        }
    }
    None
}

/// Starts Chrome for real and returns its pid.
pub(crate) fn spawn_chrome(o: &LaunchOptions, port: u16) -> Result<u32, String> {
    let mut cmd = std::process::Command::new(&o.exe);
    cmd.args(build_args(o, port))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    no_console(&mut cmd);
    cmd.spawn()
        .map(|c| c.id())
        .map_err(|e| format!("could not spawn the browser ({}): {e}", o.exe.display()))
}

/// No console window may flash (release builds have no console of their own).
fn no_console(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Best effort: a Chrome that never became ready must not be left running.
pub(crate) fn kill_tree(pid: u32) {
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
    no_console(&mut cmd);
    let _ = cmd.output();
}

const READY_TIMEOUT: Duration = Duration::from_secs(20);

/// Reuses a live browser of the pinned version; otherwise launches ours.
/// Never starts a second Chrome on the same profile.
pub async fn ensure_running(o: &LaunchOptions, version: &str, state_path: &Path) -> Result<BrowserState, String> {
    ensure_running_with(o, version, state_path, spawn_chrome, kill_tree, READY_TIMEOUT).await
}

pub(crate) async fn ensure_running_with(
    o: &LaunchOptions,
    version: &str,
    state_path: &Path,
    spawn: impl Fn(&LaunchOptions, u16) -> Result<u32, String>,
    kill: impl Fn(u32),
    ready_timeout: Duration,
) -> Result<BrowserState, String> {
    ensure_running_full(o, version, state_path, |_| {}, spawn, kill, ready_timeout).await
}

/// `prepare` runs right before a launch and NEVER when a live Chrome is reused: it is where the
/// profile's Preferences (the theme) get written, which must not happen under a running Chrome.
pub(crate) async fn ensure_running_full(
    o: &LaunchOptions,
    version: &str,
    state_path: &Path,
    prepare: impl Fn(&LaunchOptions),
    spawn: impl Fn(&LaunchOptions, u16) -> Result<u32, String>,
    kill: impl Fn(u32),
    ready_timeout: Duration,
) -> Result<BrowserState, String> {
    let _one_launch_at_a_time = LAUNCH_LOCK.lock().await;
    if let Some(s) = reusable(o, version, state_path).await {
        if read_state(state_path).as_ref() != Some(&s) {
            write_state(state_path, &s)?; // recovered through DevToolsActivePort: remember it
        }
        return Ok(s);
    }
    // A Chrome of ANOTHER version still holds our profile (after an update): ask it to quit first.
    if let Some(old_port) = devtools_active_port(&o.profile_dir) {
        if probe(old_port).await.is_some_and(|b| !is_pinned(&b, version)) {
            let _ = close_browser_on(old_port).await;
        }
    }
    let port = free_port()?;
    std::fs::create_dir_all(&o.profile_dir).map_err(|e| e.to_string())?;
    prepare(o);
    let pid = spawn(o, port)?;
    if let Err(e) = wait_ready(port, ready_timeout).await {
        kill(pid);
        return Err(format!("launch failed: {e}"));
    }
    let state = BrowserState { port, pid, version: version.to_string() };
    write_state(state_path, &state)?;
    Ok(state)
}

// ── lifecycle: prepare hook, foreign-version close, shutdown, watcher ──────

/// Asks a Chrome of ANOTHER version, running on our profile, to quit (`Browser.close`), and waits for
/// its port to go quiet. Without this, launching the new `chrome.exe` on a profile that an old
/// build still holds just hands the window to the old one and exits: "launch failed" 20 s later.
/// Only ever called for the port in OUR profile's DevToolsActivePort.
pub async fn close_browser_on(port: u16) -> Result<(), String> {
    let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(3)).build().map_err(|e| e.to_string())?;
    let v: serde_json::Value = client
        .get(format!("http://127.0.0.1:{port}/json/version"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let ws = v["webSocketDebuggerUrl"].as_str().ok_or("no browser websocket")?;
    if !ws.starts_with(&format!("ws://127.0.0.1:{port}/")) {
        return Err("browser websocket is not on the expected local port".into());
    }
    let cdp = crate::browser::cdp::CdpClient::connect(ws).await?;
    let _ = cdp.call("Browser.close", serde_json::json!({})).await; // the reply may never come: the browser exits
    for _ in 0..25 {
        if probe(port).await.is_none() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(format!("the browser on port {port} did not quit"))
}

/// Stops the browser the app started: kills its process tree and forgets it. Only if the port
/// still answers as OUR pinned Chrome, so a recycled pid is never killed by mistake.
pub async fn shutdown_with(state_path: &Path, kill: impl Fn(u32)) {
    if let Some(s) = read_state(state_path) {
        if probe(s.port).await.is_some_and(|b| is_pinned(&b, &s.version)) {
            kill(s.pid);
        }
    }
    let _ = std::fs::remove_file(state_path);
}

pub async fn shutdown(state_path: &Path) {
    shutdown_with(state_path, kill_tree).await;
}

/// What the watcher wants done after one health check.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WatchAction {
    Nothing,
    Relaunch,
    /// Too many restarts in a short time: stop trying and tell the user.
    GiveUp,
}

/// Pure policy: relaunch after 2 failed checks in a row, but never more than `MAX_RESTARTS`
/// within `RESTART_WINDOW` (a Chrome that keeps dying needs a human, not a loop).
#[derive(Default)]
pub struct Watcher {
    consecutive_failures: u32,
    restarts: std::collections::VecDeque<std::time::Instant>,
}

pub const MAX_RESTARTS: usize = 5;
pub const RESTART_WINDOW: Duration = Duration::from_secs(600);
const FAILURES_BEFORE_RELAUNCH: u32 = 2;

impl Watcher {
    pub fn observe(&mut self, alive: bool, now: std::time::Instant) -> WatchAction {
        if alive {
            self.consecutive_failures = 0;
            return WatchAction::Nothing;
        }
        self.consecutive_failures += 1;
        if self.consecutive_failures < FAILURES_BEFORE_RELAUNCH {
            return WatchAction::Nothing;
        }
        while self.restarts.front().is_some_and(|t| now.duration_since(*t) > RESTART_WINDOW) {
            self.restarts.pop_front();
        }
        if self.restarts.len() >= MAX_RESTARTS {
            return WatchAction::GiveUp;
        }
        self.restarts.push_back(now);
        self.consecutive_failures = 0;
        WatchAction::Relaunch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn temp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-mgr-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A launcher that never starts anything: these tests must not spawn or kill a real process.
    fn no_spawn(_: &LaunchOptions, _: u16) -> Result<u32, String> {
        Err("spawn refused (test: no browser is ever started)".to_string())
    }

    /// Fake /json/version endpoint answering with the given Browser string.
    async fn fake_version(browser: &'static str) -> u16 {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                let mut b = [0u8; 1024];
                let _ = s.read(&mut b).await;
                let body = format!(r#"{{"Browser":"{browser}"}}"#);
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
            }
        });
        port
    }

    #[test]
    fn production_launch_is_never_headless() {
        let o = LaunchOptions::visible("C:/c/chrome.exe".into(), "C:/p".into());
        let a = build_args(&o, 9333);
        assert!(!a.iter().any(|x| x.contains("headless")), "{a:?}");
        assert!(a.contains(&"--remote-debugging-port=9333".to_string()));
        assert!(a.iter().any(|x| x == "--user-data-dir=C:/p"));
        assert!(a.contains(&"--no-first-run".to_string()));
        assert!(a.contains(&"--no-default-browser-check".to_string()));
        assert!(a.contains(&"--start-maximized".to_string()), "the visible browser opens large: {a:?}");
    }

    #[test]
    fn test_launch_is_headless() {
        let o = LaunchOptions::headless_for_tests("c".into(), "p".into());
        let a = build_args(&o, 1);
        assert!(a.iter().any(|x| x.starts_with("--headless")));
        assert!(!a.iter().any(|x| x == "--start-maximized"), "headless has no window to maximize");
    }

    #[test]
    fn free_port_is_actually_free() {
        let p = free_port().unwrap();
        assert!(std::net::TcpListener::bind(("127.0.0.1", p)).is_ok());
    }

    #[test]
    fn state_roundtrips_and_garbage_reads_as_none() {
        let dir = temp();
        let f = dir.join("sub").join("state.json");
        let s = BrowserState { port: 9333, pid: 42, version: "154.0.1.2".into() };
        write_state(&f, &s).unwrap();
        assert_eq!(read_state(&f), Some(s));
        std::fs::write(&f, "{ nope").unwrap();
        assert_eq!(read_state(&f), None);
        assert_eq!(read_state(&dir.join("missing.json")), None);
    }

    #[tokio::test]
    async fn probe_reads_the_browser_string() {
        let port = fake_version("Chrome/154.0.1.2").await;
        assert_eq!(probe(port).await.as_deref(), Some("Chrome/154.0.1.2"));
        assert_eq!(probe(1).await, None);
    }

    #[tokio::test]
    async fn a_live_browser_with_the_pinned_version_is_reused_not_relaunched() {
        // The exe path does not exist: any attempt to spawn would fail the test.
        let port = fake_version("Chrome/154.0.1.2").await;
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        let s = ensure_running_with(&o, "154.0.1.2", &state_path, no_spawn, |_| {}, Duration::from_secs(1)).await.unwrap();
        assert_eq!(s.port, port);
    }

    #[tokio::test]
    async fn a_port_that_now_belongs_to_another_browser_is_not_reused() {
        // Review Focus 2: someone else (the user's own Chrome) answers on the old port.
        let port = fake_version("Chrome/120.0.0.0").await;
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        // It must try to launch ours (and fail here, because the exe does not exist).
        let e = ensure_running_with(&o, "154.0.1.2", &state_path, no_spawn, |_| {}, Duration::from_secs(1)).await.unwrap_err();
        assert!(e.contains("spawn") || e.contains("launch"), "{e}");
    }

    #[tokio::test]
    async fn stale_state_with_a_dead_port_launches_again() {
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port: 1, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        assert!(ensure_running_with(&o, "154.0.1.2", &state_path, no_spawn, |_| {}, Duration::from_secs(1)).await.is_err());
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const VER: &str = "154.0.1.2";
    // A pid that cannot belong to a real process, so a stray `kill` can never hurt anything.
    const FAKE_PID: u32 = u32::MAX - 7;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "claudia-launch-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Starts a fake /json/version endpoint on `port` (0 = pick one). Returns the port.
    /// Binds with std so it can run inside a synchronous spawn closure.
    fn serve_version(port: u16, browser: &'static str) -> u16 {
        let std_l = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        std_l.set_nonblocking(true).unwrap();
        let port = std_l.local_addr().unwrap().port();
        let l = tokio::net::TcpListener::from_std(std_l).unwrap();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                let mut b = [0u8; 1024];
                let _ = s.read(&mut b).await;
                let body = format!(r#"{{"Browser":"{browser}"}}"#);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(head.as_bytes()).await;
            }
        });
        port
    }

    fn opts(dir: &Path) -> LaunchOptions {
        LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("profile"))
    }

    #[tokio::test]
    async fn two_callers_at_once_start_exactly_one_browser() {
        let dir = temp("two");
        let state_path = dir.join("state.json");
        let o = opts(&dir);
        let spawns = Arc::new(AtomicUsize::new(0));
        let call = || {
            let spawns = spawns.clone();
            ensure_running_with(
                &o,
                VER,
                &state_path,
                move |_, port| {
                    spawns.fetch_add(1, Ordering::SeqCst);
                    serve_version(port, "Chrome/154.0.1.2");
                    Ok(FAKE_PID)
                },
                |_| {},
                Duration::from_secs(5),
            )
        };
        let (a, b) = tokio::join!(call(), call());
        let (a, b) = (a.unwrap(), b.unwrap());
        assert_eq!(a.port, b.port, "both callers must end up with the same browser");
        assert_eq!(spawns.load(Ordering::SeqCst), 1, "a second Chrome on the same profile must never start");
    }

    #[tokio::test]
    async fn a_second_call_after_a_successful_launch_reuses_it() {
        let dir = temp("reuse");
        let state_path = dir.join("state.json");
        let o = opts(&dir);
        let spawns = AtomicUsize::new(0);
        let spawn = |_: &LaunchOptions, port: u16| {
            spawns.fetch_add(1, Ordering::SeqCst);
            serve_version(port, "Chrome/154.0.1.2");
            Ok(FAKE_PID)
        };
        let first = ensure_running_with(&o, VER, &state_path, &spawn, |_| {}, Duration::from_secs(5)).await.unwrap();
        let second = ensure_running_with(&o, VER, &state_path, &spawn, |_| {}, Duration::from_secs(5)).await.unwrap();
        assert_eq!(first.port, second.port);
        assert_eq!(spawns.load(Ordering::SeqCst), 1);
        assert_eq!(read_state(&state_path), Some(first));
    }

    #[tokio::test]
    async fn a_failed_spawn_is_an_error_and_does_not_poison_the_next_call() {
        let dir = temp("fail");
        let state_path = dir.join("state.json");
        let o = opts(&dir);
        let e = ensure_running_with(&o, VER, &state_path, |_, _| Err("boom".to_string()), |_| {}, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(e.contains("boom"), "{e}");
        let ok = ensure_running_with(
            &o,
            VER,
            &state_path,
            |_, port| {
                serve_version(port, "Chrome/154.0.1.2");
                Ok(FAKE_PID)
            },
            |_| {},
            Duration::from_secs(5),
        )
        .await;
        assert!(ok.is_ok(), "the launch lock must be released after a failure: {ok:?}");
    }

    #[tokio::test]
    async fn a_chrome_that_never_becomes_ready_is_killed_and_reported() {
        let dir = temp("never");
        let state_path = dir.join("state.json");
        let killed = Arc::new(AtomicU32::new(0));
        let k = killed.clone();
        let e = ensure_running_with(
            &opts(&dir),
            VER,
            &state_path,
            |_, _| Ok(FAKE_PID), // nothing listens on the chosen port
            move |pid| k.store(pid, Ordering::SeqCst),
            Duration::from_millis(400),
        )
        .await
        .unwrap_err();
        assert!(e.contains("launch failed"), "{e}");
        assert_eq!(killed.load(Ordering::SeqCst), FAKE_PID, "the orphan process must be killed");
        assert!(read_state(&state_path).is_none(), "no state may be written for a browser that never came up");
    }

    #[tokio::test]
    async fn a_chrome_found_through_devtools_active_port_is_reused_and_remembered() {
        // state.json was lost, but the profile's own DevToolsActivePort points at a live Chrome of ours.
        let dir = temp("dtap");
        let o = opts(&dir);
        std::fs::create_dir_all(&o.profile_dir).unwrap();
        let port = serve_version(0, "Chrome/154.0.1.2");
        std::fs::write(o.profile_dir.join("DevToolsActivePort"), format!("{port}\n/devtools/browser/abc")).unwrap();
        let state_path = dir.join("state.json");
        let spawned = Mutex::new(false);
        let s = ensure_running_with(
            &o,
            VER,
            &state_path,
            |_, _| {
                *spawned.lock().unwrap() = true;
                Ok(FAKE_PID)
            },
            |_| {},
            Duration::from_secs(1),
        )
        .await
        .unwrap();
        assert_eq!(s.port, port);
        assert!(!*spawned.lock().unwrap(), "must not start a second Chrome on a profile that already has one");
        assert_eq!(read_state(&state_path).map(|x| x.port), Some(port), "the recovered port is written back to state.json");
    }

    #[tokio::test]
    async fn a_devtools_active_port_that_belongs_to_another_chrome_version_is_not_reused() {
        let dir = temp("dtap-other");
        let o = opts(&dir);
        std::fs::create_dir_all(&o.profile_dir).unwrap();
        let port = serve_version(0, "Chrome/120.0.0.0");
        std::fs::write(o.profile_dir.join("DevToolsActivePort"), format!("{port}\n/x")).unwrap();
        let state_path = dir.join("state.json");
        let spawned = AtomicUsize::new(0);
        let r = ensure_running_with(
            &o,
            VER,
            &state_path,
            |_, p| {
                spawned.fetch_add(1, Ordering::SeqCst);
                serve_version(p, "Chrome/154.0.1.2");
                Ok(FAKE_PID)
            },
            |_| {},
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_ne!(r.port, port);
        assert_eq!(spawned.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn devtools_active_port_parsing() {
        let dir = temp("parse");
        assert_eq!(devtools_active_port(&dir), None, "missing file");
        std::fs::write(dir.join("DevToolsActivePort"), "9333\n/devtools/browser/x").unwrap();
        assert_eq!(devtools_active_port(&dir), Some(9333));
        std::fs::write(dir.join("DevToolsActivePort"), "not a port").unwrap();
        assert_eq!(devtools_active_port(&dir), None, "garbage");
    }

    #[test]
    fn state_is_never_seen_half_written() {
        // The MCP processes read state.json while the app rewrites it: a reader must see the
        // old or the new content, never an empty or truncated file.
        let dir = temp("atomic");
        let path = dir.join("state.json");
        write_state(&path, &BrowserState { port: 1, pid: 1, version: VER.into() }).unwrap();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (p2, s2) = (path.clone(), stop.clone());
        let writer = std::thread::spawn(move || {
            let mut n = 0u32;
            while !s2.load(Ordering::SeqCst) {
                n += 1;
                let long = "x".repeat(2000 + (n % 7) as usize);
                let _ = write_state(&p2, &BrowserState { port: (n % 60000) as u16 + 1, pid: n, version: long });
            }
        });
        let mut torn = 0;
        for _ in 0..2000 {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if serde_json::from_str::<BrowserState>(&text).is_err() {
                    torn += 1;
                }
            }
        }
        stop.store(true, Ordering::SeqCst);
        writer.join().unwrap();
        assert_eq!(torn, 0, "a reader saw a half-written state.json {torn} times");
    }

    #[tokio::test]
    async fn the_probe_ignores_a_system_proxy() {
        // Even with a bogus proxy in the environment, the localhost probe must go direct.
        let port = serve_version(0, "Chrome/154.0.1.2");
        std::env::set_var("HTTP_PROXY", "http://127.0.0.1:1");
        std::env::set_var("http_proxy", "http://127.0.0.1:1");
        let got = probe(port).await;
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("http_proxy");
        assert_eq!(got.as_deref(), Some("Chrome/154.0.1.2"));
    }
}

#[cfg(test)]
mod pinned_tests {
    use super::is_pinned;

    #[test]
    fn the_pinned_version_must_match_exactly() {
        assert!(is_pinned("Chrome/154.0.8037.92", "154.0.8037.92"));
        assert!(is_pinned("HeadlessChrome/154.0.8037.92", "154.0.8037.92"));
        assert!(!is_pinned("Chrome/154.0.8037.9", "154.0.8037.92"), "shorter build");
        assert!(!is_pinned("Chrome/154.0.8037.921", "154.0.8037.92"), "longer build: a prefix match would accept it");
        assert!(!is_pinned("Chrome/153.0.8037.92", "154.0.8037.92"), "other major");
        assert!(!is_pinned("Chrome/154.0.8037.92", "54.0.8037.92"), "suffix of the major");
    }

    #[test]
    fn an_empty_pin_accepts_any_build_for_live_tests() {
        assert!(is_pinned("Chrome/120.0.0.0", ""));
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_tungstenite::tungstenite::Message;

    const VER: &str = "154.0.1.2";

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "claudia-life-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A fake browser: /json/version over HTTP plus a websocket that records Browser.close and then
    /// "quits" (stops answering). Returns the port and the flag set when Browser.close arrived.
    async fn fake_chrome(version_string: &'static str, quits_on_close: bool) -> (u16, Arc<Mutex<bool>>) {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let closed = Arc::new(Mutex::new(false));
        let c2 = closed.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut s, _)) = l.accept().await else { break };
                if *c2.lock().unwrap() && quits_on_close {
                    break; // the browser is gone: drop the listener
                }
                let c3 = c2.clone();
                tokio::spawn(async move {
                    let mut peek = [0u8; 4];
                    let n = s.peek(&mut peek).await.unwrap_or(0);
                    if n >= 3 && &peek[..3] == b"GET" {
                        let mut buf = [0u8; 2048];
                        let n = s.read(&mut buf).await.unwrap_or(0);
                        let req = String::from_utf8_lossy(&buf[..n]).to_string();
                        if req.contains("Upgrade: websocket") || req.contains("upgrade: websocket") {
                            // Re-handshake is impossible after reading; handled by the second branch below.
                        }
                        let body = format!(
                            r#"{{"Browser":"{version_string}","webSocketDebuggerUrl":"ws://127.0.0.1:{port}/devtools/browser/x"}}"#
                        );
                        let _ = s
                            .write_all(
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                    body.len()
                                )
                                .as_bytes(),
                            )
                            .await;
                    }
                    let _ = c3;
                });
            }
        });
        (port, closed)
    }

    /// Same, but the websocket path is served by tungstenite (it needs the raw upgrade request).
    async fn fake_chrome_ws(version_string: &'static str) -> (u16, Arc<Mutex<bool>>) {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let closed = Arc::new(Mutex::new(false));
        let c2 = closed.clone();
        tokio::spawn(async move {
            loop {
                if *c2.lock().unwrap() {
                    break; // after Browser.close the listener disappears
                }
                let s = match tokio::time::timeout(Duration::from_millis(100), l.accept()).await {
                    Ok(Ok((s, _))) => s,
                    _ => continue,
                };
                let c3 = c2.clone();
                tokio::spawn(async move {
                    let mut peek = [0u8; 2048];
                    let n = s.peek(&mut peek).await.unwrap_or(0);
                    let head = String::from_utf8_lossy(&peek[..n]).to_lowercase();
                    if head.contains("upgrade: websocket") {
                        let ws = tokio_tungstenite::accept_async(s).await.unwrap();
                        let (mut tx, mut rx) = ws.split();
                        while let Some(Ok(Message::Text(t))) = rx.next().await {
                            let v: serde_json::Value = serde_json::from_str(t.as_str()).unwrap();
                            if v["method"] == "Browser.close" {
                                *c3.lock().unwrap() = true;
                                let _ = tx.send(Message::Text(serde_json::json!({"id": v["id"], "result": {}}).to_string().into())).await;
                            }
                        }
                    } else {
                        let mut s = s;
                        let mut buf = [0u8; 2048];
                        let _ = s.read(&mut buf).await;
                        let body = format!(
                            r#"{{"Browser":"{version_string}","webSocketDebuggerUrl":"ws://127.0.0.1:{port}/devtools/browser/x"}}"#
                        );
                        let _ = s
                            .write_all(
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                    body.len()
                                )
                                .as_bytes(),
                            )
                            .await;
                    }
                });
            }
        });
        (port, closed)
    }

    // ── Watcher: pure policy ──────────────────────────────────────────────

    #[test]
    fn one_failed_check_is_not_enough_to_relaunch() {
        let mut w = Watcher::default();
        let t = Instant::now();
        assert_eq!(w.observe(false, t), WatchAction::Nothing);
        assert_eq!(w.observe(true, t), WatchAction::Nothing, "a good check resets the count");
        assert_eq!(w.observe(false, t), WatchAction::Nothing);
        assert_eq!(w.observe(false, t), WatchAction::Relaunch, "two in a row");
    }

    #[test]
    fn relaunches_are_capped_within_the_window_then_the_watcher_gives_up() {
        let mut w = Watcher::default();
        let t = Instant::now();
        let mut relaunches = 0;
        let mut gave_up = false;
        for _ in 0..40 {
            match w.observe(false, t) {
                WatchAction::Relaunch => relaunches += 1,
                WatchAction::GiveUp => {
                    gave_up = true;
                    break;
                }
                WatchAction::Nothing => {}
            }
        }
        assert_eq!(relaunches, MAX_RESTARTS);
        assert!(gave_up, "a Chrome that keeps dying must stop being relaunched");
    }

    #[test]
    fn old_restarts_leave_the_window() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        for _ in 0..(MAX_RESTARTS * 2) {
            w.observe(false, t0);
        }
        // Long after the window, a death is relaunched again.
        let later = t0 + RESTART_WINDOW + Duration::from_secs(1);
        assert_eq!(w.observe(false, later), WatchAction::Nothing);
        assert_eq!(w.observe(false, later), WatchAction::Relaunch);
    }

    // ── shutdown ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn shutdown_kills_our_browser_and_forgets_it() {
        let (port, _) = fake_chrome("Chrome/154.0.1.2", false).await;
        let dir = temp("down");
        let state = dir.join("state.json");
        write_state(&state, &BrowserState { port, pid: 4242, version: VER.into() }).unwrap();
        let killed = AtomicU32::new(0);
        shutdown_with(&state, |pid| killed.store(pid, Ordering::SeqCst)).await;
        assert_eq!(killed.load(Ordering::SeqCst), 4242);
        assert!(!state.exists(), "the state file is removed");
    }

    #[tokio::test]
    async fn shutdown_never_kills_a_pid_whose_port_is_not_our_browser() {
        // The pid may have been recycled by an unrelated process: the port no longer answers as ours.
        let (other_port, _) = fake_chrome("Chrome/120.0.0.0", false).await;
        let dir = temp("down-other");
        let state = dir.join("state.json");
        write_state(&state, &BrowserState { port: other_port, pid: 4242, version: VER.into() }).unwrap();
        let killed = AtomicUsize::new(0);
        shutdown_with(&state, |_| {
            killed.fetch_add(1, Ordering::SeqCst);
        })
        .await;
        assert_eq!(killed.load(Ordering::SeqCst), 0);
        assert!(!state.exists());
        // And with a dead port.
        write_state(&state, &BrowserState { port: 1, pid: 4242, version: VER.into() }).unwrap();
        shutdown_with(&state, |_| {
            killed.fetch_add(1, Ordering::SeqCst);
        })
        .await;
        assert_eq!(killed.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn shutdown_without_a_state_file_is_a_no_op() {
        let dir = temp("down-none");
        shutdown_with(&dir.join("missing.json"), |_| panic!("nothing to kill")).await;
    }

    // ── closing a Chrome of another version ───────────────────────────────

    #[tokio::test]
    async fn an_old_chrome_on_our_profile_is_asked_to_quit_and_we_wait_for_it() {
        let (port, closed) = fake_chrome_ws("Chrome/120.0.0.0").await;
        close_browser_on(port).await.unwrap();
        assert!(*closed.lock().unwrap(), "Browser.close was sent over the browser websocket");
        assert!(probe(port).await.is_none(), "and the port is quiet afterwards");
    }

    #[tokio::test]
    async fn closing_a_browser_that_will_not_quit_is_an_error_not_a_hang() {
        // This fake accepts Browser.close but keeps answering /json/version.
        let (port, _) = fake_chrome("Chrome/120.0.0.0", false).await;
        let r = tokio::time::timeout(Duration::from_secs(12), close_browser_on(port)).await;
        assert!(r.is_ok(), "must give up by itself");
        assert!(r.unwrap().is_err());
    }

    #[tokio::test]
    async fn launching_over_an_old_chrome_closes_it_first() {
        let (old_port, closed) = fake_chrome_ws("Chrome/120.0.0.0").await;
        let dir = temp("over-old");
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("profile"));
        std::fs::create_dir_all(&o.profile_dir).unwrap();
        std::fs::write(o.profile_dir.join("DevToolsActivePort"), format!("{old_port}\n/x")).unwrap();
        let spawned_after_close = Arc::new(Mutex::new(None::<bool>));
        let (sc, cl) = (spawned_after_close.clone(), closed.clone());
        let new_port_server = Arc::new(Mutex::new(0u16));
        let nps = new_port_server.clone();
        let s = ensure_running_full(
            &o,
            VER,
            &dir.join("state.json"),
            |_| {},
            move |_, port| {
                *sc.lock().unwrap() = Some(*cl.lock().unwrap());
                *nps.lock().unwrap() = port;
                // serve a fake new Chrome on `port`
                let l = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
                l.set_nonblocking(true).unwrap();
                let l = tokio::net::TcpListener::from_std(l).unwrap();
                tokio::spawn(async move {
                    while let Ok((mut s, _)) = l.accept().await {
                        let mut b = [0u8; 1024];
                        let _ = s.read(&mut b).await;
                        let body = r#"{"Browser":"Chrome/154.0.1.2"}"#;
                        let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
                    }
                });
                Ok(777)
            },
            |_| {},
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(*spawned_after_close.lock().unwrap(), Some(true), "the old Chrome was closed BEFORE the new one was spawned");
        assert_eq!(s.pid, 777);
    }

    #[tokio::test]
    async fn a_browser_that_points_its_websocket_elsewhere_is_never_connected_to() {
        // /json/version is untrusted input: a websocket URL on another host must not be followed.
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                let mut b = [0u8; 1024];
                let _ = s.read(&mut b).await;
                let body = r#"{"Browser":"Chrome/120.0.0.0","webSocketDebuggerUrl":"ws://evil.example:9/devtools/browser/x"}"#;
                let _ = s.write_all(format!("HTTP/1.1 200 OK
Content-Length: {}
Connection: close

{body}", body.len()).as_bytes()).await;
            }
        });
        let e = close_browser_on(port).await.unwrap_err();
        assert!(e.contains("expected local port"), "{e}");
    }

    // ── the prepare hook ──────────────────────────────────────────────────

    #[tokio::test]
    async fn prepare_runs_before_a_launch() {
        let dir = temp("prep-launch");
        let o = LaunchOptions::visible(dir.join("c.exe"), dir.join("profile"));
        let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let (o1, o2) = (order.clone(), order.clone());
        let _ = ensure_running_full(
            &o,
            VER,
            &dir.join("state.json"),
            move |_| o1.lock().unwrap().push("prepare"),
            move |_, _| {
                o2.lock().unwrap().push("spawn");
                Err("stop here".into())
            },
            |_| {},
            Duration::from_millis(100),
        )
        .await;
        assert_eq!(*order.lock().unwrap(), vec!["prepare", "spawn"]);
    }

    #[tokio::test]
    async fn prepare_never_runs_when_a_live_chrome_is_reused() {
        // The theme rewrites Preferences: doing that under a running Chrome would corrupt its profile.
        let (port, _) = fake_chrome("Chrome/154.0.1.2", false).await;
        let dir = temp("prep-reuse");
        let state = dir.join("state.json");
        write_state(&state, &BrowserState { port, pid: 1, version: VER.into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("c.exe"), dir.join("profile"));
        let ran = AtomicUsize::new(0);
        ensure_running_full(&o, VER, &state, |_| { ran.fetch_add(1, Ordering::SeqCst); }, |_, _| Err("no spawn".into()), |_| {}, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }

    // ── the production-only launcher ──────────────────────────────────────

    #[test]
    fn production_code_never_calls_the_headless_constructor() {
        // headless_for_tests is pub (integration tests are another crate). Scan the sources: only
        // manager.rs (its definition and unit tests) may mention it.
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        assert!(files.len() > 10, "the scan found the source tree");
        let offenders: Vec<_> = files
            .iter()
            .filter(|p| p.file_name().is_some_and(|n| n != "manager.rs"))
            .filter(|p| std::fs::read_to_string(p).unwrap_or_default().contains("headless_for_tests"))
            .collect();
        assert!(offenders.is_empty(), "production code must never ask for a headless browser: {offenders:?}");
    }
}
