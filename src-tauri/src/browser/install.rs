// Download, verify, extract and atomically place Chrome for Testing.
// Never leaves a half-installed folder: everything happens in a temp dir and
// is renamed into place only after the hash and the layout check out.
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// No bytes for this long means the download is stuck (a total timeout would
/// punish slow links: 150 MB can legitimately take many minutes).
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// A temp folder older than this belongs to an install that died.
const STALE_AFTER: Duration = Duration::from_secs(3600);
/// Antivirus often locks freshly extracted files for a moment; the rename is
/// retried before the whole download is thrown away.
const RENAME_DELAYS: [Duration; 3] =
    [Duration::from_millis(200), Duration::from_millis(400), Duration::from_millis(800)];

static NONCE: AtomicU64 = AtomicU64::new(0);

fn io(e: std::io::Error) -> InstallError {
    InstallError::Io(e.to_string())
}

fn stalled(idle: Duration) -> InstallError {
    InstallError::Network(format!("download parado: sem dados há {idle:?}"))
}

fn exe_path(root: &Path, version: &str) -> PathBuf {
    root.join(version).join("chrome-win64").join("chrome.exe")
}

/// `.tmp-<version>-<pid>-<n>`: unique per process AND per call, so two tasks
/// (or two app instances) never share a temp folder.
fn temp_dir_name(version: &str) -> String {
    format!(".tmp-{version}-{}-{}", std::process::id(), NONCE.fetch_add(1, Ordering::SeqCst))
}

/// Runs `f`, and on failure retries after each delay: `delays.len() + 1` attempts in total.
fn rename_with_retry_using<F>(from: &Path, to: &Path, delays: &[Duration], mut f: F) -> std::io::Result<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    let mut attempt = 0;
    loop {
        match f(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if attempt >= delays.len() => return Err(e),
            Err(_) => {
                std::thread::sleep(delays[attempt]);
                attempt += 1;
            }
        }
    }
}

/// Moves `extract` to `root/version`, unless a finished install is already there
/// (another app instance won the race): a good install is never touched, because
/// a Chrome may already be running from it.
fn place_into_dest<F>(extract: &Path, root: &Path, version: &str, delays: &[Duration], rename: F) -> Result<(), InstallError>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    let exe = exe_path(root, version);
    if exe.exists() {
        return Ok(());
    }
    let dest = root.join(version);
    let _ = std::fs::remove_dir_all(&dest); // a partial leftover (no chrome.exe inside)
    match rename_with_retry_using(extract, &dest, delays, rename) {
        Ok(()) => Ok(()),
        // The rename failed, but another instance may have finished meanwhile.
        Err(_) if exe.exists() => Ok(()),
        Err(e) => Err(io(e)),
    }
}

/// A foreign temp folder is stale when its `started` stamp (or, if that is
/// missing, its modification time) is older than `STALE_AFTER`. Anything
/// unreadable counts as FRESH: deleting a live instance's download is worse
/// than leaking a folder.
fn is_stale(dir: &Path, now: SystemTime) -> bool {
    let started = std::fs::read_to_string(dir.join("started"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|secs| UNIX_EPOCH + Duration::from_secs(secs));
    let when = started.or_else(|| std::fs::metadata(dir).and_then(|m| m.modified()).ok());
    match when {
        Some(t) => now.duration_since(t).is_ok_and(|age| age > STALE_AFTER),
        None => false,
    }
}

fn cleanup_stale_temp_folders(root: &Path, version: &str, now: SystemTime) {
    let prefix = format!(".tmp-{version}-");
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let is_ours_to_judge = entry.file_name().to_string_lossy().starts_with(&prefix);
        if is_ours_to_judge && is_stale(&entry.path(), now) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
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
    let exe = exe_path(root, version);
    if exe.exists() {
        return Ok(exe);
    }
    std::fs::create_dir_all(root).map_err(io)?;
    cleanup_stale_temp_folders(root, version, SystemTime::now());

    let tmp = root.join(temp_dir_name(version));
    std::fs::create_dir_all(&tmp).map_err(io)?;
    let result = install_into(&tmp, root, version, zip_url, expected_sha256, on_progress, idle_timeout).await;
    let _ = std::fs::remove_dir_all(&tmp); // on EVERY path, success or not

    match result {
        Ok(()) => Ok(exe),
        // Our attempt failed but another instance finished a good install.
        Err(_) if exe.exists() => Ok(exe),
        Err(e) => Err(e),
    }
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
    // Stamp first, so a crashed install is recognisable as stale later.
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    std::fs::write(tmp.join("started"), now.to_string()).map_err(io)?;

    let actual = download(zip_url, &tmp.join("chrome.zip"), on_progress, idle_timeout).await?;
    // The archive is not even opened until the hash matches.
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(InstallError::HashMismatch { expected: expected_sha256.to_string(), actual });
    }

    let (tmp, root, version) = (tmp.to_path_buf(), root.to_path_buf(), version.to_string());
    tokio::task::spawn_blocking(move || extract_and_place(&tmp, &root, &version))
        .await
        .map_err(|e| InstallError::Io(e.to_string()))?
}

fn extract_and_place(tmp: &Path, root: &Path, version: &str) -> Result<(), InstallError> {
    let file = std::fs::File::open(tmp.join("chrome.zip")).map_err(io)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| InstallError::BadZip(e.to_string()))?;
    let extract = tmp.join("extract");
    archive.extract(&extract).map_err(|e| InstallError::BadZip(e.to_string()))?;
    if !extract.join("chrome-win64").join("chrome.exe").exists() {
        return Err(InstallError::BadZip("chrome.exe ausente do arquivo".into()));
    }
    place_into_dest(&extract, root, version, &RENAME_DELAYS, |from, to| std::fs::rename(from, to))
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
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(|e| InstallError::Network(e.to_string()))?;

    // A server that accepts the connection but never sends headers must not hang us.
    let resp = tokio::time::timeout(idle_timeout, client.get(url).send())
        .await
        .map_err(|_| stalled(idle_timeout))?
        .map_err(|e| InstallError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(InstallError::Network(format!("HTTP {}", resp.status())));
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(to).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::time::timeout(idle_timeout, stream.next()).await.map_err(|_| stalled(idle_timeout))?;
        let Some(chunk) = next else { break };
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
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const V: &str = "1.2.3.4";

    fn temp_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "claudia-install-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sha(b: &[u8]) -> String {
        format!("{:x}", Sha256::digest(b))
    }

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

    fn chrome_zip() -> Vec<u8> {
        make_zip(&[("chrome-win64/chrome.exe", b"MZfake")])
    }

    /// Serves `body` with the given status on a random local port. Returns the URL.
    async fn serve(body: Vec<u8>, status: u16) -> String {
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

    /// A server that accepts, reads the request, and then either sends nothing
    /// at all (`headers = false`) or sends headers plus a few bytes and stalls.
    async fn serve_then_stall(headers: bool) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let _ = sock.read(&mut buf).await;
                    if headers {
                        let _ = sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\nConnection: close\r\n\r\na few bytes").await;
                    }
                    tokio::time::sleep(Duration::from_secs(30)).await; // hold the socket open
                });
            }
        });
        format!("http://127.0.0.1:{port}/chrome-win64.zip")
    }

    fn no_progress() -> impl Fn(Progress) + Send + Sync {
        |_p| {}
    }

    /// After ANY outcome, no temp folder may be left behind.
    fn assert_no_temp(root: &Path) {
        for e in std::fs::read_dir(root).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            assert!(!name.starts_with(".tmp-"), "leftover temp folder: {name}");
        }
    }

    // ── the install flow ─────────────────────────────────────────────────

    #[tokio::test]
    async fn installs_from_a_valid_zip_and_reports_progress() {
        let root = temp_root("ok");
        let zip = chrome_zip();
        let url = serve(zip.clone(), 200).await;
        let seen = Arc::new(AtomicU64::new(0));
        let s2 = seen.clone();
        let cb = move |p: Progress| s2.store(p.downloaded, Ordering::SeqCst);
        let exe = ensure_installed(&root, V, &url, &sha(&zip), &cb).await.unwrap();
        assert_eq!(exe, exe_path(&root, V));
        assert!(exe.exists());
        assert_eq!(seen.load(Ordering::SeqCst), zip.len() as u64);
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn wrong_hash_installs_nothing() {
        let root = temp_root("hash");
        let url = serve(chrome_zip(), 200).await;
        let err = ensure_installed(&root, V, &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }), "{err}");
        assert!(!root.join(V).exists());
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn wrong_hash_is_detected_before_the_archive_is_opened() {
        // Garbage with the wrong hash: if the zip were opened first, this would be BadZip.
        let root = temp_root("hash-first");
        let url = serve(b"not a zip".to_vec(), 200).await;
        let err = ensure_installed(&root, V, &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }), "{err}");
    }

    #[tokio::test]
    async fn the_expected_hash_may_be_uppercase() {
        let root = temp_root("upper");
        let zip = chrome_zip();
        let url = serve(zip.clone(), 200).await;
        let exe = ensure_installed(&root, V, &url, &sha(&zip).to_uppercase(), &no_progress()).await.unwrap();
        assert!(exe.exists());
    }

    #[tokio::test]
    async fn zip_without_chrome_exe_is_rejected() {
        let root = temp_root("noexe");
        let zip = make_zip(&[("chrome-win64/readme.txt", b"x")]);
        let url = serve(zip.clone(), 200).await;
        let err = ensure_installed(&root, V, &url, &sha(&zip), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::BadZip(_)), "{err}");
        assert!(!root.join(V).exists());
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn garbage_that_is_not_a_zip_is_rejected() {
        let root = temp_root("garbage");
        let body = b"not a zip at all".to_vec();
        let url = serve(body.clone(), 200).await;
        let err = ensure_installed(&root, V, &url, &sha(&body), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::BadZip(_)), "{err}");
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn http_error_is_a_network_error() {
        let root = temp_root("404");
        let url = serve(b"nope".to_vec(), 404).await;
        let err = ensure_installed(&root, V, &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn connection_refused_is_a_network_error() {
        let root = temp_root("refused");
        let err = ensure_installed(&root, V, "http://127.0.0.1:1/x.zip", &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn already_installed_skips_the_network() {
        let root = temp_root("again");
        let exe = exe_path(&root, V);
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"MZ").unwrap();
        let got = ensure_installed(&root, V, "http://127.0.0.1:1/unreachable.zip", &"0".repeat(64), &no_progress()).await.unwrap();
        assert_eq!(got, exe);
    }

    #[tokio::test]
    async fn unwritable_destination_is_an_io_error() {
        // A *file* where the root folder should be: create_dir_all must fail.
        // Stand-in for "disk full / no permission", which cannot be provoked portably.
        let base = temp_root("io");
        let file = base.join("im-a-file");
        std::fs::write(&file, b"x").unwrap();
        let zip = chrome_zip();
        let url = serve(zip.clone(), 200).await;
        let err = ensure_installed(&file.join("browser"), V, &url, &sha(&zip), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Io(_)), "{err}");
    }

    #[tokio::test]
    async fn a_download_that_stalls_mid_body_times_out() {
        let root = temp_root("stall-body");
        let url = serve_then_stall(true).await;
        let start = std::time::Instant::now();
        let err = ensure_installed_with(&root, V, &url, &"0".repeat(64), &no_progress(), Duration::from_millis(300)).await.unwrap_err();
        assert!(err.to_string().contains("parado"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(5), "took {:?}", start.elapsed());
        assert!(!root.join(V).exists());
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn a_server_that_never_sends_headers_times_out() {
        // The case a per-chunk timeout alone misses: send() itself must not hang.
        let root = temp_root("stall-headers");
        let url = serve_then_stall(false).await;
        let start = std::time::Instant::now();
        let err = ensure_installed_with(&root, V, &url, &"0".repeat(64), &no_progress(), Duration::from_millis(300)).await.unwrap_err();
        assert!(err.to_string().contains("parado"), "{err}");
        assert!(start.elapsed() < Duration::from_secs(5), "took {:?}", start.elapsed());
        assert_no_temp(&root);
    }

    #[tokio::test]
    async fn two_installs_at_once_both_succeed_and_leave_nothing_behind() {
        let root = temp_root("concurrent");
        let zip = chrome_zip();
        let url = serve(zip.clone(), 200).await;
        let h = sha(&zip);
        let (cb1, cb2) = (no_progress(), no_progress());
        let (a, b) = tokio::join!(
            ensure_installed(&root, V, &url, &h, &cb1),
            ensure_installed(&root, V, &url, &h, &cb2)
        );
        assert!(a.unwrap().exists());
        assert!(b.unwrap().exists());
        assert_eq!(std::fs::read(exe_path(&root, V)).unwrap(), b"MZfake");
        assert_no_temp(&root);
    }

    // ── temp folder naming and stale cleanup ─────────────────────────────

    #[test]
    fn temp_names_carry_version_pid_and_a_fresh_nonce_every_call() {
        let (a, b) = (temp_dir_name(V), temp_dir_name(V));
        assert_ne!(a, b, "two calls in one process must not share a folder");
        let pid = std::process::id().to_string();
        assert!(a.starts_with(&format!(".tmp-{V}-{pid}-")), "{a}");
    }

    fn stamped(root: &Path, name: &str, started: &str) -> PathBuf {
        let d = root.join(name);
        std::fs::create_dir_all(d.join("extract").join("chrome-win64")).unwrap();
        std::fs::write(d.join("extract").join("chrome-win64").join("junk"), b"x").unwrap();
        std::fs::write(d.join("started"), started).unwrap();
        d
    }

    fn now_secs() -> String {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()
    }

    #[test]
    fn staleness_follows_the_started_stamp_then_the_mtime() {
        let root = temp_root("stale");
        let now = SystemTime::now();
        assert!(is_stale(&stamped(&root, "a", "0"), now), "stamped long ago");
        assert!(!is_stale(&stamped(&root, "b", &now_secs()), now), "stamped just now");
        let no_stamp = root.join("c");
        std::fs::create_dir_all(&no_stamp).unwrap();
        assert!(!is_stale(&no_stamp, now), "no stamp, fresh mtime: leave it alone");
        assert!(is_stale(&no_stamp, now + Duration::from_secs(2 * 3600)), "no stamp, old mtime");
        assert!(!is_stale(&root.join("missing"), now), "unreadable counts as fresh");
        let garbage = stamped(&root, "d", "not a number");
        assert!(!is_stale(&garbage, now), "garbage stamp falls back to the (fresh) mtime");
    }

    #[test]
    fn cleanup_removes_only_stale_folders_of_this_version() {
        let root = temp_root("cleanup");
        let stale = stamped(&root, ".tmp-1.2.3.4-99999-0", "0");
        let fresh = stamped(&root, ".tmp-1.2.3.4-99998-0", &now_secs());
        let other_version = stamped(&root, ".tmp-9.9.9.9-99997-0", "0");
        let unrelated = stamped(&root, "keep-me", "0");
        cleanup_stale_temp_folders(&root, V, SystemTime::now());
        assert!(!stale.exists(), "stale folder of this version goes");
        assert!(fresh.exists(), "a live instance's folder is never touched");
        assert!(other_version.exists(), "another version's folder is not ours to judge");
        assert!(unrelated.exists());
    }

    #[tokio::test]
    async fn debris_of_a_dead_install_never_reaches_the_new_one() {
        let root = temp_root("debris");
        stamped(&root, &format!(".tmp-{V}-99999-0"), "0");
        let zip = chrome_zip();
        let url = serve(zip.clone(), 200).await;
        ensure_installed(&root, V, &url, &sha(&zip), &no_progress()).await.unwrap();
        assert!(!root.join(V).join("chrome-win64").join("junk").exists());
        assert!(!root.join(format!(".tmp-{V}-99999-0")).exists(), "the dead install's folder was cleaned");
    }

    // ── placing the extracted folder ─────────────────────────────────────

    fn extracted(root: &Path, marker: &[u8]) -> PathBuf {
        let e = root.join("work").join("extract");
        std::fs::create_dir_all(e.join("chrome-win64")).unwrap();
        std::fs::write(e.join("chrome-win64").join("chrome.exe"), marker).unwrap();
        e
    }

    const NO_DELAYS: [Duration; 0] = [];

    #[test]
    fn a_finished_install_is_never_touched() {
        let root = temp_root("place-keep");
        let exe = exe_path(&root, V);
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"OLD").unwrap();
        std::fs::write(root.join(V).join("keep.txt"), b"k").unwrap();
        let extract = extracted(&root, b"NEW");
        place_into_dest(&extract, &root, V, &NO_DELAYS, |_, _| panic!("must not rename over a finished install")).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"OLD");
        assert!(root.join(V).join("keep.txt").exists());
        assert!(extract.exists(), "the extract folder is left alone");
    }

    #[test]
    fn a_partial_leftover_without_chrome_exe_is_replaced() {
        let root = temp_root("place-partial");
        std::fs::create_dir_all(root.join(V).join("chrome-win64")).unwrap();
        std::fs::write(root.join(V).join("chrome-win64").join("half.dll"), b"x").unwrap();
        let extract = extracted(&root, b"NEW");
        place_into_dest(&extract, &root, V, &NO_DELAYS, |a, b| std::fs::rename(a, b)).unwrap();
        assert_eq!(std::fs::read(exe_path(&root, V)).unwrap(), b"NEW");
        assert!(!root.join(V).join("chrome-win64").join("half.dll").exists());
    }

    #[test]
    fn if_the_rename_fails_but_another_instance_finished_that_is_success() {
        let root = temp_root("place-race");
        let extract = extracted(&root, b"NEW");
        let exe = exe_path(&root, V);
        let r = place_into_dest(&extract, &root, V, &NO_DELAYS, |_, _| {
            // The other instance finishes in the middle of our rename.
            std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
            std::fs::write(&exe, b"THEIRS").unwrap();
            Err(std::io::Error::other("locked"))
        });
        assert!(r.is_ok());
        assert_eq!(std::fs::read(&exe).unwrap(), b"THEIRS");
    }

    #[test]
    fn if_the_rename_fails_and_nothing_was_installed_it_is_an_io_error() {
        let root = temp_root("place-fail");
        let extract = extracted(&root, b"NEW");
        let r = place_into_dest(&extract, &root, V, &NO_DELAYS, |_, _| Err(std::io::Error::other("locked")));
        assert!(matches!(r, Err(InstallError::Io(_))), "{r:?}");
    }

    // ── rename retry ─────────────────────────────────────────────────────

    #[test]
    fn rename_retry_gives_up_after_exactly_delays_plus_one_attempts() {
        let calls = AtomicUsize::new(0);
        let delays = [Duration::from_millis(1); 3];
        let r = rename_with_retry_using(Path::new("a"), Path::new("b"), &delays, |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::other("locked"))
        });
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn rename_retry_succeeds_when_a_later_attempt_works() {
        let calls = AtomicUsize::new(0);
        let delays = [Duration::from_millis(1); 3];
        let r = rename_with_retry_using(Path::new("a"), Path::new("b"), &delays, |_, _| {
            if calls.fetch_add(1, Ordering::SeqCst) < 2 { Err(std::io::Error::other("locked")) } else { Ok(()) }
        });
        assert!(r.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 3, "failed twice, succeeded on the third try");
    }

    #[test]
    fn rename_retry_does_not_retry_after_a_success() {
        let calls = AtomicUsize::new(0);
        let r = rename_with_retry_using(Path::new("a"), Path::new("b"), &[Duration::from_millis(1); 3], |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert!(r.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
