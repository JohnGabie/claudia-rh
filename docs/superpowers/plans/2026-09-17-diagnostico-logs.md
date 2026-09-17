# Diagnostic logs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist app/PTY/MCP/panic/watchdog traces locally, mirror them to `debug-logs/` in `tauri dev` so a coding agent can read them without a zip, and let the user export a redacted zip from Settings.

**Architecture:** A `diagnostics` Rust module owns paths, redaction, a 100 KiB PTY ring, JSONL events, and zip export. Official `tauri-plugin-log` writes `claudia-rh.log` with rotation. Dual-write (AppData `diagnostics/` + repo `debug-logs/` when `debug_assertions`). Settings only saves a zip; nothing uploads.

**Tech Stack:** Tauri 2, `tauri-plugin-log`, crate `log`, `chrono` (already present), crate `zip` 2.x, existing `tauri-plugin-dialog`, React Settings.

**Spec:** `docs/superpowers/specs/2026-09-17-diagnostico-logs-design.md`

## Global Constraints

- Comments and commit messages: English. New identifiers: English. UI copy: pt-BR.
- NEVER change `src-tauri/tauri.conf.json` `identifier`.
- Do not `git add` `npm-package/`, `debug-logs/`, YAML, or `*.db`.
- Zip must never contain `candidate_base.yaml`, `search_variants.yaml`, `notif.json`, or keyring dumps.
- MCP events log **tool name + ok/err only** — never tool args.
- Work on `feat/diagnostico-logs`. Merge back to `dev`. Never commit on `main`.
- Cwd git: `C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh`. Cargo: `src-tauri/`.
- `strip_ansi` is **not** on this branch (lives on unmerged `feat/fix-api-stall-continue`). Implement it under `diagnostics/`.

## File map

| File | Responsibility |
|---|---|
| `src-tauri/src/diagnostics/redact.rs` | `redact` + tests |
| `src-tauri/src/diagnostics/ansi.rs` | `strip_ansi` + tests |
| `src-tauri/src/diagnostics/mod.rs` | paths, `emit_event`, PTY ring, zip, panic hook, init |
| `src-tauri/src/diagnostics/watchdog.rs` | heartbeat + freeze detect |
| `src-tauri/src/lib.rs` | plugin-log, hook, commands, boot/exit |
| `src-tauri/src/pty_manager.rs` | ring push + pty events |
| `src-tauri/src/mcp/mod.rs` | `tool_ok` / `tool_err` without args |
| `src/components/Configuracoes.tsx` | Diagnóstico section |
| `src/i18n/en.ts`, `pt.ts` | keys |
| `src/main.tsx` | forwardConsole warn/error |
| `CLAUDE.md`, `.gitignore` | paths |

---

### Task 1: `redact` + `strip_ansi` (TDD)

**Files:**
- Create: `src-tauri/src/diagnostics/redact.rs`
- Create: `src-tauri/src/diagnostics/ansi.rs`
- Create: `src-tauri/src/diagnostics/mod.rs` (module glue only)

**Interfaces:**
- Consumes: nothing
- Produces:

```rust
pub fn redact(s: &str) -> String;
pub fn strip_ansi(input: &str) -> String;
```

- [ ] **Step 1: Create `mod.rs` that declares the two modules**

````rust
pub mod ansi;
pub mod redact;

pub use ansi::strip_ansi;
pub use redact::redact;
````

Add `mod diagnostics;` in `src-tauri/src/lib.rs` next to `mod notificacoes;`.

- [ ] **Step 2: Write failing tests in `redact.rs`**

````rust
pub fn redact(s: &str) -> String {
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn masks_email_phone_and_token() {
        let raw = "write joao@mail.com or +5511999887766 with sk-ant-abcdefghijklmnopqrstuv";
        let out = redact(raw);
        assert!(out.contains("[email]"), "{out}");
        assert!(out.contains("[phone]"), "{out}");
        assert!(out.contains("[token]"), "{out}");
        assert!(!out.contains("joao@mail.com"));
        assert!(!out.contains("sk-ant-abcdefghijklmnopqrstuv"));
    }

    #[test]
    fn leaves_clean_text() {
        assert_eq!(redact("Acme Backend vaga"), "Acme Backend vaga");
    }

    #[test]
    fn masks_bearer_and_password_assignment() {
        let out = redact("Authorization: Bearer abcdefghijklmnopqrstuvwxyz12 password=secret123");
        assert!(out.contains("Bearer [token]"), "{out}");
        assert!(out.contains("[redacted]"), "{out}");
        assert!(!out.contains("secret123"));
    }
}
````

- [ ] **Step 3: Run tests — they must fail**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh\src-tauri"
cargo test --lib diagnostics::redact -- --nocapture
```

Expected: `masks_email_phone_and_token` FAIL.

- [ ] **Step 4: Implement `redact`**

Use `regex` crate **only if** already in Cargo.toml. It is **not**. Implement with simple scans (no new regex crate):

- Email: copy chars; if you see `@` with alphanum/dot/`-`/`_` on both sides, replace the whole token with `[email]`. Simpler: iterate with a helper that uses `split` on whitespace and punctuation and rebuilds. Recommended implementation:

````rust
pub fn redact(s: &str) -> String {
    let mut out = s.to_string();
    out = mask_emails(&out);
    out = mask_phones(&out);
    out = out.replace("Bearer ", "Bearer [token]");
    // collapse leftover bearer token after the marker: "Bearer [token]XXXX" → already replaced first
    out = mask_sk_tokens(&out);
    out = mask_assignment(&out, "password=");
    out = mask_assignment(&out, "senha=");
    out
}
````

`mask_emails`: for each whitespace-separated token, if it contains exactly one `@` and a `.` after `@`, replace that token with `[email]`.

`mask_phones`: replace runs of at least 9 digits (allowing spaces, dashes, parentheses, leading `+`) with `[phone]`. Walk the string; when a `+` or digit starts a run whose digit-count ≥ 9, replace the run.

`mask_sk_tokens`: find `sk-ant-` or `sk-` followed by ≥ 20 ASCII alphanumeric chars; replace the whole match with `[token]`.

`mask_assignment`: case-insensitive find `password=` / `senha=`; replace from `=` through the next whitespace with `=[redacted]`.

- [ ] **Step 5: `strip_ansi` tests + impl in `ansi.rs`** (same CSI/OSC loop as the API-stall branch)

````rust
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) { break; }
                }
            }
            Some(']') => {
                chars.next();
                for d in chars.by_ref() {
                    if d == '\u{7}' || d == '\u{9c}' { break; }
                    if d == '\u{1b}' { let _ = chars.next(); break; }
                }
            }
            Some(_) => { let _ = chars.next(); }
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;
    #[test]
    fn joins_letters_split_by_csi() {
        assert_eq!(strip_ansi("W\u{1b}[0maiting"), "Waiting");
    }
}
````

- [ ] **Step 6: Re-run tests — all pass**

```powershell
cargo test --lib diagnostics -- --nocapture
```

Expected: redact 3 + ansi 1 passed.

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/src/diagnostics src-tauri/src/lib.rs
git commit -m "feat(diagnostics): add redact and strip_ansi helpers"
```

---

### Task 2: Paths, `emit_event`, PTY ring

**Files:**
- Modify: `src-tauri/src/diagnostics/mod.rs`

**Interfaces:**
- Consumes: `redact`, `strip_ansi`
- Produces:

```rust
pub struct DiagPaths {
    pub dir: PathBuf,          // app_data_dir/diagnostics
    pub debug_dir: Option<PathBuf>, // Some(repo/debug-logs) when cfg!(debug_assertions)
}

pub fn init_paths(app_data_dir: &Path) -> DiagPaths;

pub fn emit_event(cat: &str, evt: &str, sid: Option<i64>, msg: &str);

pub fn pty_push(chunk: &str); // strip + redact + ring 100 KiB; flush ≤2s
pub fn pty_flush();

pub static PATHS: once_cell::sync::OnceCell<DiagPaths>;
```

JSONL line exactly:

```json
{"ts":"<rfc3339 local>","lvl":"info","cat":"<cat>","evt":"<evt>","sid":42,"msg":"<redact(msg)>"}
```

`sid` JSON `null` when `None`. `ts` via `chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)`.

- [ ] **Step 1: Failing test for ring cap + redact on push** (in `mod.rs` tests, using a temp dir)

Before `init_paths` exists, write tests that call a test-only constructor:

```rust
#[cfg(test)]
pub fn init_paths_for_test(dir: PathBuf) -> DiagPaths {
    std::fs::create_dir_all(&dir).ok();
    let p = DiagPaths { dir: dir.clone(), debug_dir: None };
    let _ = PATHS.set(p.clone());
    p
}
```

`DiagPaths` must be `Clone`. If `OnceCell` is already set from a previous test, skip `set` and write directly to `dir` via an argument — **better:** don't use the global in unit tests. Prefer:

```rust
pub fn emit_event_to(dir: &Path, debug: Option<&Path>, cat: &str, evt: &str, sid: Option<i64>, msg: &str);
pub fn emit_event(...) { /* reads PATHS, calls emit_event_to */ }
```

Ring as `Mutex<PtyRing>` with `append(&str)`, `snapshot() -> String`, `max = 100 * 1024`.

Test:

````rust
#[test]
fn emit_event_writes_jsonl_and_redacts() {
    let dir = std::env::temp_dir().join(format!("claudia-diag-evt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    emit_event_to(&dir, None, "session", "started", Some(7), "user=a@b.com");
    let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    assert!(text.contains("\"cat\":\"session\""));
    assert!(text.contains("\"evt\":\"started\""));
    assert!(text.contains("[email]"));
    assert!(!text.contains("a@b.com"));
}

#[test]
fn pty_ring_caps_at_100kib() {
    let mut r = PtyRing::new();
    r.push(&"x".repeat(80_000));
    r.push(&"y".repeat(80_000));
    assert!(r.snapshot().len() <= 100 * 1024);
    assert!(r.snapshot().contains('y'));
}
````

- [ ] **Step 2: Run — fail (functions missing)**

```powershell
cargo test --lib diagnostics -- --nocapture
```

- [ ] **Step 3: Implement**

`init_paths`:

````rust
pub fn init_paths(app_data_dir: &Path) -> DiagPaths {
    let dir = app_data_dir.join("diagnostics");
    std::fs::create_dir_all(&dir).ok();
    let debug_dir = if cfg!(debug_assertions) {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("debug-logs");
        std::fs::create_dir_all(&p).ok();
        Some(p)
    } else {
        None
    };
    let paths = DiagPaths { dir, debug_dir };
    let _ = PATHS.set(paths.clone());
    paths
}

fn write_both(name: &str, append_line: &str) {
    let Some(p) = PATHS.get() else { return };
    for root in std::iter::once(&p.dir).chain(p.debug_dir.as_ref()) {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(root.join(name)) {
            let _ = f.write_all(append_line.as_bytes());
        }
    }
}
````

`pty_push`: `strip_ansi` then `redact` then ring; if `last_flush.elapsed() >= 2s` write `pty-tail.log` as the **full snapshot** (overwrite, not append — the file is the current tail). `pty_flush` always overwrites.

- [ ] **Step 4: Tests pass**

```powershell
cargo test --lib diagnostics -- --nocapture
```

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/diagnostics
git commit -m "feat(diagnostics): JSONL events and PTY ring with dual-write paths"
```

---

### Task 3: Plugin-log, panic hook, gitignore, CLAUDE.md

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `log = "0.4"`, `tauri-plugin-log = "2"`)
- Modify: `src-tauri/src/lib.rs` (`run()`)
- Modify: `src-tauri/capabilities/default.json` (add `"log:default"`)
- Modify: `.gitignore` (add `debug-logs/`)
- Modify: `CLAUDE.md` (diagnostic paths)
- Modify: `src-tauri/src/diagnostics/mod.rs` (`install_panic_hook`)

**Interfaces:**
- Consumes: `init_paths`, `write_both` / `emit_event`
- Produces: logger attached before other plugins' noisy boot; panic hook after logger

- [ ] **Step 1: Add crates**

In `src-tauri/`:

```powershell
cargo add log@0.4 tauri-plugin-log@2
```

Do not change `identifier`.

- [ ] **Step 2: Init plugin as the first `.plugin(...)` in `run()`**

````rust
.plugin(
    tauri_plugin_log::Builder::new()
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        .max_file_size(2 * 1024 * 1024)
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
        .level(log::LevelFilter::Info)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("webview", log::LevelFilter::Warn)
        .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Folder {
                path: /* cannot know app_data_dir yet — see below */,
                file_name: Some("claudia-rh".into()),
            },
        ))
        .build(),
)
````

**Problem:** `app_data_dir` is only known inside `setup`. Do **not** block on that.

Use default `LogDir` target (plugin default: LocalAppData logs) **plus** our Folder after setup is not easily reconfigurable.

**Ruling (bind this):** keep plugin default `LogDir` + `Stdout` (plugin defaults) with `KeepSome(5)` if that variant exists; **if `KeepSome(5)` does not compile**, use `KeepAll` and `max_file_size(2 * 1024 * 1024)`. Additionally, in `setup` after `init_paths`, `log::info!("diagnostics dir {}", paths.dir.display())`.

Our structured files (`events.jsonl`, `pty-tail.log`, `panic.log`) live in `diagnostics/` via `write_both`. The rotating `claudia-rh.log` from the plugin lives in the plugin LogDir. **Zip task must copy both:** plugin log dir files matching `claudia-rh.log*` **and** `diagnostics/*`.

To find plugin logs on Windows: `app.path().app_log_dir()` if available on Tauri 2 `PathResolver`; else `app.path().app_data_dir()?.parent()` join. In setup, store `app.path().app_log_dir().ok()` on `DiagPaths.log_dir: Option<PathBuf>`.

If `KeepSome` API is `KeepSome { max: 5 }` instead of `KeepSome(5)`, use the one that compiles. Do not add extra crates to guess.

- [ ] **Step 3: Panic hook** (call at end of `setup`, after `init_paths`)

````rust
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.to_string();
        log::error!("panic: {payload}");
        let line = format!(
            "{} {payload}\n",
            chrono::Local::now().to_rfc3339()
        );
        if let Some(p) = PATHS.get() {
            for root in std::iter::once(&p.dir).chain(p.debug_dir.as_ref()) {
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(root.join("panic.log")) {
                    let _ = f.write_all(line.as_bytes());
                }
            }
        }
        emit_event("panic", "panic", None, &payload);
        prev(info);
    }));
}
````

- [ ] **Step 4: `.gitignore`** — add a line `debug-logs/` after the existing `# Logs` block.

- [ ] **Step 5: `CLAUDE.md`** — after Superpowers section, add:

```markdown
## Diagnóstico

Em `npm run tauri dev`, lê `debug-logs/events.jsonl` e `debug-logs/pty-tail.log` na raiz do repo **antes** de perguntar o que aconteceu. Em produção (Windows): `%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\`.
```

- [ ] **Step 6: capabilities** — append `"log:default"` to the permissions array in `src-tauri/capabilities/default.json`.

- [ ] **Step 7: `cargo check`**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh\src-tauri"
cargo check
```

Expected: success.

- [ ] **Step 8: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/diagnostics src-tauri/capabilities/default.json .gitignore CLAUDE.md
git commit -m "feat(diagnostics): init tauri-plugin-log, panic hook, debug-logs path"
```

---

### Task 4: Wire PTY reader

**Files:**
- Modify: `src-tauri/src/pty_manager.rs` (`iniciar_claude` reader loop only)

**Interfaces:**
- Consumes: `diagnostics::pty_push`, `diagnostics::pty_flush`, `diagnostics::emit_event`, `diagnostics::strip_ansi` (or keep matching on raw `line_buf` as today)
- Produces: tail file updates; events `session.started` is **not** here (sessao.rs). Here: if this branch already has API-stall continue, also `pty.api_stall` / `pty.continue_sent` / `pty.continue_exhausted` / `pty.chrome_reconnect` / `pty.chrome_failed`. **If those watchers are absent on this branch, only add ring + chrome events that already exist.**

This branch is `origin/dev` without API-stall. Existing chrome reconnect stays. Add:

- every successful `read` chunk → `pty_push(&chunk)`
- on loop exit → `pty_flush()`
- on chrome reconnect attempt → `emit_event("pty", "chrome_reconnect", Some(session_id), "")`
- on `chrome-reconnect-failed` emit → also `emit_event("pty", "chrome_failed", Some(session_id), "")`

- [ ] **Step 1: At top of `pty_manager.rs` add `use crate::diagnostics;`**

- [ ] **Step 2: Inside `Ok(n)` after `emit("pty-output")`:**

```rust
crate::diagnostics::pty_push(&chunk);
crate::diagnostics::watchdog::tick(); // Task 5 may not exist yet — SKIP tick until Task 5. Do not call it here if the module is missing.
```

**Ruling:** Task 4 does **not** call watchdog. Task 5 adds `tick()` in both PTY and frontend.

- [ ] **Step 3: Before `break` on checkpoint and after the reader loop ends, `pty_flush()`.**

- [ ] **Step 4: `cargo check` + existing `cargo test --lib diagnostics`**

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/pty_manager.rs
git commit -m "feat(diagnostics): capture redacted PTY tail from execution session"
```

---

### Task 5: MCP events + watchdog + heartbeat

**Files:**
- Create: `src-tauri/src/diagnostics/watchdog.rs`
- Modify: `src-tauri/src/diagnostics/mod.rs` (`pub mod watchdog`)
- Modify: `src-tauri/src/mcp/mod.rs` (after `debug_log(...)`)
- Modify: `src-tauri/src/lib.rs` (register `diag_heartbeat`, start watchdog, `init_paths` in setup)
- Modify: `src/App.tsx` (5s interval invoke)

**Interfaces:**
- Consumes: `emit_event`, `DbState`
- Produces:

```rust
pub fn tick(); // AtomicU64 = now millis
#[tauri::command]
pub fn diag_heartbeat() { tick(); }

pub fn start(app: AppHandle, db: Arc<Mutex<Connection>>);
```

MCP: after existing `debug_log(cfg, tool, args, &result);` add:

```rust
match &result {
    Ok(_) => crate::diagnostics::emit_event("mcp", "tool_ok", None, tool),
    Err(e) => crate::diagnostics::emit_event("mcp", "tool_err", None, &format!("{tool}: {e}")),
}
```

Do **not** pass `args` into `emit_event`. Leave `mcp-debug.log` as-is (dev-only, not in zip).

- [ ] **Step 1: Watchdog tests** (pure: given last_tick and now and session_active, decide miss/recover)

````rust
pub fn watchdog_transition(
    session_active: bool,
    last_tick_ms: u64,
    now_ms: u64,
    already_missing: bool,
) -> Option<&'static str> {
    if !session_active {
        return None;
    }
    let stale = now_ms.saturating_sub(last_tick_ms) >= 60_000;
    match (stale, already_missing) {
        (true, false) => Some("heartbeat_miss"),
        (false, true) => Some("recovered"),
        _ => None,
    }
}
````

Tests: active + 61s + not missing → `heartbeat_miss`; then 1s + missing → `recovered`; inactive → None.

- [ ] **Step 2: Implement `start` loop** `sleep 10s`, query `SELECT COUNT(*) FROM sessoes WHERE terminada_em IS NULL`, `tick` stored in `AtomicU64`, emit events. Duration of miss: `now - last_tick` in `msg`.

- [ ] **Step 3: `init_paths(&data_dir)` + `install_panic_hook()` + `watchdog::start(...)` inside `setup` after db init.** If Task 3 already put hook there, don't duplicate.

- [ ] **Step 4: Register `diag_heartbeat` in `generate_handler!`.**

- [ ] **Step 5: App.tsx** — in the existing listener `useEffect`, add:

```ts
const hb = window.setInterval(() => {
  invoke("diag_heartbeat").catch(() => {});
}, 5000);
return () => { clearInterval(hb); /* existing unlisteners */ };
```

Merge into the **same** cleanup as the listen effect (one `useEffect`). Call `tick` conceptually via invoke only.

Also call `invoke("diag_heartbeat")` once on mount.

- [ ] **Step 6: PTY `Ok(n)` also `crate::diagnostics::watchdog::tick();`**

- [ ] **Step 7: `cargo test --lib diagnostics` + `cargo check`**

- [ ] **Step 8: Commit**

```powershell
git add src-tauri/src/diagnostics src-tauri/src/mcp/mod.rs src-tauri/src/lib.rs src-tauri/src/pty_manager.rs src/App.tsx
git commit -m "feat(diagnostics): MCP tool events and freeze watchdog"
```

---

### Task 6: Zip export

**Files:**
- Modify: `src-tauri/Cargo.toml` (`cargo add zip@2 --no-default-features --features deflate`)
- Modify: `src-tauri/src/diagnostics/mod.rs` (`export_zip`)
- Modify: `src-tauri/src/lib.rs` (command)

**Interfaces:**
- Consumes: `PATHS`, `redact`, `pty_flush`
- Produces:

```rust
#[tauri::command]
pub fn exportar_diagnostico(dest: String) -> Result<String, String>;
```

- [ ] **Step 1: Failing test** — write a diagnostics dir with `events.jsonl` containing an email and a fake `candidate_base.yaml` sitting in **parent** (must not be zipped). Export to temp zip; unzip; assert yaml absent; events line redacted.

Use `zip::ZipArchive` to read back.

Allowed zip members: `manifest.txt`, `events.jsonl`, `pty-tail.log`, `panic.log`, `claudia-rh.log`, `claudia-rh.log.1` … (basename starts with `claudia-rh.log`). **Refuse** any path containing `candidate_base`, `search_variants`, `notif.json`.

- [ ] **Step 2: Implement `export_zip(dest: &Path) -> Result<PathBuf, String>`**

`pty_flush()` first.

`manifest.txt` body:

```
app=claudia-rh
version=<CARGO_PKG_VERSION>
os=<std::env::consts::OS>
identifier=io.github.johngabie.claudia-rh
exported_at=<rfc3339>
```

For each allowed file that exists in `PATHS.dir` and in `PATHS.log_dir` (if any), read bytes, **redact as UTF-8 lossy string line by line**, write into zip with `zip::ZipWriter` + `SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)`.

- [ ] **Step 3: Command**

```rust
#[tauri::command]
pub fn exportar_diagnostico(dest: String) -> Result<String, String> {
    diagnostics::pty_flush();
    let p = diagnostics::export_zip(Path::new(&dest))?;
    Ok(p.to_string_lossy().into_owned())
}
```

Register in `generate_handler!`.

- [ ] **Step 4: Tests pass + `cargo check`**

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/diagnostics src-tauri/src/lib.rs
git commit -m "feat(diagnostics): export redacted zip for support"
```

---

### Task 7: Settings UI + i18n + console forward

**Files:**
- Modify: `src/i18n/en.ts` — inside `settings: {`, after `checkingUpdates`:

```ts
    diagnostics: "Diagnostics",
    diagnosticsDesc:
      "Creates a file with recent logs (no passwords and no résumé) for you to send to the developer.",
    diagnosticsExport: "Export diagnostics",
    diagnosticsSaved: "Saved: ",
    diagnosticsError: "Could not export diagnostics.",
```

- Modify: `src/i18n/pt.ts` — same keys:

```ts
    diagnostics: "Diagnóstico",
    diagnosticsDesc:
      "Gera um ficheiro com logs recentes (sem passwords nem o teu currículo) para enviares ao desenvolvedor.",
    diagnosticsExport: "Exportar diagnóstico",
    diagnosticsSaved: "Guardado: ",
    diagnosticsError: "Não foi possível exportar o diagnóstico.",
```

- Modify: `src/components/Configuracoes.tsx` — **after** the Updates `</Section>` (before welcome), add a Section using the same card styles as Updates.

Use `save` dialog from `@tauri-apps/plugin-dialog` (already a dependency):

```ts
import { save } from "@tauri-apps/plugin-dialog";
```

Handler:

```ts
const [diagMsg, setDiagMsg] = useState<string | null>(null);
const exportarDiagnostico = async () => {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const name = `ClaudiaRH-diagnostico-${now.getFullYear()}${pad(now.getMonth()+1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}.zip`;
  const dest = await save({ defaultPath: name, filters: [{ name: "Zip", extensions: ["zip"] }] });
  if (!dest) return;
  try {
    const path = await invoke<string>("exportar_diagnostico", { dest });
    setDiagMsg(t.settings.diagnosticsSaved + path);
  } catch {
    setDiagMsg(t.settings.diagnosticsError);
  }
};
```

Button label `t.settings.diagnosticsExport`. Description `t.settings.diagnosticsDesc`. Title `t.settings.diagnostics`.

- Modify: `src/main.tsx` — after imports:

```ts
import { warn, error } from "@tauri-apps/plugin-log";

function forwardConsole(fnName: "warn" | "error", logger: (m: string) => Promise<void>) {
  const original = console[fnName];
  console[fnName] = (...args: unknown[]) => {
    original(...args);
    logger(args.map(String).join(" ")).catch(() => {});
  };
}
forwardConsole("warn", warn);
forwardConsole("error", error);
```

Do **not** forward `console.log`.

Add `@tauri-apps/plugin-log` via `npm install @tauri-apps/plugin-log` in `claudia-rh/` (matches Cargo plugin).

- [ ] **Step 1: npm install + i18n keys + Settings section + main.tsx forward**

- [ ] **Step 2: `npx tsc --noEmit`** — if the only errors are pre-existing `plugin-dialog` / ChatView, do not fix those (out of scope). Your new files must not add errors.

- [ ] **Step 3: Commit**

```powershell
git add src/i18n/en.ts src/i18n/pt.ts src/components/Configuracoes.tsx src/main.tsx package.json package-lock.json
git commit -m "feat(diagnostics): Settings export button and console warn/error forward"
```

---

### Task 8: Boot/exit + verification

**Files:**
- Modify: `src-tauri/src/lib.rs` (`setup` boot event; `run` exit)

**Interfaces:**
- Consumes: `emit_event`, `PATHS`

- [ ] **Step 1: `previous_unclean` helper** in `diagnostics/mod.rs`

```rust
pub fn previous_unclean(dir: &Path) -> bool {
    let text = std::fs::read_to_string(dir.join("events.jsonl")).unwrap_or_default();
    let last_app = text.lines().rev().find(|l| l.contains("\"cat\":\"app\""));
    match last_app {
        None => false,
        Some(l) => !l.contains("\"evt\":\"exit\""),
    }
}
```

Test: file ending with boot and no exit → true; ending with exit → false.

- [ ] **Step 2: In `setup` after `init_paths`:**

```rust
let unclean = diagnostics::previous_unclean(&paths.dir);
diagnostics::emit_event("app", "boot", None, if unclean { "previous_unclean=true" } else { "" });
```

- [ ] **Step 3: Change `.run(tauri::generate_context!())` to:**

```rust
.build(tauri::generate_context!())
.expect("error while building tauri application")
.run(|_app, event| {
    if let tauri::RunEvent::Exit = event {
        crate::diagnostics::emit_event("app", "exit", None, "");
        crate::diagnostics::pty_flush();
    }
});
```

Note: `run()` currently chains `.invoke_handler(...).run(...)`. Replace the final `.run(generate_context!())` with `.build(generate_context!()).expect(...).run(|...|)`. Keep the invoke_handler on the Builder.

Also `sessao.rs` `iniciar_sessao` after successful spawn: `diagnostics::emit_event("session", "started", Some(session_id), motivo);`  
On session-ended path in pty_manager after the loop: `emit_event("session", "ended", Some(session_id), motivo);` — `motivo` is already a `&str` there.

Pause/resume: in `commands/sessao.rs` `registar_pausa_sessao` / `registar_retoma_sessao` add `emit_event("session", "paused"|"resumed", None, "")`.

- [ ] **Step 4: Verification commands**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git diff HEAD -- src-tauri/tauri.conf.json
# must be empty (no identifier change)

Set-Location src-tauri
cargo test --lib diagnostics -- --nocapture
cargo check
```

```powershell
Select-String -Path src-tauri\src\mcp\mod.rs -Pattern "emit_event\(\"mcp\"" 
# must not include `args`
```

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/diagnostics src-tauri/src/commands/sessao.rs src-tauri/src/pty_manager.rs
git commit -m "feat(diagnostics): boot/exit markers and session lifecycle events"
```

---

## Self-review (spec coverage)

| Spec | Task |
|---|---|
| `redact` table + tests | 1 |
| `strip_ansi` (stall branch not merged) | 1 |
| `diagnostics/` dir + `debug-logs/` dual-write | 2, 3 |
| `events.jsonl` schema | 2 |
| 100 KiB PTY tail, flush ≤2s | 2, 4 |
| `tauri-plugin-log` rotation 2 MiB KeepSome(5) | 3 (KeepAll fallback) |
| panic hook + `panic.log` | 3 |
| `.gitignore` + `CLAUDE.md` | 3 |
| PTY wire | 4 |
| MCP name-only | 5 |
| watchdog 60s | 5 |
| zip + second redact + no yaml | 6 |
| Settings button + dialog | 7 |
| forwardConsole warn/error only | 7 |
| boot/exit + previous_unclean | 8 |
| no Sentry, no identifier change | global + 8 |

No TBD. `KeepSome` compile fallback is explicit. Types: `emit_event(cat, evt, sid, msg)`, `pty_push`, `exportar_diagnostico(dest: String)` used consistently.
