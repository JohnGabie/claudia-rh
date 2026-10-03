// Screenshot files for the Activity log. The database stores paths RELATIVE to the log folder.
use std::path::{Path, PathBuf};

const FOLDER: &str = "capturas";

/// Saves one image and returns its relative path, e.g. `capturas/20261002/17-before.jpg`.
pub fn save(dir: &Path, event_id: i64, which: &str, bytes: &[u8]) -> Result<String, String> {
    let day = chrono::Utc::now().format("%Y%m%d").to_string();
    let rel = format!("{FOLDER}/{day}/{event_id}-{which}.jpg");
    let full = dir.join(&rel);
    std::fs::create_dir_all(full.parent().ok_or("no parent folder")?).map_err(|e| e.to_string())?;
    std::fs::write(&full, bytes).map_err(|e| e.to_string())?;
    Ok(rel)
}

/// Turns a stored relative path into a real path, refusing anything that could escape the captures
/// folder: the UI asks for images by this path.
pub fn resolve(dir: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    let clean = !p.is_absolute()
        && !rel.contains('\\')
        && rel.starts_with(&format!("{FOLDER}/"))
        && p.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    if !clean {
        return Err("caminho de captura inválido".into());
    }
    Ok(dir.join(p))
}

pub fn remove(dir: &Path, rel: &str) {
    if let Ok(p) = resolve(dir, rel) {
        let _ = std::fs::remove_file(p);
    }
}

pub fn remove_all(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir.join(FOLDER));
}

pub fn size(dir: &Path, rel: &str) -> u64 {
    resolve(dir, rel).ok().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0)
}
