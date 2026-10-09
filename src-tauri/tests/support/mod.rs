//! Shared helpers for the live tests.
//!
//! Deleting is guarded: only a folder this harness created, directly under the system temp dir and
//! named `claudia-live-*`, may ever be removed. Killing is guarded too: a pid is only killed when its
//! command line is the Chrome this harness started. Tests of the guards call the PURE functions with
//! invented paths and strings; nothing here is ever pointed at a real folder to prove a refusal.
#![allow(dead_code)]
use claudia_rh_lib::browser::cdp;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

pub const PROFILE_PREFIX: &str = "claudia-live-";

/// Why `CLAUDIA_TEST_CHROME` is not acceptable, or None. Pure: absolute path whose file is `chrome.exe`.
pub fn chrome_path_problem(p: &Path) -> Option<&'static str> {
    if !p.is_absolute() {
        return Some("must be an absolute path");
    }
    if p.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Some("must not contain . or .. components");
    }
    match p.file_name().and_then(|n| n.to_str()) {
        Some(n) if n.eq_ignore_ascii_case("chrome.exe") => None,
        _ => Some("the file must be named chrome.exe"),
    }
}

/// The Chrome for Testing the live tests may start. Panics (before anything is launched) when the
/// variable is missing, relative, not named chrome.exe or not an existing file.
pub fn test_chrome() -> PathBuf {
    let raw = std::env::var("CLAUDIA_TEST_CHROME").expect("set CLAUDIA_TEST_CHROME to chrome.exe");
    let p = PathBuf::from(raw);
    if let Some(why) = chrome_path_problem(&p) {
        panic!("CLAUDIA_TEST_CHROME refused ({why}): {}", p.display());
    }
    assert!(p.is_file(), "CLAUDIA_TEST_CHROME does not point to an existing file: {}", p.display());
    p
}

pub fn new_profile_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("{PROFILE_PREFIX}{tag}-{}-{nanos}", std::process::id()))
}

/// Pure check, never touches the disk: absolute, no `..`, exactly one level under `temp_root`,
/// and named with our prefix.
pub fn is_removable_profile_dir(p: &Path, temp_root: &Path) -> bool {
    if !p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return false;
    }
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else { return false };
    name.starts_with(PROFILE_PREFIX) && name.len() > PROFILE_PREFIX.len() && p.parent() == Some(temp_root)
}

/// How long the harness keeps trying to delete a profile after killing its Chrome: `taskkill /F` returns
/// before the killed processes have closed their files, so the first attempts can meet files still in use.
pub const REMOVE_PATIENCE: Duration = Duration::from_secs(20);

/// Removes a profile folder made by `new_profile_dir`, and nothing else. A failure is never silent: it
/// panics with the reason (failing the test) unless the thread is already panicking, where it only prints
/// (a second panic would abort the whole test binary).
pub fn remove_profile_dir(p: &Path) {
    if let Err(why) = try_remove_profile_dir(p, REMOVE_PATIENCE) {
        let msg = format!("live-test profile NOT deleted: {why}");
        if std::thread::panicking() {
            eprintln!("{msg}");
        } else {
            panic!("{msg}");
        }
    }
}

/// The guarded removal, retried until `patience` runs out. Every attempt re-checks the guards first.
/// Ok when the folder is gone (or was never created); Err with the last error and how long it tried.
pub fn try_remove_profile_dir(p: &Path, patience: Duration) -> Result<(), String> {
    let temp_root = std::env::temp_dir();
    let started = std::time::Instant::now();
    let mut attempts = 0u32;
    loop {
        let is_link = std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false);
        if is_link || !is_removable_profile_dir(p, &temp_root) {
            return Err(format!("refusing to delete {} (not a live-test profile under {})", p.display(), temp_root.display()));
        }
        attempts += 1;
        match std::fs::remove_dir_all(p) {
            Ok(()) => {
                if attempts > 1 {
                    eprintln!("deleted {} after {attempts} attempts in {:?}", p.display(), started.elapsed());
                }
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                if attempts == 1 {
                    eprintln!("deleting {} failed on the first attempt ({e}); retrying for up to {patience:?}", p.display());
                }
                if started.elapsed() >= patience {
                    return Err(format!("{}: {e} (after {attempts} attempts in {:?})", p.display(), started.elapsed()));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Pure: is this command line the Chrome the harness started? It must be a chrome.exe, listen on the
/// harness's debug port and use a user-data-dir inside the harness's profile folder.
pub fn is_our_chrome_cmdline(cmdline: &str, port: u16, profile: &Path) -> bool {
    let c = cmdline.to_ascii_lowercase();
    let profile = profile.display().to_string().to_ascii_lowercase();
    !profile.is_empty()
        && c.contains("chrome.exe")
        && c.contains(&format!("--remote-debugging-port={port}"))
        && c.contains(&format!("--user-data-dir={profile}"))
}

fn command_line_of(pid: u32) -> String {
    let script = format!("(Get-CimInstance Win32_Process -Filter 'ProcessId={pid}').CommandLine");
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Pure: is this command line a Chrome process (the browser or one of its helpers: renderer, GPU, utility,
/// crashpad) running on this harness's profile? The user-data-dir must be INSIDE `profile` (the harness puts
/// it in `<profile>\p`), so another harness's profile whose name merely starts the same never matches.
pub fn is_chrome_on_profile(cmdline: &str, profile: &Path) -> bool {
    let c = cmdline.to_ascii_lowercase().replace('"', "");
    let profile = profile.display().to_string().to_ascii_lowercase();
    !profile.is_empty() && c.contains("chrome.exe") && c.contains(&format!("--user-data-dir={profile}\\"))
}

/// Every chrome.exe whose command line is on this harness's profile, as (pid, command line).
pub fn chrome_on_profile(profile: &Path) -> Vec<(u32, String)> {
    let script = "Get-CimInstance Win32_Process -Filter \"Name='chrome.exe'\" | ForEach-Object { \"$($_.ProcessId)`t$($_.CommandLine)\" }";
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    out.lines()
        .filter_map(|l| l.split_once('\t'))
        .filter_map(|(pid, cmd)| pid.trim().parse().ok().map(|p: u32| (p, cmd.trim().to_string())))
        .filter(|(_, cmd)| is_chrome_on_profile(cmd, profile))
        .collect()
}

/// How long the harness waits for every Chrome process of a profile to be gone after killing it.
pub const KILL_PATIENCE: Duration = Duration::from_secs(10);

/// Kills the process tree of `pid`, but only if it still is the Chrome of this harness (a recycled
/// pid belongs to someone else and is left alone). Then sweeps the helpers `taskkill /T` missed: in the
/// 2026-10-08 Sandbox round a utility process of the profile outlived the tree kill by more than 13 s.
/// Only processes whose command line is on this harness's own profile folder are touched.
pub fn kill_our_chrome(pid: u32, port: u16, profile: &Path) {
    let cmdline = command_line_of(pid);
    if is_our_chrome_cmdline(&cmdline, port, profile) {
        kill_tree(pid);
    } else {
        eprintln!("not killing pid {pid}: its command line is not this harness's Chrome (already gone, or the pid was reused)");
    }
    if !is_removable_profile_dir(profile, &std::env::temp_dir()) {
        eprintln!("not sweeping helpers of {}: not a live-test profile", profile.display());
        return;
    }
    let started = std::time::Instant::now();
    loop {
        let left = chrome_on_profile(profile);
        if left.is_empty() {
            return;
        }
        if started.elapsed() >= KILL_PATIENCE {
            eprintln!("Chrome processes of {} still alive after {KILL_PATIENCE:?}: {:?}", profile.display(), left.iter().map(|p| p.0).collect::<Vec<_>>());
            return;
        }
        for (p, _) in &left {
            kill_tree(*p);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

pub fn kill_tree(pid: u32) {
    match std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).output() {
        Ok(o) if o.status.success() => {}
        Ok(o) => eprintln!("taskkill /PID {pid} /T /F exited with {}: {}", o.status, String::from_utf8_lossy(&o.stderr).trim()),
        Err(e) => eprintln!("taskkill /PID {pid} could not run: {e}"),
    }
}

/// Pure: does the page area (inner) belong to this window size (outer)? The window bounds change at once,
/// the page's viewport follows a few hundred ms later: in the Sandbox (headless, 2026-10-08) a window set to
/// 700x400 still had a 927x908 page for 250-500 ms, and the tests measured that stale page. Inner is never
/// bigger than outer, and the frame (borders, tab strip, scrollbar) is a few dozen pixels wide and at most a
/// couple hundred tall.
/// Cleanup guard to create RIGHT AFTER the launch: if anything panics before the real `Live` guard
/// exists (an `unwrap` on the first CDP call, say), this one closes our Chrome and drops the folder.
/// Call `disarm()` once the `Live` takes over.
pub struct Pending {
    pid: u32,
    port: u16,
    profile: PathBuf,
    armed: bool,
}

impl Pending {
    pub fn new(pid: u32, port: u16, profile: &Path) -> Self {
        Self { pid, port, profile: profile.to_path_buf(), armed: true }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if self.armed {
            kill_our_chrome(self.pid, self.port, &self.profile);
            remove_profile_dir(&self.profile);
        }
    }
}

pub fn viewport_follows_window(inner: (i64, i64), outer: (i64, i64)) -> bool {
    let (dw, dh) = (outer.0 - inner.0, outer.1 - inner.1);
    inner.0 > 0 && inner.1 > 0 && (0..=60).contains(&dw) && (0..=250).contains(&dh)
}

/// Waits (up to 5 s) until the page of `c` has the viewport of its window. A hidden tab is not laid out again
/// until it is shown, so it is not waited for. False when it never caught up.
pub async fn wait_until_viewport_follows_window(c: &cdp::CdpClient) -> bool {
    let expr = "JSON.stringify([innerWidth, innerHeight, outerWidth, outerHeight, document.visibilityState])";
    for _ in 0..50 {
        let r = c.call("Runtime.evaluate", serde_json::json!({ "expression": expr, "returnByValue": true })).await;
        let v = r.ok().and_then(|r| r["result"]["value"].as_str().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()));
        if let Some(v) = v {
            if v[4] == "hidden" {
                return true;
            }
            let n = |i: usize| v[i].as_i64().unwrap_or(-1);
            if viewport_follows_window((n(0), n(1)), (n(2), n(3))) {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Chrome may answer /json before its first tab exists: retry for up to 4 s.
pub async fn first_page_with_retry(port: u16) -> Result<cdp::Target, String> {
    let mut last = String::new();
    for _ in 0..40 {
        match cdp::first_page(port).await {
            Ok(t) => return Ok(t),
            Err(e) => last = e,
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(r"C:\Temp\fake-root")
    }

    #[test]
    fn a_profile_dir_under_temp_with_our_prefix_is_removable() {
        assert!(is_removable_profile_dir(&root().join("claudia-live-7-1"), &root()));
    }

    #[test]
    fn anything_else_is_refused() {
        let r = root();
        assert!(!is_removable_profile_dir(&r, &r), "the temp root itself");
        assert!(!is_removable_profile_dir(Path::new(r"C:\"), &r), "a disk root");
        assert!(!is_removable_profile_dir(&r.join("other"), &r), "no prefix");
        assert!(!is_removable_profile_dir(&r.join("claudia-live-"), &r), "prefix only");
        assert!(!is_removable_profile_dir(&r.join("x").join("claudia-live-1"), &r), "two levels down");
        assert!(!is_removable_profile_dir(&r.join("claudia-live-1").join(".."), &r), "parent component");
        assert!(!is_removable_profile_dir(Path::new("claudia-live-1"), &r), "relative");
        assert!(!is_removable_profile_dir(Path::new(r"C:\fora-do-temp\claudia-live-1"), &r), "outside temp");
    }

    #[test]
    fn only_an_absolute_chrome_exe_is_accepted_as_the_test_chrome() {
        assert_eq!(chrome_path_problem(Path::new(r"C:\fake\cft\chrome-win64\chrome.exe")), None);
        assert_eq!(chrome_path_problem(Path::new(r"C:\fake\CHROME.EXE")), None, "case does not matter");
        assert!(chrome_path_problem(Path::new("chrome.exe")).is_some(), "relative");
        assert!(chrome_path_problem(Path::new(r"C:\fake\notepad.exe")).is_some(), "another program");
        assert!(chrome_path_problem(Path::new(r"C:\fake\chrome.exe.bat")).is_some(), "a lookalike name");
        assert!(chrome_path_problem(Path::new(r"C:\fake\..\other\chrome.exe")).is_some(), "parent component");
        assert!(chrome_path_problem(Path::new(r"C:\fake\folder")).is_some(), "a folder");
    }

    #[test]
    fn a_pid_is_killed_only_when_its_command_line_is_our_chrome() {
        let profile = PathBuf::from(r"C:\Temp\fake-root\claudia-live-browser-7-1");
        let ours = r#""C:\fake\cft\chrome.exe" --remote-debugging-port=9333 --user-data-dir=C:\Temp\fake-root\claudia-live-browser-7-1\p --headless=new"#;
        assert!(is_our_chrome_cmdline(ours, 9333, &profile));
        assert!(!is_our_chrome_cmdline(ours, 9334, &profile), "another port");
        assert!(!is_our_chrome_cmdline(&ours.replace("claudia-live-browser-7-1", "other"), 9333, &profile), "another profile");
        assert!(!is_our_chrome_cmdline(&ours.replace("chrome.exe", "notepad.exe"), 9333, &profile), "another program");
        assert!(!is_our_chrome_cmdline("", 9333, &profile), "the process is gone");
        assert!(!is_our_chrome_cmdline(ours, 9333, Path::new("")), "an empty profile matches nothing");
    }

    #[test]
    fn a_page_viewport_belongs_to_its_window_only_when_the_frame_is_plausible() {
        // Readings seen in the Sandbox (headless): window first, page a few hundred ms later.
        assert!(viewport_follows_window((682, 248), (700, 400)), "settled at 700x400");
        assert!(viewport_follows_window((1902, 919), (1920, 1080)), "settled, maximized");
        assert!(!viewport_follows_window((927, 908), (700, 400)), "the old, bigger page after shrinking");
        assert!(!viewport_follows_window((682, 248), (1600, 500)), "the old, smaller page after growing");
        assert!(!viewport_follows_window((0, 0), (700, 400)), "no layout");
    }

    #[test]
    fn a_helper_process_counts_only_when_it_runs_on_this_harness_profile() {
        let profile = PathBuf::from(r"C:\Temp\fake-root\claudia-live-browser-7-1");
        let helper = r#""C:\fake\cft\chrome.exe" --type=utility --utility-sub-type=chrome.mojom.ProcessorMetrics --user-data-dir="C:\Temp\fake-root\claudia-live-browser-7-1\p" /prefetch:14"#;
        let main = r#""C:\fake\cft\chrome.exe" --remote-debugging-port=9333 --user-data-dir=C:\Temp\fake-root\claudia-live-browser-7-1\p --headless=new"#;
        assert!(is_chrome_on_profile(helper, &profile), "a quoted user-data-dir (helpers)");
        assert!(is_chrome_on_profile(main, &profile), "the unquoted one (the browser)");
        assert!(is_chrome_on_profile(&helper.to_uppercase(), &profile), "case does not matter");
        assert!(!is_chrome_on_profile(&helper.replace("browser-7-1", "browser-7-12"), &profile), "a profile whose name only starts the same");
        assert!(!is_chrome_on_profile(&helper.replace("claudia-live-browser-7-1", "other"), &profile), "another profile");
        assert!(!is_chrome_on_profile(&helper.replace("chrome.exe", "notepad.exe"), &profile), "another program");
        assert!(!is_chrome_on_profile(r#""C:\fake\cft\chrome.exe" --type=renderer"#, &profile), "no user-data-dir");
        assert!(!is_chrome_on_profile(helper, Path::new("")), "an empty profile matches nothing");
    }
}
