# Escrita segura do perfil — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Nenhuma escrita no `candidate_base.yaml` destrói o estado anterior sem deixar cópia e sem o dizer, e só uma sessão com o usuário presente pode gravar.

**Architecture:** Três camadas independentes em `mcp/tools/profile.rs` — rotação de backups, cálculo de diff, e um gate por tipo de sessão — mais uma tool MCP nova que dá à sessão autónoma um canal real (`propostas_perfil`, uma tabela que já existe e que nada em produção alimentava).

**Tech Stack:** Rust, `serde_yaml`, `rusqlite`, `rmcp` (servidor MCP), Tauri v2.

**Spec:** `docs/superpowers/specs/2026-09-25-perfil-escrita-segura-design.md`

## Global Constraints

- Comentários de código e mensagens de commit: **inglês**. Copy de UI e docs: **pt-BR**. (`CLAUDE.md`)
- Identificadores novos em inglês; código legado em português não se renomeia.
- **NUNCA** mudar o `identifier` Tauri `io.github.johngabie.claudia-rh`.
- `src-tauri/src/prompt_sistema_runtime.md` **não é editado** neste plano. A Task 7 depende de autorização explícita do usuário e está bloqueada.
- Testes correm com `cargo test` a partir de `src-tauri/`. A baseline ao começar é **99 passed, 0 failed**.
- O default de `SessionKind` é `Autonomous` — a opção restritiva. Um call site que se esqueça do parâmetro falha fechado.

## Review Focus

Estes casos vêm do spec mas não caem naturalmente em nenhuma task. Cada um tem o seu teste atribuído abaixo.

1. **Primeira gravação, sem ficheiro anterior** — não há o que rotacionar nem com que comparar; tem de gravar na mesma, sem erro. (Task 1, Task 2)
2. **Falha a rotacionar backups** — se o backup não se consegue criar, a escrita **não** acontece. Sem rede, não se salta. (Task 3)
3. **YAML válido idêntico ao anterior** — não anuncia perdas nem ganhos falsos. (Task 2)
4. **`dados_pessoais` esvaziado sem mexer nas listas** — as contagens de blocos não mudam, mas o CPF desapareceu. Tem de ser assinalado. (Task 2)
5. **`propose_profile_change` sem `vaga_id`** — o caso que a tabela existe para servir; tem de gravar `NULL`, não `0`. (Task 6)

---

## File Structure

| Ficheiro | Responsabilidade |
|---|---|
| `src-tauri/src/mcp/tools/profile.rs` | leitura do perfil, rotação, diff, gate, escrita |
| `src-tauri/src/mcp/tools/propostas.rs` | **novo** — inserção em `propostas_perfil` |
| `src-tauri/src/mcp/tools/mod.rs` | exports e helpers partilhados |
| `src-tauri/src/mcp/mod.rs` | `SessionKind`, `McpConfig`, despacho, parsing de CLI |
| `src-tauri/src/mcp/server.rs` | declaração rmcp da tool nova |
| `src-tauri/src/commands/perfil.rs` | `write_mcp_config` com tipo de sessão |
| `src-tauri/src/commands/{linkedin,sessao}.rs` | passam o tipo de sessão |

---

## Task 1: Rotação de backups

**Files:**
- Modify: `src-tauri/src/mcp/tools/profile.rs`

**Interfaces:**
- Consumes: nada.
- Produces: `fn rotate_backups(data_dir: &Path) -> Result<(), String>` — privada ao módulo. Move `candidate_base.yaml` para `.bak-1`, deslizando `.bak-N` para `.bak-(N+1)` até 5. No-op silencioso se o ficheiro não existir.

- [ ] **Step 1: Write the failing tests**

Acrescentar ao `mod tests` de `profile.rs`:

```rust
#[test]
fn rotation_moves_current_to_bak1() {
    let dir = temp_dir("prof-rot-1");
    std::fs::write(dir.join("candidate_base.yaml"), "v1").unwrap();
    rotate_backups(&dir).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), "v1");
    assert!(!dir.join("candidate_base.yaml").exists(), "current must have been moved");
}

#[test]
fn rotation_slides_older_backups_and_drops_the_sixth() {
    let dir = temp_dir("prof-rot-slide");
    for i in 1..=6 {
        std::fs::write(dir.join("candidate_base.yaml"), format!("v{i}")).unwrap();
        rotate_backups(&dir).unwrap();
    }
    // Newest write is v6; it is now .bak-1, and v1 fell off the end.
    assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), "v6");
    assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-5")).unwrap(), "v2");
    assert!(!dir.join("candidate_base.yaml.bak-6").exists(), "only five backups are kept");
}

/// Review Focus 1: first write ever — nothing to rotate, and that is not an error.
#[test]
fn rotation_is_a_noop_without_a_current_file() {
    let dir = temp_dir("prof-rot-none");
    rotate_backups(&dir).unwrap();
    assert!(!dir.join("candidate_base.yaml.bak-1").exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test rotation_ -- --nocapture`
Expected: FAIL — `cannot find function 'rotate_backups' in this scope`

- [ ] **Step 3: Implement**

Acrescentar a `profile.rs`, acima de `update_profile`:

```rust
/// How many previous versions of the profile we keep.
const BACKUP_DEPTH: usize = 5;

fn backup_path(data_dir: &Path, n: usize) -> std::path::PathBuf {
    data_dir.join(format!("candidate_base.yaml.bak-{n}"))
}

/// Slides candidate_base.yaml.bak-N to .bak-(N+1) and moves the current profile
/// into .bak-1. The oldest backup falls off the end.
///
/// A missing profile is not an error: the first write has nothing to preserve.
fn rotate_backups(data_dir: &Path) -> Result<(), String> {
    let current = data_dir.join("candidate_base.yaml");
    if !current.exists() {
        return Ok(());
    }
    // Walk down so each slot is free before we move into it.
    for n in (1..BACKUP_DEPTH).rev() {
        let from = backup_path(data_dir, n);
        if from.exists() {
            std::fs::rename(&from, backup_path(data_dir, n + 1))
                .map_err(|e| format!("erro ao rodar backup {n}: {e}"))?;
        }
    }
    std::fs::rename(&current, backup_path(data_dir, 1))
        .map_err(|e| format!("erro ao criar backup do perfil: {e}"))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test rotation_`
Expected: PASS — 3 passed

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mcp/tools/profile.rs
git commit -m "feat(profile): rotate five backups before overwriting the profile"
```

---

## Task 2: Diff legível entre o perfil anterior e o novo

**Files:**
- Modify: `src-tauri/src/mcp/tools/profile.rs`

**Interfaces:**
- Consumes: `crate::commands::perfil::CandidatoBase` e os seus campos (`experiencia`, `projetos`, `formacao`, `competencias`, `idiomas`, `gaps_conhecidos`, `dados_pessoais`).
- Produces:
  - `struct ChangeSummary { text: String, lossy: bool }` — privada ao módulo.
  - `fn describe_change(prev: Option<&CandidatoBase>, next: &CandidatoBase) -> ChangeSummary`. O `text` é a frase que vai para o agente, **sem** a linha do backup (essa é da Task 3). O `lossy` é o que decide se essa linha aparece.

**Porque `lossy` é um campo e não uma pesquisa no texto:** procurar o sinal `−` na frase falharia no caso em que só campos de `dados_pessoais` foram apagados — a frase diria *"apagado(s) em dados_pessoais: cpf"* sem nenhum `−`, e a escrita mais perigosa das duas não apontaria para o backup.

- [ ] **Step 1: Write the failing tests**

```rust
use crate::commands::perfil::CandidatoBase;

fn base_from(yaml: &str) -> CandidatoBase {
    crate::commands::perfil::parse_candidato_base_str(yaml).unwrap()
}

#[test]
fn diff_reports_losses_per_block() {
    let prev = base_from(
        "experiencia:\n  - empresa: A\n  - empresa: B\ncompetencias:\n  - Rust\n  - Go\n",
    );
    let next = base_from("experiencia:\n  - empresa: A\n");
    let out = describe_change(Some(&prev), &next);
    assert!(out.text.contains("−1 experiência"), "got: {}", out.text);
    assert!(out.text.contains("−2 competências"), "got: {}", out.text);
    assert!(out.lossy);
}

#[test]
fn diff_reports_gains() {
    let prev = base_from("experiencia:\n  - empresa: A\n");
    let next = base_from("experiencia:\n  - empresa: A\n  - empresa: B\n");
    let out = describe_change(Some(&prev), &next);
    assert!(out.text.contains("+1 experiência"), "got: {}", out.text);
    assert!(!out.lossy, "a pure addition is not lossy");
}

/// Review Focus 3: re-saving the same content must not invent movement.
#[test]
fn diff_says_nothing_changed_when_nothing_changed() {
    let yaml = "experiencia:\n  - empresa: A\ncompetencias:\n  - Rust\n";
    let prev = base_from(yaml);
    let next = base_from(yaml);
    let out = describe_change(Some(&prev), &next);
    assert!(out.text.contains("sem alterações"), "got: {}", out.text);
    assert!(!out.lossy);
}

/// Review Focus 4: block counts are unchanged, but personal data vanished.
/// This is the case that a "does the text contain a minus sign" check would miss.
#[test]
fn diff_flags_personal_fields_that_went_blank() {
    let prev = base_from("dados_pessoais:\n  nome_completo: Maria\n  cpf: \"000.111.222-33\"\n");
    let next = base_from("dados_pessoais:\n  nome_completo: Maria\n");
    let out = describe_change(Some(&prev), &next);
    assert!(out.text.contains("cpf"), "got: {}", out.text);
    assert!(
        !out.text.contains("nome_completo"),
        "unchanged field must stay quiet; got: {}",
        out.text
    );
    assert!(out.lossy, "losing a personal field is a loss even with no block change");
}

/// Review Focus 1: first write ever — there is no previous profile to compare against.
#[test]
fn diff_describes_a_first_write() {
    let next = base_from("experiencia:\n  - empresa: A\n");
    let out = describe_change(None, &next);
    assert!(out.text.contains("Perfil criado"), "got: {}", out.text);
    assert!(!out.lossy, "a first write cannot lose anything");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test diff_`
Expected: FAIL — `cannot find function 'describe_change' in this scope`

- [ ] **Step 3: Implement**

```rust
/// The blocks whose size we report. Names are the user-facing plural/singular
/// pair, in pt-BR, because this string is read by the model and shown to the user.
fn block_counts(base: &CandidatoBase) -> [(&'static str, &'static str, usize); 6] {
    [
        ("experiência", "experiências", base.experiencia.len()),
        ("projeto", "projetos", base.projetos.len()),
        ("formação", "formações", base.formacao.len()),
        ("competência", "competências", base.competencias.len()),
        ("idioma", "idiomas", base.idiomas.len()),
        ("gap", "gaps", base.gaps_conhecidos.len()),
    ]
}

/// Personal fields worth naming when they go from filled to blank. `links` is a
/// list and is not covered here; losing every link is rare and the block counts
/// do not track it — accepted gap, documented rather than silently ignored.
fn personal_fields(base: &CandidatoBase) -> [(&'static str, &str); 8] {
    let d = &base.dados_pessoais;
    [
        ("nome_completo", d.nome_completo.as_str()),
        ("email", d.email.as_str()),
        ("telefone", d.telefone.as_str()),
        ("localizacao_atual", d.localizacao_atual.as_str()),
        ("endereco", d.endereco.as_str()),
        ("nacionalidade", d.nacionalidade.as_str()),
        ("data_nascimento", d.data_nascimento.as_str()),
        ("cpf", d.cpf.as_str()),
    ]
}

/// What a write did, and whether anything was lost doing it.
struct ChangeSummary {
    text: String,
    lossy: bool,
}

/// Builds the sentence the model gets back after a write.
///
/// Counts, not a textual diff: what matters is what disappeared, not how the
/// YAML was reformatted. The minus sign is U+2212, so it survives terminals
/// that would swallow a leading hyphen.
fn describe_change(prev: Option<&CandidatoBase>, next: &CandidatoBase) -> ChangeSummary {
    let Some(prev) = prev else {
        let created: Vec<String> = block_counts(next)
            .iter()
            .filter(|(_, _, n)| *n > 0)
            .map(|(one, many, n)| format!("{n} {}", if *n == 1 { one } else { many }))
            .collect();
        let text = if created.is_empty() {
            "Perfil criado, ainda sem conteúdo.".to_string()
        } else {
            format!("Perfil criado: {}.", created.join(", "))
        };
        return ChangeSummary { text, lossy: false };
    };

    let before = block_counts(prev);
    let after = block_counts(next);
    let mut changes: Vec<String> = Vec::new();
    let mut lossy = false;
    for (i, (one, many, new_n)) in after.iter().enumerate() {
        let old_n = before[i].2;
        let delta = *new_n as i64 - old_n as i64;
        if delta == 0 {
            continue;
        }
        let magnitude = delta.unsigned_abs();
        let noun = if magnitude == 1 { one } else { many };
        let sign = if delta > 0 {
            "+"
        } else {
            lossy = true;
            "−"
        };
        changes.push(format!("{sign}{magnitude} {noun}"));
    }

    let prev_personal = personal_fields(prev);
    let blanked: Vec<&str> = personal_fields(next)
        .iter()
        .enumerate()
        .filter(|(i, (_, value))| value.trim().is_empty() && !prev_personal[*i].1.trim().is_empty())
        .map(|(_, (name, _))| *name)
        .collect();
    if !blanked.is_empty() {
        lossy = true;
        changes.push(format!("apagado(s) em dados_pessoais: {}", blanked.join(", ")));
    }

    let text = if changes.is_empty() {
        "Perfil gravado, sem alterações de conteúdo.".to_string()
    } else {
        format!("Perfil gravado. {}.", changes.join(", "))
    };
    ChangeSummary { text, lossy }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test diff_`
Expected: PASS — 5 passed

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mcp/tools/profile.rs
git commit -m "feat(profile): describe what a write changed instead of counting what remains"
```

---

## Task 3: Ligar backup e diff à escrita

**Files:**
- Modify: `src-tauri/src/mcp/tools/profile.rs` (a função `update_profile`)

**Interfaces:**
- Consumes: `rotate_backups` (Task 1), `describe_change` (Task 2).
- Produces: `update_profile` mantém a assinatura `(data_dir: &Path, yaml: &str) -> Result<String, String>`. A Task 5 acrescenta-lhe o parâmetro de sessão.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn write_keeps_the_previous_profile_in_bak1() {
    let dir = temp_dir("prof-keeps");
    let v1 = "experiencia:\n  - empresa: A\n  - empresa: B\n";
    update_profile(&dir, v1).unwrap();
    update_profile(&dir, "experiencia: []\n").unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), v1);
}

#[test]
fn write_that_removes_content_says_so_and_points_at_the_backup() {
    let dir = temp_dir("prof-says");
    update_profile(&dir, "experiencia:\n  - empresa: A\n  - empresa: B\n").unwrap();
    let msg = update_profile(&dir, "experiencia: []\n").unwrap();
    assert!(msg.contains("−2 experiências"), "got: {msg}");
    assert!(msg.contains("bak-1"), "a lossy write must point at the backup; got: {msg}");
}

#[test]
fn write_without_losses_does_not_mention_the_backup() {
    let dir = temp_dir("prof-nomention");
    update_profile(&dir, "experiencia:\n  - empresa: A\n").unwrap();
    let msg = update_profile(&dir, "experiencia:\n  - empresa: A\n  - empresa: B\n").unwrap();
    assert!(!msg.contains("bak-1"), "nothing was lost; got: {msg}");
}

/// A loss with no change in block counts must still point at the backup.
/// This is the case the `lossy` flag exists for.
#[test]
fn write_that_blanks_a_personal_field_points_at_the_backup() {
    let dir = temp_dir("prof-blank");
    update_profile(&dir, "dados_pessoais:\n  nome_completo: Maria\n  cpf: \"000.111.222-33\"\n")
        .unwrap();
    let msg = update_profile(&dir, "dados_pessoais:\n  nome_completo: Maria\n").unwrap();
    assert!(msg.contains("cpf"), "got: {msg}");
    assert!(msg.contains("bak-1"), "got: {msg}");
}

/// Review Focus 2: no backup, no write. Losing the rollback silently is the
/// failure mode this whole task exists to prevent.
#[test]
fn write_aborts_when_the_backup_cannot_be_made() {
    let dir = temp_dir("prof-noback");
    update_profile(&dir, "experiencia:\n  - empresa: A\n").unwrap();
    // A directory sitting where .bak-1 must go makes the rename fail.
    std::fs::create_dir(dir.join("candidate_base.yaml.bak-1")).unwrap();
    assert!(update_profile(&dir, "experiencia: []\n").is_err());
    let still = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
    assert!(still.contains("empresa: A"), "profile must be untouched; got: {still}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test write_ -- --nocapture`
Expected: FAIL — `write_keeps_the_previous_profile_in_bak1` falha porque `.bak-1` não existe; as outras falham nas asserções sobre a mensagem.

- [ ] **Step 3: Implement**

Substituir o corpo de `update_profile` (mantendo o doc comment existente, e acrescentando-lhe a nota sobre os defaults):

```rust
/// Validates the full candidate_base.yaml content against the serde structs
/// BEFORE writing. Invalid YAML never reaches disk; the parse error goes back
/// to the model so it can self-correct.
///
/// Parsing alone is not enough. Every field of CandidatoBase carries
/// #[serde(default) ] (see commands/perfil.rs) so that legacy Claude-written
/// YAMLs keep loading — which means a one-line document deserializes into a
/// complete, empty profile. That is exactly how the 2026-09-25 wipe happened.
/// The defenses here are the backup and the diff, not the parse.
pub fn update_profile(data_dir: &Path, yaml: &str) -> Result<String, String> {
    if yaml.trim().is_empty() {
        return Err("YAML vazio — envie o conteúdo completo do candidate_base.yaml".to_string());
    }
    let parsed = crate::commands::perfil::parse_candidato_base_str(yaml)?;

    let path = data_dir.join("candidate_base.yaml");
    let previous = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| crate::commands::perfil::parse_candidato_base_str(&raw).ok());

    // No backup, no write.
    rotate_backups(data_dir)?;

    let tmp = data_dir.join("candidate_base.yaml.tmp");
    std::fs::write(&tmp, yaml).map_err(|e| format!("erro ao escrever ficheiro temporário: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("erro ao gravar candidate_base.yaml: {e}"))?;

    let summary = describe_change(previous.as_ref(), &parsed);
    Ok(if summary.lossy {
        format!("{}\nAnterior em candidate_base.yaml.bak-1.", summary.text)
    } else {
        summary.text
    })
}
```

- [ ] **Step 4: Run the full suite**

Run: `cd src-tauri && cargo test`
Expected: PASS. O teste antigo `writes_valid_yaml` asserta `msg.contains("1 experiência")` — continua a passar, porque a primeira escrita produz `"Perfil criado: 1 experiência."`. Se falhar, **não** relaxe a asserção: verifique que `describe_change(None, …)` produz mesmo essa frase.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mcp/tools/profile.rs
git commit -m "feat(profile): back up and report every profile write"
```

---

## Task 4: O MCP passa a saber que sessão o invocou

**Files:**
- Modify: `src-tauri/src/mcp/mod.rs` (`McpConfig`, `cli_main`)
- Modify: `src-tauri/src/commands/perfil.rs:447` (`write_mcp_config`) e os call sites `:506`, `:708`
- Modify: `src-tauri/src/commands/linkedin.rs:68`
- Modify: `src-tauri/src/commands/sessao.rs:52`

**Interfaces:**
- Produces:
  - `pub enum SessionKind { Interactive, Autonomous }` em `mcp/mod.rs`, com `Copy`, `Clone`, `PartialEq`, `Eq`, `Debug`.
  - `SessionKind::from_flag(&str) -> SessionKind` — `"interactive"` dá `Interactive`, **tudo o resto** dá `Autonomous`.
  - `McpConfig` ganha o campo `pub session: SessionKind`.
  - `write_mcp_config(app: &AppHandle, session: SessionKind) -> Option<PathBuf>`.

- [ ] **Step 1: Write the failing tests**

Acrescentar a `mcp/mod.rs`, dentro de `mod dispatch_tests`:

```rust
/// Falling open would mean a new call site that forgets the flag gets write
/// access to the profile by accident. It must fall closed.
#[test]
fn unknown_or_missing_session_flag_means_autonomous() {
    assert_eq!(SessionKind::from_flag("interactive"), SessionKind::Interactive);
    assert_eq!(SessionKind::from_flag("autonomous"), SessionKind::Autonomous);
    assert_eq!(SessionKind::from_flag("whatever"), SessionKind::Autonomous);
    assert_eq!(SessionKind::from_flag(""), SessionKind::Autonomous);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test unknown_or_missing_session_flag`
Expected: FAIL — `cannot find type 'SessionKind' in this scope`

- [ ] **Step 3: Implement — `mcp/mod.rs`**

Acrescentar junto de `McpConfig`:

```rust
/// Which session spawned this MCP server. The profile is writable only from a
/// session where the user is present and watching the conversation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionKind {
    /// Profile / LinkedIn sessions: turn-based, user reads every reply.
    Interactive,
    /// The `claude --chrome` run: nobody is necessarily looking.
    Autonomous,
}

impl SessionKind {
    /// Anything that is not an explicit "interactive" is autonomous. A call
    /// site that forgets the flag loses write access rather than gaining it.
    pub fn from_flag(raw: &str) -> Self {
        if raw == "interactive" { Self::Interactive } else { Self::Autonomous }
    }

    pub fn as_flag(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Autonomous => "autonomous",
        }
    }
}
```

Acrescentar o campo à struct:

```rust
pub struct McpConfig {
    pub data_dir: PathBuf,
    pub notify_port: Option<u16>,
    pub debug: bool,
    pub session: SessionKind,
}
```

Em `cli_main`, a seguir ao parsing de `notify_port`:

```rust
    let session = parse_flag_value(&args, "--session-kind")
        .map(|s| SessionKind::from_flag(&s))
        .unwrap_or(SessionKind::Autonomous);
    let cfg = McpConfig {
        data_dir,
        notify_port,
        debug: args.iter().any(|a| a == "--debug"),
        session,
    };
```

Em `mod dispatch_tests`, `cfg_for` precisa do campo novo. Como estes testes exercitam tools de leitura, a sessão interativa é a escolha neutra:

```rust
    fn cfg_for(dir: &std::path::Path) -> McpConfig {
        McpConfig {
            data_dir: dir.to_path_buf(),
            notify_port: None,
            debug: false,
            session: SessionKind::Interactive,
        }
    }
```

- [ ] **Step 4: Implement — os call sites**

`commands/perfil.rs:447`, assinatura e args:

```rust
pub fn write_mcp_config(
    app: &AppHandle,
    session: crate::mcp::SessionKind,
) -> Option<std::path::PathBuf> {
```

Dentro, a seguir ao bloco do `notify_port`:

```rust
    args.push("--session-kind".to_string());
    args.push(session.as_flag().to_string());
```

Os quatro call sites, exatamente assim:

| Ficheiro:linha | Chamada |
|---|---|
| `commands/perfil.rs:506` | `write_mcp_config(&app, crate::mcp::SessionKind::Interactive)` |
| `commands/perfil.rs:708` | `write_mcp_config(&app, crate::mcp::SessionKind::Interactive)` |
| `commands/linkedin.rs:68` | `crate::commands::perfil::write_mcp_config(&app, crate::mcp::SessionKind::Interactive)` |
| `commands/sessao.rs:52` | `crate::commands::perfil::write_mcp_config(app, crate::mcp::SessionKind::Autonomous)` |

- [ ] **Step 5: Run the full suite**

Run: `cd src-tauri && cargo test`
Expected: PASS. Se o compilador se queixar de `McpConfig` incompleto noutro sítio, acrescente `session: SessionKind::Interactive` a esse literal — mas confirme primeiro que é código de teste e não um call site de produção.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mcp/mod.rs src-tauri/src/commands/perfil.rs \
        src-tauri/src/commands/linkedin.rs src-tauri/src/commands/sessao.rs
git commit -m "feat(mcp): tell the server which session spawned it"
```

---

## Task 5: A sessão autónoma não escreve no perfil

**Files:**
- Modify: `src-tauri/src/mcp/tools/profile.rs` (`update_profile`)
- Modify: `src-tauri/src/mcp/mod.rs` (o braço `"update_profile"` do `dispatch`)

**Interfaces:**
- Consumes: `SessionKind` (Task 4), `update_profile` (Task 3).
- Produces: `update_profile(data_dir: &Path, yaml: &str, session: SessionKind) -> Result<String, String>`.

- [ ] **Step 1: Write the failing tests**

Em `profile.rs`, e atualizar as chamadas existentes do `mod tests` para passar `SessionKind::Interactive`:

```rust
#[test]
fn autonomous_session_cannot_write_the_profile() {
    let dir = temp_dir("prof-auto");
    let yaml = "experiencia:\n  - empresa: A\n";
    let err = update_profile(&dir, yaml, SessionKind::Autonomous).unwrap_err();
    assert!(err.contains("propose_profile_change"), "must name the alternative; got: {err}");
    assert!(!dir.join("candidate_base.yaml").exists(), "autonomous write must not land");
}

#[test]
fn autonomous_refusal_does_not_touch_an_existing_profile() {
    let dir = temp_dir("prof-auto-keep");
    let yaml = "experiencia:\n  - empresa: A\n";
    update_profile(&dir, yaml, SessionKind::Interactive).unwrap();
    assert!(update_profile(&dir, "experiencia: []\n", SessionKind::Autonomous).is_err());
    assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap(), yaml);
    assert!(!dir.join("candidate_base.yaml.bak-1").exists(), "a refusal must not rotate backups");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test autonomous_`
Expected: FAIL — `this function takes 2 arguments but 3 arguments were supplied`

- [ ] **Step 3: Implement**

Em `profile.rs`, no topo: `use crate::mcp::SessionKind;`

Assinatura e guarda, **antes** de qualquer validação ou escrita:

```rust
pub fn update_profile(
    data_dir: &Path,
    yaml: &str,
    session: SessionKind,
) -> Result<String, String> {
    if session == SessionKind::Autonomous {
        return Err(
            "O perfil só pode ser alterado numa sessão de Perfil, onde o usuário acompanha a \
             conversa. Para registar uma mudança que o usuário deve rever, use \
             propose_profile_change."
                .to_string(),
        );
    }
    if yaml.trim().is_empty() {
```

Em `mcp/mod.rs`, o braço do despacho:

```rust
        "update_profile" => {
            let yaml = args["yaml"].as_str().ok_or("parâmetro 'yaml' em falta")?;
            tools::update_profile(&cfg.data_dir, yaml, cfg.session)
                .inspect(|_| notify(cfg, "perfil"))
        }
```

- [ ] **Step 4: Run the full suite**

Run: `cd src-tauri && cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/mcp/tools/profile.rs src-tauri/src/mcp/mod.rs
git commit -m "feat(profile): refuse profile writes from the autonomous session"
```

---

## Task 6: `propose_profile_change`, a tool que faltava

**Files:**
- Create: `src-tauri/src/mcp/tools/propostas.rs`
- Modify: `src-tauri/src/mcp/tools/mod.rs`
- Modify: `src-tauri/src/mcp/mod.rs` (despacho)
- Modify: `src-tauri/src/mcp/server.rs` (declaração rmcp)

**Interfaces:**
- Consumes: `super::open_db`.
- Produces: `pub fn propose_profile_change(data_dir: &Path, pergunta: &str, contexto: Option<&str>, vaga_id: Option<i64>) -> Result<String, String>`.

**Contexto para quem implementa:** a tabela `propostas_perfil` existe desde sempre (`db/schema.sql:36-44`) com `vaga_id` **nullable**, e toda a UI à volta dela já está feita — contador (`db/queries.rs:216`), notificação (`notificacoes.rs:49`), badge (`src/App.tsx:143`). Faltava só quem inserisse. **Não é preciso tocar em frontend nem no schema.**

- [ ] **Step 1: Write the failing tests**

Criar `propostas.rs` com o módulo de testes primeiro:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    /// Review Focus 5: the global question is the case this table exists for.
    #[test]
    fn proposal_without_vaga_stores_null() {
        let dir = temp_dir("prop-global");
        seed_db(&dir);
        propose_profile_change(&dir, "Aceita viajar?", Some("3 vagas pediram"), None).unwrap();
        let conn = open_db(&dir).unwrap();
        let vaga: Option<i64> = conn
            .query_row("SELECT vaga_id FROM propostas_perfil WHERE pergunta = 'Aceita viajar?'",
                       [], |r| r.get(0))
            .unwrap();
        assert_eq!(vaga, None, "a global question must not be pinned to a vaga");
    }

    #[test]
    fn proposal_with_vaga_keeps_the_link() {
        let dir = temp_dir("prop-vaga");
        seed_db(&dir);
        propose_profile_change(&dir, "Tem visto?", None, Some(1)).unwrap();
        let conn = open_db(&dir).unwrap();
        let vaga: Option<i64> = conn
            .query_row("SELECT vaga_id FROM propostas_perfil WHERE pergunta = 'Tem visto?'",
                       [], |r| r.get(0))
            .unwrap();
        assert_eq!(vaga, Some(1));
    }

    #[test]
    fn proposal_starts_unpromoted_and_is_counted() {
        let dir = temp_dir("prop-count");
        seed_db(&dir);
        propose_profile_change(&dir, "Pergunta", None, None).unwrap();
        let conn = open_db(&dir).unwrap();
        let open: i64 = conn
            .query_row("SELECT COUNT(*) FROM propostas_perfil WHERE promovida = 0", [], |r| r.get(0))
            .unwrap();
        assert_eq!(open, 1);
    }

    #[test]
    fn empty_question_is_rejected() {
        let dir = temp_dir("prop-empty");
        seed_db(&dir);
        assert!(propose_profile_change(&dir, "   ", None, None).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test proposal_`
Expected: FAIL — o módulo ainda não está declarado em `tools/mod.rs`, logo os testes nem compilam. Declarar o módulo é o primeiro passo do Step 3.

- [ ] **Step 3: Implement**

Topo de `propostas.rs`:

```rust
// Profile-proposal tool — the write side of `propostas_perfil`.
//
// The table has been in schema.sql since the beginning, with a nullable
// vaga_id, and the whole UI around it (counter, notification, sidebar badge)
// was already wired. Nothing in production ever inserted into it, which is why
// global questions ended up in search_variants.yaml instead.

use std::path::Path;

use super::open_db;

/// Records a change the agent thinks the profile needs, for the user to review.
///
/// `vaga_id` is None for a question that is not about one job — the case the
/// pendencias table cannot represent, because its vaga_id is NOT NULL.
pub fn propose_profile_change(
    data_dir: &Path,
    pergunta: &str,
    contexto: Option<&str>,
    vaga_id: Option<i64>,
) -> Result<String, String> {
    if pergunta.trim().is_empty() {
        return Err("parâmetro 'pergunta' vazio".to_string());
    }
    let conn = open_db(data_dir)?;
    conn.execute(
        "INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, contexto, promovida) \
         VALUES (?1, datetime('now'), ?2, ?3, 0)",
        rusqlite::params![vaga_id, pergunta.trim(), contexto],
    )
    .map_err(|e| format!("erro ao registar proposta de perfil: {e}"))?;

    Ok(format!(
        "Proposta registada para o usuário rever: {}",
        pergunta.trim()
    ))
}
```

`tools/mod.rs` — acrescentar o módulo e o export:

```rust
mod propostas;
```
```rust
pub use propostas::propose_profile_change;
```

`mcp/mod.rs` — braço de despacho, a seguir a `"create_pendencia"`:

```rust
        "propose_profile_change" => {
            let pergunta = args["pergunta"].as_str().ok_or("parâmetro 'pergunta' em falta")?;
            tools::propose_profile_change(
                &cfg.data_dir,
                pergunta,
                args["contexto"].as_str(),
                args["vaga_id"].as_i64(),
            )
            .inspect(|_| notify(cfg, "db"))
        }
```

`mcp/server.rs` — a struct de argumentos, junto das outras:

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ProposeProfileChangeArgs {
    /// A pergunta ou mudança que o perfil precisa, em uma frase
    pergunta: String,
    /// Porque surgiu: a vaga, o formulário, o padrão observado
    contexto: Option<String>,
    /// ID da vaga que motivou a proposta; omita quando a questão for geral
    vaga_id: Option<i64>,
}
```

E o método, dentro do bloco `#[tool_router] impl ClaudiaMcp`:

```rust
    #[tool(
        description = "Regista uma mudança que o perfil do candidato precisa, para o usuário rever mais tarde. Use quando perceber que falta um dado no perfil ou que uma resposta se repete entre vagas. Omita vaga_id quando a questão não for sobre uma vaga específica. Esta é a forma correta de propor mudanças ao perfil durante a sessão de execução — update_profile não funciona aí."
    )]
    async fn propose_profile_change(
        &self,
        Parameters(args): Parameters<ProposeProfileChangeArgs>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call(
            "propose_profile_change",
            serde_json::json!({
                "pergunta": args.pergunta,
                "contexto": args.contexto,
                "vaga_id": args.vaga_id,
            }),
        )
    }
```

- [ ] **Step 4: Write the dispatch reachability test**

Uma tool que existe mas não está no `dispatch` é invisível ao modelo. Acrescentar a `mod dispatch_tests` em `mcp/mod.rs`:

```rust
#[test]
fn propose_profile_change_is_reachable_through_dispatch() {
    let dir = temp_dir("dispatch-propose");
    seed_db(&dir);
    let cfg = cfg_for(&dir);
    let out = dispatch(
        &cfg,
        "propose_profile_change",
        &serde_json::json!({ "pergunta": "Aceita viajar?" }),
    )
    .unwrap();
    assert!(out.contains("Aceita viajar?"), "got: {out}");
}
```

- [ ] **Step 5: Run the full suite**

Run: `cd src-tauri && cargo test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/mcp/tools/propostas.rs src-tauri/src/mcp/tools/mod.rs \
        src-tauri/src/mcp/mod.rs src-tauri/src/mcp/server.rs
git commit -m "feat(mcp): add propose_profile_change, the missing writer for propostas_perfil"
```

---

## Task 7: Migrar `perguntas_pendentes` — **BLOQUEADA**

**Não implementar.** Esta task depende de uma decisão que o usuário ainda não tomou, registada em §5.6 do spec.

O bloco `perguntas_pendentes` em `search_variants.yaml` (`commands/perfil.rs:208`) tem 11 itens, alguns desde 21/07. Migrá-los para `propostas_perfil` exige mudar uma linha de `src-tauri/src/prompt_sistema_runtime.md:144`, que instrui o agente a escrever nesse bloco — e esse ficheiro é protegido por `CLAUDE.md`: *"Não edite sem pedido explícito."*

Migrar sem mudar o prompt deixa a instrução a contradizer o código e o agente reconstrói o canal paralelo; o estado final fica pior que o inicial.

**Quem executar este plano deve parar aqui e devolver a decisão ao usuário.** As Tasks 1 a 6 entregam valor completo sem esta.

---

## Verificação final da branch

- [ ] `cd src-tauri && cargo test` — esperado ≥ 99 + os testes novos, 0 failed
- [ ] `cd src-tauri && cargo clippy --all-targets` — sem warnings **novos**. A baseline em `dev` tem 4: `write_both` nunca usado, `PtyRing::new` nunca usado, `LASTINPUTINFO` acrónimo, e `items after a test module` em `mcp/mod.rs` e `prompt.rs`.
- [ ] `npx tsc --noEmit` na raiz — limpo. Nenhuma task toca frontend; se isto falhar, alguma coisa correu mal.
- [ ] `git diff dev --stat` — nenhum ficheiro de `src/` na lista, e `prompt_sistema_runtime.md` ausente.
