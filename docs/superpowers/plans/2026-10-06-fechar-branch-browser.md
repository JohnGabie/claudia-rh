# Fechar a branch `feat/browser-cdp-mcp`: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Versão 2 (2026-10-06).** Refeita a partir de três levantamentos completos e só de leitura: o motor do browser, documentos com leitura robusta, e o transversal (commits, testes, docs, backup). A versão 1 cobria cerca de um terço do que estava aberto.

**Goal:** Deixar `feat/browser-cdp-mcp` pronta para o usuário decidir o merge em `dev`. Todo item aberto das specs e planos desta branch sai com um destino: **feito com evidência**, **decidido pelo usuário** ou **registrado no Linear**. Sem feature nova.

**Architecture:** Quase tudo é verificação (testes, smoke, rodadas de agente, revisão) e documentação. O código novo se limita a: o harness dos testes live (CLA-39, mais uma trava de deleção), três correções pequenas que a revisão da Parte 2 apontou, e o que o usuário decidir em D7 a D9.

**Tech Stack:** Rust (Tauri 2, tokio), testes `#[ignore]` com Chrome for Testing, Node 24 (scripts, `node --test`), React/TS (`tsc`, vite), skill `claudia-teste-e2e`.

**Specs desta branch** (este plano não muda regra nenhuma delas; onde o código diverge, a Tarefa 8 alinha o texto ou leva o caso ao usuário):
- `docs/superpowers/specs/2026-10-02-browser-cdp-motor-design.md` (plano `plans/2026-10-02-browser-cdp-motor.md`, ledger `.superpowers/sdd/2026-10-02-browser-cdp-motor/`)
- `docs/superpowers/specs/2026-10-03-mcp-leitura-robusta-design.md` (sem plano nem ledger próprio)
- `docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md` (plano `plans/2026-10-04-anexar-curriculo.md`, ledger `.superpowers/sdd/2026-10-04-anexar-curriculo/`)

## ⛔ Regra número 1: nenhum arquivo do usuário é apagado, nenhum trabalho é perdido

Em 2026-10-05 um teste de "limpeza de dados", escrito por um agente e nunca revisado, apagou o disco C do usuário. Foram cerca de 12 horas de trabalho perdidas, e só parte foi recuperada.

- **Nenhum teste roda antes da Tarefa 0b** corrigir os `BLOQUEIA` achados na auditoria (`docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md`). As travas da seção 6 da auditoria valem em todas as tarefas.
- **`node --test` sempre com a lista de arquivos explícita**, nunca sem argumento: a descoberta automática pega `comum.test.mjs`.
- **Cache e arquivo temporário só se apagam por caminho literal com guarda**, nunca por `$VAR` (o estado do shell não persiste entre chamadas).
- Todo caminho que chega a `remove_dir_all`, `remove_file`, `rmSync`, `Remove-Item`, `rm -rf`, `robocopy /MIR` ou `git clean` tem de ser uma **pasta temporária criada pelo próprio teste**. Na dúvida, **não roda**: pergunte ao usuário.
- **Nunca** passe raiz de disco, `~`, `%USERPROFILE%`, `AppData`, a pasta do projeto ou qualquer caminho real a uma função que apaga, **nem para testar a recusa**. A recusa se testa na função de validação pura.
- **Todo subagente recebe este bloco literalmente no prompt.**
- Desinstalação, limpeza de dados e **todo teste cujo objetivo é apagar ou limpar coisas** rodam **só** no Windows Sandbox. Nunca nesta máquina. (Teste normal que só limpa a própria pasta temporária continua rodando aqui.)
- Antes de escrever nos dados reais, faça um **backup novo numa pasta nova**. Nunca sobrescreva nem apague um backup.
- **Evidência vai para o repositório.** `.superpowers/` está em `.git/info/exclude` e não é versionado. A prova desta branch fica em `docs/superpowers/evidencias/` (Tarefa 2), sem caminhos pessoais.
- Commit a cada passo verde. Trabalho de agente que não foi commitado conta como perdido.

## Global Constraints

- Nenhum passo de instalação novo para o usuário.
- Nunca mudar o `identifier` Tauri `io.github.johngabie.claudia-rh`.
- `src-tauri/src/prompt_sistema_runtime.md`: nenhuma edição neste plano.
- Comentários de código, identificadores novos e commits em inglês. Docs e copy da UI em pt-BR.
- Cada agente usa o próprio `CARGO_TARGET_DIR` (`C:/Users/joaog/.cache/claudia-close-<tarefa>`). No fim, apaga **só esse cache**, depois de conferir o caminho exato.
- Chrome de teste: `CLAUDIA_TEST_CHROME=%TEMP%\cft\chrome-win64\chrome.exe` (conferido em 2026-10-06).
- Sem push, sem merge e sem PR. Integrar é decisão do usuário (Tarefa 11).
- Site real, login real e envio real só com autorização explícita do usuário **naquela rodada**.
- Código de saída real em toda rodada. Nunca filtrar a saída de um jeito que esconda falha.
- Bug fora do escopo vai para o Linear e não é tratado aqui.

## Review Focus

1. **Um teste ou script apaga um caminho fora da pasta temporária dele.** Esperado: recusa antes de apagar. Coberto pela Tarefa 0 e pelo teste `anything_else_is_refused` da Tarefa 1.
2. **A migração sobre os dados reais duplica ou some com currículos.** Esperado: um documento por arquivo antigo, os originais intactos, e reabrir o app não duplica. Coberto pela Tarefa 5, passos 3 e 4.
3. **O PDF gerado dentro do app real abre uma janela visível ou não sai.** Esperado: nenhuma janela e um `%PDF-` válido. Coberto pela Tarefa 5, passo 6 (a prova anterior rodou só num binário à parte).
4. **O Chrome do app roda sem sandbox ou fica vivo depois de o app fechar.** Esperado: sem `--no-sandbox`, e o processo some ao fechar o app. Coberto pela Tarefa 5, passos 8 e 9.
5. **O banco registra um anexo que o próprio campo recusou.** Esperado: com a regra `accept` do campo recusando o formato, nada é gravado em `anexos`. Coberto pela Tarefa 3, correção A.

---

## Inventário: tudo o que está aberto, com destino

As caixas `- [ ]` dos planos nunca foram marcadas: o estado real vem dos ledgers, do código e do git. Legenda de destino: **T*n*** = tarefa deste plano; **D*n*** = decisão do usuário; **L** = Linear (Tarefa 10).

### Verificação nunca feita ou não refeita

| Item | Fonte | Destino |
|---|---|---|
| Rodadas completas depois do merge do M (`29eae20`); o último verde completo foi na Onda 3 | ledger anexar, estado da branch | T4 |
| clippy nunca rodou na Parte 2; baseline antigo de 3 avisos | ledger motor, `clippy-baseline.txt` | T4 |
| 3 testes de ícone (janela visível) nunca rodam na rodada padrão | `browser_live.rs:920,941,949` | T4 |
| Leitura robusta: nenhuma linha FAILED de mutação registrada (só "mutation-proofed" no `233629a`) | spec SR §5 | T4 |
| Mutações da Parte 2 relatadas pelos agentes, mas não registradas no ledger | ledger anexar | T2 |
| Smoke real da Parte 1 (importar, marcar preferido) e da Parte 2 (aba, gerar, tags, renomear, excluir, seletores do Perfil, migração, dois idiomas) | plano anexar T8.4, T16.4 | T5 |
| PDF pelo WebView2 **dentro do app real**, sem janela visível | plano anexar T9.5, ledger | T5 |
| Rodada de agente da Parte 2 (variantes, documentos, carta, upload com variante, escrita recusada na execução), observada pelo Opus | plano anexar T16.3 | T6 |
| Validação do Activity log (`activity-log-validate.mjs`) depois de uma sessão real | CLAUDE.md, plano motor 6.4 | T6 |
| Validações do motor 6.2: rodadas A, B e C **duas vezes** cada (o ledger tem uma); A inclui "fechar e reabrir não baixa de novo"; B estendida | plano motor:4097-4116 | T6 |
| Validações do motor 6.4: concorrência com dois processos MCP, `dev/` só leitura, retenção com linhas de 40 dias, viewer | plano motor:4203-4210 | T6 |
| Persistência do perfil entre duas aberturas (login continua) | spec motor §7 | T5 |
| Chrome do app com sandbox | ledger motor:51,59 | T5 |
| Desligamento do Chrome ao fechar o app pela janela | ledger motor:64 | T5 |
| Recorte "antes" cai sobre o alvo certo (hoje só confere que existe) | spec motor:158 | L |
| Desinstalação real; caixa nativa do NSIS em pt-BR | ledger motor:63-64 | L (só Sandbox) |
| LinkedIn real, 6.3 (a)-(f) | plano motor:4118 | D4 |
| Upload em site real (`the-internet.herokuapp.com/upload`) | plano anexar T8.5, T16.5 | D3 |
| Revisão final Opus da branch inteira: a de 2026-10-02 deu FAIL e nunca foi refeita; as correções K, L, L2 e M nunca tiveram nova revisão | ledger motor:65, ledger anexar | T9 |
| Revisão de fase da Fase 2 do motor (ciclo de vida em `manager.rs`/`service.rs`) foi pulada | ledger motor:59,66 | T9 (foco explícito) |

### Código: correções e lacunas

| Item | Fonte | Destino |
|---|---|---|
| CLA-39: Chrome de teste sobe sem página e fica órfão; harness apaga sem trava | `browser_live.rs:35`, `viewport_live.rs:23` | T1 |
| Anexo gravado mesmo quando a regra `accept` do campo recusa o formato | `browser.rs:1085-1086` | T3-A |
| `update_variante` responde com nome de arquivo (`search_variants.yaml.bak-1`) | `mcp/tools/variantes.rs:54` | T3-B |
| Teste ignorado de prova do PDF ficou no código | `commands/curriculos.rs:814-829` | T3-C |
| Janela de impressão do PDF roda com JavaScript ligado | `documentos/pdf.rs` | D9 |
| `PrintToPdf` pode gravar depois do timeout | `pdf.rs:99` | L |
| **Eventos do browser nunca vão para o Activity log** (download, instalação, lançamento, queda, crash de aba, diálogo); o filtro "browser" da tela fica sempre vazio | spec motor §5.6, `sink.rs:77,97` | D7 |
| Diálogos nativos (`alert`/`confirm`, `Page.javascriptDialogOpening`) não são tratados | ledger motor:54-56 | D8 |
| Fechar a aba ao sair, página trocada por `loaderId`, mutex de ação por instância, `bring_to_front` roubando foco | ledger motor:54-56 | L |
| Histórico não distingue "documento apagado" de "nunca anexado" | `queries.rs:205` | L |
| Atalho das Pendências não abre a sub-aba Cartas para `sem_carta` | `App.tsx:184` | L |
| Arquivo antigo cujo PDF falha é tentado de novo a cada `save_cover_letter` (espera de 45 s) | `migrar.rs` | L |
| Dashboard chama `configurar_limite_vagas_sessao`, que não existe (bug que já vem de `dev`) | `dashboard/index.tsx:277` | L |
| Porta de debug sem autenticação (processo local contorna a guarda) | ledger motor | T8 (documentar na spec §5.5) + L (CLA-30) |
| Página hostil pode enviar pelos próprios handlers JS; ramo de falha da leitura AX sem teste live | ledger motor:56 | L |
| Nomes de parâmetro misturados nas tools (`id`, `variante_id`, `documento`) | observador da Parte 2 | L (renomear depois do merge) |
| CLA-35 (vagas presas em `candidatando`), CLA-36 e CLA-38 (testes instáveis), CLA-37 (guarda), CLA-40 (carta sem nome) | Linear | D2, D6, D10 |
| Skill `tailor-application`: onde grava arquivos (o prompt ainda manda segui-la) | plano anexar T14 passo 0 | T7 |
| Pacote de diagnóstico sem documentos: não há teste que prove | spec anexar §11 | L |
| `submit_claims` criada fora das migrations | CLAUDE.md | L |
| CI roda só `cargo test --lib` e `npm run build` (sem front, clippy, live) | `.github/workflows/ci.yml` | L (prioridade máxima, depois da branch) |

### Documentação que diz o contrário do código

| Item | Fonte | Destino |
|---|---|---|
| CLAUDE.md: "não há `browser_upload`", spec "ainda rascunho", lista de tools incompleta, sem biblioteca nem aba Currículos | CLAUDE.md:13, 197, seção final | T8 |
| Arquitetura §3.2 ainda pede a extensão Claude in Chrome; não descreve documentos, aba Currículos, PDF nem as tools novas | `arquitetura-sistema-candidaturas.md:59` | T8 |
| README: linhas 7, 49, 62 e 154 falam de "Claude Code conectado ao Chrome" | `README.md` | T8 |
| Padrão de design sem a aba Currículos (navegação, grade, página A4) nem a tela Activity log | `padrao-design-claudia-rh.md:85-97` | T8 |
| CHANGELOG `[Unreleased]` vazio | `CHANGELOG.md` | T8 |
| Cabeçalhos de status: spec de anexar ("RASCUNHO"), `propostas-pos-corte.md` ("NADA aqui foi aplicado"), plano do protótipo, specs do motor, leitura robusta, roadmap, contas | specs e planos | T8 |
| Spec de anexar: §7 diz "escrita só na sessão de Perfil" (o código lista e recusa, como diz a §2.4); §7 promete checagem de fatos no `update_variante` (P4 recusou); a parte do seletor nativo tardio promete fechá-lo com Escape (ruling: não implementado); a pré-visualização diz "PDF em iframe, a verificar" (escolhido cartão); §3 sem `anexos` nem `candidaturas.documento_id` | spec anexar | T8 |
| Spec de leitura robusta §3.5: o rótulo `porcentagem` e as "refs invalidadas" não batem com o texto real do scroll | spec SR, `browser.rs:1012` | T8 |
| Spec do motor §5.7 promete duas opções no desinstalador; a Tarefa 4.2 foi cortada | spec motor, ledger motor | D5 + T8 |

### Higiene do repositório

| Item | Fonte | Destino |
|---|---|---|
| `prototipo-playwright/`: 24 arquivos versionados com **caminhos pessoais**, num repositório público | `git ls-files prototipo-playwright` | D11 + T7 |
| Worktree `agent-a0358d306d0389568` (já integrada) e a pasta solta `agent-a5609a9…` | `.claude/worktrees/` | D12 + T7 |
| Branches paradas: `feat/exec-prototipo-playwright`, `feat/prototipo-playwright-mcp`, `backup/feat-browser-cdp-mcp-pre-rebase` (manter até o merge) | `git branch` | T11 |
| `dev` 55 commits à frente do `origin/dev` | `git branch -vv` | T11 |

### Decisões que só o usuário toma (com recomendação)

| # | Pergunta | Recomendação |
|---|---|---|
| D1 | A auditoria achou `BLOQUEIA`? | O plano para e o caso vai até você |
| D2 | CLA-36 e CLA-38 (testes live instáveis) impedem o merge? | Não: ficam registrados, e o merge exige que passem isolados |
| D3 | Rodada de upload contra `the-internet.herokuapp.com`? | Sim, uma rodada, sem enviar candidatura nenhuma |
| D4 | LinkedIn real (6.3) entra nesta branch? | Não: vai para o Linear e roda quando você puder logar |
| D5 | Confirma o corte da Tarefa 4.2 (limpeza própria na desinstalação)? | Sim: depois do incidente, código de limpeza novo só com spec própria e teste em Sandbox |
| D6 | CLA-37 (a guarda de envio deixa passar submit com nome fora da lista) antes ou depois do merge? | Antes: é segurança do núcleo desta branch, e precisa de spec curta |
| D7 | Eventos do browser no Activity log, prometidos pela spec do motor | Corrigir a spec agora ("não implementado") e implementar em branch própria |
| D8 | Diálogos nativos (`alert`/`confirm`) | Mesma coisa: issue e branch própria |
| D9 | Desligar o JavaScript na janela de impressão do PDF | Sim, nesta branch, se for uma linha com teste. Senão, Linear |
| D10 | CLA-35 e CLA-40 impedem o merge? | Não |
| D11 | Pasta `prototipo-playwright/` | Tirar da branch antes do merge; o que vale já está em `docs/pesquisa/` |
| D12 | Remover a worktree já integrada e a pasta solta? | Sim, só depois de você ver o que há dentro |
| D13 | Confirmar as decisões que tomei sem você: o Chrome fecha junto com o app; pular a revisão da Fase 2 (coberta pela T9); `browser_upload` listado em toda sessão; seletor nativo tardio sem Escape; `browser_upload` responde com o nome do arquivo (não com o caminho) | Confirmar todas |

---

### Task 0: Auditoria de deleção (só leitura, Opus)

**Files:**
- Create: `docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md` (versionado; caminhos relativos ao repositório, sem caminho pessoal)

**Interfaces:**
- Produces: a tabela de auditoria. Nenhuma outra tarefa roda teste antes dela, e só se não houver `BLOQUEIA`.

- [ ] **Step 1: Listar todo ponto de deleção**

Run (em `claudia-rh/`):
```bash
grep -rnE "remove_dir_all|remove_file|rmSync|rmdirSync|fs\.rm\(|unlinkSync|Remove-Item|rm -rf|rmtree|robocopy.*/MIR|git clean" \
  --include=*.rs --include=*.mjs --include=*.js --include=*.ts --include=*.tsx --include=*.ps1 \
  src-tauri/src src-tauri/tests src tests .claude/skills
```
Expected (2026-10-06): cerca de 60 linhas em 23 arquivos. Entre elas `migration.rs:115`, que apaga a pasta de dados antiga **em produção**, `browser/install.rs:100,131,164`, `diagnostics/mod.rs`, `documentos/*`, `tests/browser_live.rs:35`, `tests/viewport_live.rs:23`, `tests/mcp-browser-smoke.mjs:133` e os scripts das skills de teste (`claudia-teste-sandbox/scripts/comum.mjs:59` recria a pasta de cenário).

- [ ] **Step 2: Classificar cada linha**

Para cada linha: leia a função inteira, rastreie de onde vem o caminho e grave uma linha na tabela:

```markdown
| arquivo:linha | produção ou teste | de onde vem o caminho | travas existentes | veredito |
|---|---|---|---|---|
| src-tauri/tests/browser_live.rs:35 | teste | temp_dir()/claudia-live-<pid>-<nanos> criado em launch() | nenhuma | OK, reforçar na Tarefa 1 |
```
Vereditos possíveis: `OK`, `OK, reforçar na Tarefa 1`, `REVISAR`, **`BLOQUEIA`**. É `BLOQUEIA` quando o caminho pode vir de fora de uma pasta temporária criada pelo teste, ou de variável que pode estar vazia ou ser relativa.

- [ ] **Step 3: Resolver os `REVISAR`** lendo os testes que chamam a função. Se a dúvida continuar, vira `BLOQUEIA`.

- [ ] **Step 4: Portão e commit**

Com algum `BLOQUEIA`: **pare o plano** e leve a tabela ao usuário. Sem nenhum:
```bash
git add docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md
git commit -m "docs(evidence): deletion audit before running any test (0 blocking)"
```

---

### Task 0b: Corrigir os `BLOQUEIA` da auditoria (antes de qualquer teste)

**Estado:** a Tarefa 0 foi feita em 2026-10-06 e achou 7 `BLOQUEIA` (`docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md`, seção 1). O plano **para aqui** até o usuário aprovar estas correções (D1).

Cada correção segue o ciclo de sempre (teste que falha antes, depois passa, e mutação), **com uma exceção obrigatória: nenhum teste desta tarefa chama uma função que apaga com caminho real**. A recusa se testa na função pura de validação.

| # | Correção | Arquivos | Modelo |
|---|---|---|---|
| B1 + B2 | `motivoRecusa(dir)` pura com lista branca (só `os.tmpdir()/claudia-teste-sandbox/`, caminho real, sem reparse point, com marcador). `novoCenarioDir` a chama antes de apagar. Os testes de recusa passam a testar só `motivoRecusa`, e as três chamadas com `REPO_ROOT`, a pasta acima dele e `C:\` saem. | `claudia-teste-sandbox/scripts/comum.mjs`, `test/comum.test.mjs`, `SKILL.md` (tirar a sugestão de `--saida` em outro disco) | Sonnet + revisão Opus |
| B3 | Marcador `.lab-deploy` exigido antes do `-Clean` | `claudia-teste-sandbox/scripts/publicar.mjs`. O `app-stage.ps1` do laboratório externo é outro repositório: só proposta ao usuário. | Sonnet |
| B4 | `subir.mjs` aborta com a automação ligada (`disparo.ativo !== false` ou `skip_permissions === true`) | `claudia-teste-e2e/scripts/subir.mjs` | Haiku |
| B5 | `invocar.mjs`: `JSON.parse` e `JSON.stringify`, lista negra mesmo com `--escrever`, `obter_credencial` sempre recusado. Junto, ampliar `rotuloPerigoso` em `regras.mjs`. | `claudia-teste-e2e/scripts/{invocar,regras}.mjs` e os testes deles | Sonnet |
| B6 | Evals com `--permission-mode plan`, `--disallowedTools` e kill da árvore | `.claude/skills/*/evals/rodar-evals.mjs` | Haiku |
| B7 | Injetar o kill em `prepare`: os testes passam `|_| {}` | `src-tauri/src/browser/service.rs` | Sonnet |

**Decisões do usuário, item por item (2026-10-06)** (valem sobre a tabela acima):

| # | Decisão |
|---|---|
| B1 | **Corrigir.** "Foi esse que ferrou meu sistema. Nunca mais faz isso." O teste nunca mais chama a função que apaga com caminho real. |
| B2 | **Endurecer** (lista branca dentro de `%TEMP%\claudia-teste-sandbox\`). Princípio do usuário: **teste cujo objetivo é apagar ou limpar coisas roda só no Sandbox.** Teste normal que só limpa a própria pasta temporária continua rodando na máquina. |
| B3 | **Pôr trava:** só apaga o `deploy` que tiver o marcador criado pelo próprio `publicar`. (O `deploy` é limpo antes de copiar, não depois da rodada.) |
| B4 | **Não recusar:** quando o agente sobe o app para teste, ele sobe com o disparo por ociosidade e o modo autônomo **desligados**, por uma variável que só vale no build de desenvolvimento. Os arquivos do usuário e o app normal ficam iguais. O `--dangerously-skip-permissions` em si fica para o usuário resolver depois. |
| B5 | **Lista negra no `--escrever`** (`apagar_`, `excluir_`, `remover_`, `limpar_`, `desinstalar`, `instalar_`, `iniciar_`, `disparar_`, `enviar_` só com `--confirmo=<comando>`) e **`obter_credencial` sempre recusado**. O parse do JSON não foi pedido. |
| B6 | **Linear, depois.** Fora do caminho do plano; os evals continuam só manuais. |
| B7 | **Higiene, não bloqueia:** o kill falso nos testes entra junto com a Tarefa 1. |

Depois, a mesma revisão Opus da auditoria relê os arquivos corrigidos e confirma que não sobrou `BLOQUEIA`. Os itens das seções 2 e 3 da auditoria vão junto com a Tarefa 1 (harness live) ou para o Linear. As travas da seção 6 passam a valer em todas as tarefas.

---

### Task 1: Harness live: trava de deleção e subida do Chrome (CLA-39)

**Files:**
- Create: `src-tauri/tests/support/mod.rs`
- Modify: `src-tauri/tests/browser_live.rs` (`mod support;`, `Drop for Live`, começo de `launch()`)
- Modify: `src-tauri/tests/viewport_live.rs` (idem)

**Interfaces:**
- Produces (`tests/support/mod.rs`): `new_profile_dir(tag: &str) -> PathBuf`; `is_removable_profile_dir(p: &Path, temp_root: &Path) -> bool` (pura); `remove_profile_dir(p: &Path)`; `kill_tree(pid: u32)`; `async first_page_with_retry(port: u16) -> Result<cdp::Target, String>`.

- [ ] **Step 1: Escrever o módulo com os testes de recusa (validação pura, nada é apagado)**

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
Os testes usam caminhos que **não existem** e só chamam a função pura.

- [ ] **Step 2: Ligar e rodar só os testes puros**

Acrescente `mod support;` no topo de `tests/browser_live.rs` e de `tests/viewport_live.rs`.
Run (em `src-tauri/`): `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-close-1 cargo test --test browser_live support::`
Expected: `2 passed`.

- [ ] **Step 3: Mutação**

Troque `p.parent() == Some(temp_root)` por `true` e rode de novo. Cole a linha `test support::tests::anything_else_is_refused ... FAILED`. Restaure.

- [ ] **Step 4: Usar os helpers nos dois harnesses**

Em `tests/browser_live.rs`:
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
    // a partir de `let client = ...`, o resto de launch() fica igual
```
Em `tests/viewport_live.rs`, a mesma troca, com `support::new_profile_dir("viewport")` e `profile.join("state.json")`. Depois, `grep -n "remove_dir_all" src-tauri/tests/*.rs` mostra **só** `tests/support/mod.rs`.

- [ ] **Step 5: Suíte live três vezes**

```bash
cd src-tauri && export CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-close-1 CLAUDIA_TEST_CHROME="$(cygpath -w "$TEMP")\\cft\\chrome-win64\\chrome.exe"
for i in 1 2 3; do cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon > live-$i.log 2>&1; echo "rodada $i exit=$?"; grep -E "test result|FAILED" live-$i.log; done
```
Expected: exit 0 nas três (`browser_live` 70 passed, `viewport_live` 5 passed). Uma falha de CLA-36 ou CLA-38 vai para D2, com a mensagem. Depois das rodadas, nenhum `chrome.exe` com `claudia-live-` na linha de comando (`Get-CimInstance Win32_Process`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/support/mod.rs src-tauri/tests/browser_live.rs src-tauri/tests/viewport_live.rs
git commit -m "test(live): guarded profile deletion and a retrying first page so a failed launch never orphans Chrome (CLA-39)"
```
Apague só `C:/Users/joaog/.cache/claudia-close-1` e os `live-*.log` que criou, conferindo os caminhos antes.

---

### Task 2: Evidência dentro do repositório

**Files:**
- Create: `docs/superpowers/evidencias/2026-10-feat-browser-cdp-mcp.md`

**Interfaces:**
- Consumes: os ledgers `.superpowers/sdd/2026-10-02-browser-cdp-motor/progress.md` e `.superpowers/sdd/2026-10-04-anexar-curriculo/progress.md`, e os registros dos agentes abaixo.
- Produces: o arquivo que as Tarefas 4 a 10 completam e que a descrição do PR cita.

- [ ] **Step 1: Escrever o arquivo**

Uma seção por ciclo (motor, leitura robusta, anexar Parte 1, anexar Parte 2). Em cada seção, uma tabela por tarefa: commit, testes novos, mutação com o teste que falhou, revisão. Copie dos ledgers o que já está lá. Para a Parte 2, use estes registros dos relatórios dos agentes de 2026-10-04/05, que nunca foram para o ledger:

| Frente | Commit (do relatório) | Mutações (teste que falhou) |
|---|---|---|
| A, PDF | `019a0f0`, `e56c5ba` | `a_failed_pdf_registers_nothing_and_leaves_no_files`, `an_invalid_pdf_is_refused` |
| B1, variante da vaga | `fe64204` | `without_a_vaga_variant_the_principal_is_used`, `an_unknown_variant_is_refused_and_nothing_is_written`, `marking_a_principal_unmarks_the_others` |
| B2, comandos | `b109f3d` | `deleting_never_touches_a_file_outside_the_folder`, `the_preview_is_read_only_from_inside_the_folder`, `a_zip_renamed_docx_is_refused`, `an_existing_destination_is_never_overwritten`, `schema_creates_all_tables`, `deleting_removes_its_files_and_its_preferred_marks_only` |
| C, aba | `2230ad6` | `ehPreferido`, `filtrar`, sandbox do `FolhaA4` (node --test, ✖) |
| D, migração | `5cb8bd9` | `running_twice_migrates_once`, `old_cvs_and_letters_become_documents_and_the_originals_stay`, `a_half_done_migration_is_retried`, `a_pending_letter_from_the_agent_becomes_a_document` |
| E, MCP | `181d51f` | 8 mutações, entre elas `update_variante_is_refused_in_the_execution_session`, `reads_never_return_a_path`, `no_document_tool_returns_a_path`, `marking_preferred_is_a_profile_session_write`, `the_execution_session_cannot_write_variants_or_preferred_documents` |
| F, upload | `b2e2b3a` | `a_failed_upload_is_logged_with_its_target`, `an_upload_result_ends_with_the_state_line`; do orquestrador: `the_upload_message_and_the_log_use_the_same_size_unit` |
| G, anexos | `2af0d70` | `history_names_the_document_used`, `the_candidatura_remembers_the_cv_that_was_attached`, `the_current_vaga_lists_its_latest_attachments`, `an_attachment_is_recorded_for_the_vaga`, `the_upload_line_in_the_activity_log_names_the_document_and_its_id` |
| H, telas | `b55edb7`, `8d4a3e6` | 4 mutações em `tests/front/onde-aparece.test.ts` (✖) |
| I, prompt | `debfbda`, `342155c` | teste do prompt sem `set_vaga_variante`; `sem_carta` trocado por `sem_documento` |
| J, anexo só no sucesso | `207a53f` | `successful_upload_calls_attached_with_vaga_type_and_document_id`, `failed_upload_does_not_call_attached` |
| K, migração concorrente | `6e9a9c7` | do orquestrador: `a_migrated_document_the_user_deletes_does_not_come_back`, `two_concurrent_migrations_register_each_file_once_and_keep_the_files` (3 de 3) |
| L2, recarga da aba | `b7b504e` | `unmounting before listen resolves still releases the late subscription`, `a burst of events reloads once` (✖) |
| M, `update_variante` | `9eaf3ec`, `6b043ae` | `update_variante_changes_only_the_requested_field`, `update_variante_preserves_unknown_keys_everywhere`, `the_screen_single_variant_save_and_weights_keep_the_rest`, `the_screen_whole_file_save_keeps_what_the_struct_does_not_model` |

A branch foi reescrita em 2026-10-05. Confira cada commit pelo assunto (`git log --format='%h %s' dev..HEAD`) e grave o hash atual.

Achados da revisão Opus da Parte 2 que só existiam na conversa, a registrar nesta seção:
- **Minors (7):** janela de impressão com JavaScript ligado; `PrintToPdf` depois do timeout; anexo com `accept` recusado; Histórico apagado contra nunca anexado; sub-aba das Pendências; arquivo cujo PDF falha repetido a cada carta; resposta com `.bak-1`.
- **Declined to judge (6):** `register_candidatura` ainda recebe `pasta_arquivos`; `guardar_variante_unica` sem backup; `apagar_documento` sem `forcar`; texto de `get_documento` pode ir para o `pty-tail.log`; PDF no app real; spec 2.3 não implementada (correto).

Nenhum caminho absoluto de usuário no arquivo: use caminhos relativos ao repositório.

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/evidencias/2026-10-feat-browser-cdp-mcp.md
git commit -m "docs(evidence): per-task tests and mutations of this branch, in the repo instead of an ignored ledger"
```

---

### Task 3: Três correções pequenas da revisão da Parte 2

**Files:**
- Modify: `src-tauri/src/mcp/tools/browser.rs:1085-1086`
- Modify: `src-tauri/tests/browser_live.rs` (novo teste, ao lado de `failed_upload_does_not_call_attached`)
- Modify: `src-tauri/src/mcp/tools/variantes.rs:52-55` e o teste `update_variante_writes_with_a_backup_and_keeps_one_principal`
- Modify: `src-tauri/src/commands/curriculos.rs:814-829` (remover)

**Interfaces:**
- Consumes: `TrackedDocs` (`browser_live.rs:1508`), `accept_allows` (`browser.rs`).

**Correção A: não registrar anexo que a regra `accept` do campo recusa.**

- [ ] **Step A1: Teste que falha**

Em `tests/browser_live.rs`:
```rust
#[tokio::test]
#[ignore]
async fn a_file_the_field_refuses_is_not_recorded_as_attached() {
    let l = launch().await;
    let tracked = TrackedDocs::new("docx");
    let t = tools(&l, Ctx::candidatando(7)).with_documents(tracked.clone());
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Só PDF"), None, None).await.unwrap();
    assert!(out.contains("o campo aceita só .pdf"), "{out}");
    assert!(tracked.get_calls().is_empty(), "a refused format is not recorded in anexos");
}
```
Run: `cargo test --test browser_live -- --ignored a_file_the_field_refuses` → Expected: FAIL em `a refused format is not recorded in anexos`.

- [ ] **Step A2: Implementar**

Em `browser.rs`, troque:
```rust
            // Success path only (every failure above returned): remember which document went to this vaga.
            self.docs.attached(vaga, tipo, att.id);
```
por:
```rust
            // Remember which document went to this vaga, unless the field's own accept rule refuses its format.
            if accept_allows(&accept, &att.formato) {
                self.docs.attached(vaga, tipo, att.id);
            }
```

- [ ] **Step A3: PASS e mutação**

O teste passa. `successful_upload_calls_attached_with_vaga_type_and_document_id` e `failed_upload_does_not_call_attached` também passam. Mutação: volte à chamada sem o `if`, cole o FAILED de `a_file_the_field_refuses_is_not_recorded_as_attached`, restaure.

**Correção B: `update_variante` responde sem nome de arquivo.**

- [ ] **Step B1: Teste que falha**

Em `update_variante_writes_with_a_backup_and_keeps_one_principal` (`mcp/tools/variantes.rs`), troque a primeira chamada para guardar a resposta e acrescente a asserção:
```rust
        let msg = update_variante(&d, "backend", &serde_json::json!({ "peso": 70, "principal": true }), SessionKind::Interactive).unwrap();
        assert!(!msg.contains(".yaml") && !msg.contains(".bak"), "no file name in the answer: {msg}");
```
Run: `cargo test --lib update_variante_writes_with_a_backup` → Expected: FAIL.

- [ ] **Step B2: Implementar**

```rust
    Ok(format!(
        "Variante {id} atualizada ({}). A versão anterior fica guardada como cópia de segurança.",
        obj.keys().cloned().collect::<Vec<_>>().join(", ")
    ))
```

- [ ] **Step B3: PASS e mutação.** Volte ao texto antigo, cole o FAILED e restaure.

**Correção C: remover o teste de prova do PDF.**

- [ ] **Step C1:** apague de `commands/curriculos.rs` o bloco que começa em `/// Writes a sample CV (fake profile) for the PDF proof.` e termina no `}` de `write_sample_cv_for_the_pdf_proof` (linhas 814 a 829 em 2026-10-06). Run: `cargo test --lib` → Expected: `0 ignored` na linha `test result`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/mcp/tools/browser.rs src-tauri/tests/browser_live.rs src-tauri/src/mcp/tools/variantes.rs src-tauri/src/commands/curriculos.rs
git commit -m "fix: no anexo for a format the field refuses, no file name in update_variante's answer, drop the PDF proof test"
```

---

### Task 4: Rodadas completas sobre o HEAD

**Files:** nenhum de produção. Números em `docs/superpowers/evidencias/2026-10-feat-browser-cdp-mcp.md`.

- [ ] **Step 1: Unitários, frontend e clippy**

```bash
cd claudia-rh && npx tsc --noEmit; echo tsc=$?
node --test "tests/front/*.test.ts"; echo front=$?
npm run build; echo build=$?; grep -rl "mock-documentos-dev-only\|__mockEmit\|mockInvoke" dist && echo "MOCK NO BUNDLE" || echo mock-fora
cd src-tauri && export CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-target
cargo test --lib; echo lib=$?
cargo clippy --all-targets; echo clippy=$?
```
Expected: exit 0 em todos e `mock-fora`. Os avisos do clippy são comparados ao baseline (3 avisos antigos). Um aviso novo é corrigido ou registrado.

- [ ] **Step 2: Integração**

A suíte live da Tarefa 1, Step 5, uma vez. Depois os 3 testes de ícone, **uma vez**. Eles abrem uma janela visível do Chrome de teste, com perfil temporário:
`cargo test --test browser_live -- --ignored icon --test-threads=1; echo icon=$?`
E o smoke MCP:
`cargo build && CLAUDIA_TEST_CHROME=... CLAUDIA_ALLOW_LOCAL_URLS=1 node tests/mcp-browser-smoke.mjs "C:/Users/joaog/.cache/claudia-target/debug/claudia-rh.exe"; echo smoke=$?` → a última linha é `TUDO OK`.

- [ ] **Step 3: Mutações da leitura robusta** (nunca coladas)

Uma regra por vez. Quebre, rode o teste nomeado, cole o FAILED e restaure:
- viewport mínimo (`browser/viewport.rs`, limite 1200×800) → os testes de `viewport_live.rs`;
- `browser_text` nunca devolve valor digitado (`browser/text.rs`) → `what_was_typed_into_a_field_never_comes_out_not_even_a_password`;
- detecção de `login_wall` (`browser/pagestate.rs`) → o teste unitário de `login_wall`;
- detector de loop (`loopguard.rs`, `THRESHOLD = 4` trocado por `40`) → o teste de loop;
- `browser_scroll` em contêiner (`browser.rs`) → `scroll_with_a_ref_moves_that_elements_own_container_and_says_when_it_is_at_the_limit`.

Use `git diff --stat` para confirmar que tudo foi restaurado.

- [ ] **Step 4: Registrar** cada comando com exit code no arquivo de evidência. Commit: `docs(evidence): full rounds on <hash>`.

---

### Task 5: Smoke de uso real no app (autorizado pelo usuário em 2026-10-05)

**Files:** nenhum de produção. Prints em `debug-logs/prints/fechamento/`, que o git ignora e pode ter dados pessoais.

- [ ] **Step 1: Backup novo**

Feche o app só se foi este plano que o abriu. Se foi o usuário, pergunte antes.
```powershell
$dst = "C:\Users\joaog\Documents\ClaudiaRH-backup-$(Get-Date -Format yyyyMMdd-HHmm)"
if (Test-Path $dst) { throw "pasta de backup já existe: $dst" }
robocopy "$env:APPDATA\io.github.johngabie.claudia-rh" "$dst\appdata" /E /R:1 /W:1 /NFL /NDL
robocopy "$env:LOCALAPPDATA\io.github.johngabie.claudia-rh" "$dst\localappdata" /E /R:1 /W:1 /NFL /NDL /XD browser
```
**Sem** `/MIR` nem `/PURGE`. No robocopy, exit de 0 a 7 é sucesso. Confira que `$dst\appdata\claudia_rh.db` existe.

- [ ] **Step 2: Contagens antes**

`node .claude/skills/claudia-teste-e2e/scripts/banco.mjs "SELECT (SELECT COUNT(*) FROM vagas) vagas, (SELECT COUNT(*) FROM candidaturas) cands"` e, se a tabela existir, `SELECT COUNT(*) FROM documentos`. Liste nome e tamanho de cada arquivo em `curriculos\` e `cover_letters\` do `%APPDATA%` do app.

- [ ] **Step 3: Subir o app e conferir a migração**

`node .claude/skills/claudia-teste-e2e/scripts/subir.mjs`. Se a ação for `perguntar`, pergunte ao usuário. Expected: um documento por arquivo antigo, a listagem dos originais **idêntica** à do Step 2, e a aba Currículos mostrando os migrados sem recarregar.

- [ ] **Step 4: Reabrir não duplica.** Feche o app (foi este plano que o abriu) e suba de novo. Expected: as mesmas contagens.

- [ ] **Step 5: Aba Currículos nos dois idiomas**

Pelo `ver.mjs`: grade de currículos, grade de cartas, uma página A4, Voltar, filtros por tag, editar tag, renomear, marcar preferido geral e por variante, e o select da variante principal no Perfil. Prints em pt-BR e em en. No fim, volte ao idioma original.

- [ ] **Step 6: Gerar currículo e carta**

"Gerar" na aba, com um template. Expected: o documento novo com formato `pdf`, o arquivo começando com `%PDF-` em `documentos\`, **nenhuma janela extra** no print da tela inteira (degrau 3) e a grade atualizada sozinha. Repita com uma cover letter.

- [ ] **Step 7: Importar e excluir**

Escreva um PDF fictício no scratchpad (nunca um arquivo pessoal) e importe com `invocar.mjs --escrever importar_documento`. Depois exclua esse mesmo documento pela UI. Expected: o arquivo some de `documentos\` e **nenhum outro arquivo** sai. Compare a listagem antes e depois.

- [ ] **Step 8: Chrome com sandbox**

Com o browser do app aberto pelo botão "Abrir o browser":
```powershell
Get-CimInstance Win32_Process -Filter "Name='chrome.exe'" | Where-Object { $_.ExecutablePath -like "*io.github.johngabie.claudia-rh*" -and $_.CommandLine -notmatch '--type=' } | Select-Object -ExpandProperty CommandLine
```
Expected: sem `--no-sandbox`. Se aparecer, é achado de segurança e bloqueia o fechamento.

- [ ] **Step 9: Persistência e desligamento**

Anote o PID do Chrome do app e um site em que o usuário já está logado, se houver (só leitura). Feche o app pela janela. Expected: o PID some em até 10 s e o `state.json` sai. Reabra o app e o browser: o login continua (persistência do perfil, spec do motor §7). Sem nenhum login, anote "não verificado: sem sessão".

- [ ] **Step 10: Registrar** os números, os caminhos dos prints e as divergências entre a UI, o banco e o Activity log. Toda divergência é **achado**. Commit no arquivo de evidência.

---

### Task 6: Rodadas de agente e validações do motor

**Files:** nenhum de produção. Transcrições em `debug-logs/rodadas/`.

- [ ] **Step 1: Rodada de agente da Parte 2 (Haiku), observada pelo Opus**

MCP real, sessão de **execução**, contra a fixture local `upload-form.html` servida em `127.0.0.1` (`CLAUDIA_ALLOW_LOCAL_URLS=1`), com pasta de dados temporária. A sequência:
1. `list_variantes`;
2. `set_vaga_variante` com uma variante válida e com uma inventada;
3. `list_documentos` e `get_documento`;
4. `browser_upload` na variante gravada;
5. `save_cover_letter`;
6. `update_variante` e `set_documento_preferido`, que precisam ser **recusados**.

Depois, o passo 6 na sessão de **Perfil**: precisa funcionar, e `update_variante` precisa manter os campos que não conhece. O Opus cruza o transcript com o Activity log e com o banco.

- [ ] **Step 2: Upload em site real.** Só se D3 for "sim": o mesmo upload em `the-internet.herokuapp.com/upload`, sem enviar candidatura nenhuma.

- [ ] **Step 3: Validação do Activity log**

Leia `src-tauri/tests/activity-log-validate.mjs` antes, para confirmar que ele só lê. Depois do smoke da Tarefa 5: `node src-tauri/tests/activity-log-validate.mjs <dados-locais>`. Expected: os campos e as contagens batem com as ações da Tarefa 5.

- [ ] **Step 4: Validações 6.2 do motor, segunda execução**

Rodadas A (subir, baixar, fechar e reabrir sem baixar de novo), B (estendida: clicar `Avançar` pela ref, ler `Avaliar`, escrever no Nome, envio recusado sem confirmação) e C, na fixture, uma vez cada. Somadas às de `docs/pesquisa/2026-10-02-validacao-execucao.md`, completam as duas execuções que o plano pedia. Uma falha intermitente conta como falha.

- [ ] **Step 5: Validações 6.4 do motor, cenários 4 a 7**

- **4:** retenção. Num `activity-log.db` **temporário**, insira linhas com 40 dias e confira que a limpeza as remove.
- **5:** dois `mcp-browser-smoke.mjs` ao mesmo tempo, cada um com a sua pasta temporária, gravando no log.
- **6:** `dev/` só leitura. A ferramenta continua funcionando, e o log falha em silêncio.
- **7:** a tela do Activity log abre esses registros.

Tudo em pastas temporárias, **nunca** no `%LOCALAPPDATA%` real.

- [ ] **Step 6: Registrar** e fazer o commit no arquivo de evidência.

---

### Task 7: Investigação e limpeza do repositório

- [ ] **Step 1: `tailor-application`** (só leitura). Procure onde a skill grava arquivos: em `~/.claude/skills`, nos plugins e no texto do prompt (`prompt_sistema_runtime.md:157`). Se gravar fora de `<dados>/documentos/`, o currículo que o agente gera nunca entra na biblioteca: registre no Linear. Anote o resultado na evidência.

- [ ] **Step 2: `prototipo-playwright/`** (só se D11 for "tirar")

Liste os arquivos com caminho pessoal: `git grep -n "joaog" -- prototipo-playwright`. Confirme que `docs/pesquisa/` tem as conclusões. Então `git rm -r prototipo-playwright`: tira da branch só o que o git rastreia nessa pasta. Confira com `git status` que nada fora dela mudou. Commit: `chore: drop the throwaway Playwright prototype from the branch (conclusions live in docs/pesquisa)`. O histórico antigo continua com os caminhos; se o usuário quiser reescrevê-lo, é decisão à parte.

- [ ] **Step 3: Worktrees** (só com D12)

Mostre ao usuário o estado da worktree registrada (`git -C .claude/worktrees/agent-a0358d306d0389568 status` e `log -1`) e o conteúdo da pasta solta `agent-a5609a9…`. Remova **só** com o sim dele: `git worktree remove .claude/worktrees/agent-a0358d306d0389568` para a registrada. A pasta solta só sai depois de ele ver a lista e confirmar.

---

### Task 8: Documentação batendo com o código

**Files:** `CLAUDE.md`, `README.md`, `CHANGELOG.md`, `docs/arquitetura-sistema-candidaturas.md`, `docs/padrao-design-claudia-rh.md`, e as specs e os planos citados abaixo.

- [ ] **Step 1: `CLAUDE.md`**
  - Ordem de leitura, item 5: tirar "ainda rascunho".
  - "Dívida conhecida": tirar "Agente sem como anexar arquivo…". Acrescentar "LinkedIn real e desinstalação nunca testados (Linear)", "Testes live instáveis: CLA-36, CLA-38" e, se D7 for "depois", "Eventos do browser não vão para o Activity log (Linear)".
  - "O que este repo já é" → Ferramentas do agente: acrescentar `browser_upload` (anexa o documento que o app escolhe, nunca recebe caminho) e as tools de documentos e variantes (`list_documentos`, `get_documento`, `save_cover_letter`, `list_variantes`, `get_variante`, `set_vaga_variante`; `update_variante` e `set_documento_preferido` são recusadas na execução). Nova linha: "Documentos: biblioteca em `<dados>/documentos/`, aba Currículos, PDF gerado pelo WebView2 do app".

- [ ] **Step 2: Arquitetura.** Na §3.2, a linha 59 (pré-requisitos da extensão) sai e vira "o browser é o Chrome for Testing baixado pelo app". Nova seção curta, "Documentos": a biblioteca, a escolha do arquivo pela variante, as tools sem caminho e o PDF pelo WebView2.

- [ ] **Step 3: README.** Linhas 7, 49, 62 e 154: trocar "Claude Code connected to Chrome" por "the app's own browser". Conferir que a instalação não pede a extensão.

- [ ] **Step 4: Padrão de design.** Na §3 (navegação), acrescentar "Currículos" (ícone `FileText`, entre Perfil e Histórico) e ajustar a numeração "(itens 2-7)". Acrescentar o layout da grade e da página A4, e a tela Activity log em Configurações.

- [ ] **Step 5: CHANGELOG.** No `[Unreleased]`: browser próprio (Chrome for Testing), sem a extensão, Activity log, anexar currículo, biblioteca e aba Currículos.

- [ ] **Step 6: Cabeçalhos de status**
  - Spec de anexar: `> **Status: APROVADA (2026-10-04) e implementada nesta branch.** Decisões nas seções 2.2 e 2.4; o conteúdo por variante saiu para \`feat/curriculo-por-variante\`.`
  - Specs do motor e da leitura robusta: `> **Status: implementada em \`feat/browser-cdp-mcp\`.** Pendências: \`docs/superpowers/evidencias/2026-10-feat-browser-cdp-mcp.md\`.`
  - Roadmap de 2026-09-27: `> **Status: superado pela decisão CDP direto** (\`docs/pesquisa/2026-09-28-DECISAO-browser.md\`). As Fases 3 e 4 (CLA-27, CLA-30, CLA-8) continuam abertas no Linear.`
  - Spec de contas: `> **Status: NÃO INICIADA.** Ciclo 2, branch própria \`feat/browser-contas\`, depois do merge desta. Precisa de aprovação.`
  - `propostas-pos-corte.md`: o título passa a terminar em `APLICADAS no commit beaeb33`.
  - Plano do protótipo: `> **Status: CONCLUÍDO.** Resposta da CLA-7: CDP direto.`

- [ ] **Step 7: Alinhar as specs ao código e às decisões**
  - Spec de anexar:
    - na §7, "escrita só na sessão de Perfil" passa a "listada nas duas sessões, recusada na de execução (§2.4)";
    - tirar "nunca afirma fato fora do `candidate_base.yaml`" (P4 recusou a checagem);
    - seletor nativo tardio: "não implementado; o Chrome só abre o seletor dentro dos 5 s do clique (ruling de 2026-10-04)";
    - pré-visualização do PDF importado: "cartão, por escolha";
    - na §3: acrescentar `anexos` e `candidaturas.documento_id`.
  - Spec de leitura robusta, §3.5: o texto do resultado é o real (`Posição: N% … fim_da_pagina`).
  - Spec do motor:
    - §5.5: documentar a porta de debug sem autenticação como risco aceito, com a CLA-30 como contenção;
    - §5.7: conforme D5;
    - §5.6: se D7 for "depois", "eventos do browser no log: não implementado (Linear)".

- [ ] **Step 8: Conferir**
```bash
grep -n "ainda rascunho\|RASCUNHO\|NADA aqui foi aplicado\|Agente sem como anexar\|connected to Chrome" CLAUDE.md README.md docs/*.md docs/superpowers/specs/*.md docs/superpowers/plans/2026-10-02-propostas-pos-corte.md
```
Expected: nenhuma linha.

- [ ] **Step 9: Commit:** `docs: specs, plans, CLAUDE.md, architecture, README, design and changelog match what this branch shipped`

---

### Task 9: Revisão final da branch inteira (Opus)

- [ ] **Step 1:** gere o diff `git diff dev...HEAD` num arquivo. Lance um revisor Opus de contexto novo, com `requesting-code-review`, com a tabela da Tarefa 0 e o arquivo de evidência em mãos. Foco:
  - todos os pontos de deleção;
  - a guarda de envio e a porta de debug;
  - o ciclo de vida do Chrome em `manager.rs`/`service.rs` (a revisão da Fase 2 foi pulada);
  - as correções K, L, L2 e M (sem revisão nova);
  - as tools MCP sem caminho;
  - os dados do usuário (migração, apagar, importar).

  Esta revisão substitui a de 2026-10-02, que deu FAIL.

- [ ] **Step 2:** Critical e Important são corrigidos com teste que falha antes, mutação e as rodadas da Tarefa 4 de novo. Minor vai para o Linear.

---

### Task 10: Linear

Skill `linear-claudia`. Se o MCP pedir autenticação, peça ao usuário. Até lá, as issues ficam em `docs/superpowers/evidencias/linear-pendente.md`, versionado.

- [ ] **Step 1: Criar uma issue para cada linha "L" do inventário**, com descrição que se explica sozinha. Mais estas:
  - **Renomear os parâmetros das tools do MCP para nomes uniformes**, numa branch curta a partir de `dev`, depois do merge. O erro de parâmetro diz o nome certo e de onde tirar o valor. **Não é refazer o MCP**: a tipagem e a auditoria completas, perdidas no incidente, só voltam se uma segunda IA tropeçar.
  - Os "declined to judge" das revisões das Partes 1 e 2 (Tarefa 2), cada um com o destino que o usuário der.
  - **Urgent: "Nunca mais perder trabalho".** Leva o incidente de 2026-10-05 (12 horas perdidas), os 158 commits sem `dev`, o `dev` 55 commits à frente do `origin`, a evidência guardada em pasta ignorada pelo git e o CI que não roda front, clippy nem live. A solução é **depois** desta branch, com spec aprovada; a primeira proposta foi recusada por ser burocrática.

- [ ] **Step 2:** grave os ids na evidência.

---

### Task 11: Entrega ao usuário

- [ ] **Step 1: Resumo.** O que foi verificado (números das Tarefas 4 a 6), o que ficou "não verificado" e por quê, as issues e os caminhos dos prints.

- [ ] **Step 2: Merge de ensaio.** `git merge-tree --write-tree dev feat/browser-cdp-mcp`, que não mexe na árvore de trabalho nem nas branches. Em 2026-10-06 saiu limpo. Liste os conflitos, se houver.

- [ ] **Step 3: Perguntar** (nunca automático):
  - (a) o merge de `feat/browser-cdp-mcp` em `dev`;
  - (b) o push de `dev` para `origin`;
  - (c) refazer a base de `feat/curriculo-por-variante` sobre `dev`;
  - (d) o destino de `feat/exec-prototipo-playwright`, `feat/prototipo-playwright-mcp` e, depois do merge, de `backup/feat-browser-cdp-mcp-pre-rebase`.

## Depois desta branch: prioridade máxima

**Nunca mais perder trabalho.** Antes de qualquer feature nova, uma spec aprovada pelo usuário para:
1. código que apaga arquivos: travas que não dependem de alguém lembrar da regra;
2. evidência e ledgers que não vivem só numa pasta ignorada pelo git;
3. branches que crescem sem ir para `dev` e um `dev` que não vai para `origin`;
4. CI que rode o que o processo exige.

A primeira proposta foi recusada em 2026-10-06 por ser burocrática. A próxima precisa de menos papel e mais automação.
