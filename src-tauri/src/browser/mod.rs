// Browser engine: our own Chrome for Testing, driven over CDP.
pub mod cdp;
pub mod guard;
pub mod install;
pub mod pin;
pub mod refs;
pub mod theme;

use std::path::{Path, PathBuf};

/// Folder that holds everything browser-related (binary + profile).
pub fn browser_root(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join("browser")
}

/// Persistent Chrome profile: cookies, i.e. the accounts the user logged into.
pub fn profile_dir(local_data_dir: &Path) -> PathBuf {
    browser_root(local_data_dir).join("profile")
}

/// Where the app tells MCP processes which debug port the browser uses.
pub fn state_file(local_data_dir: &Path) -> PathBuf {
    browser_root(local_data_dir).join("state.json")
}

pub fn chrome_exe(local_data_dir: &Path, version: &str) -> PathBuf {
    browser_root(local_data_dir).join(version).join("chrome-win64").join("chrome.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_live_under_one_browser_folder() {
        let local = Path::new("C:/L/io.github.johngabie.claudia-rh");
        assert_eq!(browser_root(local), local.join("browser"));
        assert_eq!(profile_dir(local), local.join("browser").join("profile"));
        assert_eq!(state_file(local), local.join("browser").join("state.json"));
        assert_eq!(
            chrome_exe(local, "154.0.1.2"),
            local.join("browser").join("154.0.1.2").join("chrome-win64").join("chrome.exe")
        );
    }
}
