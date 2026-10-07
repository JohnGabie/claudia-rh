# Laboratório do Sandbox dentro do claudia-rh — Plano de implementação

## ⛔ REGRAS DURAS PARA TODO AGENTE EXECUTOR (ler antes de qualquer comando)

Origem: em 2026-10-05 um teste passou `C:\` para uma função de apagar e destruiu o disco do usuário. Estas regras valem para todas as tarefas abaixo, sem exceção, e vencem qualquer passo que pareça dizer outra coisa.

1. **Nunca apagar nada fora de pasta temporária criada pelo próprio teste.**
2. **Nunca passar caminho real a função que apaga, nem em teste.** Nem a raiz do disco, nem o repositório, nem a pasta acima dele, nem `AppData`, nem a pasta do laboratório. A recusa se testa só na função de validação pura, com caminhos inventados (`Z:\...`).
3. **Teste que cria ou apaga pasta roda só no Windows Sandbox.** No PC do usuário rodam só testes puros (sem disco), compilação, `tsc`, build e leitura.
4. **Nenhum caminho absoluto de máquina** em código, teste, script, skill, plano, doc ou mensagem de commit. Use caminhos relativos à raiz do repositório, o que o `git rev-parse` devolve, `os.tmpdir()` ou variáveis padrão do Windows.
5. **Nunca o Chrome pessoal do usuário** (nem o perfil dele, nem cópia). Só o Chrome for Testing do laboratório ou o Chrome do Claudia-RH.
6. **Cópia só para destino NOVO e VAZIO.** Sem `/MIR`, `/MOV`, `/MOVE`, `/PURGE`, `Move-Item`, `Remove-Item` na cópia. **O original nunca é apagado** nem alterado.
7. **`git add` só com lista explícita de arquivos.** Nunca `git add -A`, `git add .` nem `git add <pasta>`.
8. **Antes de qualquer commit**, conferir que `sandbox-lab/shared/secrets`, `sandbox-lab/shared/tools` e qualquer `*.key` **nunca** entram no git: `git ls-files` sem nada deles e `git check-ignore -v` mostrando a regra (comandos exatos no passo "Conferência de segredo" de cada tarefa).
9. **Na dúvida, não rode.** Pare e relate ao orquestrador.

---

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** trazer o laboratório Windows Sandbox para `sandbox-lab/` dentro do repositório, achado só pela raiz do repositório, sem variável de caminho, com a chave e as ferramentas fora do git e a trava do `-Clean` nos dois lados.

**Architecture:** scripts e perfis do laboratório passam a ser versionados em `sandbox-lab/`; ferramentas (~592 MB) e a chave do MCP ficam em pastas ignoradas do **checkout principal**, conferidas por um manifesto SHA-256 versionado. A skill `claudia-teste-sandbox` acha o checkout principal por `git rev-parse --git-common-dir` (laboratório e pasta de trabalho `.sandbox-work/`) e usa o checkout atual como projeto em teste. Toda decisão de apagar é uma função pura (JS e PowerShell), testada sem disco; o que cria ou apaga pasta é testado só no Sandbox.

**Tech Stack:** Node ≥ 22.13 (`node:test`, sem dependências npm), PowerShell 7 no host e Windows PowerShell 5.1 no guest, Windows Sandbox (`wsb`), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-07-laboratorio-no-repo-design.md`. A seção "Decisões do usuário (2026-10-07)" (L1–L9) vale sobre as propostas P1–P8.

## Global Constraints

- L1: o laboratório fica em `sandbox-lab/` na raiz do repositório. Versionados: `scripts/`, `sandbox/` (sem gerados), os quatro `lab.config*.json`, `OBJECTIVES.md`, `sample-project/`, `shared/db/`, `shared/extensions/`, `shared/mcp.json`, `shared/test-page.html`, `shared/tools.sha256`, `.gitignore`, `README.md`.
- Ignorados: `sandbox-lab/shared/tools/`, `sandbox-lab/shared/secrets/`, `sandbox-lab/shared/out/`, `sandbox-lab/evidence/`, `sandbox-lab/sandbox/sandbox.generated.wsb`, `sandbox-lab/sandbox/guest.config.json`, `sandbox-lab/.mcp.json`, `.sandbox-work/`.
- L2: ferramentas copiadas uma vez; manifesto versionado `sandbox-lab/shared/tools.sha256`. Script de download: issue no Linear, fora deste plano.
- L3: **nenhuma** variável de ambiente de caminho. `SANDBOX_LAB_DIR`, `LAB_APP_PROJECT` e `LAB_WORK_DIR` deixam de ser lidas por qualquer script da skill e do laboratório.
- L4: laboratório e pasta de trabalho = os do checkout principal (`git rev-parse --path-format=absolute --git-common-dir`, só leitura); projeto em teste = `git rev-parse --show-toplevel` do checkout atual.
- L5: a chave `shared/secrets/mcp-auth.key` é copiada como está para a pasta ignorada. Seu conteúdo nunca é exibido nem aberto em editor; só o hash é comparado.
- L6: a cópia é feita por um agente, com as travas da regra 6 acima.
- L7: a skill `sandbox-lab` vira `.claude/skills/claudia-teste-sandbox/references/laboratorio.md`; `winapp-sandbox` e `winapp-ui-automation` vão para `.claude/skills/` com `WINAPP-LICENSE.txt` (MIT, Microsoft) ao lado.
- L8: `shared/out/` e `evidence/` antigos **não** são copiados.
- L9: a branch `feat/laboratorio-no-repo` entra na `feat/browser-cdp-mcp`. Integrar é decisão do usuário.
- Comentários de código e mensagens de commit em inglês; docs em pt-BR. Os nomes de funções da skill seguem a convenção já existente nela (português: `motivoRecusa`, `arquivosProibidos`, `achaCaminhoPessoal`); funções PowerShell novas usam Verbo-Substantivo em inglês.
- `node --test` sempre com a lista de arquivos explícita. Nunca glob nem pasta.
- **Teste puro que pode rodar no PC** = o arquivo de teste não importa `node:fs`, `node:child_process` nem `./util.mjs`, e toda função chamada recebe as dependências de disco injetadas. Antes de rodar no PC, o executor confere com o comando "Portão de teste puro" da tarefa. Teste marcado **Sandbox** só roda pela Tarefa 9.
- `comum.test.mjs` continua sem rodar fora do Sandbox.
- Sem push, sem merge, sem PR.

## Review Focus

1. **A chave ou as ferramentas entrarem no git** (um `git add` largo, um `.mcp.json` com a chave, um `*.key` em outra pasta). Esperado: impossível de commitar sem ver, e o CI falha. Teste: `proibidos.test.mjs` → "flags the lab secret, tools, output and evidence folders" e "flags key files anywhere" (Tarefa 0), o passo "Conferência de segredo" de cada tarefa e o job `repo-guard` do CI (Tarefa 3).
2. **O `-Clean` apagar fora do lugar**: caminho diferente do exato, `.sandbox-work` ou `deploy` como junction, junction **dentro** do `deploy`, pasta sem marcador, raiz vazia, relativa, UNC ou raiz de disco, `-WorkDir` antigo apontando para outra pasta, e o `robocopy /MIR` sem `-Clean`. Esperado: recusa com o motivo, nada apagado. Testes: `caminhos-lab.test.mjs` → "motivoRecusaLimparDeploy: …" (Tarefa 4), `travas.test.ps1` → "clean: …" (Tarefa 6) e `travas.sandbox.ps1` (Tarefa 6, rodado na Tarefa 9).
3. **Um teste que cria ou apaga pasta rodar no PC por engano** (agente que esquece a regra, `node --test` com a lista errada). Esperado: o próprio teste se recusa antes de criar qualquer coisa. Testes: `caminhos-lab.test.mjs` → "estaNoSandbox is true only for the Sandbox user" (Tarefa 4), a guarda em `tempDir()` de `util.mjs` (Tarefa 4) e o cabeçalho de `travas.sandbox.ps1` (sai com 3 fora do Sandbox).
4. **A skill achar a raiz errada a partir de uma worktree** (laboratório da worktree, sem ferramentas; barra `/` do git; `.git/modules/...` de submódulo; caminho UNC; checkout na raiz do disco). Esperado: laboratório e trabalho do checkout principal, projeto = worktree, ou erro claro. Testes: `caminhos-lab.test.mjs` → "raizPrincipalDe: …" e `onde.test.mjs` → "from a worktree: …" (Tarefa 4).
5. **A cópia parecer completa sem estar** (origem errada com zero arquivos, mesmo tamanho com conteúdo diferente, maiúsculas diferentes, arquivo a mais, `tools.sha256` confundido com `tools/`). Esperado: diferença listada e saída 1. Testes: `manifesto.test.mjs` → "compararListas: …" e "excluido: …" (Tarefa 1), `conferir-copia.mjs` sai com 1 quando a origem tem 0 arquivos (Tarefa 1), listagem da origem antes e depois (Tarefa 2).

## Mapa de arquivos

| Arquivo | Responsabilidade | Tarefa |
|---|---|---|
| `.gitignore` (raiz) | regras de ignore do laboratório e da pasta de trabalho | 0 |
| `.claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs` | PURO: `arquivosProibidos`, `achaCaminhoPessoal` | 0, 3 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs` | teste puro | 0, 3 |
| `.claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs` | só leitura: `git ls-files -z` + funções acima; CI e local | 0, 3 |
| `.claude/skills/claudia-teste-sandbox/scripts/manifesto.mjs` | PURO: manifesto, `compararListas`, `excluido` | 1 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs` | teste puro | 1 |
| `.claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs` | só leitura dos dois lados; `listarArquivos` | 1 |
| `.claude/skills/claudia-teste-sandbox/scripts/manifesto-ferramentas.mjs` | lê `tools/`, grava só um `.sha256` novo | 1 |
| `sandbox-lab/**` (37 arquivos) + `sandbox-lab/shared/tools.sha256` | cópia fiel do laboratório | 2 |
| `.github/workflows/ci.yml` | job `repo-guard` | 3 |
| `.claude/skills/claudia-teste-sandbox/scripts/caminhos-lab.mjs` | PURO: raiz principal, caminhos do lab, trava do deploy, `estaNoSandbox` | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs` | teste puro | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/comum.mjs` | `onde()`, `labDir()` sem variável; reexporta a trava | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs` | teste puro (git e disco injetados) | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/onde.mjs` | CLI: imprime `onde()` em JSON | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/util.mjs` | `tempDir()` recusa fora do Sandbox | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs` | sai o teste de `SANDBOX_LAB_DIR` | 4 |
| `.claude/skills/claudia-teste-sandbox/scripts/publicar.mjs` | sem `--work-dir`; trava exata; deps injetáveis | 5 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs` | **Sandbox** | 5 |
| `.claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs` | sem `--work-dir`/`LAB_WORK_DIR` | 5 |
| `.claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs` | roda testes Sandbox no guest via `guest-build.ps1` | 5 |
| `.claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs` | teste puro do comando | 5 |
| `sandbox-lab/scripts/lib/travas.ps1` | PURO: recusas do `-Clean` e do `shared\out`, ferramentas faltando | 6, 7 |
| `sandbox-lab/scripts/lib/disco.ps1` | leitura de reparse points, `Clear-LabOut`, `Test-LabTools` | 6, 7 |
| `sandbox-lab/scripts/test/travas.test.ps1` | teste puro PowerShell | 6, 7 |
| `sandbox-lab/scripts/test/travas.sandbox.ps1` | **Sandbox** | 6 |
| `sandbox-lab/scripts/app-stage.ps1` | trava antes de qualquer escrita no deploy; marcador | 6 |
| `sandbox-lab/scripts/test-up.ps1` | `-Project`, ferramentas, `Clear-LabOut` | 6, 7 |
| `sandbox-lab/scripts/mcp-sync.ps1` | sem `-Literal` | 7 |
| `sandbox-lab/scripts/validate-config.ps1` | ferramentas, `lib/`, skills da raiz | 7 |
| `sandbox-lab/lab.config.app*.json` | caminhos relativos | 7 |
| `sandbox-lab/sandbox/app/guest-build.ps1` | exclusões na cópia do projeto | 7 |
| `sandbox-lab/.gitignore` | + `.mcp.json` | 7 |
| `.claude/skills/winapp-*`, `.claude/skills/WINAPP-LICENSE.txt` | movidos de `sandbox-lab/.claude/skills/` | 8 |
| `.claude/skills/claudia-teste-sandbox/references/laboratorio.md` | ex-`sandbox-lab` SKILL.md | 8 |
| `sandbox-lab/README.md` | ex-`CLAUDE.md` do laboratório | 8 |
| `.claude/skills/claudia-teste-sandbox/SKILL.md`, `references/limites.md`, `CLAUDE.md` (raiz) | docs | 8 |

## Execução em frentes

Cada tarefa roda num agente próprio, numa worktree própria criada a partir de `feat/laboratorio-no-repo` atualizada. Frentes em paralelo só mexem em arquivos diferentes; o contrato entre elas são as assinaturas dos blocos **Interfaces**. O orquestrador integra cada tarefa na `feat/laboratorio-no-repo` antes de abrir a seguinte da mesma frente.

| Ordem | Frente | Tarefa | Modelo | Depende de |
|---|---|---|---|---|
| 1 | A | 0 — `.gitignore` e `arquivosProibidos` | Haiku | — |
| 1 | B | 1 — manifesto e conferência da cópia | Sonnet | — |
| 2 | — | 2 — a cópia | Sonnet | 0 e 1 integradas; **sim do usuário** |
| 3 | A | 3 — caminhos pessoais e CI | Sonnet | 2 |
| 3 | B | 4 — raiz principal, trava JS, guarda do Sandbox | Sonnet | 2 |
| 3 | B | 5 — `publicar`, `banco-guest`, `testes-no-guest` | Sonnet | 4 |
| 3 | C | 6 — trava PowerShell e `app-stage` | Sonnet | 2 |
| 3 | C | 7 — perfis, `-Project`, ferramentas, `mcp-sync`, `guest-build` | Sonnet | 6 |
| 3 | D | 8 — skills e docs | Haiku | 2 |
| 4 | — | 9 — verificação no Sandbox | Opus | 3–8 integradas; **usuário integra na `feat/browser-cdp-mcp`** |
| 5 | — | 10 — revisão de código | Opus | 9 |

Arquivos por frente na rodada 3 (disjuntos): A = `proibidos.mjs`, `test/proibidos.test.mjs`, `conferir-repo.mjs`, `.github/workflows/ci.yml`. B = `caminhos-lab.mjs`, `comum.mjs`, `onde.mjs`, `publicar.mjs`, `banco-guest.mjs`, `testes-no-guest.mjs` e seus testes, `test/util.mjs`, `test/comum.test.mjs`. C = tudo em `sandbox-lab/scripts/`, `sandbox-lab/lab.config*.json`, `sandbox-lab/sandbox/app/guest-build.ps1`, `sandbox-lab/.gitignore`. D = `sandbox-lab/.claude/**`, `sandbox-lab/CLAUDE.md` → `README.md`, `.claude/skills/winapp-*`, `.claude/skills/WINAPP-LICENSE.txt`, `.claude/skills/claudia-teste-sandbox/SKILL.md`, `references/*.md`, `CLAUDE.md` da raiz.

Todo agente, antes de começar: `git log -1 --oneline` na worktree, conferir que a base é a ponta atual de `feat/laboratorio-no-repo` e, se não for, `git rebase feat/laboratorio-no-repo` (conflito → `git rebase --abort` e relatar).

### Conferência de segredo (usada por toda tarefa antes de commitar)

Na raiz da worktree, depois do `git add` e antes do `git commit`:

```powershell
git ls-files -- 'sandbox-lab/shared/secrets' 'sandbox-lab/shared/tools' 'sandbox-lab/shared/out' 'sandbox-lab/evidence' '.sandbox-work' '*.key' '*.key.pub' 'sandbox-lab/.mcp.json'
git check-ignore -v sandbox-lab/shared/secrets/mcp-auth.key sandbox-lab/shared/tools/uv/uv.exe sandbox-lab/.mcp.json .sandbox-work/deploy/x
node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
```

Expected: o primeiro comando não imprime nada; o segundo imprime 4 linhas, cada uma com a regra do `.gitignore`; o terceiro sai com 0 e a última linha é `{"arquivos":N,"proibidos":0,...}`. Qualquer outra coisa: `git restore --staged -- <arquivo>` só do arquivo listado, parar e relatar.

---

### Task 0: `.gitignore` primeiro e a função pura `arquivosProibidos`

Modelo: **Haiku**. Roda no PC: só o teste puro.

**Files:**
- Modify: `.gitignore` (raiz; acrescentar no fim)
- Create: `.claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs`

**Interfaces:**
- Consumes: nada.
- Produces: `arquivosProibidos(arquivos: string[]): { arquivo: string, motivo: string }[]` em `proibidos.mjs` (caminhos como o `git ls-files` imprime). CLI `node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs` → última linha JSON `{arquivos, proibidos, caminhos}`, exit 0 sem achados, 1 com achados. A Tarefa 3 acrescenta `achaCaminhoPessoal` aos dois arquivos.

- [ ] **Step 1: Acrescentar as regras ao `.gitignore` da raiz**

Acrescentar no fim do arquivo, sem mudar nada acima:

```gitignore

# Windows Sandbox lab (sandbox-lab/): tools, the MCP key, guest output, evidence and generated files never go to git.
# sandbox-lab/.gitignore repeats these on purpose (two layers).
sandbox-lab/shared/tools/
sandbox-lab/shared/secrets/
sandbox-lab/shared/out/
sandbox-lab/evidence/
sandbox-lab/sandbox/sandbox.generated.wsb
sandbox-lab/sandbox/guest.config.json
sandbox-lab/.mcp.json
# Sandbox work folder (deploy + guest-rw). Always the one in the main checkout.
.sandbox-work/
```

- [ ] **Step 2: Conferir as regras sem criar arquivo nenhum**

```powershell
git check-ignore -v sandbox-lab/shared/tools/uv/uv.exe sandbox-lab/shared/secrets/mcp-auth.key sandbox-lab/shared/out/status.json sandbox-lab/evidence/O1.png sandbox-lab/sandbox/sandbox.generated.wsb sandbox-lab/sandbox/guest.config.json sandbox-lab/.mcp.json .sandbox-work/deploy/x; "exit=$LASTEXITCODE"
git check-ignore sandbox-lab/shared/tools.sha256 sandbox-lab/shared/mcp.json sandbox-lab/shared/db/backup.sqlite sandbox-lab/scripts/test-up.ps1; "exit=$LASTEXITCODE"
```

Expected: o primeiro imprime 8 linhas `.gitignore:<n>:<regra>\t<caminho>` e `exit=0`; o segundo não imprime caminho e mostra `exit=1` (nenhum desses é ignorado).

- [ ] **Step 3: Escrever o teste que falha**

`.claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs`:

```js
// Pure tests: lists of strings only, no disk, no git.
import test from "node:test";
import assert from "node:assert/strict";
import { arquivosProibidos } from "../proibidos.mjs";

const nomes = (lista) => arquivosProibidos(lista).map((a) => a.arquivo);

test("flags the lab secret, tools, output and evidence folders", () => {
  const lista = [
    "sandbox-lab/shared/secrets/mcp-auth.key",
    "sandbox-lab/shared/secrets/outro.txt",
    "sandbox-lab/shared/tools/uv/uv.exe",
    "sandbox-lab/shared/out/status.json",
    "sandbox-lab/evidence/O1-x.png",
    ".sandbox-work/deploy/app/x.exe",
  ];
  assert.deepEqual(nomes(lista), lista);
});

test("flags key files anywhere, including .key.pub", () => {
  assert.deepEqual(nomes(["src-tauri/assinatura.key", "x/y/z.key.pub", "docs/chave.KEY"]),
    ["src-tauri/assinatura.key", "x/y/z.key.pub", "docs/chave.KEY"]);
});

test("flags generated lab files and the lab .mcp.json", () => {
  const lista = [
    "sandbox-lab/.mcp.json",
    "sandbox-lab/sandbox/sandbox.generated.wsb",
    "sandbox-lab/sandbox/guest.config.json",
  ];
  assert.deepEqual(nomes(lista), lista);
});

test("accepts the files the lab versions (tools.sha256 is not tools/)", () => {
  assert.deepEqual(arquivosProibidos([
    "sandbox-lab/scripts/test-up.ps1",
    "sandbox-lab/shared/mcp.json",
    "sandbox-lab/shared/tools.sha256",
    "sandbox-lab/shared/db/backup.sqlite",
    "sandbox-lab/sandbox/sandbox.template.wsb",
    ".claude/skills/claudia-teste-sandbox/SKILL.md",
    "src/main.tsx",
  ]), []);
});

test("is case-insensitive and accepts backslashes and ./", () => {
  assert.deepEqual(nomes(["Sandbox-Lab\\Shared\\Secrets\\x.txt", "./sandbox-lab/shared/TOOLS/a"]),
    ["Sandbox-Lab\\Shared\\Secrets\\x.txt", "./sandbox-lab/shared/TOOLS/a"]);
});

test("every finding has a reason", () => {
  for (const a of arquivosProibidos(["sandbox-lab/shared/secrets/k", "a.key"])) assert.ok(a.motivo.length > 0);
});

test("ignores empty and non-string entries", () => {
  assert.deepEqual(arquivosProibidos(["", "   ", null, undefined, 3]), []);
  assert.deepEqual(arquivosProibidos(undefined), []);
});
```

- [ ] **Step 4: Rodar e ver falhar**

Portão de teste puro (antes de rodar qualquer teste no PC):

```powershell
Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs -Pattern 'node:fs|node:child_process|util\.mjs|mkdir|rmSync|writeFile'
```

Expected: nenhuma linha. Depois:

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs`
Expected: FAIL com `Cannot find module` / `ERR_MODULE_NOT_FOUND` para `proibidos.mjs`.

- [ ] **Step 5: Implementar**

`.claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs`:

```js
// PURE (no disk, no git). What must never be versioned, and (Task 3) machine paths in versioned text.

const PASTAS_PROIBIDAS = [
  ["sandbox-lab/shared/secrets/", "lab secret (MCP key)"],
  ["sandbox-lab/shared/tools/", "heavy lab tools (ignored; checked by shared/tools.sha256)"],
  ["sandbox-lab/shared/out/", "generated guest output"],
  ["sandbox-lab/evidence/", "generated evidence"],
  [".sandbox-work/", "sandbox work folder"],
];

const ARQUIVOS_PROIBIDOS = [
  ["sandbox-lab/.mcp.json", "lab .mcp.json is generated by mcp-sync.ps1"],
  ["sandbox-lab/sandbox/sandbox.generated.wsb", "generated by test-up.ps1 (holds host paths)"],
  ["sandbox-lab/sandbox/guest.config.json", "generated by test-up.ps1"],
];

/**
 * Versioned files that must never be in git, each with the reason.
 * `arquivos` are paths as `git ls-files` prints them; comparison is case-insensitive (Windows).
 */
export function arquivosProibidos(arquivos) {
  const achados = [];
  for (const original of arquivos ?? []) {
    if (typeof original !== "string" || original.trim() === "") continue;
    const a = original.trim().replace(/\\/g, "/").replace(/^\.\//, "").toLowerCase();
    let motivo = null;
    for (const [prefixo, m] of PASTAS_PROIBIDAS) if (a.startsWith(prefixo)) { motivo = m; break; }
    if (!motivo) for (const [exato, m] of ARQUIVOS_PROIBIDOS) if (a === exato) { motivo = m; break; }
    if (!motivo && (a.endsWith(".key") || a.endsWith(".key.pub"))) motivo = "key file";
    if (motivo) achados.push({ arquivo: original, motivo });
  }
  return achados;
}
```

- [ ] **Step 6: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs; "exit=$LASTEXITCODE"`
Expected: `# pass 7`, `# fail 0`, `exit=0`.

- [ ] **Step 7: Prova de mutação**

Apagar a linha `["sandbox-lab/shared/secrets/", "lab secret (MCP key)"],` de `proibidos.mjs`, rodar o mesmo comando, colar no relato a linha `not ok ... flags the lab secret, tools, output and evidence folders`, e restaurar com `git checkout -- .claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs` se o arquivo já estiver commitado, ou desfazendo a edição à mão. Rodar de novo: `# fail 0`.

- [ ] **Step 8: Escrever `conferir-repo.mjs` (só leitura)**

```js
// Read-only repo guard for CI and local use. Lists versioned files with `git ls-files -z` and fails when a file that
// must never be versioned is there. Never writes, moves or deletes anything.
// Usage (from any folder of the checkout): node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
import path from "node:path";
import { spawnSync } from "node:child_process";
import { arquivosProibidos } from "./proibidos.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const r = spawnSync("git", ["-C", RAIZ, "ls-files", "-z"], { encoding: "utf8" });
if (r.status !== 0) {
  console.error(`git ls-files failed: ${(r.stderr || "").trim()}`);
  process.exit(1);
}
const arquivos = r.stdout.split("\0").filter(Boolean);
const proibidos = arquivosProibidos(arquivos);
for (const p of proibidos) console.error(`FORBIDDEN ${p.arquivo}: ${p.motivo}`);
console.log(JSON.stringify({ arquivos: arquivos.length, proibidos: proibidos.length, caminhos: 0 }));
process.exit(proibidos.length ? 1 : 0);
```

Run: `node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs; "exit=$LASTEXITCODE"`
Expected: `{"arquivos":<N>,"proibidos":0,"caminhos":0}` e `exit=0`. (Ele só lê o índice do git.)

- [ ] **Step 9: Conferência de segredo e commit**

```powershell
git add -- .gitignore .claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
```

Rodar a "Conferência de segredo" (seção acima). Depois:

```powershell
git commit -m "chore(sandbox-lab): ignore lab tools, key and outputs before the copy; add forbidden-files guard" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 1: manifesto e conferência da cópia

Modelo: **Sonnet**. Roda no PC: só o teste puro. Os dois scripts de linha de comando só são executados na Tarefa 2.

**Files:**
- Create: `.claude/skills/claudia-teste-sandbox/scripts/manifesto.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/manifesto-ferramentas.mjs`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `type Entrada = { caminho: string, tamanho: number, sha256?: string }` (caminho relativo, `/`).
  - `escreverManifesto(entradas: Entrada[]): string` — linhas `<sha256> <tamanho> <caminho>`, ordenadas por caminho, terminando em `\n`.
  - `lerManifesto(texto: string): Entrada[]` — lança com o número da linha malformada.
  - `compararListas(esperado: Entrada[], achado: Entrada[]): { faltando: string[], diferentes: {caminho, motivo}[], sobrando: string[] }`.
  - `excluido(caminho: string, excluir: string[]): boolean`.
  - `listarArquivos(raiz: string, excluir?: string[]): Entrada[]` exportada por `conferir-copia.mjs` (lê o disco, nunca escreve; recusa link).
  - CLI `conferir-copia.mjs --copia <dir> (--origem <dir> | --manifesto <arq>) [--excluir a,b/c]` → JSON `{origem, copia, faltando, diferentes, sobrando}` (contagens); exit 1 se houver diferença ou se a origem tiver 0 arquivos.
  - CLI `manifesto-ferramentas.mjs --raiz <dir> --saida <arq.sha256>` → grava só `--saida`, que não pode existir.

- [ ] **Step 1: Escrever o teste que falha**

`.claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs`:

```js
// Pure tests: in-memory lists only.
import test from "node:test";
import assert from "node:assert/strict";
import { lerManifesto, escreverManifesto, compararListas, excluido } from "../manifesto.mjs";

const H1 = "a".repeat(64);
const H2 = "b".repeat(64);

test("manifest round-trip sorts by path, lowercases the hash and uses forward slashes", () => {
  const texto = escreverManifesto([
    { caminho: "uv\\uv.exe", tamanho: 10, sha256: H2 },
    { caminho: "chrome/chrome-win64/chrome.exe", tamanho: 5, sha256: H1.toUpperCase() },
  ]);
  assert.equal(texto, `${H1} 5 chrome/chrome-win64/chrome.exe\n${H2} 10 uv/uv.exe\n`);
  assert.deepEqual(lerManifesto(texto), [
    { sha256: H1, tamanho: 5, caminho: "chrome/chrome-win64/chrome.exe" },
    { sha256: H2, tamanho: 10, caminho: "uv/uv.exe" },
  ]);
});

test("paths with spaces survive the round-trip", () => {
  const e = [{ caminho: "python/Lib/site packages/x y.py", tamanho: 0, sha256: H1 }];
  assert.deepEqual(lerManifesto(escreverManifesto(e)), [{ sha256: H1, tamanho: 0, caminho: "python/Lib/site packages/x y.py" }]);
});

test("manifest with a malformed line is refused with its line number", () => {
  assert.throws(() => lerManifesto(`${H1} 5 a.txt\nlixo\n`), /line 2/);
  assert.throws(() => lerManifesto(`${H1} -5 a.txt\n`), /line 1/);
  assert.throws(() => lerManifesto(`${"g".repeat(64)} 5 a.txt\n`), /line 1/);
});

test("blank lines and CRLF are accepted", () => {
  assert.equal(lerManifesto(`\r\n${H1} 1 a.txt\r\n\r\n`).length, 1);
});

test("compararListas: identical lists give no differences", () => {
  const l = [{ caminho: "a.txt", tamanho: 1, sha256: H1 }];
  assert.deepEqual(compararListas(l, l), { faltando: [], diferentes: [], sobrando: [] });
});

test("compararListas: missing, extra, size and hash differences", () => {
  const esperado = [
    { caminho: "a.txt", tamanho: 1, sha256: H1 },
    { caminho: "b.txt", tamanho: 2, sha256: H1 },
    { caminho: "c.txt", tamanho: 3, sha256: H1 },
  ];
  const achado = [
    { caminho: "a.txt", tamanho: 1, sha256: H2 },
    { caminho: "b.txt", tamanho: 9, sha256: H1 },
    { caminho: "d.txt", tamanho: 4, sha256: H1 },
  ];
  assert.deepEqual(compararListas(esperado, achado), {
    faltando: ["c.txt"],
    diferentes: [{ caminho: "a.txt", motivo: "sha256 differs" }, { caminho: "b.txt", motivo: "size 9 != 2" }],
    sobrando: ["d.txt"],
  });
});

test("compararListas: case and separators do not matter on Windows", () => {
  const r = compararListas([{ caminho: "Chrome/Chrome.exe", tamanho: 1, sha256: H1 }], [{ caminho: "chrome\\chrome.exe", tamanho: 1, sha256: H1.toUpperCase() }]);
  assert.deepEqual(r, { faltando: [], diferentes: [], sobrando: [] });
});

test("compararListas: without sha256 on one side, size alone decides", () => {
  assert.deepEqual(compararListas([{ caminho: "a", tamanho: 1, sha256: H1 }], [{ caminho: "a", tamanho: 1 }]).diferentes, []);
  assert.equal(compararListas([{ caminho: "a", tamanho: 1, sha256: H1 }], [{ caminho: "a", tamanho: 2 }]).diferentes.length, 1);
});

test("excluido: folder prefix and exact file, never a sibling with the same start", () => {
  const ex = ["shared/tools", "sandbox/guest.config.json"];
  assert.equal(excluido("shared/tools/uv/uv.exe", ex), true);
  assert.equal(excluido("shared/tools", ex), true);
  assert.equal(excluido("shared/tools.sha256", ex), false);
  assert.equal(excluido("Sandbox\\Guest.Config.json", ex), true);
  assert.equal(excluido("sandbox/guest.config.json.bak", ex), false);
  assert.equal(excluido("qualquer", []), false);
});
```

- [ ] **Step 2: Rodar e ver falhar**

Portão de teste puro: `Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs -Pattern 'node:fs|node:child_process|util\.mjs|mkdir|rmSync|writeFile'` → nenhuma linha.

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs`
Expected: FAIL com `ERR_MODULE_NOT_FOUND` para `manifesto.mjs`.

- [ ] **Step 3: Implementar `manifesto.mjs`**

```js
// PURE (no disk). Tools manifest (sandbox-lab/shared/tools.sha256) and list comparison for the lab copy.
// Line format: "<sha256 hex> <size in bytes> <path relative to the listed root, with />".

const LINHA = /^([0-9a-f]{64}) (\d+) (.+)$/;
const chave = (c) => String(c).replace(/\\/g, "/").toLowerCase();

export function escreverManifesto(entradas) {
  const linhas = [...entradas]
    .map((e) => ({ ...e, caminho: e.caminho.replace(/\\/g, "/") }))
    .sort((a, b) => (a.caminho < b.caminho ? -1 : a.caminho > b.caminho ? 1 : 0))
    .map((e) => `${e.sha256.toLowerCase()} ${e.tamanho} ${e.caminho}`);
  return linhas.join("\n") + "\n";
}

export function lerManifesto(texto) {
  const entradas = [];
  String(texto ?? "").split(/\r?\n/).forEach((l, i) => {
    if (l.trim() === "") return;
    const m = l.match(LINHA);
    if (!m) throw new Error(`manifest line ${i + 1} is malformed: ${l}`);
    entradas.push({ sha256: m[1], tamanho: Number(m[2]), caminho: m[3] });
  });
  return entradas;
}

/** Expected vs found, by path (case-insensitive, / or \), then size, then sha256 when both sides have it. */
export function compararListas(esperado, achado) {
  const mapa = new Map(achado.map((e) => [chave(e.caminho), e]));
  const vistos = new Set();
  const faltando = [];
  const diferentes = [];
  for (const e of esperado) {
    const k = chave(e.caminho);
    vistos.add(k);
    const a = mapa.get(k);
    if (!a) { faltando.push(e.caminho); continue; }
    if (a.tamanho !== e.tamanho) diferentes.push({ caminho: e.caminho, motivo: `size ${a.tamanho} != ${e.tamanho}` });
    else if (e.sha256 && a.sha256 && e.sha256.toLowerCase() !== a.sha256.toLowerCase()) diferentes.push({ caminho: e.caminho, motivo: "sha256 differs" });
  }
  const sobrando = achado.filter((a) => !vistos.has(chave(a.caminho))).map((a) => a.caminho);
  return { faltando, diferentes, sobrando };
}

/** True when `caminho` is one of `excluir` or inside one of them (folder prefix followed by "/"). */
export function excluido(caminho, excluir) {
  const k = chave(caminho);
  return excluir.some((e) => { const x = chave(e).replace(/\/+$/, ""); return k === x || k.startsWith(x + "/"); });
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs; "exit=$LASTEXITCODE"`
Expected: `# pass 9`, `# fail 0`, `exit=0`.

- [ ] **Step 5: Prova de mutação**

Trocar em `compararListas` a linha `if (a.tamanho !== e.tamanho)` por `if (false)`. Rodar; colar a linha `not ok ... compararListas: missing, extra, size and hash differences`. Restaurar e rodar de novo: `# fail 0`.

- [ ] **Step 6: Escrever `conferir-copia.mjs` (só leitura)**

```js
// Read-only check of the lab copy. Both sides are only read; nothing is written, moved or deleted.
// Prints counts as JSON and the differing paths on stderr. Never prints file contents or hashes (the MCP key is hashed too).
// Usage: node conferir-copia.mjs --copia <dir> (--origem <dir> | --manifesto <file.sha256>) [--excluir a,b/c]
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { lerManifesto, compararListas, excluido } from "./manifesto.mjs";

/** Every regular file under `raiz` (relative path with /, size, sha256). Refuses links instead of following them. */
export function listarArquivos(raiz, excluir = []) {
  const saida = [];
  for (const e of fs.readdirSync(raiz, { recursive: true, withFileTypes: true })) {
    const abs = path.join(e.parentPath ?? e.path, e.name);
    const rel = path.relative(raiz, abs).split(path.sep).join("/");
    if (excluido(rel, excluir)) continue;
    if (e.isSymbolicLink()) throw new Error(`link found, refusing to follow it: ${rel}`);
    if (!e.isFile()) continue;
    const buf = fs.readFileSync(abs);
    saida.push({ caminho: rel, tamanho: buf.length, sha256: crypto.createHash("sha256").update(buf).digest("hex") });
  }
  return saida;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: { origem: { type: "string" }, copia: { type: "string" }, manifesto: { type: "string" }, excluir: { type: "string", default: "" } } });
    if (!values.copia || Boolean(values.origem) === Boolean(values.manifesto)) {
      console.error("Usage: node conferir-copia.mjs --copia <dir> (--origem <dir> | --manifesto <file>) [--excluir a,b/c]");
      process.exit(2);
    }
    const excluir = values.excluir.split(",").map((s) => s.trim()).filter(Boolean);
    const esperado = values.manifesto ? lerManifesto(fs.readFileSync(values.manifesto, "utf8")) : listarArquivos(values.origem, excluir);
    const achado = listarArquivos(values.copia, excluir);
    const r = compararListas(esperado, achado);
    for (const f of r.faltando) console.error(`MISSING ${f}`);
    for (const d of r.diferentes) console.error(`DIFFERS ${d.caminho}: ${d.motivo}`);
    for (const s of r.sobrando) console.error(`EXTRA ${s}`);
    if (esperado.length === 0) console.error("the expected side has 0 files: wrong origin?");
    console.log(JSON.stringify({ origem: esperado.length, copia: achado.length, faltando: r.faltando.length, diferentes: r.diferentes.length, sobrando: r.sobrando.length }));
    process.exit(esperado.length === 0 || r.faltando.length || r.diferentes.length || r.sobrando.length ? 1 : 0);
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
```

- [ ] **Step 7: Escrever `manifesto-ferramentas.mjs` (grava só um arquivo novo)**

```js
// Writes the tools manifest for sandbox-lab/shared/tools. Reads `--raiz`; writes only `--saida`, which must not exist
// yet and must end in .sha256. Never overwrites, moves or deletes anything.
// Usage: node manifesto-ferramentas.mjs --raiz <tools folder> --saida <new file.sha256>
import fs from "node:fs";
import { parseArgs } from "node:util";
import { escreverManifesto } from "./manifesto.mjs";
import { listarArquivos } from "./conferir-copia.mjs";

try {
  const { values } = parseArgs({ options: { raiz: { type: "string" }, saida: { type: "string" } } });
  if (!values.raiz || !values.saida || !values.saida.endsWith(".sha256")) {
    console.error("Usage: node manifesto-ferramentas.mjs --raiz <folder> --saida <new file.sha256>");
    process.exit(2);
  }
  const entradas = listarArquivos(values.raiz);
  if (entradas.length === 0) throw new Error(`no files under ${values.raiz}`);
  fs.writeFileSync(values.saida, escreverManifesto(entradas), { flag: "wx" }); // "wx": fails if the file exists
  console.log(JSON.stringify({ arquivos: entradas.length, bytes: entradas.reduce((s, e) => s + e.tamanho, 0) }));
} catch (e) {
  console.error(e.message);
  process.exit(1);
}
```

- [ ] **Step 8: Conferir só a sintaxe dos dois scripts (sem executá-los)**

Run: `node --check .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs; node --check .claude/skills/claudia-teste-sandbox/scripts/manifesto-ferramentas.mjs; "exit=$LASTEXITCODE"`
Expected: nenhuma saída, `exit=0`.

- [ ] **Step 9: Conferência de segredo e commit**

```powershell
git add -- .claude/skills/claudia-teste-sandbox/scripts/manifesto.mjs .claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs .claude/skills/claudia-teste-sandbox/scripts/manifesto-ferramentas.mjs
```

Conferência de segredo. Depois:

```powershell
git commit -m "feat(sandbox-skill): tools manifest and read-only copy check for the lab migration" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: a cópia do laboratório

Modelo: **Sonnet**. Pré-requisitos: Tarefas 0 e 1 integradas na `feat/laboratorio-no-repo`; **o orquestrador mostra ao usuário os comandos dos Steps 4 a 7 e só segue com o sim dele**.

Dois destinos: a parte **versionada** vai para `sandbox-lab/` da worktree do agente (e é commitada); as ferramentas e a chave vão para `sandbox-lab/shared/tools/` e `sandbox-lab/shared/secrets/` do **checkout principal** (pastas ignoradas, nunca commitadas). A origem é a pasta atual do laboratório (irmã do workspace), só lida.

**Files:**
- Create: os 37 arquivos versionados de `sandbox-lab/` listados no Step 9, mais `sandbox-lab/shared/tools.sha256`.
- Fora do git (checkout principal): `sandbox-lab/shared/tools/**`, `sandbox-lab/shared/secrets/mcp-auth.key`, `.sandbox-work/migracao/*.txt`.
- Local, não versionado: três linhas a mais em `.git/info/exclude` (vale para todas as worktrees).

**Interfaces:**
- Consumes: `conferir-copia.mjs`, `manifesto-ferramentas.mjs` (Tarefa 1); regras do `.gitignore` (Tarefa 0).
- Produces: `sandbox-lab/` versionado e cópia fiel (as tarefas 6, 7 e 8 editam esses arquivos); `sandbox-lab/shared/tools.sha256` (formato da Tarefa 1).

- [ ] **Step 1: Localizar as três pastas, sem escrever nada**

Na raiz da worktree do agente, em PowerShell. Tudo sai de `git rev-parse` e de um caminho relativo; nada é escrito à mão:

```powershell
$wt = (git rev-parse --show-toplevel) -replace '/', '\'
$main = Split-Path ((git rev-parse --path-format=absolute --git-common-dir) -replace '/', '\') -Parent
$origem = (Resolve-Path -LiteralPath (Join-Path $main '..\..\win-sandbox-lab')).Path
"worktree: $wt"; "principal: $main"; "origem: $origem"
foreach ($f in 'scripts\app-stage.ps1', 'lab.config.json', 'shared\tools', 'shared\secrets\mcp-auth.key') { "{0} -> {1}" -f $f, (Test-Path -LiteralPath (Join-Path $origem $f)) }
"destino versionado existe: $(Test-Path -LiteralPath (Join-Path $wt 'sandbox-lab'))"
"destino tools existe: $(Test-Path -LiteralPath (Join-Path $main 'sandbox-lab\shared\tools'))"
"destino secrets existe: $(Test-Path -LiteralPath (Join-Path $main 'sandbox-lab\shared\secrets'))"
"origem dentro do repo: $($origem.StartsWith($main + '\', 'OrdinalIgnoreCase'))"; "repo dentro da origem: $($main.StartsWith($origem + '\', 'OrdinalIgnoreCase'))"
"livre (GB): {0:N1}" -f ((Get-PSDrive -Name $main.Substring(0, 1)).Free / 1GB)
```

Expected: os quatro itens da origem `True`; os três destinos `False`; as duas linhas "dentro" `False`; espaço livre maior que 2 GB. **Qualquer diferença: parar e relatar.** O agente relata as três pastas ao orquestrador **sem** colá-las em arquivo versionado nem em commit.

As variáveis do PowerShell não persistem entre chamadas: cada Step abaixo começa repetindo as três primeiras linhas deste Step e, a partir do Step 4, também `$mig = Join-Path $main '.sandbox-work\migracao'` (só a atribuição; o `New-Item` do Step 3 roda uma vez).

O próprio texto do plano também não pode ter caminho de máquina: os testes da Tarefa 3 montam os exemplos com `join` e a regex usa `[p]`, para o `conferir-repo.mjs` não acusar os próprios arquivos da skill.

- [ ] **Step 2: Rede de segurança local no `.git/info/exclude`**

Só acrescenta linhas (não apaga nem reescreve nada):

```powershell
$exclude = Join-Path ((git rev-parse --path-format=absolute --git-common-dir) -replace '/', '\') 'info\exclude'
Add-Content -LiteralPath $exclude -Value @('', '# claudia-rh sandbox lab: local safety net (the versioned rules are in .gitignore)', 'sandbox-lab/shared/tools/', 'sandbox-lab/shared/secrets/', '.sandbox-work/')
git -C $main check-ignore -v sandbox-lab/shared/secrets/mcp-auth.key sandbox-lab/shared/tools/uv/uv.exe .sandbox-work/migracao/x; "exit=$LASTEXITCODE"
```

Expected: 3 linhas com a regra (no checkout principal, que pode estar em outra branch, a regra vem de `.git/info/exclude` ou do `*.key`) e `exit=0`. Se alguma linha faltar: parar.

- [ ] **Step 3: Listagem da origem antes da cópia**

```powershell
$mig = Join-Path $main '.sandbox-work\migracao'
New-Item -ItemType Directory -Path $mig | Out-Null   # fails if it already exists: stop and report
Get-ChildItem -LiteralPath $origem -Recurse -Force -File | ForEach-Object { "{0}`t{1}`t{2:o}" -f $_.FullName.Substring($origem.Length), $_.Length, $_.LastWriteTimeUtc } | Sort-Object | Set-Content -LiteralPath (Join-Path $mig 'origem-antes.txt')
(Get-Content -LiteralPath (Join-Path $mig 'origem-antes.txt')).Count
```

Expected: um número perto de 2.650 (2.599 em `tools`, 1 em `secrets`, o resto pequeno).

- [ ] **Step 4: Copiar as ferramentas (destino novo; sem `/MIR`, `/MOV`, `/PURGE`)**

```powershell
robocopy (Join-Path $origem 'shared\tools') (Join-Path $main 'sandbox-lab\shared\tools') /E /COPY:DAT /DCOPY:T /XJ /R:1 /W:1 /NP /NFL /NDL
if ($LASTEXITCODE -ge 8) { throw "robocopy failed ($LASTEXITCODE)" } else { "robocopy ok ($LASTEXITCODE)" }
```

Expected: `robocopy ok (1)`.

- [ ] **Step 5: Copiar a chave (destino novo; conteúdo nunca exibido)**

```powershell
robocopy (Join-Path $origem 'shared\secrets') (Join-Path $main 'sandbox-lab\shared\secrets') /E /COPY:DAT /DCOPY:T /XJ /R:1 /W:1 /NP /NFL /NDL
if ($LASTEXITCODE -ge 8) { throw "robocopy failed ($LASTEXITCODE)" } else { "robocopy ok ($LASTEXITCODE)" }
git -C $main status --porcelain -- sandbox-lab
```

Expected: `robocopy ok (1)` e o `git status` **sem nenhuma linha** (as duas pastas estão ignoradas). Se aparecer qualquer linha: parar e relatar, sem `git add`.

- [ ] **Step 6: Copiar a parte versionada para a worktree (destino novo)**

Fica de fora o que é gerado ou pesado (L8): `shared\tools`, `shared\secrets`, `shared\out`, `evidence`, os dois gerados de `sandbox\` e o `.mcp.json`.

```powershell
robocopy $origem (Join-Path $wt 'sandbox-lab') /E /COPY:DAT /DCOPY:T /XJ /R:1 /W:1 /NP /NFL /NDL `
  /XD (Join-Path $origem 'shared\tools') (Join-Path $origem 'shared\secrets') (Join-Path $origem 'shared\out') (Join-Path $origem 'evidence') `
  /XF (Join-Path $origem 'sandbox\sandbox.generated.wsb') (Join-Path $origem 'sandbox\guest.config.json') (Join-Path $origem '.mcp.json')
if ($LASTEXITCODE -ge 8) { throw "robocopy failed ($LASTEXITCODE)" } else { "robocopy ok ($LASTEXITCODE)" }
```

Expected: `robocopy ok (1)`.

- [ ] **Step 7: Conferir arquivo por arquivo, lendo os dois lados**

```powershell
node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --origem (Join-Path $origem 'shared\tools') --copia (Join-Path $main 'sandbox-lab\shared\tools'); "exit=$LASTEXITCODE"
node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --origem (Join-Path $origem 'shared\secrets') --copia (Join-Path $main 'sandbox-lab\shared\secrets'); "exit=$LASTEXITCODE"
node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --origem $origem --copia (Join-Path $wt 'sandbox-lab') --excluir "shared/tools,shared/secrets,shared/out,evidence,sandbox/sandbox.generated.wsb,sandbox/guest.config.json,.mcp.json"; "exit=$LASTEXITCODE"
```

Expected, nesta ordem: `{"origem":2599,"copia":2599,"faltando":0,"diferentes":0,"sobrando":0}`, `{"origem":1,"copia":1,...0,0,0}`, `{"origem":37,"copia":37,...0,0,0}`, cada um com `exit=0`. Contagem diferente da esperada (2599 e 37 vêm da listagem de 2026-10-07): relatar o número visto antes de seguir.

- [ ] **Step 8: Manifesto versionado e conferência contra a origem**

```powershell
node .claude/skills/claudia-teste-sandbox/scripts/manifesto-ferramentas.mjs --raiz (Join-Path $main 'sandbox-lab\shared\tools') --saida (Join-Path $wt 'sandbox-lab\shared\tools.sha256'); "exit=$LASTEXITCODE"
node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --manifesto (Join-Path $wt 'sandbox-lab\shared\tools.sha256') --copia (Join-Path $origem 'shared\tools'); "exit=$LASTEXITCODE"
Get-ChildItem -LiteralPath $origem -Recurse -Force -File | ForEach-Object { "{0}`t{1}`t{2:o}" -f $_.FullName.Substring($origem.Length), $_.Length, $_.LastWriteTimeUtc } | Sort-Object | Set-Content -LiteralPath (Join-Path $mig 'origem-depois.txt')
Compare-Object (Get-Content -LiteralPath (Join-Path $mig 'origem-antes.txt')) (Get-Content -LiteralPath (Join-Path $mig 'origem-depois.txt')); "comparacao terminou"
```

Expected: `{"arquivos":2599,"bytes":<~620 milhões>}` e `exit=0`; depois `{"origem":2599,"copia":2599,...0,0,0}` e `exit=0`; o `Compare-Object` não imprime nada antes de `comparacao terminou` (origem intacta).

- [ ] **Step 9: `git add` com a lista explícita, conferência de segredo e commit**

```powershell
git add -- `
  sandbox-lab/.gitignore sandbox-lab/CLAUDE.md sandbox-lab/OBJECTIVES.md `
  sandbox-lab/lab.config.json sandbox-lab/lab.config.app.json sandbox-lab/lab.config.app-winapp.json sandbox-lab/lab.config.app-build.json `
  sandbox-lab/.claude/skills/WINAPP-LICENSE.txt sandbox-lab/.claude/skills/sandbox-lab/SKILL.md `
  sandbox-lab/.claude/skills/winapp-sandbox/SKILL.md sandbox-lab/.claude/skills/winapp-ui-automation/SKILL.md `
  sandbox-lab/.claude/skills/winapp-ui-automation/references/ui-json-envelope.md `
  sandbox-lab/sample-project/hello.txt sandbox-lab/sample-project/README.md `
  sandbox-lab/sandbox/bootstrap-mcp.ps1 sandbox-lab/sandbox/bootstrap.ps1 sandbox-lab/sandbox/sandbox.template.wsb sandbox-lab/sandbox/serve-lab.ps1 `
  sandbox-lab/sandbox/app/guest-build.ps1 sandbox-lab/sandbox/app/guest-chrome-probe.ps1 sandbox-lab/sandbox/app/guest-click.ps1 `
  sandbox-lab/sandbox/app/guest-firewall.ps1 sandbox-lab/sandbox/app/guest-observe.ps1 sandbox-lab/sandbox/app/guest-run.ps1 `
  sandbox-lab/scripts/app-stage.ps1 sandbox-lab/scripts/mcp-sync.ps1 sandbox-lab/scripts/test-down.ps1 sandbox-lab/scripts/test-up.ps1 sandbox-lab/scripts/validate-config.ps1 `
  sandbox-lab/shared/mcp.json sandbox-lab/shared/test-page.html `
  sandbox-lab/shared/db/backup.sha256 sandbox-lab/shared/db/backup.sqlite sandbox-lab/shared/db/verify.expected.txt sandbox-lab/shared/db/verify.sql `
  sandbox-lab/shared/extensions/lab-ext/content.js sandbox-lab/shared/extensions/lab-ext/manifest.json `
  sandbox-lab/shared/tools.sha256
git diff --cached --name-only | Measure-Object | ForEach-Object Count
git status --porcelain --untracked-files=all -- sandbox-lab
```

Expected: `38`; o `git status` mostra só as 38 linhas `A `. Qualquer `??` restante: relatar (não adicionar). Rodar a Conferência de segredo. Depois:

```powershell
git commit -m "chore(sandbox-lab): verbatim copy of the Windows Sandbox lab (versioned part) and tools manifest" -m "Tools and the MCP key were copied to the ignored folders of the main checkout only. Source left untouched; every file checked by size and SHA-256." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: caminhos pessoais fora dos versionados e o job do CI

Modelo: **Sonnet**. Roda no PC: teste puro e `conferir-repo.mjs` (só leitura).

**Files:**
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs` (acrescentar `achaCaminhoPessoal`)
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs` (acrescentar testes)
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs` (versão final abaixo)
- Modify: `.github/workflows/ci.yml` (job novo)

**Interfaces:**
- Consumes: `arquivosProibidos` (Tarefa 0). O CI chama arquivos das Tarefas 1, 4 e 6 pelos nomes deste plano: `test/manifesto.test.mjs`, `test/caminhos-lab.test.mjs`, `test/testes-no-guest.test.mjs`, `sandbox-lab/scripts/test/travas.test.ps1`.
- Produces: `achaCaminhoPessoal(texto: string): { linha: number, trecho: string }[]`.

- [ ] **Step 1: Escrever os testes que falham**

Acrescentar ao fim de `test/proibidos.test.mjs` (e trocar o import por `import { arquivosProibidos, achaCaminhoPessoal } from "../proibidos.mjs";`). Os exemplos são montados com `join` para o próprio arquivo de teste não conter um caminho de máquina:

```js
const B = "\\";
const naoAcha = (t) => assert.deepEqual(achaCaminhoPessoal(t), [], t);
const acha = (t) => assert.equal(achaCaminhoPessoal(t).length, 1, t);

test("achaCaminhoPessoal: Windows user folder with \\, / and JSON-escaped \\\\", () => {
  acha(["C:", "Users", "fulano", "x"].join(B));
  acha(["c:", "users", "fulano"].join("/"));
  acha(["D:", "Users", "fulano", "a"].join(B + B));
});

test("achaCaminhoPessoal: macOS and Linux home folders", () => {
  acha(`cd ${"/" + ["Users", "fulano", "x"].join("/")}`);
  acha(`"${"/" + ["home", "fulano"].join("/")}"`);
});

test("achaCaminhoPessoal: IDE project folders", () => {
  acha("Pycharm" + "Projects/x");
  acha("Idea" + "Projects");
  acha(["source", "repos", "x"].join(B));
});

test("achaCaminhoPessoal: placeholders, variables, the Sandbox user and guest paths are fine", () => {
  naoAcha(["C:", "Users", "<nome>", "x"].join(B));
  naoAcha(["C:", "Users", "%USERNAME%"].join(B));
  naoAcha(["C:", "Users", "$env:USERNAME"].join(B));
  naoAcha(["C:", "Users", "WDAGUtilityAccount", "AppData"].join(B));
  naoAcha(["C:", "Users", "Public"].join(B));
  naoAcha("%USERPROFILE%\\.rustup\\toolchains");
  naoAcha("C:\\lab\\tools\\sqlite\\sqlite3.exe and C:\\labwork\\db");
  naoAcha("https://example.com/ho" + "me/x");
});

test("achaCaminhoPessoal: Public-like names are still flagged", () => {
  acha(["C:", "Users", "Publicador"].join(B));
});

test("achaCaminhoPessoal: reports the line number and the match", () => {
  const r = achaCaminhoPessoal(`a\nb\nx ${["C:", "Users", "fulano"].join(B)} y`);
  assert.deepEqual(r, [{ linha: 3, trecho: ["C:", "Users", "fulano"].join(B) }]);
});
```

- [ ] **Step 2: Rodar e ver falhar**

Portão de teste puro (mesmo comando da Tarefa 0) → nenhuma linha.
Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs`
Expected: FAIL com `achaCaminhoPessoal is not a function` (ou erro de import com o nome).

- [ ] **Step 3: Implementar**

Acrescentar ao fim de `proibidos.mjs`:

```js
// Machine paths that must not appear in versioned text. Placeholders (<nome>), variables (%USERNAME%, $env:) and the
// Windows Sandbox user are not names, so they do not match.
const PADROES_CAMINHO = [
  /[a-z]:(?:\\{1,2}|\/)users(?:\\{1,2}|\/)(?!(?:public|default|wdagutilityaccount)(?![a-z0-9]))[^\\/\s"'<>%$]+/i,
  /(?:^|[\s"'(=])\/(?:users|home)\/[^/\s"'<>$]+/i,
  // "[p]" keeps this source line from matching itself when conferir-repo scans the skill.
  /pycharm[p]rojects|idea[p]rojects|source(?:\\{1,2}|\/)repos/i,
];

/** One finding per line that holds a machine path: { linha (1-based), trecho }. */
export function achaCaminhoPessoal(texto) {
  const achados = [];
  String(texto ?? "").split(/\r?\n/).forEach((l, i) => {
    for (const p of PADROES_CAMINHO) {
      const m = l.match(p);
      if (m) { achados.push({ linha: i + 1, trecho: m[0].trim() }); break; }
    }
  });
  return achados;
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs; "exit=$LASTEXITCODE"`
Expected: `# pass 13`, `# fail 0`, `exit=0`.

- [ ] **Step 5: Prova de mutação**

Trocar `(?:\\{1,2}|\/)users(?:\\{1,2}|\/)` por `\\users\\` no primeiro padrão. Rodar; colar `not ok ... achaCaminhoPessoal: Windows user folder with \, / and JSON-escaped \\`. Restaurar e rodar: `# fail 0`.

- [ ] **Step 6: `conferir-repo.mjs` final (só leitura)**

Substituir o arquivo inteiro por:

```js
// Read-only repo guard for CI and local use. Lists versioned files with `git ls-files -z` and fails when
// (1) a file that must never be versioned is there, or (2) a versioned text file under sandbox-lab/ or .claude/skills/
// holds a machine path. Never writes, moves or deletes anything.
// Usage (from any folder of the checkout): node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { arquivosProibidos, achaCaminhoPessoal } from "./proibidos.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const r = spawnSync("git", ["-C", RAIZ, "ls-files", "-z"], { encoding: "utf8" });
if (r.status !== 0) {
  console.error(`git ls-files failed: ${(r.stderr || "").trim()}`);
  process.exit(1);
}
const arquivos = r.stdout.split("\0").filter(Boolean);
const proibidos = arquivosProibidos(arquivos);
const caminhos = [];
for (const a of arquivos) {
  if (!/^(sandbox-lab|\.claude\/skills)\//i.test(a)) continue;
  const abs = path.join(RAIZ, a);
  if (!fs.existsSync(abs)) continue; // staged deletion
  const buf = fs.readFileSync(abs);
  if (buf.subarray(0, 8000).includes(0)) continue; // binary (e.g. shared/db/backup.sqlite)
  for (const c of achaCaminhoPessoal(buf.toString("utf8"))) caminhos.push({ arquivo: a, ...c });
}
for (const p of proibidos) console.error(`FORBIDDEN ${p.arquivo}: ${p.motivo}`);
for (const c of caminhos) console.error(`MACHINE PATH ${c.arquivo}:${c.linha}: ${c.trecho}`);
console.log(JSON.stringify({ arquivos: arquivos.length, proibidos: proibidos.length, caminhos: caminhos.length }));
process.exit(proibidos.length || caminhos.length ? 1 : 0);
```

Run: `node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs; "exit=$LASTEXITCODE"`
Expected: `{"arquivos":<N>,"proibidos":0,"caminhos":0}` e `exit=0`. Se aparecer `MACHINE PATH`: relatar o arquivo e a linha ao orquestrador; não corrigir arquivo de outra frente.

- [ ] **Step 7: Job `repo-guard` no CI**

Acrescentar em `.github/workflows/ci.yml`, dentro de `jobs:`, depois do job `npm-install` (mesma indentação dos outros jobs):

```yaml
  repo-guard:
    name: repo guard (lab files, keys, machine paths)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: "lts/*"

      - name: forbidden files and machine paths
        run: node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs

      - name: pure guard tests (node)
        run: >-
          node --test
          .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs
          .claude/skills/claudia-teste-sandbox/scripts/test/manifesto.test.mjs
          .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs
          .claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs

      - name: pure guard tests (PowerShell)
        shell: pwsh
        run: ./sandbox-lab/scripts/test/travas.test.ps1
```

Os arquivos `caminhos-lab.test.mjs`, `testes-no-guest.test.mjs` e `travas.test.ps1` chegam pelas frentes B e C; o job só roda depois da integração. Conferir só o YAML:

Run: `node -e "const t=require('fs').readFileSync('.github/workflows/ci.yml','utf8'); if(!/\n  repo-guard:\n/.test(t)) process.exit(1)"; "exit=$LASTEXITCODE"`
Expected: `exit=0`.

- [ ] **Step 8: Conferência de segredo e commit**

```powershell
git add -- .claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs .github/workflows/ci.yml
```

Conferência de segredo. Depois:

```powershell
git commit -m "feat(sandbox-skill): flag machine paths in versioned lab and skill files; repo-guard CI job" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: raiz principal, caminhos do laboratório, trava do deploy (JS) e guarda do Sandbox

Modelo: **Sonnet**. Roda no PC: `caminhos-lab.test.mjs` e `onde.test.mjs` (puros). `comum.test.mjs` não roda.

**Files:**
- Create: `.claude/skills/claudia-teste-sandbox/scripts/caminhos-lab.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/onde.mjs`
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/comum.mjs:26-32` (`labDir`) e `:85-96` (`MARCADOR_DEPLOY`, `podeLimparDeploy`)
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/test/util.mjs:10-15` (`tempDir`)
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs:6` e `:62-77`

**Interfaces:**
- Consumes: nada de outras frentes.
- Produces (usados pela Tarefa 5 e citados pela Tarefa 8):
  - `caminhos-lab.mjs`: `MARCADOR_DEPLOY = ".lab-deploy"`, `USUARIO_SANDBOX = "WDAGUtilityAccount"`, `raizPrincipalDe(gitCommonDir: string): string`, `caminhosDoLab(raizPrincipal: string): { lab, trabalho, deploy, guestRw, appStage, testUp }`, `linksAte(dir: string, raiz: string, ehLink: (p: string) => boolean): string[]`, `podeLimparDeploy({ existe, temMarcador, ehLink }): string|null`, `motivoRecusaLimparDeploy({ deploy, raizPrincipal, existe, temMarcador, linksNoCaminho }): string|null`, `estaNoSandbox(usuario: string): boolean`.
  - `comum.mjs`: `onde(cwd?: string, git?: (args: string[], cwd: string) => string): { raizPrincipal, projeto, lab, trabalho, deploy, guestRw, appStage, testUp }`; `labDir({ cwd?, git?, existe? }?): string`; continua exportando `MARCADOR_DEPLOY` e `podeLimparDeploy` (reexportados).
  - CLI `node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs` → última linha JSON de `onde()`.
  - `test/util.mjs`: `exigirSandbox(): void` e `tempDir(prefixo)` que chama `exigirSandbox()` antes de criar qualquer pasta.

- [ ] **Step 1: Escrever o teste puro que falha**

`.claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs`:

```js
// Pure tests: invented Windows paths (Z:\...), injected predicates, no disk. Runs on any OS (path.win32 inside).
import test from "node:test";
import assert from "node:assert/strict";
import {
  raizPrincipalDe, caminhosDoLab, linksAte, podeLimparDeploy, motivoRecusaLimparDeploy, estaNoSandbox, MARCADOR_DEPLOY,
} from "../caminhos-lab.mjs";

const RAIZ = "Z:\\proj x\\claudia-rh";
const DEPLOY = `${RAIZ}\\.sandbox-work\\deploy`;
const ok = { deploy: DEPLOY, raizPrincipal: RAIZ, existe: true, temMarcador: true, linksNoCaminho: [] };

test("raizPrincipalDe: git's forward slashes, a trailing slash and backslashes", () => {
  assert.equal(raizPrincipalDe("Z:/proj x/claudia-rh/.git"), RAIZ);
  assert.equal(raizPrincipalDe("Z:/proj x/claudia-rh/.git/\n"), RAIZ);
  assert.equal(raizPrincipalDe(`${RAIZ}\\.GIT`), RAIZ);
});

test("raizPrincipalDe: refuses empty, relative, UNC, submodule and drive-root answers", () => {
  assert.throws(() => raizPrincipalDe(""), /empty/);
  assert.throws(() => raizPrincipalDe(".git"), /absolute/);
  assert.throws(() => raizPrincipalDe("\\\\srv\\share\\r\\.git"), /absolute local/);
  assert.throws(() => raizPrincipalDe("Z:/r/.git/modules/sub"), /unexpected/);
  assert.throws(() => raizPrincipalDe("Z:/.git"), /drive root/);
});

test("caminhosDoLab: everything under the main checkout", () => {
  assert.deepEqual(caminhosDoLab(RAIZ), {
    lab: `${RAIZ}\\sandbox-lab`,
    trabalho: `${RAIZ}\\.sandbox-work`,
    deploy: DEPLOY,
    guestRw: `${RAIZ}\\.sandbox-work\\guest-rw`,
    appStage: `${RAIZ}\\sandbox-lab\\scripts\\app-stage.ps1`,
    testUp: `${RAIZ}\\sandbox-lab\\scripts\\test-up.ps1`,
  });
});

test("linksAte: checks deploy and .sandbox-work, stops before the root", () => {
  const vistos = [];
  const r = linksAte(DEPLOY, RAIZ, (p) => { vistos.push(p); return p.endsWith(".sandbox-work"); });
  assert.deepEqual(vistos, [DEPLOY, `${RAIZ}\\.sandbox-work`]);
  assert.deepEqual(r, [`${RAIZ}\\.sandbox-work`]);
  assert.throws(() => linksAte("Z:\\outro\\deploy", RAIZ, () => false), /not inside/);
});

test("motivoRecusaLimparDeploy: the exact deploy with the marker or absent is allowed", () => {
  assert.equal(motivoRecusaLimparDeploy(ok), null);
  assert.equal(motivoRecusaLimparDeploy({ ...ok, existe: false, temMarcador: false }), null);
  assert.equal(motivoRecusaLimparDeploy({ ...ok, deploy: DEPLOY.toUpperCase() }), null);
});

test("motivoRecusaLimparDeploy: any other path is refused", () => {
  for (const deploy of [
    `${RAIZ}\\.sandbox-work`, `${RAIZ}\\.sandbox-work\\deploy2`, `${RAIZ}\\.sandbox-work\\deploy\\app`,
    `${RAIZ}\\.sandbox-work\\deploy\\..\\..`, `${RAIZ}\\.sandbox-work\\deploy.`, DEPLOY.replace(/\\/g, "/"),
    "Z:\\", "Z:\\outro\\.sandbox-work\\deploy", "relativo\\deploy",
  ]) assert.match(motivoRecusaLimparDeploy({ ...ok, deploy }), /not exactly|required/, deploy);
});

test("motivoRecusaLimparDeploy: bad roots are refused", () => {
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "" }), /required/);
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "claudia-rh" }), /absolute/);
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "\\\\srv\\share" }), /absolute/);
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "Z:\\", deploy: "Z:\\.sandbox-work\\deploy" }), /drive root/);
});

test("motivoRecusaLimparDeploy: a link in the path wins over the marker", () => {
  assert.match(motivoRecusaLimparDeploy({ ...ok, linksNoCaminho: [`${RAIZ}\\.sandbox-work`] }), /link/);
});

test("motivoRecusaLimparDeploy: an existing deploy without the marker is refused", () => {
  assert.match(motivoRecusaLimparDeploy({ ...ok, temMarcador: false }), new RegExp(MARCADOR_DEPLOY.replace(".", "\\.")));
});

test("podeLimparDeploy keeps its old contract", () => {
  assert.equal(podeLimparDeploy({ existe: false, temMarcador: false, ehLink: false }), null);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: true, ehLink: true }), /link/);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: false, ehLink: false }), /marker/);
});

test("estaNoSandbox is true only for the Sandbox user", () => {
  assert.equal(estaNoSandbox("WDAGUtilityAccount"), true);
  assert.equal(estaNoSandbox("wdagutilityaccount"), true);
  for (const u of ["", undefined, null, "fulano", "WDAGUtilityAccount2", "Administrator"]) assert.equal(estaNoSandbox(u), false, String(u));
});
```

`.claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs`:

```js
// Pure tests: git and the disk check are injected; nothing is spawned or read.
import test from "node:test";
import assert from "node:assert/strict";
import { onde, labDir } from "../comum.mjs";

const MAIN = "Z:\\proj x\\claudia-rh";
const WT = `${MAIN}\\.claude\\worktrees\\agent-x`;
const gitFalso = (topo) => (args) => {
  const k = args.join(" ");
  if (k === "rev-parse --path-format=absolute --git-common-dir") return "Z:/proj x/claudia-rh/.git";
  if (k === "rev-parse --show-toplevel") return topo.replace(/\\/g, "/");
  throw new Error(`unexpected git ${k}`);
};

test("from a worktree: lab and work folder from the main checkout, project = the worktree", () => {
  const o = onde(WT, gitFalso(WT));
  assert.equal(o.raizPrincipal, MAIN);
  assert.equal(o.projeto, WT);
  assert.equal(o.lab, `${MAIN}\\sandbox-lab`);
  assert.equal(o.trabalho, `${MAIN}\\.sandbox-work`);
});

test("from the main checkout: project = the main checkout", () => {
  const o = onde(MAIN, gitFalso(MAIN));
  assert.equal(o.projeto, MAIN);
  assert.equal(o.raizPrincipal, MAIN);
});

test("labDir names the missing app-stage.ps1 and ignores SANDBOX_LAB_DIR", () => {
  const antigo = process.env.SANDBOX_LAB_DIR;
  process.env.SANDBOX_LAB_DIR = "Z:\\outro-lab";
  try {
    assert.throws(() => labDir({ cwd: WT, git: gitFalso(WT), existe: () => false }), /sandbox-lab\\scripts\\app-stage\.ps1/);
    const alvo = `${MAIN}\\sandbox-lab\\scripts\\app-stage.ps1`;
    assert.equal(labDir({ cwd: WT, git: gitFalso(WT), existe: (p) => p === alvo }), `${MAIN}\\sandbox-lab`);
  } finally {
    if (antigo === undefined) delete process.env.SANDBOX_LAB_DIR; else process.env.SANDBOX_LAB_DIR = antigo;
  }
});
```

- [ ] **Step 2: Rodar e ver falhar**

Portão de teste puro:

```powershell
Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs, .claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs -Pattern 'node:fs|node:child_process|util\.mjs|mkdir|rmSync|writeFile'
```

Expected: nenhuma linha.

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs`
Expected: FAIL (`ERR_MODULE_NOT_FOUND` para `caminhos-lab.mjs`; `onde` não exportada).

- [ ] **Step 3: Implementar `caminhos-lab.mjs`**

```js
// PURE (no disk, no git). Where the lab and the work folder live, and whether the work folder's deploy may be wiped.
// Windows path semantics on every platform (path.win32), so the tests also run in CI on Linux.
import path from "node:path";

const w = path.win32;

export const MARCADOR_DEPLOY = ".lab-deploy";
export const USUARIO_SANDBOX = "WDAGUtilityAccount";

/** `git rev-parse --path-format=absolute --git-common-dir` output -> main checkout root. Throws on anything else. */
export function raizPrincipalDe(gitCommonDir) {
  const d = String(gitCommonDir ?? "").trim();
  if (d === "") throw new Error("git common dir is empty");
  if (d.startsWith("\\\\") || d.startsWith("//")) throw new Error(`git common dir must be an absolute local path: ${d}`);
  const n = w.normalize(d).replace(/\\+$/, "");
  if (!/^[a-z]:\\/i.test(n)) throw new Error(`git common dir must be an absolute local path: ${d}`);
  if (w.basename(n).toLowerCase() !== ".git") throw new Error(`unexpected git common dir (expected <checkout>\\.git): ${d}`);
  const raiz = w.dirname(n);
  if (w.parse(raiz).root === raiz) throw new Error(`refusing a main checkout at a drive root: ${raiz}`);
  return raiz;
}

export function caminhosDoLab(raizPrincipal) {
  const lab = w.join(raizPrincipal, "sandbox-lab");
  const trabalho = w.join(raizPrincipal, ".sandbox-work");
  return {
    lab,
    trabalho,
    deploy: w.join(trabalho, "deploy"),
    guestRw: w.join(trabalho, "guest-rw"),
    appStage: w.join(lab, "scripts", "app-stage.ps1"),
    testUp: w.join(lab, "scripts", "test-up.ps1"),
  };
}

/** Folders from `dir` up to, not including, `raiz` for which `ehLink(p)` is true. `ehLink` is injected. */
export function linksAte(dir, raiz, ehLink) {
  const fim = raiz.replace(/\\+$/, "").toLowerCase();
  const achados = [];
  let atual = dir;
  while (atual.replace(/\\+$/, "").toLowerCase() !== fim) {
    if (ehLink(atual)) achados.push(atual);
    const pai = w.dirname(atual);
    if (pai === atual) throw new Error(`${dir} is not inside ${raiz}`);
    atual = pai;
  }
  return achados;
}

/** Old contract (B3), kept for callers: null when allowed, otherwise the reason. */
export function podeLimparDeploy({ existe, temMarcador, ehLink }) {
  if (!existe) return null;
  if (ehLink) return "deploy is a link (symlink/junction/reparse point)";
  if (!temMarcador) return `deploy exists without the ${MARCADOR_DEPLOY} marker (not created by this script)`;
  return null;
}

/**
 * Whether publicar/app-stage may wipe `deploy`. null = allowed, otherwise the reason.
 * Allow-list: `deploy` must be exactly <raizPrincipal>\.sandbox-work\deploy (same case-insensitive string, normalized),
 * with no link in it or in .sandbox-work, and, when it exists, with the marker.
 */
export function motivoRecusaLimparDeploy({ deploy, raizPrincipal, existe, temMarcador, linksNoCaminho = [] }) {
  if (typeof deploy !== "string" || deploy.trim() === "" || typeof raizPrincipal !== "string" || raizPrincipal.trim() === "") {
    return "deploy and raizPrincipal are required";
  }
  if (raizPrincipal.startsWith("\\\\") || !/^[a-z]:\\/i.test(raizPrincipal)) return "raizPrincipal must be an absolute local path";
  if (w.parse(raizPrincipal).root === raizPrincipal) return "raizPrincipal is a drive root";
  const esperado = w.join(raizPrincipal, ".sandbox-work", "deploy");
  if (deploy !== w.normalize(deploy) || deploy.toLowerCase() !== esperado.toLowerCase()) return `deploy is not exactly ${esperado}`;
  if (linksNoCaminho.length > 0) return `link (symlink/junction) in the path: ${linksNoCaminho[0]}`;
  return podeLimparDeploy({ existe, temMarcador, ehLink: false });
}

export function estaNoSandbox(usuario) {
  return String(usuario ?? "").toLowerCase() === USUARIO_SANDBOX.toLowerCase();
}
```

Observação para o executor: `w.normalize("...\\deploy.")` mantém o ponto, mas a comparação com `esperado` recusa; `"relativo\\deploy"` não é igual a `esperado` e cai em "not exactly".

- [ ] **Step 4: `comum.mjs` sem variável de ambiente**

Em `comum.mjs`:

1. Acrescentar ao bloco de imports: `import { raizPrincipalDe, caminhosDoLab } from "./caminhos-lab.mjs";`
2. Substituir `labDir()` (linhas 26-32) por:

```js
function gitLer(args, cwd) {
  const r = spawnSync("git", ["-C", cwd, ...args], { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`git ${args.join(" ")} failed: ${(r.stderr || r.error?.message || "").trim()}`);
  return r.stdout.trim();
}

/**
 * Main checkout (lab + work folder) and current checkout (project under test). Read-only: only `git rev-parse`.
 * `git` is injectable so the logic is tested without spawning git.
 */
export function onde(cwd = REPO_ROOT, git = gitLer) {
  const raizPrincipal = raizPrincipalDe(git(["rev-parse", "--path-format=absolute", "--git-common-dir"], cwd));
  const projeto = path.win32.normalize(git(["rev-parse", "--show-toplevel"], cwd));
  return { raizPrincipal, projeto, ...caminhosDoLab(raizPrincipal) };
}

/** sandbox-lab of the main checkout; throws naming the missing file. Never reads an environment variable. */
export function labDir({ cwd = REPO_ROOT, git = gitLer, existe = fs.existsSync } = {}) {
  const o = onde(cwd, git);
  if (!existe(o.appStage)) {
    throw new Error(`Lab not found: ${o.appStage} is missing in the main checkout. The lab lives in sandbox-lab/ of the main checkout (SKILL.md, section 1).`);
  }
  return o.lab;
}
```

3. Substituir as linhas 85-96 (`export const MARCADOR_DEPLOY ...` até o fim de `podeLimparDeploy`) por:

```js
export { MARCADOR_DEPLOY, podeLimparDeploy } from "./caminhos-lab.mjs";
```

- [ ] **Step 5: `onde.mjs` (CLI só leitura)**

```js
// Prints where the lab, the work folder and the project under test are, as JSON. Read-only (git rev-parse only).
// Usage: node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs
import { onde, falhaDe } from "./comum.mjs";

try {
  console.log(JSON.stringify(onde()));
} catch (e) { falhaDe(e); }
```

- [ ] **Step 6: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/motivo-recusa.test.mjs; "exit=$LASTEXITCODE"`
Expected: `# fail 0`, `exit=0` (o `motivo-recusa.test.mjs` existente continua passando com o `podeLimparDeploy` reexportado).

Run: `node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs; "exit=$LASTEXITCODE"`
Expected: JSON com `raizPrincipal` = checkout principal e `projeto` = a worktree do agente; `exit=0`. Não colar esse JSON em arquivo versionado.

- [ ] **Step 7: Prova de mutação (duas travas)**

a) Em `motivoRecusaLimparDeploy`, trocar `if (linksNoCaminho.length > 0)` por `if (false)`. Rodar `caminhos-lab.test.mjs`; colar `not ok ... motivoRecusaLimparDeploy: a link in the path wins over the marker`. Restaurar.
b) Em `raizPrincipalDe`, apagar a linha do `basename(...) !== ".git"`. Rodar; colar `not ok ... raizPrincipalDe: refuses empty, relative, UNC, submodule and drive-root answers`. Restaurar. Rodar de novo: `# fail 0`.

- [ ] **Step 8: Guarda do Sandbox em `test/util.mjs`**

Substituir o cabeçalho e `tempDir` (linhas 1-15) por:

```js
// Small helpers shared by the scenario tests.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { estaNoSandbox } from "../caminhos-lab.mjs";

export const SCRIPTS = path.join(import.meta.dirname, "..");

/** Tests that create or delete folders run only inside Windows Sandbox (CLAUDE.md, hard rules). Throws elsewhere. */
export function exigirSandbox() {
  const usuario = os.userInfo().username;
  if (!estaNoSandbox(usuario)) {
    throw new Error(`SANDBOX ONLY: this test creates and deletes folders; it runs only inside Windows Sandbox (user is ${usuario}).`);
  }
}

/** Temp folder with a space in its name, so every test also covers paths with spaces. Sandbox only. */
export function tempDir(prefixo) {
  exigirSandbox(); // before any folder is created
  // Must live under <tmp>/claudia-teste-sandbox: the scenario scripts refuse anything else (allow-list).
  const base = path.join(fs.realpathSync.native(os.tmpdir()), "claudia-teste-sandbox");
  fs.mkdirSync(base, { recursive: true });
  return fs.mkdtempSync(path.join(base, `cs test ${prefixo} `));
}
```

Não rodar nenhum teste que use `tempDir` no PC. Conferir só por leitura que todo teste que cria pasta passa por `tempDir` ou `exigirSandbox` antes:

```powershell
Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/test/*.test.mjs -Pattern 'mkdirSync|mkdtempSync|rmSync|symlinkSync|rodar\(' | Select-Object Filename -Unique
Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/test/*.test.mjs -Pattern 'tempDir\(|exigirSandbox\(' | Select-Object Filename -Unique
```

Expected: todo arquivo da primeira lista que cria pasta aparece na segunda. Exceção conhecida: `novo.test.mjs` "opcao desconhecida sai com 2" chama `rodar` sem criar pasta (o script sai antes). Relatar qualquer outro.

- [ ] **Step 9: Tirar de `comum.test.mjs` o teste de `SANDBOX_LAB_DIR`**

Na linha 6, tirar `labDir, ` do import. Apagar o teste inteiro `test("lab ausente ou incompleto nomeia o que falta", ...)` (linhas 62-77); ele passa a ser coberto por `onde.test.mjs`. Não rodar `comum.test.mjs`.

Run: `node --check .claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs; node --check .claude/skills/claudia-teste-sandbox/scripts/test/util.mjs; "exit=$LASTEXITCODE"`
Expected: `exit=0`.

Run: `Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/*.mjs, .claude/skills/claudia-teste-sandbox/scripts/test/*.mjs -Pattern 'SANDBOX_LAB_DIR' | Select-Object Filename, LineNumber`
Expected: só `onde.test.mjs` (o teste que prova que a variável é ignorada). `publicar.mjs`, `publicar.test.mjs` e `banco-guest.mjs` ainda citam `LAB_WORK_DIR`/`SANDBOX_LAB_DIR`: mudam na Tarefa 5.

- [ ] **Step 10: Conferência de segredo e commit**

```powershell
git add -- .claude/skills/claudia-teste-sandbox/scripts/caminhos-lab.mjs .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/onde.test.mjs .claude/skills/claudia-teste-sandbox/scripts/onde.mjs .claude/skills/claudia-teste-sandbox/scripts/comum.mjs .claude/skills/claudia-teste-sandbox/scripts/test/util.mjs .claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs
```

Conferência de segredo. Depois:

```powershell
git commit -m "feat(sandbox-skill): find the lab from the main checkout via git, exact-path deploy guard, Sandbox-only temp folders" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: `publicar`, `banco-guest` e `testes-no-guest`

Modelo: **Sonnet**. Roda no PC: só `testes-no-guest.test.mjs` (puro). `publicar.test.mjs` é **Sandbox** (Tarefa 9).

**Files:**
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/publicar.mjs` (arquivo inteiro)
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs` (arquivo inteiro)
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs:1-15`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs`

**Interfaces:**
- Consumes: `onde`, `labDir`, `falhaDe`, `APP_ID`, `sandboxId`, `noGuest`, `validarInt` (`comum.mjs`); `caminhosDoLab`, `linksAte`, `motivoRecusaLimparDeploy`, `MARCADOR_DEPLOY` (`caminhos-lab.mjs`); `tempDir`, `rodar`, `SCRIPTS` (`test/util.mjs`).
- Produces:
  - `publicar({ app, cenario }, { raizPrincipal, rodarStage: (ps1: string, args: string[]) => number }): { workDir, deploy, cenario }`. CLI: `node publicar.mjs --app <exe|instalador|pasta> --cenario <pasta>` (opção desconhecida, inclusive `--work-dir`, sai com 2).
  - `banco-guest.mjs [--id <id>] [--timeout 30]` (sem `--work-dir`).
  - `comandoDeTesteNoGuest({ node?: string[], ps?: string|null, nonce: string }): string`. CLI: `node testes-no-guest.mjs [--id <id>] (--node <arq.test.mjs> ... | --ps sandbox-lab/scripts/test/<x>.sandbox.ps1) [--timeout 900]` → JSON `{exit, log}`; exit 0 só se os testes passaram no guest.
  - Contrato com o guest (sem mudar o `guest-build.ps1` além da Tarefa 7): chama `C:\labsandbox\app\guest-build.ps1 -Install '' -Build '<comando>'` e lê `<guestRw>\build\result.json` (`steps.build.cmd` contém o nonce; `steps.build.exit`; `end`).

- [ ] **Step 1: Teste puro do comando do guest (falha)**

`.claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs`:

```js
// Pure tests of the guest command builder: strings only.
import test from "node:test";
import assert from "node:assert/strict";
import { comandoDeTesteNoGuest } from "../testes-no-guest.mjs";

const NONCE = "0123abcd0123abcd";

test("node tests: explicit files, guest temp under C:\\labdata, nonce first", () => {
  const c = comandoDeTesteNoGuest({ node: [".claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs"], nonce: NONCE });
  assert.equal(c, `set LAB_NONCE=${NONCE}&& set TEMP=C:\\labdata\\tmp&& set TMP=C:\\labdata\\tmp&& (if not exist C:\\labdata\\tmp mkdir C:\\labdata\\tmp)&& node --test .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs`);
});

test("ps test: only sandbox-lab/scripts/test/*.sandbox.ps1, with backslashes", () => {
  const c = comandoDeTesteNoGuest({ ps: "sandbox-lab/scripts/test/travas.sandbox.ps1", nonce: NONCE });
  assert.match(c, /&& powershell -NoProfile -ExecutionPolicy Bypass -File sandbox-lab\\scripts\\test\\travas\.sandbox\.ps1$/);
});

test("refuses globs, folders, '..', spaces, quotes and other files", () => {
  for (const f of ["*.test.mjs", ".claude/skills/x/", ".claude/skills/../../x.test.mjs", ".claude/skills/a b.test.mjs", ".claude/skills/a\".test.mjs", "src/x.test.mjs"]) {
    assert.throws(() => comandoDeTesteNoGuest({ node: [f], nonce: NONCE }), /not a skill test file|'\.\.'/, f);
  }
  assert.throws(() => comandoDeTesteNoGuest({ ps: "sandbox-lab/scripts/test-up.ps1", nonce: NONCE }), /not a lab Sandbox test/);
});

test("needs exactly one kind of test and a hex nonce", () => {
  assert.throws(() => comandoDeTesteNoGuest({ nonce: NONCE }), /either/);
  assert.throws(() => comandoDeTesteNoGuest({ node: [".claude/skills/a/x.test.mjs"], ps: "sandbox-lab/scripts/test/t.sandbox.ps1", nonce: NONCE }), /either/);
  assert.throws(() => comandoDeTesteNoGuest({ node: [".claude/skills/a/x.test.mjs"], nonce: "x&del" }), /nonce/);
});
```

Portão de teste puro (mesmo padrão) → nenhuma linha no arquivo de teste.
Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs`
Expected: FAIL com `ERR_MODULE_NOT_FOUND`.

- [ ] **Step 2: Implementar `testes-no-guest.mjs`**

```js
// Runs Sandbox-only tests inside the running guest through the lab's guest-build.ps1 (profile lab.config.app-build.json,
// which mounts the host Node at C:\hosttools\nodejs). The guest copies C:\labproject (the project passed to test-up -Project)
// to C:\labdata\project and runs the command there. The host only reads <work folder>\guest-rw\build\result.json.
// Usage: node testes-no-guest.mjs [--id <id>] (--node <file.test.mjs> ... | --ps sandbox-lab/scripts/test/<x>.sandbox.ps1) [--timeout 900]
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { onde, sandboxId, noGuest, validarInt, falhaDe } from "./comum.mjs";

const ARQ_NODE = /^\.claude\/skills\/[A-Za-z0-9_\-/.]+\.test\.mjs$/;
const ARQ_PS = /^sandbox-lab\/scripts\/test\/[A-Za-z0-9_\-.]+\.sandbox\.ps1$/;

/** PURE. The cmd.exe line guest-build.ps1 runs as its "build" step. No quotes, so nothing needs escaping. */
export function comandoDeTesteNoGuest({ node = [], ps = null, nonce }) {
  if (!/^[a-f0-9]{8,64}$/.test(String(nonce))) throw new Error("nonce must be 8-64 hex characters");
  if ((node.length > 0) === Boolean(ps)) throw new Error("pass either --node files or one --ps file");
  for (const f of [...node, ...(ps ? [ps] : [])]) if (String(f).split("/").includes("..")) throw new Error(`'..' is not allowed: ${f}`);
  for (const f of node) if (!ARQ_NODE.test(f)) throw new Error(`not a skill test file: ${f}`);
  if (ps && !ARQ_PS.test(ps)) throw new Error(`not a lab Sandbox test: ${ps}`);
  const prefixo = `set LAB_NONCE=${nonce}&& set TEMP=C:\\labdata\\tmp&& set TMP=C:\\labdata\\tmp&& (if not exist C:\\labdata\\tmp mkdir C:\\labdata\\tmp)`;
  const corpo = ps ? `powershell -NoProfile -ExecutionPolicy Bypass -File ${ps.replace(/\//g, "\\")}` : `node --test ${node.join(" ")}`;
  return `${prefixo}&& ${corpo}`;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: { id: { type: "string" }, node: { type: "string", multiple: true }, ps: { type: "string" }, timeout: { type: "string", default: "900" } } });
    const timeout = validarInt(values.timeout, 1, 7200, "--timeout");
    const nonce = crypto.randomBytes(8).toString("hex");
    const build = comandoDeTesteNoGuest({ node: values.node ?? [], ps: values.ps ?? null, nonce });
    const id = sandboxId(values.id);
    const result = path.join(onde().guestRw, "build", "result.json");
    noGuest(id, "ExistingLogin", `& C:\\labsandbox\\app\\guest-build.ps1 -Install '' -Build '${build}'`);
    const fim = Date.now() + timeout * 1000;
    let j = null;
    while (Date.now() < fim) {
      try {
        const t = JSON.parse(fs.readFileSync(result, "utf8"));
        if (t.end && String(t.steps?.build?.cmd ?? "").includes(nonce)) { j = t; break; }
      } catch { /* not written yet */ }
      await new Promise((r) => setTimeout(r, 2000));
    }
    if (!j) throw new Error(`no result for this run in ${result} after ${timeout}s`);
    const exit = j.steps.build.exit;
    console.log(JSON.stringify({ exit, log: path.join(path.dirname(result), "guest-build.log") }));
    process.exit(exit === 0 ? 0 : 1);
  } catch (e) { falhaDe(e); }
}
```

- [ ] **Step 3: Rodar e ver passar; mutação**

Run: `node --test .claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs; "exit=$LASTEXITCODE"`
Expected: `# pass 4`, `# fail 0`.

Mutação: apagar a linha do `'..' is not allowed`. Rodar; colar `not ok ... refuses globs, folders, '..', spaces, quotes and other files`. Restaurar; `# fail 0`.

- [ ] **Step 4: Reescrever `publicar.mjs`**

Arquivo inteiro:

```js
// Publishes the app plus a scenario folder to the lab's work folder (<main checkout>\.sandbox-work) via app-stage.ps1 -Clean.
// Usage: node publicar.mjs --app <exe|installer|folder> --cenario <scenario folder>
// The work folder is fixed (no --work-dir, no LAB_WORK_DIR). -Clean only runs when <main>\.sandbox-work\deploy is
// absent or carries the .lab-deploy marker, with no link on the way (app-stage.ps1 checks the same again).
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { APP_ID, falhaDe, labDir, onde } from "./comum.mjs";
import { caminhosDoLab, linksAte, motivoRecusaLimparDeploy, MARCADOR_DEPLOY } from "./caminhos-lab.mjs";

function ehLinkNoDisco(p) {
  try { return fs.lstatSync(p).isSymbolicLink(); } catch (e) { if (e.code === "ENOENT") return false; throw e; }
}

/** `raizPrincipal` and `rodarStage` are injected so the Sandbox test uses a fake checkout and a fake app-stage. */
export function publicar({ app, cenario }, { raizPrincipal, rodarStage }) {
  const meta = path.join(cenario, "cenario.json");
  if (!fs.existsSync(meta)) throw new Error(`Not a scenario folder (cenario.json missing): ${cenario}`);
  const c = caminhosDoLab(raizPrincipal);
  let st = null;
  try { st = fs.lstatSync(c.deploy); } catch (e) { if (e.code !== "ENOENT") throw e; }
  const motivo = motivoRecusaLimparDeploy({
    deploy: c.deploy,
    raizPrincipal,
    existe: st !== null,
    temMarcador: st !== null && fs.existsSync(path.join(c.deploy, MARCADOR_DEPLOY)),
    linksNoCaminho: linksAte(c.deploy, raizPrincipal, ehLinkNoDisco),
  });
  if (motivo) throw new Error(`Refusing -Clean: ${motivo}: ${c.deploy}`);
  const status = rodarStage(c.appStage, ["-App", app, "-AppData", path.join(cenario, "appdata", APP_ID), "-WorkDir", c.trabalho, "-Clean"]);
  if (status !== 0) throw new Error(`app-stage.ps1 failed (${status})`);
  fs.mkdirSync(c.deploy, { recursive: true });
  fs.writeFileSync(path.join(c.deploy, MARCADOR_DEPLOY), "");
  return { workDir: c.trabalho, deploy: c.deploy, cenario: JSON.parse(fs.readFileSync(meta, "utf8")).cenario };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: { app: { type: "string" }, cenario: { type: "string" } } });
    if (!values.app || !values.cenario) {
      throw Object.assign(new Error("Usage: node publicar.mjs --app <exe|installer|folder> --cenario <folder>"), { code: "ERR_PARSE_ARGS_USAGE" });
    }
    labDir();
    const { raizPrincipal } = onde();
    const rodarStage = (ps1, args) => {
      const r = spawnSync("pwsh", ["-NoProfile", "-File", ps1, ...args], { stdio: ["ignore", "inherit", "inherit"] });
      return r.status ?? r.error?.message;
    };
    console.log(JSON.stringify(publicar(values, { raizPrincipal, rodarStage })));
  } catch (e) { falhaDe(e); }
}
```

- [ ] **Step 5: Reescrever `publicar.test.mjs` (Sandbox)**

Arquivo inteiro:

```js
// SANDBOX ONLY: creates folders and junctions. tempDir() refuses to run outside Windows Sandbox.
// Run through: node testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs
// Nothing is cleaned up on purpose: the guest is discarded by test-down.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";
import { publicar } from "../publicar.mjs";
import { MARCADOR_DEPLOY } from "../caminhos-lab.mjs";

function montar(nome) {
  const base = tempDir(`publicar ${nome}`);
  const raiz = path.join(base, "repo x");
  fs.mkdirSync(path.join(raiz, "sandbox-lab", "scripts"), { recursive: true });
  const app = path.join(base, "app x.exe");
  fs.writeFileSync(app, "x");
  const cen = path.join(base, "cen x");
  const r = rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--saida", cen]);
  assert.equal(r.code, 0, r.err);
  const chamadas = [];
  const deps = { raizPrincipal: raiz, rodarStage: (ps1, args) => { chamadas.push({ ps1, args }); return 0; } };
  const sentinela = (dir) => { fs.mkdirSync(dir, { recursive: true }); const f = path.join(dir, "keep.txt"); fs.writeFileSync(f, "keep"); return f; };
  return { base, raiz, app, cen, chamadas, deps, sentinela, deploy: path.join(raiz, ".sandbox-work", "deploy") };
}

test("first publish: app-stage gets -WorkDir <root>\\.sandbox-work and -Clean; marker written", () => {
  const m = montar("a");
  const r = publicar({ app: m.app, cenario: m.cen }, m.deps);
  assert.equal(m.chamadas.length, 1);
  assert.equal(m.chamadas[0].ps1, path.join(m.raiz, "sandbox-lab", "scripts", "app-stage.ps1"));
  assert.deepEqual(m.chamadas[0].args.slice(-3), ["-WorkDir", path.join(m.raiz, ".sandbox-work"), "-Clean"]);
  assert.match(m.chamadas[0].args[3], /appdata[\\/]io\.github\.johngabie\.claudia-rh$/);
  assert.ok(fs.existsSync(path.join(m.deploy, MARCADOR_DEPLOY)));
  assert.equal(r.cenario, "novo");
});

test("second publish with the marker is allowed", () => {
  const m = montar("b");
  publicar({ app: m.app, cenario: m.cen }, m.deps);
  publicar({ app: m.app, cenario: m.cen }, m.deps);
  assert.equal(m.chamadas.length, 2);
});

test("deploy without the marker: refused, app-stage not called, files kept", () => {
  const m = montar("c");
  const keep = m.sentinela(m.deploy);
  assert.throws(() => publicar({ app: m.app, cenario: m.cen }, m.deps), /marker/);
  assert.equal(m.chamadas.length, 0);
  assert.ok(fs.existsSync(keep));
});

test("deploy as a junction (even with the marker inside): refused, target kept", () => {
  const m = montar("d");
  const alvo = path.join(m.base, "alvo");
  const keep = m.sentinela(alvo);
  fs.writeFileSync(path.join(alvo, MARCADOR_DEPLOY), "");
  fs.mkdirSync(path.dirname(m.deploy), { recursive: true });
  fs.symlinkSync(alvo, m.deploy, "junction");
  assert.throws(() => publicar({ app: m.app, cenario: m.cen }, m.deps), /link/);
  assert.equal(m.chamadas.length, 0);
  assert.ok(fs.existsSync(keep));
});

test(".sandbox-work as a junction: refused, target kept", () => {
  const m = montar("e");
  const alvo = path.join(m.base, "alvo-work");
  const keep = m.sentinela(path.join(alvo, "deploy"));
  fs.writeFileSync(path.join(alvo, "deploy", MARCADOR_DEPLOY), "");
  fs.symlinkSync(alvo, path.join(m.raiz, ".sandbox-work"), "junction");
  assert.throws(() => publicar({ app: m.app, cenario: m.cen }, m.deps), /link/);
  assert.equal(m.chamadas.length, 0);
  assert.ok(fs.existsSync(keep));
});

test("a folder without cenario.json is refused", () => {
  const m = montar("f");
  assert.throws(() => publicar({ app: m.app, cenario: m.base }, m.deps), /cenario\.json/);
});

test("CLI: --work-dir no longer exists and a missing --cenario exits 2", () => {
  const script = path.join(SCRIPTS, "publicar.mjs");
  assert.equal(rodar(script, ["--app", "x", "--cenario", "y", "--work-dir", "z"]).code, 2);
  assert.equal(rodar(script, ["--app", "x"]).code, 2);
});
```

Não rodar no PC. Conferir só a sintaxe:
Run: `node --check .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs; node --check .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs; "exit=$LASTEXITCODE"`
Expected: `exit=0`.

- [ ] **Step 6: `banco-guest.mjs` sem `--work-dir`**

Trocar as linhas 1-15 por:

```js
// Copies a consistent snapshot of the app database from the sandbox guest to the host.
// Prints {db}: use it as CLAUDIA_DB with the claudia-teste-e2e scripts. Repeat after each action that changes data.
// Usage: node banco-guest.mjs [--id <sandbox>] [--timeout 30]
// The snapshot lands in <main checkout>\.sandbox-work\guest-rw\db (the folder mounted as C:\labwork).
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { APP_ID, sandboxId, noGuest, validarInt, falhaDe, erroBancoAusente, onde } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { id: { type: "string" }, timeout: { type: "string", default: "30" } } });
  const id = sandboxId(values.id);
  const timeout = validarInt(values.timeout, 1, 0xFFFFFFFF, "--timeout");
  const hostDb = path.join(onde().guestRw, "db", "claudia_rh.db");
```

O resto do arquivo (a partir de `const hostDbTmp = hostDb + ".tmp";`) fica igual.

Run: `node --check .claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs; "exit=$LASTEXITCODE"` → `exit=0`.

- [ ] **Step 7: Nenhuma variável de caminho nos scripts da skill**

Run: `Select-String -Path .claude/skills/claudia-teste-sandbox/scripts/*.mjs, .claude/skills/claudia-teste-sandbox/scripts/test/*.mjs -Pattern 'SANDBOX_LAB_DIR|LAB_APP_PROJECT|LAB_WORK_DIR|work-dir' | Select-Object Filename, LineNumber, Line`
Expected: só `onde.test.mjs` (`SANDBOX_LAB_DIR`, prova de que é ignorada) e `publicar.test.mjs` (`--work-dir`, prova de que sai com 2).

- [ ] **Step 8: Conferência de segredo e commit**

```powershell
git add -- .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs .claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs .claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs
```

Conferência de segredo. Depois:

```powershell
git commit -m "feat(sandbox-skill): publish to the fixed work folder of the main checkout, guest DB snapshot without env vars, run Sandbox tests in the guest" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: trava PowerShell e `app-stage.ps1`

Modelo: **Sonnet**. Roda no PC: só `travas.test.ps1` (puro) e o parser do PowerShell. `travas.sandbox.ps1` é **Sandbox** (Tarefa 9).

**Files:**
- Create: `sandbox-lab/scripts/lib/travas.ps1`
- Create: `sandbox-lab/scripts/lib/disco.ps1`
- Create: `sandbox-lab/scripts/test/travas.test.ps1`
- Create: `sandbox-lab/scripts/test/travas.sandbox.ps1`
- Modify: `sandbox-lab/scripts/app-stage.ps1` (bloco `param` até o primeiro `New-Item`)
- Modify: `sandbox-lab/scripts/test-up.ps1` (só a limpeza de `shared\out`)

**Interfaces:**
- Consumes: nada de outras frentes. Contrato com o JS (Tarefa 4/5): o marcador é `.lab-deploy`; o deploy é exatamente `<raiz>\.sandbox-work\deploy`; `publicar.mjs` passa `-WorkDir <raiz>\.sandbox-work -Clean`.
- Produces:
  - `travas.ps1` (puro): `$script:LabDeployMarker`, `ConvertTo-LabPathKey([string]) -> string`, `Get-LabRootProblem([string]) -> string|$null`, `Get-LabCleanRefusal -Deploy -RepoRoot -Exists -HasMarker [-Links] -> string|$null`, `Get-LabOutCleanRefusal -OutDir -LabDir [-Links] -> string|$null`. A Tarefa 7 acrescenta `Get-LabMissingTools`.
  - `disco.ps1` (lê o disco; a única deleção é a de `Clear-LabOut`, depois da trava): `Get-LabLinksInPath([string]$Dir, [string]$StopAt) -> string[]`, `Get-LabReparsePoints([string]$Dir) -> string[]`, `Clear-LabOut -OutDir -LabDir`. A Tarefa 7 acrescenta `Test-LabTools`.

- [ ] **Step 1: Escrever o teste puro que falha**

`sandbox-lab/scripts/test/travas.test.ps1`:

```powershell
# Pure tests for lib/travas.ps1: invented strings only, no disk. Runs on any OS:
#   pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../lib/travas.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

$R = 'Z:\proj x\claudia-rh'
$D = 'Z:\proj x\claudia-rh\.sandbox-work\deploy'
$L = 'Z:\proj x\claudia-rh\sandbox-lab'
$O = 'Z:\proj x\claudia-rh\sandbox-lab\shared\out'

Check 'clean: absent deploy at the exact path is allowed' { Should-BeNull (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $false -HasMarker $false) }
Check 'clean: existing deploy with the marker is allowed' { Should-BeNull (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $true) }
Check 'clean: letter case does not matter' { Should-BeNull (Get-LabCleanRefusal -Deploy $D.ToUpper() -RepoRoot $R -Exists $true -HasMarker $true) }
Check 'clean: existing deploy without the marker is refused' { Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $false) 'marker' }
Check 'clean: any other path is refused' {
  foreach ($p in @("$R\.sandbox-work", "$R\.sandbox-work\deploy2", "$D\app", "$D\..\..", "$D.", ($D -replace '\\', '/'), 'Z:\', 'Z:\outro\.sandbox-work\deploy', 'deploy')) {
    Should-Match (Get-LabCleanRefusal -Deploy $p -RepoRoot $R -Exists $true -HasMarker $true) 'not exactly'
  }
}
Check 'clean: a link wins over the marker' { Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $true -Links @("$R\.sandbox-work")) 'link' }
Check 'clean: empty, relative, UNC, drive-root and dotted roots are refused' {
  Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot '' -Exists $false -HasMarker $false) 'required'
  Should-Match (Get-LabCleanRefusal -Deploy 'claudia-rh\.sandbox-work\deploy' -RepoRoot 'claudia-rh' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy '\\srv\s\r\.sandbox-work\deploy' -RepoRoot '\\srv\s\r' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy 'Z:\.sandbox-work\deploy' -RepoRoot 'Z:\' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy 'Z:\a\..\b\.sandbox-work\deploy' -RepoRoot 'Z:\a\..\b' -Exists $false -HasMarker $false) 'repo root'
}
Check 'out: the exact shared\out is allowed' { Should-BeNull (Get-LabOutCleanRefusal -OutDir $O -LabDir $L) }
Check 'out: any other path is refused' {
  foreach ($p in @("$L\shared", "$O\sub", $L, 'Z:\', ($O -replace '\\', '/'))) { Should-Match (Get-LabOutCleanRefusal -OutDir $p -LabDir $L) 'not exactly' }
}
Check 'out: a link is refused' { Should-Match (Get-LabOutCleanRefusal -OutDir $O -LabDir $L -Links @("$O\x")) 'link' }
Check 'out: bad lab dirs are refused' {
  Should-Match (Get-LabOutCleanRefusal -OutDir 'Z:\shared\out' -LabDir 'Z:\') 'lab dir'
  Should-Match (Get-LabOutCleanRefusal -OutDir $O -LabDir '') 'required'
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
```

- [ ] **Step 2: Rodar e ver falhar**

Portão de teste puro PowerShell:

```powershell
Select-String -Path sandbox-lab/scripts/test/travas.test.ps1 -Pattern 'Remove-Item|New-Item|Set-Content|Out-File|Copy-Item|Move-Item|robocopy|WriteAll|disco\.ps1'
```

Expected: nenhuma linha.
Run: `pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1; "exit=$LASTEXITCODE"`
Expected: erro de dot-source (arquivo `travas.ps1` não existe) e `exit=1`.

- [ ] **Step 3: Implementar `lib/travas.ps1` (puro)**

```powershell
# Pure guards for the lab scripts: string checks only, no disk access, no [IO.Path] (the tests run on any OS).
# Dot-source this file. A guard returns $null when the action is allowed, otherwise the reason.

$script:LabDeployMarker = '.lab-deploy'

function ConvertTo-LabPathKey([string]$Path) {
  # Windows paths compare case-insensitively. No separator rewriting: '/' never matches the exact expected path.
  return $Path.TrimEnd('\').ToLowerInvariant()
}

function Get-LabRootProblem([string]$Root) {
  if ($Root -notmatch '^[A-Za-z]:\\[^\\]') { return 'must be an absolute local path below a drive root' }
  if ($Root -match '(^|\\)\.{1,2}(\\|$)') { return "has '.' or '..' segments" }
  return $null
}

function Get-LabCleanRefusal {
  param([string]$Deploy, [string]$RepoRoot, [bool]$Exists, [bool]$HasMarker, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($Deploy) -or [string]::IsNullOrWhiteSpace($RepoRoot)) { return 'deploy and repo root are required' }
  $root = $RepoRoot.TrimEnd('\')
  $why = Get-LabRootProblem $root
  if ($why) { return "repo root $why" }
  $expected = "$root\.sandbox-work\deploy"
  if ((ConvertTo-LabPathKey $Deploy) -ne (ConvertTo-LabPathKey $expected)) { return "deploy is not exactly $expected" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  if (-not $Exists) { return $null }
  if (-not $HasMarker) { return "deploy exists without the $script:LabDeployMarker marker (not created by the lab)" }
  return $null
}

function Get-LabOutCleanRefusal {
  param([string]$OutDir, [string]$LabDir, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($OutDir) -or [string]::IsNullOrWhiteSpace($LabDir)) { return 'out dir and lab dir are required' }
  $lab = $LabDir.TrimEnd('\')
  $why = Get-LabRootProblem $lab
  if ($why) { return "lab dir $why" }
  $expected = "$lab\shared\out"
  if ((ConvertTo-LabPathKey $OutDir) -ne (ConvertTo-LabPathKey $expected)) { return "out dir is not exactly $expected" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  return $null
}
```

Observação: `"$D."` ("...\deploy.") e `"$D\..\.."` diferem da chave esperada e caem em "not exactly"; `'Z:\'` vira `'Z:'` depois do `TrimEnd` e cai em "repo root must be…".

- [ ] **Step 4: Rodar e ver passar; mutação**

Run: `pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1; "exit=$LASTEXITCODE"`
Expected: 11 linhas `ok - ...`, `all passed`, `exit=0`.

Mutação: em `Get-LabCleanRefusal`, trocar `if ($l.Count) {` por `if ($false) {`. Rodar; colar `FAILED - clean: a link wins over the marker: ...`. Restaurar; `all passed`.

- [ ] **Step 5: `lib/disco.ps1`**

```powershell
# Disk helpers for the lab guards. Reading attributes only, except Clear-LabOut, which deletes inside shared\out after
# the pure guard (travas.ps1) said yes. Dot-source travas.ps1 first.

function Get-LabLinksInPath([string]$Dir, [string]$StopAt) {
  # Reparse points from $Dir up to, not including, $StopAt. Missing folders are not links.
  $links = @()
  $cur = $Dir
  $stop = ConvertTo-LabPathKey $StopAt
  while ((ConvertTo-LabPathKey $cur) -ne $stop) {
    $item = Get-Item -LiteralPath $cur -Force -ErrorAction SilentlyContinue
    if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { $links += $cur }
    $parent = Split-Path $cur -Parent
    if (-not $parent -or $parent -eq $cur) { throw "$Dir is not inside $StopAt" }
    $cur = $parent
  }
  return ,$links
}

function Get-LabReparsePoints([string]$Dir) {
  # Every reparse point at or under $Dir, without ever descending into one (same behavior on PowerShell 5.1 and 7).
  $found = @()
  $stack = New-Object System.Collections.Stack
  $stack.Push($Dir)
  while ($stack.Count) {
    $d = $stack.Pop()
    $item = Get-Item -LiteralPath $d -Force -ErrorAction SilentlyContinue
    if (-not $item) { continue }
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { $found += $d; continue }
    if (-not $item.PSIsContainer) { continue }
    foreach ($e in [IO.Directory]::EnumerateFileSystemEntries($d)) { $stack.Push($e) }
  }
  return ,$found
}

function Clear-LabOut([string]$OutDir, [string]$LabDir) {
  $links = @(Get-LabLinksInPath $OutDir $LabDir) + @(Get-LabReparsePoints $OutDir)
  $refusal = Get-LabOutCleanRefusal -OutDir $OutDir -LabDir $LabDir -Links $links
  if ($refusal) { throw "Refusing to clean ${OutDir}: $refusal" }
  Get-ChildItem -LiteralPath $OutDir -Force | Where-Object { $_.Name -ne '.gitkeep' } | Remove-Item -Recurse -Force
}
```

- [ ] **Step 6: `app-stage.ps1` com a trava antes de qualquer escrita**

Substituir, em `sandbox-lab/scripts/app-stage.ps1`, do comentário `.PARAMETER WorkDir` até a linha `New-Item -ItemType Directory -Force "$Deploy\app", ...` (inclusive) por:

```powershell
.PARAMETER WorkDir       optional; must be exactly <repo root>\.sandbox-work (the default)
.PARAMETER Clean         wipes the previous deploy before publishing (only a deploy carrying the .lab-deploy marker)
#>
param(
  [Parameter(Mandatory)][string]$App,
  [string[]]$AppData = @(),
  [string[]]$LocalAppData = @(),
  [string]$WorkDir,
  [switch]$Clean
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
function Copy-Tree([string]$from, [string]$to) {
  robocopy $from $to /MIR /NFL /NDL /NP /R:1 /W:1 | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "robocopy falhou ($LASTEXITCODE): $from -> $to" }
}
# The lab always lives in <repo root>\sandbox-lab, so the repo root is two levels above this script.
$RepoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$Expected = Join-Path $RepoRoot '.sandbox-work'
if ($WorkDir -and ((ConvertTo-LabPathKey ([IO.Path]::GetFullPath($WorkDir))) -ne (ConvertTo-LabPathKey $Expected))) {
  throw "WorkDir must be exactly $Expected (got $WorkDir)."
}
$WorkDir = $Expected
$Deploy = Join-Path $WorkDir 'deploy'
# robocopy /MIR below also deletes inside deploy, so the guard runs with or without -Clean.
$exists = Test-Path -LiteralPath $Deploy
$links = @(Get-LabLinksInPath $Deploy $RepoRoot) + @(Get-LabReparsePoints $Deploy)
$refusal = Get-LabCleanRefusal -Deploy $Deploy -RepoRoot $RepoRoot -Exists $exists `
  -HasMarker ($exists -and (Test-Path -LiteralPath (Join-Path $Deploy $script:LabDeployMarker))) -Links $links
if ($refusal) { throw "Refusing to touch ${Deploy}: $refusal" }
if ($Clean -and $exists) { Remove-Item -LiteralPath $Deploy -Recurse -Force }
New-Item -ItemType Directory -Force "$Deploy\app", "$Deploy\appdata", "$Deploy\localappdata", (Join-Path $WorkDir 'guest-rw') | Out-Null
[IO.File]::WriteAllText((Join-Path $Deploy $script:LabDeployMarker), '')
```

E trocar a linha `.SYNOPSIS` `Publica o app em %LAB_WORK_DIR%\deploy` por `Publica o app em <raiz do repo>\.sandbox-work\deploy`. O resto do arquivo (cópia do app e das pastas de dados, resumo, `exit 0`) fica igual.

- [ ] **Step 7: `test-up.ps1` usa `Clear-LabOut`**

Logo depois de `$ErrorActionPreference = 'Stop'`, acrescentar:

```powershell
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
```

Trocar o bloco

```powershell
# --- limpa shared\out ---
Get-ChildItem $OutDir -Force | Where-Object { $_.Name -ne '.gitkeep' } | Remove-Item -Recurse -Force
```

por

```powershell
# --- limpa shared\out (only the exact <lab>\shared\out, no link in it or under it) ---
Clear-LabOut -OutDir $OutDir -LabDir $Root
```

- [ ] **Step 8: Teste Sandbox `travas.sandbox.ps1`** (escrever; roda só na Tarefa 9)

```powershell
# SANDBOX ONLY. Creates folders and junctions under C:\labdata and runs app-stage.ps1 / Clear-LabOut against fake
# checkouts there. Refuses to run anywhere else. Nothing is cleaned up: the guest is discarded by test-down.
# Run through: node testes-no-guest.mjs --ps sandbox-lab/scripts/test/travas.sandbox.ps1
$ErrorActionPreference = 'Stop'
if ($env:USERNAME -ne 'WDAGUtilityAccount') { Write-Host 'REFUSED: this test runs only inside Windows Sandbox'; exit 3 }
$Base = Join-Path 'C:\labdata' ('travas-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Base | Out-Null
$Lib = Join-Path $PSScriptRoot '..\lib'
. (Join-Path $Lib 'travas.ps1')
. (Join-Path $Lib 'disco.ps1')
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function New-Sentinel([string]$Dir) { New-Item -ItemType Directory -Force $Dir | Out-Null; $f = Join-Path $Dir 'keep.txt'; [IO.File]::WriteAllText($f, 'keep'); return $f }
function New-FakeRepo([string]$Name) {
  $repo = Join-Path $Base $Name
  $scripts = Join-Path $repo 'sandbox-lab\scripts'
  New-Item -ItemType Directory -Path (Join-Path $scripts 'lib') | Out-Null
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\app-stage.ps1') -Destination $scripts
  Copy-Item -LiteralPath (Join-Path $Lib 'travas.ps1'), (Join-Path $Lib 'disco.ps1') -Destination (Join-Path $scripts 'lib')
  return $repo
}
$App = Join-Path $Base 'app x.exe'
[IO.File]::WriteAllText($App, 'x')
function Invoke-Stage([string]$Repo, [string[]]$Extra = @()) {
  $ErrorActionPreference = 'Continue'   # native stderr must not throw here
  $stage = Join-Path $Repo 'sandbox-lab\scripts\app-stage.ps1'
  $null = & powershell -NoProfile -ExecutionPolicy Bypass -File $stage -App $App @Extra 2>&1
  return $LASTEXITCODE
}
function Should-Refuse([scriptblock]$Body) { $threw = $false; try { & $Body } catch { $threw = $true }; if (-not $threw) { throw 'expected a refusal' } }

Check 'first publish creates deploy, the marker and copies the app' {
  $r = New-FakeRepo 'a'
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'app-stage failed' }
  if (-not (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\.lab-deploy'))) { throw 'marker missing' }
  if (-not (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\app\app x.exe'))) { throw 'app not copied' }
}
Check 'second publish with the marker wipes the old deploy' {
  $r = New-FakeRepo 'b'
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'first publish failed' }
  $old = New-Sentinel (Join-Path $r '.sandbox-work\deploy\velho')
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'second publish failed' }
  if (Test-Path -LiteralPath $old) { throw 'old file survived -Clean' }
}
Check 'deploy without the marker is refused and kept (with -Clean)' {
  $r = New-FakeRepo 'c'
  $keep = New-Sentinel (Join-Path $r '.sandbox-work\deploy')
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'file deleted' }
}
Check 'deploy without the marker is refused and kept (without -Clean: robocopy /MIR also deletes)' {
  $r = New-FakeRepo 'c2'
  $keep = New-Sentinel (Join-Path $r '.sandbox-work\deploy\app')
  if ((Invoke-Stage $r) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'file deleted' }
}
Check 'deploy as a junction is refused, target kept' {
  $r = New-FakeRepo 'd'
  $target = Join-Path $Base 'alvo-d'; $keep = New-Sentinel $target
  [IO.File]::WriteAllText((Join-Path $target '.lab-deploy'), '')
  New-Item -ItemType Directory -Path (Join-Path $r '.sandbox-work') | Out-Null
  New-Item -ItemType Junction -Path (Join-Path $r '.sandbox-work\deploy') -Target $target | Out-Null
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check '.sandbox-work as a junction is refused, target kept' {
  $r = New-FakeRepo 'e'
  $target = Join-Path $Base 'alvo-e'; $keep = New-Sentinel (Join-Path $target 'deploy')
  [IO.File]::WriteAllText((Join-Path $target 'deploy\.lab-deploy'), '')
  New-Item -ItemType Junction -Path (Join-Path $r '.sandbox-work') -Target $target | Out-Null
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check 'a junction inside deploy is refused, target kept' {
  $r = New-FakeRepo 'f'
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'first publish failed' }
  $target = Join-Path $Base 'alvo-f'; $keep = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $r '.sandbox-work\deploy\app\link') -Target $target | Out-Null
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check 'a different -WorkDir is refused' {
  $r = New-FakeRepo 'g'
  $keep = New-Sentinel (Join-Path $Base 'outro-work\deploy')
  if ((Invoke-Stage $r @('-WorkDir', (Join-Path $Base 'outro-work'), '-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'file deleted' }
}
Check 'Clear-LabOut empties shared\out but keeps .gitkeep' {
  $lab = Join-Path $Base 'lab-h'; $out = Join-Path $lab 'shared\out'
  $f = New-Sentinel (Join-Path $out 'sub'); [IO.File]::WriteAllText((Join-Path $out '.gitkeep'), '')
  Clear-LabOut -OutDir $out -LabDir $lab
  if (Test-Path -LiteralPath $f) { throw 'out not cleaned' }
  if (-not (Test-Path -LiteralPath (Join-Path $out '.gitkeep'))) { throw '.gitkeep removed' }
}
Check 'Clear-LabOut refuses a junction inside out, target kept' {
  $lab = Join-Path $Base 'lab-i'; $out = Join-Path $lab 'shared\out'
  New-Item -ItemType Directory -Force $out | Out-Null
  $target = Join-Path $Base 'alvo-i'; $keep = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $out 'link') -Target $target | Out-Null
  Should-Refuse { Clear-LabOut -OutDir $out -LabDir $lab }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check 'Clear-LabOut refuses shared\out as a junction, target kept' {
  $lab = Join-Path $Base 'lab-j'
  New-Item -ItemType Directory -Force (Join-Path $lab 'shared') | Out-Null
  $target = Join-Path $Base 'alvo-j'; $keep = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $lab 'shared\out') -Target $target | Out-Null
  Should-Refuse { Clear-LabOut -OutDir (Join-Path $lab 'shared\out') -LabDir $lab }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
```

- [ ] **Step 9: Conferir a sintaxe de todos os `.ps1` tocados (parser, sem executar)**

```powershell
$e = $null; $falhou = 0
foreach ($f in 'sandbox-lab/scripts/lib/travas.ps1','sandbox-lab/scripts/lib/disco.ps1','sandbox-lab/scripts/test/travas.test.ps1','sandbox-lab/scripts/test/travas.sandbox.ps1','sandbox-lab/scripts/app-stage.ps1','sandbox-lab/scripts/test-up.ps1') {
  [void][Management.Automation.Language.Parser]::ParseFile((Resolve-Path $f).Path, [ref]$null, [ref]$e)
  if ($e) { "$f : $($e[0].Message)"; $falhou = 1 }
}
"parse exit=$falhou"
```

Expected: só `parse exit=0`.

- [ ] **Step 10: Conferência de segredo e commit**

```powershell
git add -- sandbox-lab/scripts/lib/travas.ps1 sandbox-lab/scripts/lib/disco.ps1 sandbox-lab/scripts/test/travas.test.ps1 sandbox-lab/scripts/test/travas.sandbox.ps1 sandbox-lab/scripts/app-stage.ps1 sandbox-lab/scripts/test-up.ps1
```

Conferência de segredo. Depois:

```powershell
git commit -m "feat(sandbox-lab): exact-path, marker and reparse-point guard before app-stage touches deploy; guarded shared\out cleanup" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: perfis relativos, `-Project`, ferramentas, `mcp-sync`, `validate-config`, `guest-build`

Modelo: **Sonnet**. Roda no PC: `travas.test.ps1` (puro) e o parser.

**Files:**
- Modify: `sandbox-lab/scripts/lib/travas.ps1` (acrescentar `Get-LabMissingTools`)
- Modify: `sandbox-lab/scripts/lib/disco.ps1` (acrescentar `Test-LabTools`)
- Modify: `sandbox-lab/scripts/test/travas.test.ps1` (acrescentar testes antes do bloco final)
- Modify: `sandbox-lab/scripts/test-up.ps1` (`-Project`, ferramentas)
- Modify: `sandbox-lab/scripts/mcp-sync.ps1` (sem `-Literal`)
- Modify: `sandbox-lab/scripts/validate-config.ps1`
- Modify: `sandbox-lab/lab.config.app.json`, `lab.config.app-winapp.json`, `lab.config.app-build.json`
- Modify: `sandbox-lab/sandbox/app/guest-build.ps1` (linha do `robocopy C:\labproject`)
- Modify: `sandbox-lab/.gitignore`

**Interfaces:**
- Consumes: `travas.ps1`, `disco.ps1` (Tarefa 6); formato do manifesto (Tarefa 1).
- Produces: `Get-LabMissingTools -ManifestText <string> -FoundSizes <hashtable> -> string[]` (vazio = tudo certo); `Test-LabTools -LabDir <string> -> string[]`; `test-up.ps1 -Project <caminho absoluto>` (prioridade sobre `projectPath`).

- [ ] **Step 1: Testes que falham para `Get-LabMissingTools`**

Acrescentar em `travas.test.ps1`, antes de `if ($script:failed)`:

```powershell
$H = 'a' * 64
$M = "$H 10 chrome/chrome-win64/chrome.exe`n$H 3 uv/uv.exe`n"
Check 'tools: everything present with the right size' {
  $r = Get-LabMissingTools -ManifestText $M -FoundSizes @{ 'chrome/chrome-win64/chrome.exe' = 10; 'uv/uv.exe' = 3; 'extra.txt' = 1 }
  if ($r.Count) { throw "unexpected: $($r -join '; ')" }
}
Check 'tools: missing and size differences are listed' {
  $r = Get-LabMissingTools -ManifestText $M -FoundSizes @{ 'uv/uv.exe' = 4 }
  if ($r.Count -ne 2 -or $r[0] -notmatch '^missing: chrome/' -or $r[1] -notmatch '^size differs: uv/uv\.exe') { throw "got: $($r -join '; ')" }
}
Check 'tools: letter case does not matter' {
  $r = Get-LabMissingTools -ManifestText $M -FoundSizes @{ 'Chrome/Chrome-Win64/Chrome.exe' = 10; 'UV/uv.exe' = 3 }
  if ($r.Count) { throw "unexpected: $($r -join '; ')" }
}
Check 'tools: an empty or malformed manifest is a problem, never a pass' {
  Should-Match ((Get-LabMissingTools -ManifestText '' -FoundSizes @{}) -join ';') 'empty'
  Should-Match ((Get-LabMissingTools -ManifestText "lixo`n" -FoundSizes @{}) -join ';') 'malformed'
}
```

Run: `pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1; "exit=$LASTEXITCODE"`
Expected: 4 linhas `FAILED - tools: ...` (`Get-LabMissingTools` não reconhecido) e `exit=1`.

- [ ] **Step 2: Implementar `Get-LabMissingTools` (fim de `travas.ps1`)**

```powershell
function Get-LabMissingTools {
  # Compares shared\tools.sha256 with the files found (relative path with '/', size). Size only: hashing ~600 MB on
  # every test-up is too slow; validate-config and the copy check use the hash. Extra files are not a problem.
  param([string]$ManifestText, [hashtable]$FoundSizes)
  $problems = New-Object System.Collections.Generic.List[string]
  $entries = 0
  foreach ($line in ($ManifestText -split "`r?`n")) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line -notmatch '^([0-9a-f]{64}) (\d+) (.+)$') { $problems.Add("malformed manifest line: $line"); continue }
    $entries++
    $size = [long]$Matches[2]; $rel = $Matches[3]
    $key = @($FoundSizes.Keys | Where-Object { $_ -ieq $rel }) | Select-Object -First 1
    if ($null -eq $key) { $problems.Add("missing: $rel") }
    elseif ([long]$FoundSizes[$key] -ne $size) { $problems.Add("size differs: $rel") }
  }
  if ($entries -eq 0 -and $problems.Count -eq 0) { $problems.Add('manifest is empty') }
  return ,$problems.ToArray()
}
```

A busca da chave com `-ieq` percorre a tabela a cada linha; com ~2.600 arquivos é O(n²) mas em memória. Se ficar lento no `test-up` (mais de 5 s), trocar por um `Dictionary[string,long]` com `StringComparer.OrdinalIgnoreCase` montado por quem chama, mantendo os testes.

Run: `pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1; "exit=$LASTEXITCODE"`
Expected: 15 `ok`, `all passed`, `exit=0`.

Mutação: trocar `$problems.Add('manifest is empty')` por `$null = 0`. Rodar; colar `FAILED - tools: an empty or malformed manifest is a problem, never a pass`. Restaurar.

- [ ] **Step 3: `Test-LabTools` (fim de `disco.ps1`)**

```powershell
function Test-LabTools([string]$LabDir) {
  # Problems with shared\tools against shared\tools.sha256 (empty array = fine). Reads only.
  $manifest = Join-Path $LabDir 'shared\tools.sha256'
  if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { return ,@('manifest not found: shared\tools.sha256') }
  $toolsDir = Join-Path $LabDir 'shared\tools'
  $found = @{}
  if (Test-Path -LiteralPath $toolsDir -PathType Container) {
    $base = (Get-Item -LiteralPath $toolsDir).FullName.TrimEnd('\') + '\'
    Get-ChildItem -LiteralPath $toolsDir -Recurse -File -Force | ForEach-Object { $found[$_.FullName.Substring($base.Length).Replace('\', '/')] = $_.Length }
  }
  return ,(Get-LabMissingTools -ManifestText (Get-Content -LiteralPath $manifest -Raw) -FoundSizes $found)
}
```

- [ ] **Step 4: `test-up.ps1`: `-Project` e ferramentas**

1. No bloco de ajuda, acrescentar depois de `.PARAMETER Config`:

```
.PARAMETER Project      Pasta do projeto montada read-only em C:\labproject (absoluta). Tem prioridade sobre o
                        projectPath do perfil. A skill claudia-teste-sandbox passa o checkout atual (onde.mjs: projeto).
```

2. No `param(...)`, acrescentar `[string]$Project` depois de `[string]$Config = 'lab.config.json'` (com vírgula na linha anterior).
3. Trocar `$ProjectDir = Resolve-LabPath $cfg.projectPath` por:

```powershell
$ProjectDir = $(if ($Project) {
  if (-not [IO.Path]::IsPathRooted($Project)) { throw "-Project must be an absolute path: $Project" }
  [IO.Path]::GetFullPath($Project)
} else { Resolve-LabPath $cfg.projectPath })
```

4. Logo depois do `foreach ($d in @($SharedDir, ...)) { ... }`, acrescentar:

```powershell
if (-not $GenerateOnly) {
  $missingTools = Test-LabTools -LabDir $Root
  if ($missingTools.Count) {
    throw ("[test-up] ferramentas faltando ou diferentes em shared\tools ({0}): {1}. Nada foi baixado e nenhum Sandbox subiu. Como obter: sandbox-lab\README.md, secao Ferramentas." -f $missingTools.Count, (($missingTools | Select-Object -First 10) -join '; '))
  }
}
```

- [ ] **Step 5: Perfis com caminhos relativos**

Nos três `lab.config.app*.json`:
- `"projectPath": "%LAB_APP_PROJECT%"` → `"projectPath": ".."` (raiz do checkout principal; a skill passa `-Project`).
- `"host": "%LAB_WORK_DIR%\\deploy"` → `"host": "..\\.sandbox-work\\deploy"`.
- `"host": "%LAB_WORK_DIR%\\guest-rw"` → `"host": "..\\.sandbox-work\\guest-rw"`.
- No `$comment`, trocar `Requer LAB_APP_PROJECT e LAB_WORK_DIR (ver skill sandbox-lab).` / `Requer LAB_APP_PROJECT e LAB_WORK_DIR.` por `Projeto: -Project do test-up.ps1 (padrao: a raiz do repositorio). Pasta de trabalho: <raiz>\\.sandbox-work (publicar.mjs / app-stage.ps1).`

As entradas `%ProgramFiles%`, `%ProgramFiles(x86)%` e `%USERPROFILE%` do perfil de build ficam (variáveis padrão do Windows para as toolchains do host).

Run: `Get-Content sandbox-lab/lab.config.app.json, sandbox-lab/lab.config.app-winapp.json, sandbox-lab/lab.config.app-build.json -Raw | ForEach-Object { $_ | ConvertFrom-Json | Out-Null }; Select-String -Path sandbox-lab/lab.config*.json -Pattern 'LAB_'; "exit=$LASTEXITCODE"`
Expected: nenhuma linha do `Select-String` (o JSON parseia sem erro).

- [ ] **Step 6: `mcp-sync.ps1` sem `-Literal`**

- Apagar do bloco de ajuda a linha `  ou use -Literal para gravar a chave literal (o .mcp.json passa a conter segredo).` e a linha `.PARAMETER Literal  Grava a chave literal no header em vez de ${LAB_MCP_AUTH_KEY}.`
- No `param(...)`, apagar `[switch]$Literal,`.
- Trocar `$auth = if ($Literal) { "Bearer $key" } else { "Bearer `${$envName}" }` por `$auth = "Bearer `${$envName}"   # the key itself never goes to .mcp.json`.
- Trocar `Write-Host ".mcp.json -> $url (auth: $(if ($Literal) { 'chave literal' } else { "`${$envName}" }))"` por `Write-Host ".mcp.json -> $url (auth: `${$envName})"`.
- Trocar `if (-not $Literal -and -not [Environment]::GetEnvironmentVariable($envName)) {` por `if (-not [Environment]::GetEnvironmentVariable($envName)) {`.

Run: `Select-String -Path sandbox-lab/scripts/mcp-sync.ps1 -Pattern 'Literal'`
Expected: nenhuma linha.

- [ ] **Step 7: `validate-config.ps1`**

1. Depois de `$Root = Split-Path $PSScriptRoot -Parent`, acrescentar:

```powershell
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
```

2. Antes do comentário `# 2. scripts PowerShell parseiam`, acrescentar:

```powershell
# 1b. ferramentas (shared\tools) batem com o manifesto versionado
$missingTools = Test-LabTools -LabDir $Root
if ($missingTools.Count) { Add-Result FAIL 'shared/tools bate com shared/tools.sha256' ("{0}: {1}" -f $missingTools.Count, (($missingTools | Select-Object -First 5) -join '; ')) }
else { Add-Result PASS 'shared/tools bate com shared/tools.sha256' }
```

3. Em `# 2.`, trocar `Get-ChildItem (Join-Path $Root 'scripts'), (Join-Path $Root 'sandbox') -Filter *.ps1` por `Get-ChildItem (Join-Path $Root 'scripts'), (Join-Path $Root 'scripts\lib'), (Join-Path $Root 'scripts\test'), (Join-Path $Root 'sandbox'), (Join-Path $Root 'sandbox\app') -Filter *.ps1`.
4. Em `# 5. skills`, trocar `$skillsDir = Join-Path $Root '.claude\skills'` por `$skillsDir = Join-Path (Split-Path $Root -Parent) '.claude\skills'   # skills live in the repo root`.

- [ ] **Step 8: `guest-build.ps1`: o guest não copia ferramentas, chave, pasta de trabalho nem worktrees**

Trocar a linha `robocopy C:\labproject $Src /E /XD node_modules target dist .git /NFL /NDL /NP /R:1 /W:1 | Out-Null` por:

```powershell
# Full paths in /XD exclude only these folders of the project root, not any folder with the same name.
robocopy C:\labproject $Src /E /XD node_modules target dist .git C:\labproject\sandbox-lab\shared C:\labproject\.sandbox-work C:\labproject\.claude\worktrees /NFL /NDL /NP /R:1 /W:1 | Out-Null
```

Desvio consciente da spec (P1 dizia excluir `.claude` inteiro): os testes da skill estão em `.claude/skills/` e precisam ir para o guest (Tarefa 9). Sai só `.claude\worktrees`. **Precisa do sim do usuário** (ver fim do plano).

- [ ] **Step 9: `sandbox-lab/.gitignore`**

Substituir o conteúdo por:

```gitignore
# Secrets, pre-downloaded binaries, guest/test output and generated files.
# The repository root .gitignore repeats these on purpose (two layers).
shared/secrets/
shared/tools/
shared/out/
evidence/
sandbox/sandbox.generated.wsb
sandbox/guest.config.json
.mcp.json
```

- [ ] **Step 10: Parser e busca de variáveis**

```powershell
$e = $null; $falhou = 0
foreach ($f in Get-ChildItem sandbox-lab/scripts, sandbox-lab/scripts/lib, sandbox-lab/scripts/test, sandbox-lab/sandbox, sandbox-lab/sandbox/app -Filter *.ps1) {
  [void][Management.Automation.Language.Parser]::ParseFile($f.FullName, [ref]$null, [ref]$e)
  if ($e) { "$($f.Name): $($e[0].Message)"; $falhou = 1 }
}
"parse exit=$falhou"
Select-String -Path sandbox-lab/scripts/*.ps1, sandbox-lab/scripts/lib/*.ps1, sandbox-lab/lab.config*.json -Pattern 'LAB_APP_PROJECT|LAB_WORK_DIR|SANDBOX_LAB_DIR' | Select-Object Filename, LineNumber
```

Expected: `parse exit=0` e nenhuma linha da busca.

Run: `pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1; "exit=$LASTEXITCODE"` → `all passed`, `exit=0`.

- [ ] **Step 11: Conferência de segredo e commit**

```powershell
git add -- sandbox-lab/scripts/lib/travas.ps1 sandbox-lab/scripts/lib/disco.ps1 sandbox-lab/scripts/test/travas.test.ps1 sandbox-lab/scripts/test-up.ps1 sandbox-lab/scripts/mcp-sync.ps1 sandbox-lab/scripts/validate-config.ps1 sandbox-lab/lab.config.app.json sandbox-lab/lab.config.app-winapp.json sandbox-lab/lab.config.app-build.json sandbox-lab/sandbox/app/guest-build.ps1 sandbox-lab/.gitignore
```

Conferência de segredo. Depois:

```powershell
git commit -m "feat(sandbox-lab): repo-relative profiles, -Project, tools manifest check, no literal MCP key, guest-build exclusions" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: skills e docs

Modelo: **Haiku**. Só texto e `git mv`. Nada roda além de buscas.

**Files:**
- Move: `sandbox-lab/.claude/skills/winapp-sandbox/` → `.claude/skills/winapp-sandbox/`
- Move: `sandbox-lab/.claude/skills/winapp-ui-automation/` → `.claude/skills/winapp-ui-automation/`
- Move: `sandbox-lab/.claude/skills/WINAPP-LICENSE.txt` → `.claude/skills/WINAPP-LICENSE.txt`
- Move + Modify: `sandbox-lab/.claude/skills/sandbox-lab/SKILL.md` → `.claude/skills/claudia-teste-sandbox/references/laboratorio.md`
- Move + Modify: `sandbox-lab/CLAUDE.md` → `sandbox-lab/README.md`
- Modify: `.claude/skills/claudia-teste-sandbox/SKILL.md`, `.claude/skills/claudia-teste-sandbox/references/limites.md`, `CLAUDE.md` (raiz)

**Interfaces:**
- Consumes (só nomes, para citar nos docs): `onde.mjs` (JSON `{raizPrincipal, projeto, lab, trabalho, deploy, guestRw, appStage, testUp}`), `publicar.mjs --app --cenario`, `banco-guest.mjs [--id] [--timeout]`, `testes-no-guest.mjs`, `test-up.ps1 -Config -Project`, `sandbox-lab/shared/tools.sha256`, `conferir-copia.mjs`.
- Produces: docs.

- [ ] **Step 1: Mover com `git mv` (versionados; nada é apagado)**

```powershell
git mv sandbox-lab/.claude/skills/winapp-sandbox .claude/skills/winapp-sandbox
git mv sandbox-lab/.claude/skills/winapp-ui-automation .claude/skills/winapp-ui-automation
git mv sandbox-lab/.claude/skills/WINAPP-LICENSE.txt .claude/skills/WINAPP-LICENSE.txt
git mv sandbox-lab/.claude/skills/sandbox-lab/SKILL.md .claude/skills/claudia-teste-sandbox/references/laboratorio.md
git mv sandbox-lab/CLAUDE.md sandbox-lab/README.md
git status --porcelain
```

Expected: 6 linhas `R ` (renomeações). Os `SKILL.md` das winapp não mudam de conteúdo (licença MIT preservada ao lado).

- [ ] **Step 2: `references/laboratorio.md`**

1. Apagar o frontmatter (as linhas entre os dois `---` iniciais, inclusive).
2. Trocar o título `# sandbox-lab` e as duas linhas seguintes por:

```markdown
# Laboratório Windows Sandbox (referência)

Origem: skill `sandbox-lab` do laboratório, trazida para o repositório em 2026-10-07 (spec `docs/superpowers/specs/2026-10-07-laboratorio-no-repo-design.md`). **Onde este texto e o `CLAUDE.md` da raiz ou a `SKILL.md` da `claudia-teste-sandbox` divergem, valem eles.**

O laboratório fica em `sandbox-lab/` do checkout principal (`node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs` → `lab`). Contrato e critérios de aprovação do laboratório genérico: `sandbox-lab/OBJECTIVES.md`.
```

3. Trocas de texto, uma a uma:
   - `Skills detalhadas: \`winapp-sandbox\` e \`winapp-ui-automation\` (nesta pasta).` → `Skills detalhadas: \`.claude/skills/winapp-sandbox\` e \`.claude/skills/winapp-ui-automation\`.`
   - `Caminhos nos perfis aceitam variaveis de ambiente (\`%USERPROFILE%\`).` → `Caminhos nos perfis são relativos a \`sandbox-lab/\`; o perfil de build usa \`%USERPROFILE%\` e \`%ProgramFiles%\` só para as toolchains do host.`
   - O item que começa com `- Variaveis de ambiente no host: \`LAB_APP_PROJECT\`` (duas linhas mais a linha `Use copias: ...`) → `- Projeto: \`test-up.ps1 -Project <pasta>\` (a skill passa o checkout atual; montado read-only em \`C:\labproject\`). Pasta de trabalho: \`.sandbox-work/\` do checkout principal (\`deploy\\\` vira \`C:\appdeploy\` read-only, \`guest-rw\\\` vira \`C:\labwork\` gravável). Sem variável de ambiente.`
   - `Publicar o app: \`scripts\app-stage.ps1 -App <exe ou pasta> [-AppData <pasta>...] [-LocalAppData <pasta>...]\`.` → `Publicar o app: \`node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app <exe|instalador|pasta> --cenario <pasta>\` (chama \`scripts\app-stage.ps1 -Clean\` com a trava do marcador \`.lab-deploy\`).`
   - `Saidas em \`C:\labwork\...\` (= \`%LAB_WORK_DIR%\guest-rw\...\` no host).` → `Saidas em \`C:\labwork\...\` (= \`.sandbox-work\guest-rw\...\` no host).`
   - `- Host: \`evidence\\\` (screenshots, relatorios)` → `- Host: \`sandbox-lab/evidence/\` (ignorada pelo git; screenshots, relatorios)`
   - Na seção MCP, depois do bloco de código, acrescentar: `O \`.mcp.json\` do laboratório (\`sandbox-lab/.mcp.json\`) é gerado pelo \`mcp-sync.ps1\`, fica fora do git e nunca contém a chave (a opção \`-Literal\` foi removida).`

Run: `Select-String -Path .claude/skills/claudia-teste-sandbox/references/laboratorio.md -Pattern 'LAB_APP_PROJECT|LAB_WORK_DIR|SANDBOX_LAB_DIR|nesta pasta|-Literal'`
Expected: só a linha nova que cita "`-Literal` foi removida".

- [ ] **Step 3: `sandbox-lab/README.md`**

Substituir o conteúdo inteiro por:

```markdown
# sandbox-lab

Laboratório Windows Sandbox do claudia-rh: um Windows limpo e descartável para testar o app, controlado por scripts e por IA.

- **Como usar:** skill `claudia-teste-sandbox` (`.claude/skills/claudia-teste-sandbox/SKILL.md`). Referência completa do laboratório: `.claude/skills/claudia-teste-sandbox/references/laboratorio.md`.
- **Objetivos do laboratório genérico (O1–O10):** `OBJECTIVES.md`.
- **Onde fica:** sempre o `sandbox-lab/` do checkout principal. De uma worktree, a skill usa este laboratório e monta a worktree como projeto (`onde.mjs`).

## O que está no git e o que não está

| Versionado | Fora do git (ignorado) |
|---|---|
| `scripts/`, `sandbox/` (sem os gerados), `lab.config*.json`, `sample-project/`, `shared/db/`, `shared/extensions/`, `shared/mcp.json`, `shared/test-page.html`, `shared/tools.sha256` | `shared/tools/` (~592 MB), `shared/secrets/` (chave do MCP), `shared/out/`, `evidence/`, `sandbox/sandbox.generated.wsb`, `sandbox/guest.config.json`, `.mcp.json`, e `.sandbox-work/` na raiz do repositório |

## Ferramentas

`shared/tools/` (Chrome for Testing, Python portátil, `uv`, wheels do `windows-mcp`, `sqlite`) não está no git. A lista de arquivos com tamanho e SHA-256 está em `shared/tools.sha256`. Se faltar algo, o `test-up.ps1` para antes de subir o Sandbox e diz o quê. Para obter: copiar da pasta antiga do laboratório para `shared/tools/` (destino novo, sem `/MIR`) e conferir com `node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --manifesto sandbox-lab/shared/tools.sha256 --copia sandbox-lab/shared/tools`. Um script que baixa as ferramentas é trabalho futuro (Linear).

## Regras

- **Nunca** automatize o desktop do host: winapp sempre com `--on sandbox`; MCP só o `windows-sandbox` (roda no guest).
- Só existe um Sandbox por vez: suba com `scripts\test-up.ps1`, derrube com `scripts\test-down.ps1`.
- `shared\` é read-only no guest (exceto `shared\out\`). Nunca escreva em `shared\db\backup.sqlite`.
- A chave do MCP fica em `shared\secrets\` e nunca entra no git. Depois de cada boot, `scripts\mcp-sync.ps1` atualiza o IP no `.mcp.json` (que usa `${LAB_MCP_AUTH_KEY}`, nunca a chave).
- Regras do repositório (`CLAUDE.md` da raiz) valem sobre este arquivo.
```

- [ ] **Step 4: `SKILL.md` da `claudia-teste-sandbox`**

1. Frontmatter, `description`: trocar `(Windows Sandbox, laboratório em SANDBOX_LAB_DIR)` por `(Windows Sandbox, laboratório em sandbox-lab/ do repositório)`.
2. Seção 1 inteira (do título `## 1. Pré-requisitos` até antes de `## 2.`) por:

```markdown
## 1. Pré-requisitos

- O laboratório fica em `sandbox-lab/` do **checkout principal** do repositório; a pasta de trabalho, em `.sandbox-work/` do mesmo checkout. Descubra os caminhos com `node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs` (só lê o git) → `{raizPrincipal, projeto, lab, trabalho, deploy, guestRw, appStage, testUp}`. De uma worktree, `projeto` é a worktree e o resto é do checkout principal.
- Sem variável de ambiente de caminho: `SANDBOX_LAB_DIR`, `LAB_APP_PROJECT` e `LAB_WORK_DIR` não existem mais.
- Ferramentas (`sandbox-lab/shared/tools/`, conferidas por `sandbox-lab/shared/tools.sha256`) e a chave do MCP (`sandbox-lab/shared/secrets/`) ficam fora do git. Se o `onde.mjs` falhar ou o `test-up.ps1` disser que falta ferramenta: **pare e diga ao usuário o que falta** (`sandbox-lab/README.md`, seção Ferramentas); não baixe nada nem crie laboratório.
- Windows Sandbox habilitado (`wsb` no PATH). Só existe **um** Sandbox por vez.
```

3. Seção 2: `| Peça (em \`%SANDBOX_LAB_DIR%\`) | Uso |` → `| Peça (em \`lab\` do \`onde.mjs\`) | Uso |`; a linha `| \`scripts\test-up.ps1 -Config <perfil>\` / ...` → `| \`scripts\test-up.ps1 -Config <perfil> -Project <projeto>\` / \`scripts\test-down.ps1\` | sobe / derruba (descarta tudo do guest); id em \`wsb list\`; \`<projeto>\` vem do \`onde.mjs\` |`; `\`C:\labwork\` (= \`<work-dir>\guest-rw\`, gravável)` → `\`C:\labwork\` (= \`.sandbox-work\guest-rw\`, gravável)`; `Documentação completa: \`%SANDBOX_LAB_DIR%\.claude\skills\sandbox-lab\SKILL.md\`.` → `Documentação completa: \`references/laboratorio.md\`.`
4. Seção 5, primeiro parágrafo: trocar por `\`node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app <exe|instalador|pasta> --cenario <pasta>\`. A pasta de trabalho é sempre \`.sandbox-work/\` do checkout principal (não há \`--work-dir\`). O \`-Clean\` só roda se \`.sandbox-work\deploy\` não existir ou tiver o marcador \`.lab-deploy\`, sem junction nem link no caminho ou dentro dele; a mesma trava está no \`publicar.mjs\` e no \`app-stage.ps1\`. O app vai para \`C:\appdeploy\app\`, os dados para \`C:\appdeploy\appdata\`.`
5. Seção 6, item 3: `[--id <id>] [--work-dir <p>] [--timeout 30]` → `[--id <id>] [--timeout 30]`.
6. Seção 6, "Fluxo típico": `\`test-up.ps1 -Config lab.config.app-winapp.json\`` → `\`test-up.ps1 -Config lab.config.app-winapp.json -Project <projeto do onde.mjs>\``.
7. Seção 8: `(o laboratório tem processo próprio, fora deste repositório)` → `(o laboratório está em \`sandbox-lab/\` e segue o processo deste repositório: spec, plano, revisão)`.
8. Seção 9 inteira (do título até antes de `## Saída`) por:

```markdown
## 9. Testes

Você desenha os testes a partir da pergunta e decide se vale escrever testes automáticos.

- **Puros (rodam no PC e no CI):** `scripts/test/proibidos.test.mjs`, `manifesto.test.mjs`, `caminhos-lab.test.mjs`, `onde.test.mjs`, `testes-no-guest.test.mjs`, `motivo-recusa.test.mjs`, `sandbox.test.mjs`, e `sandbox-lab/scripts/test/travas.test.ps1`. Não tocam o disco.
- **Sandbox (criam ou apagam pastas):** `comum.test.mjs`, `novo.test.mjs`, `mock.test.mjs`, `copia.test.mjs`, `publicar.test.mjs` e `sandbox-lab/scripts/test/travas.sandbox.ps1`. Rodam **só no guest**: `test-up.ps1 -Config lab.config.app-build.json -Project <projeto>` e depois `node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node <arquivos>` ou `--ps sandbox-lab/scripts/test/travas.sandbox.ps1`. Fora do Sandbox, `tempDir()` e o cabeçalho do `.sandbox.ps1` recusam antes de criar qualquer pasta.

```
node --test <arquivo1>.test.mjs <arquivo2>.test.mjs
```

Glob e `node --test <pasta>/` não são permitidos (a forma de pasta falha no Node 24 e o glob incluiria testes de Sandbox).
```

Run: `Select-String -Path .claude/skills/claudia-teste-sandbox/SKILL.md -Pattern 'SANDBOX_LAB_DIR|LAB_WORK_DIR|LAB_APP_PROJECT|work-dir'`
Expected: só a linha da seção 1 que diz que as três variáveis não existem mais.

- [ ] **Step 5: `references/limites.md`**

- Linha 5: `Comandos de host a partir de \`%SANDBOX_LAB_DIR%\`;` → `Comandos de host a partir do \`lab\` do \`onde.mjs\` (\`sandbox-lab/\` do checkout principal);`
- Toda ocorrência de `\`.claude\skills\sandbox-lab\SKILL.md\`` e de `\`sandbox-lab\SKILL.md\`` → `\`references/laboratorio.md\``.
- Item 3: `lido no host em \`<work-dir>\guest-rw\...\`` → `lido no host em \`.sandbox-work\guest-rw\...\``.
- Linha do disco virtual: `mesmo quando a pasta de trabalho (\`SANDBOX_LAB_DIR\`) está em outro disco` → `mesmo quando o repositório (e a pasta de trabalho \`.sandbox-work/\`) está em outro disco`.

Run: `Select-String -Path .claude/skills/claudia-teste-sandbox/references/*.md -Pattern 'SANDBOX_LAB_DIR|LAB_WORK_DIR|LAB_APP_PROJECT|<work-dir>'`
Expected: nenhuma linha.

- [ ] **Step 6: `CLAUDE.md` da raiz**

- Regra "Nada de caminho absoluto de máquina": `Use variáveis de ambiente (\`SANDBOX_LAB_DIR\`, \`CARGO_TARGET_DIR\`, \`USERPROFILE\`)` → `Use variáveis de ambiente (\`CARGO_TARGET_DIR\`, \`USERPROFILE\`)`.
- Seção "Teste em máquina limpa", parágrafo depois da tabela: trocar a primeira frase (`A \`claudia-teste-sandbox\` usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável \`SANDBOX_LAB_DIR\`.`) por:

```markdown
A `claudia-teste-sandbox` usa o laboratório Windows Sandbox em `sandbox-lab/`, na raiz do repositório. Scripts e perfis são versionados; as ferramentas (`sandbox-lab/shared/tools/`, conferidas por `sandbox-lab/shared/tools.sha256`), a chave do MCP (`sandbox-lab/shared/secrets/`), as saídas e a pasta de trabalho (`.sandbox-work/`) ficam fora do git. De uma worktree, a skill usa o laboratório e a pasta de trabalho do checkout principal e testa o projeto da worktree. Nenhuma variável de ambiente de caminho.
```

O resto do parágrafo (cenários, falha do app) fica igual.

Run: `Select-String -Path CLAUDE.md -Pattern 'SANDBOX_LAB_DIR'`
Expected: nenhuma linha.

- [ ] **Step 7: Conferência de segredo e commit**

```powershell
git add -- .claude/skills/winapp-sandbox/SKILL.md .claude/skills/winapp-ui-automation/SKILL.md .claude/skills/winapp-ui-automation/references/ui-json-envelope.md .claude/skills/WINAPP-LICENSE.txt .claude/skills/claudia-teste-sandbox/references/laboratorio.md .claude/skills/claudia-teste-sandbox/references/limites.md .claude/skills/claudia-teste-sandbox/SKILL.md sandbox-lab/README.md CLAUDE.md
git status --porcelain
```

Expected: as renomeações do Step 1 e as modificações acima, todas preparadas; nenhuma linha `??` nem ` M` sobrando. Conferência de segredo (inclui `conferir-repo.mjs`, que agora varre as skills novas por caminho de máquina). Depois:

```powershell
git commit -m "docs(sandbox-lab): fold the lab skill into claudia-teste-sandbox, bring winapp skills with their MIT license, drop SANDBOX_LAB_DIR from docs" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: verificação no Windows Sandbox

Modelo: **Opus**. Pré-requisitos: Tarefas 0–8 integradas na `feat/laboratorio-no-repo`, unitários puros verdes, e **o usuário integrou a `feat/laboratorio-no-repo` na `feat/browser-cdp-mcp` do checkout principal** (L9): o laboratório só funciona a partir do checkout principal (L4), e as ferramentas e a chave já estão lá desde a Tarefa 2. Uso do Sandbox autorizado pelo usuário nesta rodada.

Os testes rodam a partir de uma worktree da `feat/browser-cdp-mcp` (projeto = worktree), para provar o critério 4 junto.

**Files:**
- Create: `docs/superpowers/evidencias/2026-10-07-laboratorio-no-repo-sandbox.md` (relato, sem caminho de máquina)

- [ ] **Step 1: Onde está tudo**

Run (na worktree): `node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs; "exit=$LASTEXITCODE"`
Expected: `raizPrincipal` = checkout principal, `projeto` = a worktree, `exit=0`. No relato, escrever só "raiz principal" e "worktree", não os caminhos.

- [ ] **Step 2: Laboratório válido**

Run: `pwsh -NoProfile -File <lab>\scripts\validate-config.ps1; "exit=$LASTEXITCODE"` (com `<lab>` do Step 1)
Expected: `shared/tools bate com shared/tools.sha256` PASS, `Resumo: ... 0 FAIL`, `exit=0`.

- [ ] **Step 3: Pasta de trabalho criada pelo fluxo normal**

```powershell
node .claude/skills/claudia-teste-sandbox/scripts/cenario-novo.mjs
node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app sandbox-lab/sample-project/hello.txt --cenario <saida do cenario-novo>
```

Expected: o segundo imprime `{"workDir":...,"deploy":...,"cenario":"novo"}`; `<trabalho>\deploy\.lab-deploy` existe. Isto é uso normal da skill, não teste: o `cenario-novo` só recria pasta dentro da lista branca de `%TEMP%\claudia-teste-sandbox\` (B2), e o `-Clean` do `publicar` passa pela trava desta branch. Se qualquer um recusar, parar e relatar; não contornar.

- [ ] **Step 4: Subir com o perfil de build e o projeto da worktree**

Conferir antes o espaço livre do C: (mais de 10 GB). Depois:
Run: `pwsh -NoProfile -File <lab>\scripts\test-up.ps1 -Config lab.config.app-build.json -Project <projeto>; "exit=$LASTEXITCODE"`
Expected: `[test-up] OK: bootstrap=ok`, `exit=0`.

- [ ] **Step 5: Critério 4 — o guest vê a worktree como projeto e o usuário é o do Sandbox**

```powershell
$id = (node -e "import('./.claude/skills/claudia-teste-sandbox/scripts/comum.mjs').then(m=>console.log(m.sandboxId()))")
wsb exec --id $id -r ExistingLogin -c "powershell -NoProfile -Command `"(Get-Content C:\labproject\.git -Raw) + '|' + `$env:USERNAME | Set-Content C:\labwork\prova.txt`""
Get-Content -LiteralPath (Join-Path <guestRw> 'prova.txt')
```

Expected: `gitdir: .../worktrees/<nome da worktree>|WDAGUtilityAccount`. Isso prova a montagem da worktree e o nome do usuário que `estaNoSandbox` espera.

- [ ] **Step 6: Testes Sandbox da skill**

Run:
```powershell
node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/novo.test.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/mock.test.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/copia.test.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs; "exit=$LASTEXITCODE"
```
Expected: `{"exit":0,"log":...}` e `exit=0`. Ler o `guest-build.log` e colar no relato as linhas `# pass` e `# fail 0`.

- [ ] **Step 7: Teste Sandbox da trava PowerShell**

Run: `node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --ps sandbox-lab/scripts/test/travas.sandbox.ps1; "exit=$LASTEXITCODE"`
Expected: `{"exit":0,...}`; no log, 11 linhas `ok - ...` e `all passed`.

- [ ] **Step 8: Mutação no Sandbox (as travas que apagam)**

Na **worktree** (é o que o guest copia a cada rodada):
a) Em `sandbox-lab/scripts/lib/travas.ps1`, `Get-LabCleanRefusal`: trocar `if ($l.Count) {` por `if ($false) {`. Rodar o Step 7; colar as linhas `FAILED - deploy as a junction is refused, target kept` e `FAILED - a junction inside deploy is refused, target kept`. Restaurar com `git checkout -- sandbox-lab/scripts/lib/travas.ps1` e rodar o Step 7 de novo: `all passed`.
b) Em `caminhos-lab.mjs`, `motivoRecusaLimparDeploy`: trocar `if (linksNoCaminho.length > 0)` por `if (false)`. Rodar o Step 6 só com `publicar.test.mjs`; colar `not ok ... deploy as a junction (even with the marker inside): refused, target kept`. Restaurar com `git checkout -- .claude/skills/claudia-teste-sandbox/scripts/caminhos-lab.mjs` e rodar de novo: `# fail 0`.
c) Em `test/util.mjs`, comentar a chamada `exigirSandbox();` não prova nada no guest (o usuário é o do Sandbox); a guarda fica provada pelo teste puro de `estaNoSandbox` e pelo Step 5. Registrar no relato.

- [ ] **Step 9: Critério 7 — fluxo típico com o app**

Só com o sim do usuário para o build. Build com `CARGO_TARGET_DIR` próprio (`$env:USERPROFILE\.cache\claudia-lab-9`), `npm run tauri -- build`. Depois: `test-down.ps1`; `cenario-mock.mjs`; `publicar.mjs --app <instalador>`; `test-up.ps1 -Config lab.config.app-winapp.json -Project <projeto>`; `guest-firewall.ps1` como System; instalar com `/S`; `guest-run.ps1 -NoStart` e abrir o app; `winapp ui inspect --on sandbox -a claudia-rh -d 30`; print da janela; `banco-guest.mjs`; `test-down.ps1`. Expected: árvore UIA com o conteúdo web do app, print salvo em `sandbox-lab/evidence/`, banco copiado. O que não abrir é resultado: registrar no Linear.

- [ ] **Step 10: Derrubar e relatar**

Run: `pwsh -NoProfile -File <lab>\scripts\test-down.ps1; "exit=$LASTEXITCODE"` → `Sandbox encerrado`, `exit=0`.

Escrever `docs/superpowers/evidencias/2026-10-07-laboratorio-no-repo-sandbox.md` com: comandos rodados, saídas (contagens, linhas `ok`/`FAILED` das mutações), critérios de aceite 1–7 da spec com "verificado" ou "não verificado" e o motivo, sem nenhum caminho de máquina. Conferência de segredo. `git add -- docs/superpowers/evidencias/2026-10-07-laboratorio-no-repo-sandbox.md` e commit `docs(sandbox-lab): Sandbox verification of the in-repo lab`.

---

### Task 10: revisão de código

Modelo: **Opus** (skill `superpowers:requesting-code-review`). Lê a spec, este plano e o diff inteiro da `feat/laboratorio-no-repo` desde `56ced76`. Foco: os cinco itens do Review Focus, a regra 2 (nenhum teste passa caminho real a função que apaga), a ordem trava → deleção no `app-stage.ps1` e no `Clear-LabOut`, o que o `conferir-repo.mjs` deixaria passar, e se algum script ainda lê variável de caminho. Achados corrigidos numa tarefa nova ou registrados no Linear. Não roda nada que crie ou apague pasta.

---

## Precisa do usuário

1. **Aprovar este plano.**
2. **Tarefa 2:** ver os comandos dos Steps 2 a 7 (inclui acrescentar 3 linhas ao `.git/info/exclude`) e dizer "sim" antes da cópia.
3. **Desvio da spec na Tarefa 7, Step 8:** o `guest-build.ps1` exclui `.claude\worktrees`, não `.claude` inteiro, para os testes da skill irem ao guest. Aprovar ou pedir outra solução.
4. **Decisões do plano não escritas na spec** (implementam regras dela): `onde.mjs`; `testes-no-guest.mjs` (usa o `guest-build.ps1` como está); guarda do Sandbox em `tempDir()` e no cabeçalho do `.sandbox.ps1`; a trava do `app-stage.ps1` roda também sem `-Clean` (porque o `robocopy /MIR` apaga dentro do deploy); recusa de reparse point **dentro** do deploy e do `shared\out`; `-WorkDir` aceito só igual ao padrão; o `app-stage.ps1` também grava o marcador; checagem de ferramentas por tamanho no `test-up` (hash só na cópia e no `conferir-copia`).
5. **Integrar a `feat/laboratorio-no-repo` na `feat/browser-cdp-mcp` (L9) antes da Tarefa 9**, e autorizar o Sandbox e o build do app nessa rodada.
6. Evals de disparo da `claudia-teste-sandbox` e da `claudia-teste-e2e` depois de entrarem as skills `winapp-*` (L7): continuam manuais e só com autorização (B6).
7. Issue no Linear para o script que baixa as ferramentas (L2).

## Não verificado (no momento do plano)

- Nada foi executado: nenhum teste, script ou cópia. Só leitura de arquivos e listagens.
- Se o `fs.lstatSync(...).isSymbolicLink()` do Node devolve `true` para junction no Windows (a Tarefa 9 prova pelo `publicar.test.mjs`).
- Se o usuário do Windows Sandbox é `WDAGUtilityAccount` nesta versão (Tarefa 9, Step 5).
- Se o comando passado ao `guest-build.ps1` por `wsb exec` chega sem ser mexido pelo PowerShell 5.1 do guest (Tarefa 9, Steps 6 e 7).
- Montar `.sandbox-work\guest-rw` (gravável) dentro de uma pasta do projeto montada read-only, quando o projeto é o checkout principal (spec, seção 9). Com o projeto numa worktree, as pastas não se sobrepõem.
- Se o Claude Code usa o `sandbox-lab/.mcp.json` quando aberto na raiz do repositório (provavelmente não; o MCP do guest só vale abrindo o Claude Code dentro de `sandbox-lab/`).
- Se o Vite ou o PyCharm varrem `sandbox-lab/shared/tools/`.
- O conteúdo de `shared/db/backup.sqlite` (não aberto; o hash é conferido pelo `validate-config.ps1`).
