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

/// Removes a profile folder made by `new_profile_dir`, and nothing else.
pub fn remove_profile_dir(p: &Path) {
    let temp_root = std::env::temp_dir();
    let is_link = std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    if is_link || !is_removable_profile_dir(p, &temp_root) {
        eprintln!("refusing to delete {} (not a live-test profile under {})", p.display(), temp_root.display());
        return;
    }
    let _ = std::fs::remove_dir_all(p);
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

/// Kills the process tree of `pid`, but only if it still is the Chrome of this harness (a recycled
/// pid belongs to someone else and is left alone).
pub fn kill_our_chrome(pid: u32, port: u16, profile: &Path) {
    let cmdline = command_line_of(pid);
    if is_our_chrome_cmdline(&cmdline, port, profile) {
        kill_tree(pid);
    } else {
        eprintln!("not killing pid {pid}: its command line is not this harness's Chrome (already gone, or the pid was reused)");
    }
}

pub fn kill_tree(pid: u32) {
    let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).output();
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
}
