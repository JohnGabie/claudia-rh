# Honest Claude session spawn — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Spawn Claude with a short argv (`--system-prompt-file` + positional query), resolve WinGet `claude.exe`, and only mark the session active after spawn succeeds — with failures written to the terminal.

**Architecture:** Extract a pure `build_claude_cli_args` (unit-tested: no YAML body, no `--system-prompt`). `iniciar_sessao` / LinkedIn write the assembled prompt to `workspace/.claude-system-prompt.txt`, spawn, then emit `session-started`. `iniciar_claude` drops the 5s type-into-PTY timer. Errors mark `sessoes.motivo_termino = spawn_err` and emit `session-ended`.

**Tech Stack:** existing Tauri/portable-pty; Claude Code CLI v2.1+ (`--system-prompt-file`, `--strict-mcp-config`, `--debug-file`).

**Spec:** `docs/superpowers/specs/2026-09-17-sessao-spawn-design.md`

## Global Constraints

- Comments and commits: English. New identifiers: English. UI copy that users see: pt-BR.
- NEVER change `src-tauri/tauri.conf.json` `identifier`.
- Do not pass the system-prompt **body** as a CLI argument. Path only.
- Do not remove `--chrome`.
- Do not `git add` yaml, `npm-package/`, `debug-logs/`.
- Work on `feat/fix-sessao-spawn`. Merge to `dev`. Never `main`.
- Cwd: `C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh`. Cargo tests: `src-tauri/`.

## File map

| File | Responsibility |
|---|---|
| `src-tauri/src/commands/sessao.rs` | `build_claude_cli_args`, prompt file, spawn-then-started, spawn_err |
| `src-tauri/src/commands/mod.rs` | `claude_program` + WinGet search |
| `src-tauri/src/pty_manager.rs` | drop 5s timer; args already include query |
| `src-tauri/src/commands/linkedin.rs` | same argv + lifecycle as sessao |
| `src-tauri/src/commands/perfil.rs` | MCP `--debug` already debug-only; keep |
| `src-tauri/src/diagnostics/mod.rs` | zip allow `claude-startup.log` |

---

### Task 1: Pure argv builder + tests (TDD)

**Files:**
- Modify: `src-tauri/src/commands/sessao.rs` (add `pub(crate) fn build_claude_cli_args` + `#[cfg(test)]`)

**Interfaces:**
- Consumes: nothing
- Produces:

```rust
pub(crate) fn build_claude_cli_args(
    skip_permissions: bool,
    mcp_config: Option<&std::path::Path>,
    prompt_file: &std::path::Path,
    debug_file: Option<&std::path::Path>,
    initial_query: &str,
) -> Vec<String>
```

- [ ] **Step 1: Write failing tests at the bottom of `sessao.rs`**

````rust
#[cfg(test)]
mod tests {
    use super::build_claude_cli_args;
    use std::path::Path;

    #[test]
    fn args_use_prompt_file_not_body() {
        let args = build_claude_cli_args(
            true,
            Some(Path::new("C:/data/mcp-config.json")),
            Path::new("C:/data/workspace/.claude-system-prompt.txt"),
            Some(Path::new("C:/data/diagnostics/claude-startup.log")),
            "Inicia a sessao de candidaturas.",
        );
        let joined = args.join("\x1e");
        assert!(args.contains(&"--system-prompt-file".to_string()));
        assert!(!args.iter().any(|a| a == "--system-prompt"));
        assert!(!joined.contains("nome_completo"));
        assert!(!args.iter().any(|a| a.len() > 4000), "an arg is huge: {}", args.iter().map(|a| a.len()).max().unwrap_or(0));
        assert!(args.contains(&"--strict-mcp-config".to_string()));
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(args.contains(&"--chrome".to_string()));
        assert_eq!(args.last().unwrap(), "Inicia a sessao de candidaturas.");
        let i = args.iter().position(|a| a == "--system-prompt-file").unwrap();
        assert!(args[i + 1].replace('\\', "/").ends_with("workspace/.claude-system-prompt.txt"));
    }

    #[test]
    fn supervised_omits_skip_permissions() {
        let args = build_claude_cli_args(false, None, Path::new("p.txt"), None, "hi");
        assert!(!args.iter().any(|a| a == "--dangerously-skip-permissions"));
        assert!(!args.iter().any(|a| a == "--mcp-config"));
        assert!(!args.iter().any(|a| a == "--debug-file"));
        assert_eq!(args.last().unwrap(), "hi");
    }
}
````

Do **not** implement `build_claude_cli_args` yet (or stub `todo!()` so tests compile and panic).

- [ ] **Step 2: Run tests — they must fail**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh\src-tauri"
cargo test --lib commands::sessao::tests -- --nocapture
```

Expected: compile fail (`build_claude_cli_args` missing) or test panic on `todo!()`.

- [ ] **Step 3: Implement**

````rust
pub(crate) fn build_claude_cli_args(
    skip_permissions: bool,
    mcp_config: Option<&std::path::Path>,
    prompt_file: &std::path::Path,
    debug_file: Option<&std::path::Path>,
    initial_query: &str,
) -> Vec<String> {
    let mut args = Vec::new();
    if skip_permissions {
        args.push("--dangerously-skip-permissions".to_string());
    }
    args.push("--chrome".to_string());
    if let Some(mcp) = mcp_config {
        args.push("--mcp-config".to_string());
        args.push(mcp.to_string_lossy().into_owned());
        args.push("--strict-mcp-config".to_string());
    }
    args.push("--system-prompt-file".to_string());
    args.push(prompt_file.to_string_lossy().into_owned());
    if let Some(df) = debug_file {
        args.push("--debug-file".to_string());
        args.push(df.to_string_lossy().into_owned());
    }
    args.push(initial_query.to_string());
    args
}
````

- [ ] **Step 4: Tests pass**

```powershell
cargo test --lib commands::sessao::tests -- --nocapture
```

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/commands/sessao.rs
git commit -m "test(session): CLI args use --system-prompt-file not prompt body"
```

---

### Task 2: `claude_program` finds WinGet

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `claude_program()` still `-> String`; plus testable helper:

```rust
pub(crate) fn first_existing_claude(candidates: &[std::path::PathBuf]) -> Option<std::path::PathBuf>
```

- [ ] **Step 1: Failing test** (in `mod.rs` `#[cfg(test)]`)

````rust
#[cfg(test)]
mod tests {
    use super::first_existing_claude;
    use std::path::PathBuf;

    #[test]
    fn first_existing_skips_missing() {
        let missing = PathBuf::from("C:/definitely-not-a-claude-xxxx.exe");
        let cargo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let got = first_existing_claude(&[missing, cargo.clone()]);
        assert_eq!(got.as_ref(), Some(&cargo));
    }

    #[test]
    fn first_existing_none() {
        assert!(first_existing_claude(&[PathBuf::from("C:/nope-a.exe"), PathBuf::from("C:/nope-b.exe")]).is_none());
    }
}
````

- [ ] **Step 2: Run — fail (function missing)**

```powershell
cargo test --lib commands::tests -- --nocapture
```

- [ ] **Step 3: Implement helper and use it in `claude_program`**

````rust
pub(crate) fn first_existing_claude(candidates: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    candidates.iter().find(|p| p.is_file()).cloned()
}
````

In the Windows branch of `claude_program`, **after** the existing npm + PATH loops, before `return "claude"`:

````rust
        // 3) WinGet install (Claude Code native package)
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            let mut winget = vec![
                local.join("Microsoft").join("WinGet").join("Links").join("claude.exe"),
            ];
            let pkg = local
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if let Ok(rd) = std::fs::read_dir(&pkg) {
                for ent in rd.flatten() {
                    let name = ent.file_name();
                    let n = name.to_string_lossy();
                    if n.starts_with("Anthropic.ClaudeCode") {
                        winget.push(ent.path().join("claude.exe"));
                    }
                }
            }
            if let Some(p) = first_existing_claude(&winget) {
                return p.to_string_lossy().into_owned();
            }
        }
````

Keep steps 1–2 (npm, PATH) **before** WinGet. Final fallback remains `"claude".to_string()`.

- [ ] **Step 4: Tests pass + `cargo check`**

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/commands/mod.rs
git commit -m "fix(session): resolve WinGet claude.exe"
```

---

### Task 3: `iniciar_sessao` lifecycle + prompt file

**Files:**
- Modify: `src-tauri/src/commands/sessao.rs` (`iniciar_sessao` only, reuse `build_claude_cli_args`)
- Modify: `src-tauri/src/diagnostics/mod.rs` (`zip_member_allowed` add `"claude-startup.log"`)

**Interfaces:**
- Consumes: `build_claude_cli_args`, `claude_program`, `pty_manager::iniciar_claude` (still with `initial_task` until Task 4 — **pass empty string** for now if signature unchanged, OR Task 4 lands first)

**Ruling:** Task 4 will remove `initial_task`. This task still calls `iniciar_claude(..., query.clone())` with the same signature so it compiles. Task 4 then stops using that parameter as a typed delay.

- [ ] **Step 1: Rewrite `iniciar_sessao` body after workspace git init**

Replace from `let sys_prompt = {` through `Ok(())` with:

````rust
    let sys_prompt = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        prompt::montar_prompt_sistema(&conn, &data_dir, &db_path)
    };
    let prompt_file = workspace.join(".claude-system-prompt.txt");
    std::fs::write(&prompt_file, &sys_prompt).map_err(|e| e.to_string())?;

    let skip_permissions = ler_skip_permissions(&data_dir);
    let mcp_config = crate::commands::perfil::write_mcp_config(app);
    let debug_file = crate::diagnostics::PATHS
        .get()
        .map(|p| p.dir.join("claude-startup.log"));
    let query = "Inicia a sessao de candidaturas.".to_string();
    let args = build_claude_cli_args(
        skip_permissions,
        mcp_config.as_deref(),
        &prompt_file,
        debug_file.as_deref(),
        &query,
    );

    let claude = crate::commands::claude_program();
    if cfg!(windows) && claude == "claude" {
        // last-resort string; still try spawn, but warn
        let msg = "\r\n\x1b[1;31m[Claudia RH]\x1b[0m claude.exe not found on PATH/WinGet/npm — trying 'claude' anyway.\r\n";
        app.emit("pty-output", msg).ok();
    }

    let spawn = pty_manager::iniciar_claude(
        app.clone(),
        claude.clone(),
        args,
        24,
        80,
        session_id,
        Arc::clone(&db),
        workspace.to_string_lossy().into_owned(),
        query,
    );

    match spawn {
        Ok(()) => {
            let modo_txt = if skip_permissions { "autonomous" } else { "supervised" };
            let notice = format!(
                "\r\n\x1b[1;33m[Claudia RH]\x1b[0m Starting Claude session (reason: {motivo} · mode: {modo_txt} · bin: {claude})…\r\n"
            );
            app.emit("pty-output", notice).ok();
            app.emit("session-started", session_id).ok();
            crate::diagnostics::emit_event("session", "spawn_ok", Some(session_id), motivo);
            crate::diagnostics::emit_event("session", "started", Some(session_id), motivo);
            Ok(())
        }
        Err(e) => {
            let msg = format!(
                "\r\n\x1b[1;31m[Claudia RH]\x1b[0m Falha ao iniciar: {e}\r\n"
            );
            app.emit("pty-output", msg).ok();
            if let Ok(conn) = db.lock() {
                let _ = conn.execute(
                    "UPDATE sessoes SET terminada_em = datetime('now'), motivo_termino = 'spawn_err' WHERE id = ?1",
                    rusqlite::params![session_id],
                );
            }
            crate::diagnostics::emit_event("session", "spawn_err", Some(session_id), &e);
            app.emit("session-ended", "spawn_err".to_string()).ok();
            Err(e)
        }
    }
````

**Delete** the old block that `emit`s notice + `session-started` **before** `iniciar_claude`.

- [ ] **Step 2: Zip allowlist**

In `zip_member_allowed`, change the `matches!` to:

```rust
    matches!(name, "events.jsonl" | "pty-tail.log" | "panic.log" | "claude-startup.log")
        || name.starts_with("claudia-rh.log")
```

- [ ] **Step 3: `cargo test --lib commands::sessao::tests` + `cargo test --lib diagnostics` + `cargo check`**

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/commands/sessao.rs src-tauri/src/diagnostics/mod.rs
git commit -m "fix(session): spawn before session-started; prompt via file"
```

---

### Task 4: Drop the 5-second PTY typer

**Files:**
- Modify: `src-tauri/src/pty_manager.rs` (`iniciar_claude`)

**Interfaces:**
- Consumes: `args` already contains the positional query (Task 1/3)
- Produces: `iniciar_claude` **still has** `initial_task: String` so LinkedIn compiles, but **must not spawn the 5s thread**. Prefix `_initial_task` and ignore it. Task 5 will remove the parameter.

- [ ] **Step 1: Delete the entire `std::thread::spawn` block that sleeps 5s and writes `initial_task` (lines ~117–136).** Rename param to `_initial_task`.

- [ ] **Step 2: `cargo check`** — sessao + linkedin still compile.

- [ ] **Step 3: Commit**

```powershell
git add src-tauri/src/pty_manager.rs
git commit -m "fix(pty): do not type the initial query after a 5s delay"
```

---

### Task 5: LinkedIn same lifecycle + drop unused param

**Files:**
- Modify: `src-tauri/src/commands/linkedin.rs` (the spawn function that currently emits started then `iniciar_claude`)
- Modify: `src-tauri/src/pty_manager.rs` (remove `_initial_task` parameter entirely)
- Modify: `src-tauri/src/commands/sessao.rs` (call site without query-as-timer; query already in args)

**Interfaces:**
- `iniciar_claude(..., cwd: String)` — **no** `initial_task` argument.

- [ ] **Step 1: Change signature** of `iniciar_claude` to drop the last `initial_task: String`. Update both call sites.

- [ ] **Step 2: LinkedIn** — write `montar_prompt_linkedin` to `workspace/.claude-system-prompt-linkedin.txt` (separate file so it does not clobber the job-search prompt). Build args via `crate::commands::sessao::build_claude_cli_args(skip, mcp, &prompt_file, debug, "Starting LinkedIn network scan for jobs shared by connections.")`. Spawn **then** emit cyan notice + `linkedin-session-started` + `session-started`. On `Err`, emit red `Falha ao iniciar` + `session-ended` `spawn_err` + DB `motivo_termino='spawn_err'` like sessao. **Do not** emit started before spawn.

- [ ] **Step 3: `cargo check` + sessao tests**

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/pty_manager.rs src-tauri/src/commands/linkedin.rs src-tauri/src/commands/sessao.rs
git commit -m "fix(session): LinkedIn spawn order and drop PTY initial_task"
```

---

### Task 6: Verification

**Files:** none unless a test failed.

- [ ] **Step 1:**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git diff HEAD -- src-tauri/tauri.conf.json
# must be empty

Set-Location src-tauri
cargo test --lib commands::sessao::tests -- --nocapture
cargo test --lib commands::tests -- --nocapture
cargo test --lib diagnostics -- --nocapture
cargo check
```

```powershell
Select-String -Path src-tauri\src\commands\sessao.rs,src-tauri\src\commands\linkedin.rs -Pattern "--system-prompt\""
# must NOT match a push of "--system-prompt" (only --system-prompt-file)
```

- [ ] **Step 2:** Confirm `iniciar_claude` has no `sleep(Duration::from_secs(5))`.

```powershell
Select-String -Path src-tauri\src\pty_manager.rs -Pattern "from_secs\(5\)"
# expected: no matches
```

- [ ] **Step 3: No extra commit unless something failed and was fixed.**

Human check: click procurar vagas → within 10s either Claude TUI/text in Terminal **or** a red Falha ao iniciar. Dashboard must not pulse green on spawn failure.

---

## Self-review (spec coverage)

| Spec | Task |
|---|---|
| `--system-prompt-file`, no body in argv | 1, 3 |
| Positional initial query, no 5s typer | 1, 4, 5 |
| WinGet `claude.exe` | 2 |
| Spawn before `session-started`; spawn_err in PTY + DB | 3, 5 |
| `--strict-mcp-config` | 1 |
| `--debug-file` + zip allow `claude-startup.log` | 1, 3 |
| MCP config still `write_mcp_config` immediately before spawn | 3 (call order) |
| `--chrome` kept | 1 |
| LinkedIn same hang | 5 |
| identifier untouched | 6 |
| PTY 24×80 unchanged | global |

No TBD. `iniciar_claude` signature: Task 3 still passes `query` as last arg; Task 4 ignores it; Task 5 removes it.
