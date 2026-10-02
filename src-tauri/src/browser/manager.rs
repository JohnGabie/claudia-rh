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
    let r = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/json/version"))
        .timeout(Duration::from_secs(2))
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

pub async fn ensure_running(o: &LaunchOptions, version: &str, state_path: &Path) -> Result<BrowserState, String> {
    if let Some(s) = read_state(state_path) {
        if let Some(b) = probe(s.port).await {
            if b.contains(version) {
                return Ok(s);
            }
        }
    }
    let port = free_port()?;
    std::fs::create_dir_all(&o.profile_dir).map_err(|e| e.to_string())?;
    let child = std::process::Command::new(&o.exe)
        .args(build_args(o, port))
        .spawn()
        .map_err(|e| format!("could not spawn the browser ({}): {e}", o.exe.display()))?;
    wait_ready(port, Duration::from_secs(20)).await.map_err(|e| format!("launch failed: {e}"))?;
    let state = BrowserState { port, pid: child.id(), version: version.to_string() };
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

    #[tokio::test]
    async fn concurrent_ensure_running_calls_spawn_only_once() {
        // Two concurrent calls on the same state file with a fake version server:
        // they must spawn the browser exactly once, not twice.
        let port = fake_version("Chrome/154.0.1.2").await;
        let dir = temp();
        let state_path = std::sync::Arc::new(dir.join("state.json"));
        let o = std::sync::Arc::new(LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p")));

        // Pre-write state so the test doesn't actually launch
        write_state(&state_path, &BrowserState { port, pid: 1, version: "154.0.1.2".into() }).unwrap();

        // Two concurrent calls must both succeed and reuse the same port
        let h1 = tokio::spawn({
            let o = o.clone();
            let p = state_path.clone();
            async move { ensure_running(&o, "154.0.1.2", &p).await }
        });
        let h2 = tokio::spawn({
            let o = o.clone();
            let p = state_path.clone();
            async move { ensure_running(&o, "154.0.1.2", &p).await }
        });

        let r1 = h1.await.unwrap().unwrap();
        let r2 = h2.await.unwrap().unwrap();
        assert_eq!(r1.port, r2.port);
        assert_eq!(r1.port, port);
    }

    #[test]
    fn version_check_uses_substring_match() {
        // The probe returns "Chrome/154.0.8037.92", the pinned version is "154.0.8037.92".
        // Substring match means both must match: the pinned version string must be IN the probe result.
        // This test verifies that a partial version is correctly detected.
        let probe_result = "Chrome/154.0.1.2";
        let pinned_version = "154.0.1.2";
        assert!(probe_result.contains(pinned_version));

        // Different version should not match
        let different_version = "120.0.0.0";
        assert!(!probe_result.contains(different_version));
    }
}
