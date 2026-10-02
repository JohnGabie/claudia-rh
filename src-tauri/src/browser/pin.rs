// The pinned Chrome for Testing build. Version and hash live in browser-pin.json
// so a Chrome bump is a data change, not a code change.
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Pin {
    pub version: String,
    pub sha256: String,
    pub base_url: String,
}

impl Pin {
    pub fn zip_url(&self) -> String {
        format!("{}/{}/win64/chrome-win64.zip", self.base_url.trim_end_matches('/'), self.version)
    }
}

pub fn pin() -> Pin {
    serde_json::from_str(include_str!("../../browser-pin.json")).expect("browser-pin.json is valid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_file_is_well_formed() {
        let p = pin();
        assert_eq!(p.version.split('.').count(), 4, "version looks like 154.0.8037.92");
        assert_eq!(p.sha256.len(), 64);
        assert!(p.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(p.zip_url().starts_with("https://"));
        assert!(p.zip_url().ends_with("/win64/chrome-win64.zip"));
    }
}
