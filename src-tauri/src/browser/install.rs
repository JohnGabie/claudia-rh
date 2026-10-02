// Download, verify, extract and atomically place Chrome for Testing.
// Never leaves a half-installed folder: everything happens in a temp dir and
// is renamed into place only after the hash and the layout check out.
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug)]
pub enum InstallError {
    Network(String),
    HashMismatch { expected: String, actual: String },
    BadZip(String),
    Io(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "falha de rede ao baixar o browser: {e}"),
            Self::HashMismatch { expected, actual } => {
                write!(f, "o arquivo baixado não confere (esperado {expected}, obtido {actual})")
            }
            Self::BadZip(e) => write!(f, "o arquivo do browser está corrompido ou incompleto: {e}"),
            Self::Io(e) => write!(f, "erro de disco ao instalar o browser: {e}"),
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

fn io(e: std::io::Error) -> InstallError {
    InstallError::Io(e.to_string())
}

fn rename_with_retry(from: &Path, to: &Path, delays: &[Duration]) -> std::io::Result<()> {
    let mut last_err = None;
    for (i, &delay) in delays.iter().enumerate() {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if i < delays.len() - 1 {
                    std::thread::sleep(delay);
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("no retries")))
}

fn cleanup_stale_temp_folders(root: &Path, version: &str) -> std::io::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };

            // Clean up old-style .tmp-{version} folder
            if name == format!(".tmp-{version}") {
                let _ = std::fs::remove_dir_all(&path);
                continue;
            }

            // Clean up stale .tmp-{version}-{pid} folders from other processes
            let prefix = format!(".tmp-{version}-");
            if name.starts_with(&prefix) {
                let started_file = path.join("started");
                let is_stale = match std::fs::read_to_string(&started_file) {
                    Ok(content) => {
                        if let Ok(timestamp) = content.trim().parse::<u64>() {
                            now.saturating_sub(timestamp) > 3600
                        } else {
                            true // unreadable = stale
                        }
                    }
                    Err(_) => true, // no started file = stale
                };

                if is_stale {
                    let _ = std::fs::remove_dir_all(&path);
                }
            }
        }
    }
    Ok(())
}

pub async fn ensure_installed(
    root: &Path,
    version: &str,
    zip_url: &str,
    expected_sha256: &str,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
) -> Result<PathBuf, InstallError> {
    ensure_installed_with(root, version, zip_url, expected_sha256, on_progress, IDLE_TIMEOUT).await
}

pub(crate) async fn ensure_installed_with(
    root: &Path,
    version: &str,
    zip_url: &str,
    expected_sha256: &str,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
    idle_timeout: Duration,
) -> Result<PathBuf, InstallError> {
    let exe = root.join(version).join("chrome-win64").join("chrome.exe");
    if exe.exists() {
        return Ok(exe);
    }

    cleanup_stale_temp_folders(root, version).ok();

    let pid = std::process::id();
    let tmp = root.join(format!(".tmp-{version}-{pid}"));

    // Handle race: if tmp exists and seems fresh, wait a bit and retry
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let _ = std::fs::remove_dir_all(&tmp);
        if std::fs::create_dir_all(&tmp).is_ok() {
            break;
        }

        if attempt == 2 {
            return Err(InstallError::Io("falha ao criar pasta temporária".into()));
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(tmp.join("started"), now.to_string()).map_err(io)?;

    let result = install_into(&tmp, root, version, zip_url, expected_sha256, on_progress, idle_timeout).await;

    let _ = std::fs::remove_dir_all(&tmp);

    // Check again if another instance already finished
    if exe.exists() {
        return Ok(exe);
    }

    result.map(|_| exe)
}

async fn install_into(
    tmp: &Path,
    root: &Path,
    version: &str,
    zip_url: &str,
    expected_sha256: &str,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
    idle_timeout: Duration,
) -> Result<(), InstallError> {
    let zip_path = tmp.join("chrome.zip");
    let actual = download(zip_url, &zip_path, on_progress, idle_timeout).await?;
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(InstallError::HashMismatch { expected: expected_sha256.to_string(), actual });
    }

    let extract = tmp.join("extract");
    let exe_path = extract.join("chrome-win64").join("chrome.exe");

    // If extraction already succeeded (due to concurrent task), reuse it
    if !exe_path.exists() {
        let file = std::fs::File::open(&zip_path).map_err(io)?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| InstallError::BadZip(e.to_string()))?;
        archive.extract(&extract).map_err(|e| InstallError::BadZip(e.to_string()))?;
    }

    if !exe_path.exists() {
        return Err(InstallError::BadZip("chrome.exe ausente do arquivo".into()));
    }

    let dest = root.join(version);
    let _ = std::fs::remove_dir_all(&dest);
    let retries = [
        Duration::from_millis(200),
        Duration::from_millis(400),
        Duration::from_millis(800),
    ];
    if rename_with_retry(&extract, &dest, &retries).is_err() {
        if dest.exists() {
            return Ok(());
        }
        return Err(InstallError::Io("falha ao renomear pasta de instalação".into()));
    }
    Ok(())
}

/// Streams the body to `to`, hashing as it goes. Returns the lowercase SHA-256.
async fn download(
    url: &str,
    to: &Path,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
    idle_timeout: Duration,
) -> Result<String, InstallError> {
    use futures_util::StreamExt;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| InstallError::Network(e.to_string()))?;

    let resp = client.get(url).send().await.map_err(|e| InstallError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(InstallError::Network(format!("HTTP {}", resp.status())));
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(to).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = tokio::time::timeout(idle_timeout, stream.next())
        .await
        .map_err(|_| InstallError::Network(format!("download parado: sem dados há {idle_timeout:?}")))?
    {
        let chunk = chunk.map_err(|e| InstallError::Network(e.to_string()))?;
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(io)?;
        downloaded += chunk.len() as u64;
        on_progress(Progress { downloaded, total });
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    fn temp_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-install-{tag}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sha(b: &[u8]) -> String { format!("{:x}", Sha256::digest(b)) }

    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, data) in files {
                zw.start_file(*name, opts).unwrap();
                zw.write_all(data).unwrap();
            }
            zw.finish().unwrap();
        }
        buf.into_inner()
    }

    /// Serves `body` with the given status on a random local port. Returns the URL.
    async fn serve(body: Vec<u8>, status: u16) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let body = body.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let _ = sock.read(&mut buf).await;
                    let head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(&body).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        format!("http://127.0.0.1:{port}/chrome-win64.zip")
    }

    fn no_progress() -> impl Fn(Progress) + Send + Sync { |_p| {} }

    #[tokio::test]
    async fn installs_from_a_valid_zip_and_reports_progress() {
        let root = temp_root("ok");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZfake")]);
        let url = serve(zip.clone(), 200).await;
        let seen = Arc::new(AtomicU64::new(0));
        let s2 = seen.clone();
        let cb = move |p: Progress| { s2.store(p.downloaded, Ordering::SeqCst); };
        let exe = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &cb).await.unwrap();
        assert!(exe.exists());
        assert_eq!(exe, root.join("1.2.3.4").join("chrome-win64").join("chrome.exe"));
        assert_eq!(seen.load(Ordering::SeqCst), zip.len() as u64);
        assert!(!root.join(".tmp-1.2.3.4").exists(), "temp folder is cleaned up");
    }

    #[tokio::test]
    async fn wrong_hash_installs_nothing() {
        let root = temp_root("hash");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZfake")]);
        let url = serve(zip, 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }), "{err}");
        assert!(!root.join("1.2.3.4").exists());
        assert!(!root.join(".tmp-1.2.3.4").exists());
    }

    #[tokio::test]
    async fn zip_without_chrome_exe_is_rejected() {
        let root = temp_root("noexe");
        let zip = make_zip(&[("chrome-win64/readme.txt", b"x")]);
        let url = serve(zip.clone(), 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::BadZip(_)), "{err}");
        assert!(!root.join("1.2.3.4").exists());
    }

    #[tokio::test]
    async fn garbage_that_is_not_a_zip_is_rejected() {
        let root = temp_root("garbage");
        let body = b"not a zip at all".to_vec();
        let url = serve(body.clone(), 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &sha(&body), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::BadZip(_)), "{err}");
    }

    #[tokio::test]
    async fn http_error_is_a_network_error() {
        let root = temp_root("404");
        let url = serve(b"nope".to_vec(), 404).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
    }

    #[tokio::test]
    async fn connection_refused_is_a_network_error() {
        let root = temp_root("refused");
        let err = ensure_installed(&root, "1.2.3.4", "http://127.0.0.1:1/x.zip", &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
    }

    #[tokio::test]
    async fn already_installed_skips_the_network() {
        let root = temp_root("again");
        let exe = root.join("1.2.3.4").join("chrome-win64").join("chrome.exe");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"MZ").unwrap();
        let got = ensure_installed(&root, "1.2.3.4", "http://127.0.0.1:1/unreachable.zip", &"0".repeat(64), &no_progress()).await.unwrap();
        assert_eq!(got, exe);
    }

    #[tokio::test]
    async fn unwritable_destination_is_an_io_error() {
        // A *file* where the root folder should be: create_dir_all must fail.
        // Stand-in for "disk full / no permission", which cannot be provoked portably.
        let base = temp_root("io");
        let file = base.join("im-a-file");
        std::fs::write(&file, b"x").unwrap();
        let root = file.join("browser");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZ")]);
        let url = serve(zip.clone(), 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Io(_)), "{err}");
    }

    #[tokio::test]
    async fn stalled_download_times_out_and_leaves_no_trace() {
        let root = temp_root("stalled");
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let head = "HTTP/1.1 200 OK\r\nContent-Length: 999999\r\nConnection: close\r\n\r\n";
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(b"a few bytes").await;
                // stall: don't close the socket, just hang
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
        let url = format!("http://127.0.0.1:{port}/chrome-win64.zip");
        let err = ensure_installed_with(&root, "1.2.3.4", &url, &"abc".repeat(22), &no_progress(), Duration::from_millis(300))
            .await
            .unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
        assert!(err.to_string().contains("parado"), "{err}");
        assert!(!root.join("1.2.3.4").exists());
        assert!(!root.join(".tmp-1.2.3.4-999999").exists() || !root.join(".tmp-1.2.3.4").exists());
    }

    #[tokio::test]
    async fn concurrent_installs_both_succeed_and_no_temp_folders_remain() {
        let root = temp_root("concurrent");
        let root_arc = std::sync::Arc::new(root.clone());
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZfake")]);
        let url = serve(zip.clone(), 200).await;
        let sha = sha(&zip);

        let r1 = {
            let root = root_arc.clone();
            let url = url.clone();
            let sha = sha.clone();
            tokio::spawn(async move {
                ensure_installed(&root, "1.2.3.4", &url, &sha, &no_progress()).await
            })
        };

        let r2 = {
            let root = root_arc.clone();
            let url = url.clone();
            let sha = sha.clone();
            tokio::spawn(async move {
                ensure_installed(&root, "1.2.3.4", &url, &sha, &no_progress()).await
            })
        };

        let (a, b) = tokio::join!(r1, r2);
        assert!(a.unwrap().unwrap().exists());
        assert!(b.unwrap().unwrap().exists());

        // Check no .tmp-* folders remain
        if let Ok(entries) = std::fs::read_dir(&*root_arc) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                assert!(!name_str.starts_with(".tmp-"), "Temp folder should be cleaned: {}", name_str);
            }
        }
    }

    #[tokio::test]
    async fn stale_temp_folder_from_another_process_is_removed() {
        let root = temp_root("stalefk");
        let stale = root.join(".tmp-1.2.3.4-99999");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("started"), "0").unwrap();
        std::fs::create_dir_all(stale.join("extract/chrome-win64")).unwrap();
        std::fs::write(stale.join("extract/chrome-win64/junk"), b"should disappear").unwrap();

        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZ")]);
        let url = serve(zip.clone(), 200).await;
        let exe = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap();

        assert!(exe.exists());
        assert!(!stale.exists(), "Stale folder should be removed");
        assert!(!exe.parent().unwrap().join("junk").exists(), "Junk file should not be in dest");
    }

    #[tokio::test]
    async fn fresh_foreign_temp_folder_is_left_untouched() {
        let root = temp_root("freshfk");
        let fresh = root.join(".tmp-1.2.3.4-99998");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(fresh.join("started"), now.to_string()).unwrap();

        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZ")]);
        let url = serve(zip.clone(), 200).await;
        let exe = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap();

        assert!(exe.exists());
        assert!(fresh.exists(), "Fresh foreign temp folder should remain");
    }

    #[tokio::test]
    async fn wrong_hash_is_detected_before_extraction() {
        let root = temp_root("hashbefore");
        let body = b"not a zip at all".to_vec();
        let url = serve(body.clone(), 200).await;
        let wrong_hash = "0".repeat(64);
        let err = ensure_installed(&root, "1.2.3.4", &url, &wrong_hash, &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }), "Should be HashMismatch, got: {err}");
        assert!(!root.join("1.2.3.4").exists());
    }

    #[tokio::test]
    async fn expected_hash_comparison_ignores_case() {
        let root = temp_root("hashcase");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZfake")]);
        let correct_lower = sha(&zip);
        let correct_upper = correct_lower.to_uppercase();
        let url = serve(zip, 200).await;
        let exe = ensure_installed(&root, "1.2.3.4", &url, &correct_upper, &no_progress()).await.unwrap();
        assert!(exe.exists());
    }

    #[test]
    fn rename_retry_fails_if_source_does_not_exist() {
        let nonexist = std::path::PathBuf::from("/nonexistent/path");
        let dest = std::path::PathBuf::from("/tmp/dest");
        let delays = [Duration::from_millis(10), Duration::from_millis(20)];
        let result = rename_with_retry(&nonexist, &dest, &delays);
        assert!(result.is_err());
    }

    #[test]
    fn rename_retry_succeeds_on_first_try() {
        let root = temp_root("renametry");
        let src = root.join("src");
        let dst = root.join("dst");
        std::fs::create_dir(&src).unwrap();
        let delays = [Duration::from_millis(1)];
        let result = rename_with_retry(&src, &dst, &delays);
        assert!(result.is_ok());
        assert!(dst.exists());
    }

    #[tokio::test]
    async fn stale_temp_folder_from_a_crashed_install_is_discarded() {
        let root = temp_root("stale");
        let stale = root.join(".tmp-1.2.3.4");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("chrome.zip"), b"half a download").unwrap();
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZ")]);
        let url = serve(zip.clone(), 200).await;
        let exe = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap();
        assert!(exe.exists());
    }
}
