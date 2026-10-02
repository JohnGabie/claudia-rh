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
    let wanted = format!("Chrome/{version}");
    for (port, pid) in candidates {
        if probe(port).await.is_some_and(|b| b.contains(&wanted)) {
            return Some(BrowserState { port, pid, version: version.to_string() });
        }
    }
    None
}

/// Starts Chrome for real and returns its pid.
fn spawn_chrome(o: &LaunchOptions, port: u16) -> Result<u32, String> {
    std::process::Command::new(&o.exe)
        .args(build_args(o, port))
        .spawn()
        .map(|c| c.id())
        .map_err(|e| format!("could not spawn the browser ({}): {e}", o.exe.display()))
}

/// Best effort: a Chrome that never became ready must not be left running.
fn kill_tree(pid: u32) {
    let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).output();
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
    let _one_launch_at_a_time = LAUNCH_LOCK.lock().await;
    if let Some(s) = reusable(o, version, state_path).await {
        if read_state(state_path).as_ref() != Some(&s) {
            write_state(state_path, &s)?; // recovered through DevToolsActivePort: remember it
        }
        return Ok(s);
    }
    let port = free_port()?;
    std::fs::create_dir_all(&o.profile_dir).map_err(|e| e.to_string())?;
    let pid = spawn(o, port)?;
    if let Err(e) = wait_ready(port, ready_timeout).await {
        kill(pid);
        return Err(format!("launch failed: {e}"));
    }
    let state = BrowserState { port, pid, version: version.to_string() };
    write_state(state_path, &state)?;
    Ok(state)
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
    }

    #[test]
    fn test_launch_is_headless() {
        let o = LaunchOptions::headless_for_tests("c".into(), "p".into());
        assert!(build_args(&o, 1).iter().any(|x| x.starts_with("--headless")));
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
        let s = ensure_running(&o, "154.0.1.2", &state_path).await.unwrap();
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
        let e = ensure_running(&o, "154.0.1.2", &state_path).await.unwrap_err();
        assert!(e.contains("spawn") || e.contains("launch"), "{e}");
    }

    #[tokio::test]
    async fn stale_state_with_a_dead_port_launches_again() {
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port: 1, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        assert!(ensure_running(&o, "154.0.1.2", &state_path).await.is_err());
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
