# Native toast + i18n leaks — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fire real Windows toasts for open pendências/propostas (with repeat + `notif.json` toggle) and route the five hardcoded UI strings through i18n.

**Architecture:** Extract a pure `decide_notify` (unit-tested, no OS). The existing 30s poll loop in `notificacoes.rs` calls it, then `NotificationExt::show` and/or `app.emit`. Pass `Arc<Mutex<NotifConfig>>` into `start` so Configurações takes effect without restart. Frontend listens for `navigate-to-pendencias` and replaces five hardcodes with `t.*`.

**Tech Stack:** Rust (Tauri 2, `tauri-plugin-notification`, rusqlite, tokio), React 19, existing `src/i18n/{en,pt}.ts`.

**Spec:** `docs/superpowers/specs/2026-09-16-fix-notificacoes-i18n-design.md`

## Global Constraints

- Comments and commit messages: English. New identifiers: English.
- UI copy: pt-BR. Backend toast strings this cycle: the exact pt-BR constants in the spec.
- NEVER change `src-tauri/tauri.conf.json` `identifier`.
- Do not `git add` `npm-package/`.
- Do not split `perfil/` or `dashboard/` further. Do not touch `schema.sql` ALTERs.
- Work on `feat/fix-notificacoes-i18n`. Merge back to `dev`. Never commit on `main`.
- Cwd for git: `C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh`. Cargo tests: `src-tauri/`.

## File map

| File | Responsibility |
|---|---|
| `src-tauri/src/notificacoes.rs` | `NotifyDecision`, `decide_notify`, tests, poll loop, native `.show()` |
| `src-tauri/src/lib.rs` | Pass `notif_arc` into `start` |
| `src/App.tsx` | `listen("navigate-to-pendencias")` + JS `onAction` if available |
| `src/i18n/en.ts`, `src/i18n/pt.ts` | `dashboard.editLimit`, `dashboard.todayTotal` |
| `src/components/dashboard/index.tsx` | use those keys |
| `src/components/perfil/index.tsx` | `t.profile.loading` |
| `src/components/perfil/CoverLettersView.tsx` | `t.common.loading` |

---

### Task 1: `decide_notify` + unit tests (TDD)

**Files:**
- Modify: `src-tauri/src/notificacoes.rs`
- Test: same file, `#[cfg(test)]` module at the bottom

**Interfaces:**
- Consumes: nothing
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotifyDecision {
    pub send_native: bool,
    pub emit_event: bool,
}

pub fn decide_notify(
    first_seen: bool,
    still_open: bool,
    last_native: Option<std::time::Instant>,
    now: std::time::Instant,
    interval: std::time::Duration,
    native_enabled: bool,
) -> NotifyDecision
```

Do **not** change `start` in this task. Existing poll loop must still compile.

- [ ] **Step 1: Write the failing tests first** (append at the end of `notificacoes.rs` before implementing `decide_notify`)

````rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn t0() -> Instant { Instant::now() }

    #[test]
    fn first_seen_enabled_emits_and_sends() {
        let now = t0();
        let d = decide_notify(true, true, None, now, Duration::from_secs(600), true);
        assert_eq!(d, NotifyDecision { send_native: true, emit_event: true });
    }

    #[test]
    fn first_seen_disabled_emits_but_no_native() {
        let now = t0();
        let d = decide_notify(true, true, None, now, Duration::from_secs(600), false);
        assert_eq!(d, NotifyDecision { send_native: false, emit_event: true });
    }

    #[test]
    fn repeat_after_interval_native_only() {
        let t0 = t0();
        let later = t0 + Duration::from_secs(600);
        let d = decide_notify(false, true, Some(t0), later, Duration::from_secs(600), true);
        assert_eq!(d, NotifyDecision { send_native: true, emit_event: false });
    }

    #[test]
    fn inside_interval_sends_nothing() {
        let t0 = t0();
        let later = t0 + Duration::from_secs(60);
        let d = decide_notify(false, true, Some(t0), later, Duration::from_secs(600), true);
        assert_eq!(d, NotifyDecision { send_native: false, emit_event: false });
    }

    #[test]
    fn closed_sends_nothing() {
        let now = t0();
        let d = decide_notify(true, false, None, now, Duration::from_secs(600), true);
        assert_eq!(d, NotifyDecision { send_native: false, emit_event: false });
    }

    #[test]
    fn not_first_seen_never_toasted_sends_native_if_enabled() {
        let now = t0();
        let d = decide_notify(false, true, None, now, Duration::from_secs(600), true);
        assert_eq!(d, NotifyDecision { send_native: true, emit_event: false });
    }
}
````

Also add (above the tests, still without a body if you are strictly TDD — or add the struct + `todo!()` body so the tests compile and fail on assert):

````rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotifyDecision {
    pub send_native: bool,
    pub emit_event: bool,
}

pub fn decide_notify(
    first_seen: bool,
    still_open: bool,
    last_native: Option<std::time::Instant>,
    now: std::time::Instant,
    interval: std::time::Duration,
    native_enabled: bool,
) -> NotifyDecision {
    let _ = (first_seen, still_open, last_native, now, interval, native_enabled);
    NotifyDecision { send_native: false, emit_event: false }
}
````

- [ ] **Step 2: Run tests — they must fail (stub returns both false)**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh\src-tauri"
cargo test decide_notify -- --nocapture
```

Expected: compile OK; `first_seen_enabled_emits_and_sends` FAIL (got `{false,false}`).

- [ ] **Step 3: Implement `decide_notify` for real**

Replace the stub body with:

````rust
pub fn decide_notify(
    first_seen: bool,
    still_open: bool,
    last_native: Option<std::time::Instant>,
    now: std::time::Instant,
    interval: std::time::Duration,
    native_enabled: bool,
) -> NotifyDecision {
    if !still_open {
        return NotifyDecision { send_native: false, emit_event: false };
    }
    if first_seen {
        return NotifyDecision {
            emit_event: true,
            send_native: native_enabled,
        };
    }
    if !native_enabled {
        return NotifyDecision { send_native: false, emit_event: false };
    }
    let due = match last_native {
        None => true,
        Some(t) => now.duration_since(t) >= interval,
    };
    NotifyDecision { send_native: due, emit_event: false }
}
````

- [ ] **Step 4: Re-run tests — all pass**

```powershell
cargo test decide_notify -- --nocapture
```

Expected: 6 passed.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/notificacoes.rs
git commit -m "test(notif): add decide_notify schedule helper"
```

---

### Task 2: Wire the poll loop to config + native toast

**Files:**
- Modify: `src-tauri/src/notificacoes.rs` (`start` and queries)
- Modify: `src-tauri/src/lib.rs` line that calls `notificacoes::start` (currently `notificacoes::start(app.handle().clone(), Arc::clone(&conn_arc));`)

**Interfaces:**
- Consumes: `decide_notify` from Task 1; `crate::NotifConfig`
- Produces: `pub fn start(app: AppHandle, db: Arc<Mutex<Connection>>, notif: Arc<Mutex<crate::NotifConfig>>)`

- [ ] **Step 1: Change `start` signature and the `lib.rs` call**

In `lib.rs`, replace:

```rust
notificacoes::start(app.handle().clone(), Arc::clone(&conn_arc));
```

with:

```rust
notificacoes::start(app.handle().clone(), Arc::clone(&conn_arc), Arc::clone(&notif_arc));
```

(`notif_arc` is already created a few lines above.)

- [ ] **Step 2: Widen queries (body text for the toast)**

Replace `query_pending_ids` / `query_proposta_ids` with:

````rust
struct OpenItem {
    id: i64,
    body: String,
}

fn query_open_pendencias(db: &Arc<Mutex<Connection>>) -> Vec<OpenItem> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare(
        "SELECT id, COALESCE(categoria, ''), COALESCE(descricao, '') FROM pendencias WHERE resolvida = 0",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| {
        let id: i64 = r.get(0)?;
        let cat: String = r.get(1)?;
        let desc: String = r.get(2)?;
        let body = match (cat.trim(), desc.trim()) {
            ("", "") => "Tens uma pendência por resolver.".to_string(),
            ("", d) => d.to_string(),
            (c, "") => c.to_string(),
            (c, d) => format!("{c}: {d}"),
        };
        Ok(OpenItem { id, body })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn query_open_propostas(db: &Arc<Mutex<Connection>>) -> Vec<OpenItem> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut stmt = match conn.prepare(
        "SELECT id, COALESCE(pergunta, '') FROM propostas_perfil WHERE promovida = 0",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |r| {
        let id: i64 = r.get(0)?;
        let pergunta: String = r.get(1)?;
        let body = if pergunta.trim().is_empty() {
            "Há uma pergunta nova para o teu perfil.".to_string()
        } else {
            pergunta
        };
        Ok(OpenItem { id, body })
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}
````

- [ ] **Step 3: Rewrite `start` to use `decide_notify` + `NotificationExt`**

Add at top of `notificacoes.rs`:

```rust
use std::collections::HashMap;
use std::time::Instant;
use tauri_plugin_notification::NotificationExt;
use crate::NotifConfig;
```

Keep `HashSet` if still needed for `seen`. Use `HashSet<i64>` for seen (first_seen) and `HashMap<i64, Instant>` for `last_native`.

Full `start`:

````rust
pub fn start(
    app: AppHandle,
    db: Arc<Mutex<Connection>>,
    notif: Arc<Mutex<NotifConfig>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut seen_pendencias: HashSet<i64> = HashSet::new();
        let mut seen_propostas: HashSet<i64> = HashSet::new();
        let mut last_native_pend: HashMap<i64, Instant> = HashMap::new();
        let mut last_native_prop: HashMap<i64, Instant> = HashMap::new();

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let (native_enabled, interval) = {
                match notif.lock() {
                    Ok(cfg) => (
                        cfg.ativo,
                        Duration::from_secs(cfg.intervalo_minutos.max(1) as u64 * 60),
                    ),
                    Err(_) => continue,
                }
            };
            let now = Instant::now();

            let pend = query_open_pendencias(&db);
            let pend_ids: HashSet<i64> = pend.iter().map(|p| p.id).collect();
            seen_pendencias.retain(|id| pend_ids.contains(id));
            last_native_pend.retain(|id, _| pend_ids.contains(id));
            for item in &pend {
                let first_seen = !seen_pendencias.contains(&item.id);
                let decision = decide_notify(
                    first_seen,
                    true,
                    last_native_pend.get(&item.id).copied(),
                    now,
                    interval,
                    native_enabled,
                );
                if decision.emit_event {
                    seen_pendencias.insert(item.id);
                    let _ = app.emit("nova-pendencia", item.id);
                } else if first_seen {
                    seen_pendencias.insert(item.id);
                }
                if decision.send_native {
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Claudia RH — Pendência")
                        .body(&item.body)
                        .show()
                    {
                        eprintln!("[notif] show pendencia: {e}");
                    }
                    last_native_pend.insert(item.id, now);
                }
            }

            let prop = query_open_propostas(&db);
            let prop_ids: HashSet<i64> = prop.iter().map(|p| p.id).collect();
            seen_propostas.retain(|id| prop_ids.contains(id));
            last_native_prop.retain(|id, _| prop_ids.contains(id));
            for item in &prop {
                let first_seen = !seen_propostas.contains(&item.id);
                let decision = decide_notify(
                    first_seen,
                    true,
                    last_native_prop.get(&item.id).copied(),
                    now,
                    interval,
                    native_enabled,
                );
                if decision.emit_event {
                    seen_propostas.insert(item.id);
                    let _ = app.emit("nova-proposta", item.id);
                } else if first_seen {
                    seen_propostas.insert(item.id);
                }
                if decision.send_native {
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Claudia RH — Perfil")
                        .body(&item.body)
                        .show()
                    {
                        eprintln!("[notif] show proposta: {e}");
                    }
                    last_native_prop.insert(item.id, now);
                }
            }
        }
    });
}
````

- [ ] **Step 4: Re-run `decide_notify` tests + `cargo check`**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh\src-tauri"
cargo test decide_notify -- --nocapture
cargo check
```

Expected: tests pass; check succeeds. If `NotificationExt` is missing, add `use tauri_plugin_notification::NotificationExt;` — do not add a new crate (already in `Cargo.toml`).

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/notificacoes.rs src-tauri/src/lib.rs
git commit -m "fix(notif): send native Windows toasts honoring notif.json"
```

---

### Task 3: Click toast → Pendências tab

**Files:**
- Modify: `src/App.tsx` (the `Promise.all([ listen(...) ])` block around lines 56–62)
- Optional: `src-tauri/src/lib.rs` only if a Rust click hook is required — prefer JS.

**Interfaces:**
- Consumes: event name `"navigate-to-pendencias"` (unit payload)
- Produces: `setView("pendencias")` when that event fires

`@tauri-apps/plugin-notification` is already in `package.json` (`^2.3.3`).

- [ ] **Step 1: Listen in `App.tsx`**

Add import:

```ts
import { onAction } from "@tauri-apps/plugin-notification";
```

In the existing listener `useEffect` (same one that listens for `nova-pendencia`), add:

```ts
listen("navigate-to-pendencias", () => setView("pendencias")),
```

After `Promise.all([...]).then(...)`, also:

```ts
onAction(() => {
  setView("pendencias");
}).then((unlisten) => {
  if (active) unlisteners.push(() => { unlisten(); });
  else unlisten();
}).catch(() => {
  // Desktop click routing is best-effort; toast show still works without it.
});
```

`setView` is already in scope. `View` already includes `"pendencias"`.

- [ ] **Step 2: Typecheck**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
npx tsc --noEmit
```

Expected: exit 0. If `onAction` types fail, drop the `onAction` import/call, keep only `listen("navigate-to-pendencias", ...)`, and note that in the commit message body. Do not add a new npm dependency.

- [ ] **Step 3: Commit**

```powershell
git add src/App.tsx
git commit -m "feat(notif): navigate to Pendencias on notification action"
```

---

### Task 4: i18n leaks

**Files:**
- Modify: `src/i18n/en.ts` (object `dashboard`, after `jobsAnalyzed`)
- Modify: `src/i18n/pt.ts` (same keys)
- Modify: `src/components/dashboard/index.tsx` lines with `title="Editar limite"` and `hoje ·`
- Modify: `src/components/perfil/index.tsx` (add `useT`, replace loading span)
- Modify: `src/components/perfil/CoverLettersView.tsx` (already has `const t = useT()`)

**Interfaces:**
- Consumes: existing `t.profile.loading` (`"Loading profile…"` / `"A carregar perfil…"`) and `t.common.loading` (`"Loading…"` / `"Carregando…"`)
- Produces: `t.dashboard.editLimit`, `t.dashboard.todayTotal`

- [ ] **Step 1: Add keys**

In `en.ts` inside `dashboard: {`, immediately after `jobsAnalyzed: "Jobs analyzed",`:

```ts
    editLimit: "Edit limit",
    todayTotal: "today",
```

In `pt.ts` inside `dashboard: {`, immediately after `jobsAnalyzed: "Vagas analisadas",`:

```ts
    editLimit: "Editar limite",
    todayTotal: "hoje",
```

Do not add any other keys.

- [ ] **Step 2: Dashboard replacements** (file already imports `useT` and has `const t = useT()`)

Replace all three:

```tsx
title="Editar limite"
```

with:

```tsx
title={t.dashboard.editLimit}
```

Replace:

```tsx
hoje · <span style={{ color: "var(--text-secondary)", fontWeight: 500 }}>{vagasTotal}</span> total
```

with:

```tsx
{t.dashboard.todayTotal} · <span style={{ color: "var(--text-secondary)", fontWeight: 500 }}>{vagasTotal}</span> total
```

- [ ] **Step 3: Perfil loading**

In `src/components/perfil/index.tsx`, add:

```ts
import { useT } from "../../i18n";
```

Inside `Perfil`, after the existing `useState` lines:

```ts
  const t = useT();
```

Replace:

```tsx
<span style={{ fontSize: 13, color: "var(--text-tertiary)" }}>A carregar perfil…</span>
```

with:

```tsx
<span style={{ fontSize: 13, color: "var(--text-tertiary)" }}>{t.profile.loading}</span>
```

In `CoverLettersView.tsx` replace:

```tsx
<div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>A carregar…</div>
```

with:

```tsx
<div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>{t.common.loading}</div>
```

(`t` already exists in that component.)

- [ ] **Step 4: Verify hardcodes gone + typecheck**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
npx tsc --noEmit
```

On Windows without `rg`:

```powershell
Select-String -Path src\components\dashboard\index.tsx,src\components\perfil\index.tsx,src\components\perfil\CoverLettersView.tsx -Pattern "Editar limite|hoje ·|A carregar"
```

Expected: zero matches. `tsc` exit 0.

- [ ] **Step 5: Commit**

```powershell
git add src/i18n/en.ts src/i18n/pt.ts src/components/dashboard/index.tsx src/components/perfil/index.tsx src/components/perfil/CoverLettersView.tsx
git commit -m "fix(i18n): route dashboard and profile loading strings through t()"
```

---

### Task 5: Final verification

**Files:** none written (read-only), unless Step 1 of Task 2 left `tauri.conf.json` dirty — it must not.

- [ ] **Step 1: Tests + identifier + plugin still init**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git diff HEAD -- src-tauri/tauri.conf.json
Select-String -Path src-tauri\src\lib.rs -Pattern "tauri_plugin_notification::init"
Set-Location src-tauri
cargo test decide_notify -- --nocapture
```

Expected: empty diff on `tauri.conf.json`; one `init` hit; 6 tests pass.

- [ ] **Step 2: Branch diff must not include `npm-package/` or identifier**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git diff --name-only origin/dev...HEAD
```

Expected files only under: `src-tauri/src/notificacoes.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`, `src/i18n/*`, `src/components/dashboard/index.tsx`, `src/components/perfil/index.tsx`, `src/components/perfil/CoverLettersView.tsx`, plus this plan/spec under `docs/superpowers/`.

- [ ] **Step 3: No extra commit unless something failed and was fixed**

Human check (not required to automate): app running, `notif.json.ativo=true`, insert unresolved `pendencias` row → Windows toast within 30s.

---

## Self-review (spec coverage)

| Spec requirement | Task |
|---|---|
| `decide_notify` + 5 listed tests (+ the `last_native=None` row) | 1 |
| Native `.show()` via existing plugin | 2 |
| Honor `ativo` / `intervalo_minutos` live | 2 |
| Toast copy constants (Pendência / Perfil) | 2 |
| Query categoria/descricao when cheap | 2 |
| Event still on first seen; UI not blinded when `ativo=false` | 2 (`emit_event` independent of native) |
| Click → Pendências (best-effort) | 3 |
| i18n five strings | 4 |
| identifier / npm-package / no extra splits | 5 |
| `cargo test decide_notify` | 1, 2, 5 |

No TBD. `onAction` failure path is specified (keep `listen` only). Types: `NotifyDecision` / `decide_notify` / `start(..., notif: Arc<Mutex<NotifConfig>>)` used consistently in Tasks 1–2.
