# Fechar a branch `feat/browser-cdp-mcp`: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deixar a branch `feat/browser-cdp-mcp` pronta para o usuário decidir o merge em `dev`. Para isso: testes verdes com evidência, smoke no app real, pendências de cada spec resolvidas ou registradas, e documentação batendo com o código. **Sem feature nova.**

**Architecture:** Não entra código novo de produto. Só mexemos no harness dos testes live (CLA-39, mais uma trava de deleção), em documentação, e em correções que a revisão final apontar. O resto é verificação: rodadas de teste, smoke e revisão.

**Tech Stack:** Rust (Tauri 2, tokio), testes `#[ignore]` com Chrome for Testing, Node 24 (scripts, `node --test`), React/TS (`tsc`, vite), skill `claudia-teste-e2e`.

**Specs que esta branch implementa** (o plano não muda nenhuma regra delas):
- `docs/superpowers/specs/2026-10-02-browser-cdp-motor-design.md` (ciclo 1 do browser; plano `plans/2026-10-02-browser-cdp-motor.md`)
- `docs/superpowers/specs/2026-10-03-mcp-leitura-robusta-design.md`
- `docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md` (plano `plans/2026-10-04-anexar-curriculo.md`)
- Levantamento que originou este plano: `.superpowers/estado-feat-browser-cdp-mcp-2026-10-06.md`

## ⛔ Regra número 1 deste plano: nenhum arquivo do usuário é apagado

Em 2026-10-05 um teste de "limpeza de dados" escrito por um agente, que ninguém revisou, apagou o disco C do usuário. Foram cerca de 12 horas de trabalho perdidas, e só parte foi recuperada. Neste plano isso **não pode** se repetir:

- **Nenhum comando de teste roda antes da Tarefa 0** (auditoria de deleção) estar concluída e sem bloqueio.
- Todo caminho que chega a `remove_dir_all`, `remove_file`, `rmSync`, `Remove-Item`, `rm -rf`, `robocopy /MIR` ou `git clean` tem de ser uma **pasta temporária criada pelo próprio teste**. Na dúvida, **não roda**: pergunta ao usuário.
- **Nunca** passar raiz de disco, `~`, `%USERPROFILE%`, `AppData`, a pasta do projeto ou qualquer caminho real a uma função que apaga, **nem para testar que o caminho é recusado**. Para isso, testa-se a função de validação pura.
- **Todo subagente recebe este bloco literalmente no prompt.**
- Desinstalação e qualquer "limpeza de dados" do app **só** rodam no Windows Sandbox, nunca nesta máquina (Tarefa 4).
- Antes de qualquer escrita nos dados reais, faz-se **um backup novo numa pasta nova**; não se sobrescreve nem se apaga o backup anterior.

## Global Constraints

- Nenhum passo de instalação novo para o usuário.
- Nunca mudar o `identifier` Tauri `io.github.johngabie.claudia-rh`.
- `src-tauri/src/prompt_sistema_runtime.md`: nenhuma edição neste plano.
- Comentários de código, identificadores novos e commits em inglês; docs e copy da UI em pt-BR.
- Cada agente usa o próprio `CARGO_TARGET_DIR` fora do repo (`C:/Users/joaog/.cache/claudia-close-<tarefa>`). Ao terminar, o agente apaga **só esse cache**, depois de conferir o caminho exato.
- Chrome de teste: `CLAUDIA_TEST_CHROME=%TEMP%\cft\chrome-win64\chrome.exe` (existe e foi conferido em 2026-10-06).
- Sem push, sem merge e sem PR: integrar é decisão do usuário (Tarefa 8).
- Envio de candidatura real, login real e sites reais só com autorização explícita do usuário **naquela rodada**.
- Código de saída real em toda rodada. Nunca filtrar a saída de um jeito que esconda falha.
- Bug achado no caminho que não bloqueia o fechamento vai para o Linear e não é tratado aqui.

## Review Focus

1. **Um teste ou script apaga um caminho fora da pasta temporária dele.** Esperado: recusa antes de apagar. Fixado pela Tarefa 0 (auditoria) e pela Tarefa 1 (`is_removable_profile_dir`, com testes de recusa que não apagam nada).
2. **A migração, rodando sobre os dados reais, duplica ou some com currículos.** Esperado: cada currículo e carta antigo vira um documento, os originais continuam lá, e reabrir o app não duplica. Fixado na Tarefa 3, passos 3 e 4 (contagens antes e depois de duas aberturas).
3. **O Chrome de teste sobe sem página e fica órfão** (CLA-39). Esperado: retentativa curta, e se falhar, o Chrome é morto. Fixado na Tarefa 1.
4. **O Chrome do app roda sem sandbox.** Esperado: o processo não tem `--no-sandbox`. Fixado na Tarefa 4, passo 1.
5. **O merge em `dev` conflita** (`dev` recebeu `feat/clean-machine-testing` depois do ponto de partida desta branch). Esperado: os conflitos aparecem antes de o usuário decidir. Fixado na Tarefa 8, passo 2 (merge de ensaio numa worktree temporária).

---

### Task 0: Auditoria de deleção (só leitura, Opus)

**Files:**
- Create: `.superpowers/sdd/2026-10-06-fechar-branch-browser/auditoria-delecao.md`

**Interfaces:**
- Produces: a tabela de auditoria; nenhuma outra tarefa roda teste antes de ela existir sem linha "BLOQUEIA".

- [ ] **Step 1: Listar todo ponto de deleção**

Run (na raiz `claudia-rh/`):
```bash
grep -rnE "remove_dir_all|remove_file|rmSync|rmdirSync|fs\.rm\(|unlinkSync|Remove-Item|rm -rf|rmtree|robocopy.*/MIR|git clean" \
  --include=*.rs --include=*.mjs --include=*.js --include=*.ts --include=*.tsx --include=*.ps1 \
  src-tauri/src src-tauri/tests src tests .claude/skills scripts
```
Expected (2026-10-06): cerca de 60 linhas em 23 arquivos, entre eles `migration.rs:115` (apaga a pasta de dados antiga **em produção**), `browser/install.rs:100,131,164`, `diagnostics/mod.rs`, `documentos/*`, `tests/browser_live.rs:35`, `tests/viewport_live.rs:23`, `tests/mcp-browser-smoke.mjs:133`, e os scripts das skills `claudia-teste-e2e` e `claudia-teste-sandbox` (`comum.mjs:59` recria a pasta de cenário).

- [ ] **Step 2: Classificar cada linha**

Para cada linha, leia a função inteira e rastreie de onde vem o caminho. Grave uma linha por ponto em `auditoria-delecao.md`:

```markdown
| arquivo:linha | produção ou teste | de onde vem o caminho | travas existentes | veredito |
|---|---|---|---|---|
| src-tauri/tests/browser_live.rs:35 | teste | temp_dir()/claudia-live-<pid>-<nanos> criado em launch() | nenhuma | OK, reforçar na Tarefa 1 |
| src-tauri/src/migration.rs:115 | produção | parent(app_data_dir)/<LEGACY_DIR_NAME> | só apaga se arquivos e linhas do banco estão cobertos | REVISAR: os testes passam pasta temporária? |
```
Veredito possível: `OK`, `OK, reforçar na Tarefa 1`, `REVISAR` (precisa de leitura mais funda) ou **`BLOQUEIA`** (o caminho pode vir de fora de uma pasta temporária criada pelo teste, ou de variável que pode estar vazia ou ser relativa).

- [ ] **Step 3: Resolver os `REVISAR`**

Para cada `REVISAR`, leia os testes que chamam a função e confirme qual caminho eles passam. Se continuar havendo dúvida, o veredito vira `BLOQUEIA`.

- [ ] **Step 4: Portão**

Se existe algum `BLOQUEIA`: **pare o plano inteiro** e leve a tabela ao usuário. Não rode teste nenhum. Sem `BLOQUEIA`, registre no ledger `.superpowers/sdd/2026-10-06-fechar-branch-browser/progress.md`: `Task 0: auditoria concluída, N pontos, 0 BLOQUEIA`.

---

### Task 1: Harness live: trava de deleção e subida do Chrome (CLA-39)

**Files:**
- Create: `src-tauri/tests/support/mod.rs`
- Modify: `src-tauri/tests/browser_live.rs:1-75` (imports, `Drop for Live`, `launch()`)
- Modify: `src-tauri/tests/viewport_live.rs:1-40` (idem)

**Interfaces:**
- Produces (em `tests/support/mod.rs`):
  - `pub fn new_profile_dir(tag: &str) -> PathBuf`
  - `pub fn is_removable_profile_dir(p: &Path, temp_root: &Path) -> bool` (pura: não toca o disco)
  - `pub fn remove_profile_dir(p: &Path)` (apaga só se `is_removable_profile_dir(p, &std::env::temp_dir())`; senão, só registra em `eprintln!`)
  - `pub fn kill_tree(pid: u32)`
  - `pub async fn first_page_with_retry(port: u16) -> Result<cdp::Target, String>`

- [ ] **Step 1: Escrever os testes de recusa (só validação pura, nada é apagado)**

Crie `src-tauri/tests/support/mod.rs`:

```rust
//! Shared helpers for the live tests. Deleting is guarded: only a folder this harness created,
//! directly under the system temp dir and named `claudia-live-*`, may ever be removed.
#![allow(dead_code)]
use claudia_rh_lib::browser::cdp;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

pub const PROFILE_PREFIX: &str = "claudia-live-";

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

pub fn remove_profile_dir(p: &Path) {
    let temp_root = std::env::temp_dir();
    let is_link = std::fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    if is_link || !is_removable_profile_dir(p, &temp_root) {
        eprintln!("refusing to delete {} (not a live-test profile under {})", p.display(), temp_root.display());
        return;
    }
    let _ = std::fs::remove_dir_all(p);
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
        assert!(!is_removable_profile_dir(Path::new(r"C:\Users\joaog\claudia-live-1"), &r), "outside temp");
    }
}
```

Estes testes só chamam a função pura com caminhos que **não existem** e não são passados a nada que apague.

- [ ] **Step 2: Ligar o módulo e rodar (ainda sem usar nos harnesses)**

No topo de `tests/browser_live.rs` e de `tests/viewport_live.rs`, acrescente `mod support;`.
Run (em `src-tauri/`, sem `--ignored`, só os testes puros):
`CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-close-1 cargo test --test browser_live support::`
Expected: `2 passed`.

- [ ] **Step 3: Mutação da trava**

Troque `p.parent() == Some(temp_root)` por `true`, rode o mesmo comando e cole a linha `test support::tests::anything_else_is_refused ... FAILED`. Restaure e confirme com `git diff --stat src-tauri/tests/support/mod.rs` que só os acréscimos do Step 1 aparecem.

- [ ] **Step 4: Usar os helpers nos dois harnesses**

Em `tests/browser_live.rs`, troque o `Drop` e o começo de `launch()`:

```rust
impl Drop for Live {
    fn drop(&mut self) {
        support::kill_tree(self.pid);
        support::remove_profile_dir(&self.profile);
    }
}

async fn launch() -> Live {
    let profile = support::new_profile_dir("browser");
    let state_path = profile.join("state.json");
    let o = LaunchOptions::headless_for_tests(chrome(), profile.join("p"));
    let st = manager::ensure_running(&o, "", &state_path).await.expect("chrome launches");
    let page = match support::first_page_with_retry(st.port).await {
        Ok(p) => p,
        Err(e) => {
            // No Live exists yet, so nothing else would close this Chrome: do it before failing.
            support::kill_tree(st.pid);
            support::remove_profile_dir(&profile);
            panic!("chrome started without a page: {e}");
        }
    };
    // ... o resto de launch() continua igual, a partir de `let client = ...`
```

Em `tests/viewport_live.rs`, a mesma troca: `Drop` com `support::kill_tree(self.pid); support::remove_profile_dir(&self.profile);`, `let profile = support::new_profile_dir("viewport");` e o mesmo `match` em volta de `first_page_with_retry`.

Confira: `grep -n "remove_dir_all" src-tauri/tests/*.rs` deve mostrar **só** `tests/support/mod.rs`.

- [ ] **Step 5: Rodar a suíte live três vezes**

Run (só depois da Tarefa 0 sem `BLOQUEIA`):
```bash
cd src-tauri && export CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-close-1 CLAUDIA_TEST_CHROME="$(cygpath -w "$TEMP")\\cft\\chrome-win64\\chrome.exe"
for i in 1 2 3; do cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon > live-$i.log 2>&1; echo "rodada $i exit=$?"; grep -E "test result|FAILED" live-$i.log; done
```
Expected: exit 0 nas três (`browser_live` 70 passed, `viewport_live` 5 passed). Uma falha em `the_wheel_does_not_change_a_focused_number_field` (CLA-38) ou `a_wide_but_short_window_grows_in_height_only` (CLA-36) é registrada com a mensagem e vai para a decisão D2. Depois das rodadas, `Get-Process chrome` não pode mostrar nenhum Chrome com perfil `claudia-live-*` (confira a linha de comando com `Get-CimInstance Win32_Process`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/support/mod.rs src-tauri/tests/browser_live.rs src-tauri/tests/viewport_live.rs
git commit -m "test(live): guarded profile deletion and a retrying first page so a failed launch never orphans Chrome (CLA-39)"
```
Apague só `C:/Users/joaog/.cache/claudia-close-1`, conferindo o caminho antes, e os `live-*.log` que você criou.

---

### Task 2: Rodadas completas sobre o HEAD (orquestrador)

**Files:** nenhum de produção. Ledger: `.superpowers/sdd/2026-10-06-fechar-branch-browser/progress.md`.

**Interfaces:**
- Consumes: Tarefa 1 integrada.
- Produces: os números de cada rodada, com código de saída, no ledger.

- [ ] **Step 1: Unitários e frontend**

```bash
cd claudia-rh && npx tsc --noEmit; echo tsc=$?
node --test "tests/front/*.test.ts"; echo front=$?
npm run build; echo build=$?
cd src-tauri && CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-target cargo test --lib; echo lib=$?
```
Expected: os quatro com exit 0; `cargo test --lib` com cerca de 503 passed (número de 2026-10-05, depois do M).

- [ ] **Step 2: Integração**

Live: o mesmo comando da Tarefa 1, Step 5, uma vez. Depois o smoke MCP:
```bash
cargo build; echo cbuild=$?
CLAUDIA_TEST_CHROME=... CLAUDIA_ALLOW_LOCAL_URLS=1 node tests/mcp-browser-smoke.mjs "C:/Users/joaog/.cache/claudia-target/debug/claudia-rh.exe"; echo smoke=$?
```
Expected: `smoke=0` e a última linha `TUDO OK`. O smoke apaga só a pasta que ele mesmo criou com `mkdtempSync` (`mcp-browser-smoke.mjs:133`, já auditado na Tarefa 0).

- [ ] **Step 3: Registrar**

Grave no ledger uma linha por rodada, com o comando e o exit code. Qualquer falha nova que não seja CLA-36 ou CLA-38: `superpowers:systematic-debugging`; se não bloqueia o fechamento, vai para o Linear.

---

### Task 3: Smoke de uso real no app (autorizado pelo usuário em 2026-10-05)

**Files:** nenhum de produção. Prints em `debug-logs/prints/fechamento/`.

**Interfaces:**
- Consumes: Tarefa 2 verde.
- Produces: evidência para a descrição do PR (prints, linhas do banco, Activity log).

- [ ] **Step 1: Backup novo (não apagar nem sobrescrever o anterior)**

Feche o app só se foi este plano que o abriu; se o usuário o abriu, pergunte antes. Depois:
```powershell
$dst = "C:\Users\joaog\Documents\ClaudiaRH-backup-$(Get-Date -Format yyyyMMdd-HHmm)"
if (Test-Path $dst) { throw "pasta de backup já existe: $dst" }
robocopy "$env:APPDATA\io.github.johngabie.claudia-rh" "$dst\appdata" /E /R:1 /W:1 /NFL /NDL
robocopy "$env:LOCALAPPDATA\io.github.johngabie.claudia-rh" "$dst\localappdata" /E /R:1 /W:1 /NFL /NDL /XD browser
```
`robocopy` **sem** `/MIR` e **sem** `/PURGE`. Confira que `$dst\appdata\claudia_rh.db` existe. Exit 0 a 7 no robocopy é sucesso.

- [ ] **Step 2: Contagens antes**

```bash
node .claude/skills/claudia-teste-e2e/scripts/banco.mjs "SELECT (SELECT COUNT(*) FROM documentos) docs, (SELECT COUNT(*) FROM vagas) vagas, (SELECT COUNT(*) FROM candidaturas) cands"
```
(Se `documentos` ainda não existe, anote "tabela ausente": ela é criada na primeira abertura.) Liste também os arquivos de `%APPDATA%\io.github.johngabie.claudia-rh\curriculos\` e `cover_letters\`, com nome e tamanho.

- [ ] **Step 3: Subir o app e conferir a migração**

`node .claude/skills/claudia-teste-e2e/scripts/subir.mjs` (`acao` igual a `perguntar` → pergunte ao usuário). Repita a contagem do Step 2. Expected: `docs` cresceu com uma linha por currículo e carta antigos, e a listagem dos originais é **idêntica** à do Step 2.

- [ ] **Step 4: Segunda abertura não duplica**

Feche o app (foi este plano que o abriu) e suba de novo. Expected: `docs` igual ao do Step 3.

- [ ] **Step 5: Aba Currículos, nos dois idiomas**

Com `ver.mjs`: abra Currículos, a sub-aba Cover letters, um documento (folha A4) e Voltar. Tire prints em pt-BR e em en (troque o idioma nas Configurações e volte ao idioma original no fim).

- [ ] **Step 6: Gerar um currículo (PDF pelo WebView2 dentro do app real, ainda não verificado)**

Clique em "Gerar" na aba e escolha um template. Expected: um documento novo com formato `pdf` no banco, o arquivo começando com `%PDF-` em `documentos\`, nenhuma janela extra visível na área de trabalho (print da tela inteira, degrau 3 da escada) e a grade atualizada sem recarregar.

- [ ] **Step 7: Preferido e anexo por uma rodada de agente**

Marque o gerado como preferido. A rodada de agente roda contra a fixture local `src-tauri/tests/fixtures/upload-form.html`, servida em `127.0.0.1` com `CLAUDIA_ALLOW_LOCAL_URLS=1` no `tauri dev`. Só vai a `the-internet.herokuapp.com/upload` se o usuário autorizar **nesta rodada** (decisão D3). Expected: o `browser_upload` diz "pela regra: preferido geral"; o Activity log tem a linha `documento <id> | <nome> | pdf | ...`; a tabela `anexos` tem a linha da vaga.

- [ ] **Step 8: Desligamento ao fechar o app (pendência da Fase 6 do motor)**

Anote o PID do Chrome do app (`state.json` em `%LOCALAPPDATA%\...\browser\`). Feche o app pela janela. Expected: o processo some em até 10 s e o `state.json` deixa de existir.

- [ ] **Step 9: Registrar**

Ledger: o que foi feito, os números, os caminhos dos prints. Qualquer divergência entre a UI, o banco e o Activity log é **achado**: corrija (com teste que falha antes) ou registre no Linear se não bloquear.

---

### Task 4: Pendências restantes do motor do browser

**Files:** `docs/superpowers/plans/2026-10-02-browser-cdp-motor.md` (anotar o resultado no fim). Nada de produção, salvo um achado.

- [ ] **Step 1: O Chrome do app roda com sandbox**

Com o app aberto (Tarefa 3):
```powershell
Get-CimInstance Win32_Process -Filter "Name='chrome.exe'" | Where-Object { $_.ExecutablePath -like "*io.github.johngabie.claudia-rh*" -and $_.CommandLine -notmatch '--type=' } | Select-Object -ExpandProperty CommandLine
```
Expected: a linha **não** contém `--no-sandbox`. Se contiver, é achado de segurança e bloqueia o fechamento.

- [ ] **Step 2: O que fica para fora desta branch (Linear, uma issue cada)**

- **6.3, candidatura real no LinkedIn:** precisa do login do usuário e de "pode enviar". Fora do fechamento, salvo pedido dele (decisão D4).
- **Desinstalação testada:** só no Windows Sandbox (`SANDBOX_LAB_DIR` está vazio em 2026-10-06), **nunca nesta máquina**. Fica no Linear com essa regra escrita na issue.
- **Tarefa 4.2 cortada** (limpeza própria na desinstalação; ficou só a caixa nativa do instalador): confirmação do usuário (decisão D5).

---

### Task 5: Documentação batendo com o código

**Files:**
- Modify: `CLAUDE.md` (ordem de leitura item 5, "Dívida conhecida", "O que este repo já é")
- Modify: `docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md` (cabeçalho)
- Modify: `docs/superpowers/specs/2026-10-02-contas-do-browser-design.md` (cabeçalho)
- Modify: `docs/superpowers/plans/2026-10-02-propostas-pos-corte.md` (título e primeiro parágrafo)
- Modify: `docs/superpowers/plans/2026-09-28-prototipo-playwright-mcp.md` (linha de status logo abaixo do título)

- [ ] **Step 1: `CLAUDE.md`**

- Ordem de leitura, item 5: trocar "documentos: `2026-10-04-anexar-curriculo-design.md`, ainda rascunho" por "documentos: `2026-10-04-anexar-curriculo-design.md`".
- "Dívida conhecida": remover o item "Agente sem como anexar arquivo…". Acrescentar: "Candidatura real no LinkedIn e desinstalação nunca testadas (Linear)" e "Testes live instáveis: CLA-36, CLA-38".
- "O que este repo já é" → "Ferramentas do agente": acrescentar `browser_upload` (anexa o currículo ou a carta escolhidos pelo app, nunca por caminho) e as tools de documentos e variantes (`list_documentos`, `get_documento`, `save_cover_letter`, `list_variantes`, `get_variante`, `set_vaga_variante`; `update_variante` e `set_documento_preferido` são recusadas na sessão de execução). Acrescentar uma linha "Documentos: biblioteca em `<dados>/documentos/`, aba Currículos, PDF gerado pelo WebView2 do app".

- [ ] **Step 2: Cabeçalhos das specs e planos**

- Spec de anexar: trocar a linha `> **Status: RASCUNHO, aguardando aprovação…` por `> **Status: APROVADA (2026-10-04) e implementada nesta branch.** Decisões do usuário nas seções 2.2 e 2.4; o conteúdo por variante saiu para \`feat/curriculo-por-variante\`.`
- Spec de contas: acrescentar logo abaixo do título `> **Status: NÃO INICIADA.** Ciclo 2, branch própria \`feat/browser-contas\`, depois que \`feat/browser-cdp-mcp\` entrar em \`dev\`. Ainda precisa da aprovação do usuário.`
- `propostas-pos-corte.md`: título para `# Propostas para cortar a dependência do Claude (Tarefa 5.1): APLICADAS no commit beaeb33`, e acrescentar no primeiro parágrafo "Aplicado em `beaeb33` (sessão com `--no-chrome`, welcome com `browser_pronto`, prompt sem a extensão). O texto abaixo é o registro da proposta."
- Plano do protótipo: logo abaixo do título, `> **Status: CONCLUÍDO.** Resposta da CLA-7: CDP direto (\`docs/pesquisa/2026-09-28-DECISAO-browser.md\`).`

- [ ] **Step 3: Conferir**

```bash
grep -n "ainda rascunho\|RASCUNHO\|NADA aqui foi aplicado\|Agente sem como anexar" CLAUDE.md docs/superpowers/specs/*.md docs/superpowers/plans/2026-10-02-propostas-pos-corte.md
```
Expected: nenhuma linha.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md docs/superpowers/specs/2026-10-02-contas-do-browser-design.md docs/superpowers/plans/2026-10-02-propostas-pos-corte.md docs/superpowers/plans/2026-09-28-prototipo-playwright-mcp.md
git commit -m "docs: status headers and CLAUDE.md match what this branch shipped"
```

---

### Task 6: Linear: tudo o que fica fora da branch tem issue

**Files:** nenhum. Skill `linear-claudia` (o MCP do Linear precisa de autenticação; se falhar, peça ao usuário para autenticar e grave as issues num arquivo em `.superpowers/sdd/2026-10-06-fechar-branch-browser/linear-pendente.md` até lá).

- [ ] **Step 1: Criar as issues**

Uma issue por item, com a descrição que se explica sozinha:
1. Os 9 minors adiados do ledger de anexar (as linhas `Final: minor (deferred)` de `.superpowers/sdd/2026-10-04-anexar-curriculo/progress.md`). Podem ir juntos numa issue com checklist.
2. **Renomear os parâmetros das tools do MCP para nomes uniformes**, depois do merge, numa branch curta a partir de `dev`. Hoje `id`, `variante_id` e `documento` se misturam, e a IA errou isso na rodada de teste. O erro de parâmetro passa a dizer o nome certo e de onde tirar o valor (ex.: "use `id`, de `list_variantes`"). Atualizar as descrições das tools, o prompt de execução (só os nomes; precisa de autorização do usuário) e o smoke. **Não é refazer o MCP**: a tipagem e a auditoria completas (trabalho perdido no incidente de 2026-10-05) só voltam se uma segunda IA no app tropeçar nos schemas.
3. 6.3, candidatura real no LinkedIn (Tarefa 4).
4. Desinstalação testada **só no Sandbox** (Tarefa 4).
5. **Prioridade máxima (Urgent):** "Nunca mais perder trabalho: processo para branches longas e proteção contra código que apaga arquivos". Texto: o incidente de 2026-10-05, as 12 horas perdidas, os 158 commits sem `dev` e o `dev` 55 commits à frente do `origin`. A solução é para **depois** desta branch e precisa de spec aprovada; a primeira proposta foi recusada pelo usuário.

- [ ] **Step 2: Registrar os ids no ledger**

---

### Task 7: Revisão final da branch inteira (Opus)

**Files:** o pacote `review-<dev>..HEAD.diff` em `.superpowers/sdd/2026-10-06-fechar-branch-browser/`.

- [ ] **Step 1: Pacote e revisão**

Gere o diff `git diff dev...HEAD` em arquivo e lance um revisor Opus de contexto novo, com `requesting-code-review`. Foco: segurança do browser e da guarda de envio, todos os pontos de deleção (com a tabela da Tarefa 0 em mãos), dados do usuário e as tools MCP. Esta revisão substitui a revisão final que deu FAIL em 2026-10-02 e nunca foi refeita.

- [ ] **Step 2: Achados**

Critical e Important: corrigir cada um com um teste que falha antes e passa depois, mais mutação, e rodar a Tarefa 2 de novo. Minor: Linear.

---

### Task 8: Entrega ao usuário

- [ ] **Step 1: Resumo**

Um resumo curto para o usuário: o que foi verificado (números das Tarefas 2 e 3), o que ficou "não verificado" e por quê, as issues criadas e os caminhos dos prints.

- [ ] **Step 2: Merge de ensaio, sem tocar em `dev`**

```bash
git worktree add ../claudia-merge-ensaio dev --detach
cd ../claudia-merge-ensaio && git merge --no-commit --no-ff feat/browser-cdp-mcp; git status --short | grep -E "^(UU|AA|DU|UD)"
git merge --abort; cd .. && git -C claudia-rh worktree remove ../claudia-merge-ensaio
```
`worktree remove` só aceita uma worktree registrada pelo git; o caminho é o que este passo acabou de criar. Liste os conflitos para o usuário.

- [ ] **Step 3: Perguntar (nunca automático)**

Pergunte ao usuário: (a) merge de `feat/browser-cdp-mcp` em `dev`; (b) push de `dev` para `origin` (55 commits à frente em 2026-10-06, mais esta branch); (c) o destino de `feat/curriculo-por-variante` (refazer a base sobre `dev` depois do merge).

---

## Decisões que só o usuário toma

- **D1** — Tarefa 0: qualquer `BLOQUEIA` para o plano e vai ao usuário.
- **D2** — CLA-36 e CLA-38 (testes live instáveis) impedem o merge, ou entram como instabilidade conhecida, registrada?
- **D3** — Rodada de agente do smoke contra o site real `the-internet.herokuapp.com/upload`, ou só a fixture local?
- **D4** — A candidatura real no LinkedIn (6.3) fica fora desta branch?
- **D5** — Confirmar o corte da Tarefa 4.2 (limpeza própria na desinstalação; ficou só a caixa nativa do instalador).
- **D6** — CLA-37 (a guarda de envio deixa passar um submit cujo nome está fora da lista) é segurança do núcleo desta branch: corrigir antes do merge ou depois?

## Depois desta branch: prioridade máxima

**Nunca mais perder trabalho.** Antes de qualquer feature nova, sai uma spec própria, aprovada pelo usuário, para dois problemas:
1. Código que apaga arquivos: travas que não dependem de alguém lembrar da regra.
2. Branches que crescem sem ir para `dev` e um `dev` que não vai para `origin`.

A primeira proposta (cabeçalhos de estado, índice, limites de commits) foi recusada em 2026-10-06 por ser burocrática. A próxima precisa ser mais inteligente, com menos papel e mais automação. Fica registrada na issue Urgent da Tarefa 6.
