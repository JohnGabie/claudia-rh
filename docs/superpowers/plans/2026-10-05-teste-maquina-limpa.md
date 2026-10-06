# Teste em máquina limpa e observação do app — plano de implementação

> **Para agentes executores:** SUB-SKILL OBRIGATÓRIA: use superpowers:subagent-driven-development para executar este plano tarefa a tarefa. Os passos usam checkbox (`- [ ]`).

**Objetivo:** duas skills especializadas — `claudia-sandbox` (nova: Windows Sandbox limpo, cenários de dados, publicar/instalar, CDP no guest) e `claudia-app` (a `claudia-teste` renomeada: observar o app por CDP localmente ou no Sandbox, com prints em escada).

**Arquitetura:** o específico do ambiente vive em `.claude/skills/claudia-sandbox/` (scripts Node, fixtures, referências, evals); a observação do app continua nos scripts Node da skill renomeada `.claude/skills/claudia-app/`, que passam a aceitar `CDP_HOST` e `CLAUDIA_DB`. O laboratório é localizado por `SANDBOX_LAB_DIR` e usado só pela sua interface pública. Cada cenário produz uma pasta com `appdata\io.github.johngabie.claudia-rh\` e `cenario.json`; `publicar.mjs` entrega app + cenário ao laboratório; `cdp-guest.mjs` expõe o CDP do app no guest ao host.

**Stack:** Node ≥ 22.13 (`node:test`, `node:sqlite`, `node:util.parseArgs`, `child_process` para `wsb`/`winapp`/`pwsh`), PowerShell só onde é nativo do Windows (`print-janela.ps1`, comandos enviados ao guest, `app-stage.ps1` do laboratório), winapp CLI, skill-creator.

**Spec:** `docs/superpowers/specs/2026-10-05-teste-maquina-limpa-design.md`

## Restrições globais

- Identifier do app: `io.github.johngabie.claudia-rh` (nunca alterar).
- Comentários de código e mensagens de commit em inglês; textos da skill, referências e docs em pt-BR.
- Sem dependências novas para o usuário final; nada instalado globalmente na máquina do dev.
- Scripts não escrevem fora da pasta `--saida`/`--work-dir` informada; a origem do cenário `copia` só é lida.
- `copia` sem `--autorizado` sai com exit 2 e não lê a origem.
- `disparo.json` gravado pelos scripts tem sempre todos os campos (`ativo`, `limiar_minutos`, `limite_diario`, `limite_tempo_minutos`, `janelas`): sem `limiar_minutos` o app ignora o arquivo e usa `ativo: true` (`src-tauri/src/lib.rs`, `load_idle_config`).
- Arquivos agnósticos: nenhum arquivo novo ou alterado menciona sessões de trabalho, pessoas, modelos, nomes de máquina, IPs, caminhos pessoais ou pastas fora do repositório além de `SANDBOX_LAB_DIR`/`LAB_WORK_DIR`/`%TEMP%`/`%APPDATA%`/`%LOCALAPPDATA%`.
- Cada agente executor trabalha em worktree própria e usa `CARGO_TARGET_DIR` fora do `target/` do repositório quando compilar.

## Foco de revisão

1. Origem do `copia` com o app aberto (banco em uso, `-wal` com dados não consolidados): a cópia precisa abrir e conter as linhas do `-wal`. Teste na Tarefa 4 (`copia consolida wal`).
2. `disparo.json` existente com campos extras ou faltando `limiar_minutos`: o resultado precisa ser JSON completo com `ativo: false`. Teste na Tarefa 1 (`automacao completa a partir de arquivo parcial`).
3. `CDP_HOST` remoto sem resposta: `regras.decidirSubida` nunca devolve `subir` com `remoto: true` (logo `subir.mjs` não sobe `tauri dev` no host); aborta com motivo. Teste na Tarefa 7 (`remoto sem CDP aborta`).
4. `--saida` já existente com lixo de execução anterior: a pasta é recriada, sem sobras. Teste na Tarefa 1 (`saida recriada`).
5. Caminhos com espaços (`--saida`, `--origem`, `--app`): todos os scripts funcionam. Coberto usando pastas temporárias com espaço no nome em todos os testes.

---

## Estrutura de arquivos

| Arquivo | Responsabilidade |
|---|---|
| `.claude/skills/claudia-sandbox/scripts/comum.mjs` | constantes e funções: `APP_ID`, `REPO_ROOT`, `sair`, `falhaDe`, `labDir`, `novoCenarioDir`, `escreverCenario`, `desligarAutomacao`; funções de Sandbox (`idsDeWsbList`, `sandboxId`, `sandboxIp`, `noGuest`) |
| `.../scripts/cenario-novo.mjs` | cenário sem dados |
| `.../scripts/cenario-mock.mjs` | cenário com fixtures |
| `.../scripts/cenario-copia.mjs` | cenário com cópia dos dados do usuário |
| `.../scripts/publicar.mjs` | app + cenário → `app-stage.ps1` |
| `.../scripts/test/util.mjs` | `tempDir`, `rodar`, `SCRIPTS` |
| `.../scripts/test/*.test.mjs` | um arquivo de teste por script (`node --test`) |
| `.../fixtures/*` | dados fictícios |
| `.../references/limites.md`, `pastas-de-dados.md` | referências da skill |
| `.../SKILL.md`, `.../evals/evals.json` | skill e evals de disparo |
| `.claude/skills/claudia-sandbox/scripts/cdp-guest.mjs`, `banco-guest.mjs` | CDP do app no guest exposto ao host; snapshot do banco do guest |
| `.claude/skills/claudia-app/` (renomeada de `claudia-teste`) | `SKILL.md`, `evals/`, `scripts/regras.mjs`, `cdp.mjs`, `subir.mjs`, `ver.mjs`, `invocar.mjs`, `observar.mjs`, `banco.mjs` com `CDP_HOST`/`CLAUDIA_DB` |
| `.claude/skills/claudia-app/scripts/print-janela.ps1` (+ teste) | print da janela nativa ou da tela inteira, sem CDP |
| `CLAUDE.md` | nome novo da skill + seção "Teste em máquina limpa" |

## Ondas de execução

- **Onda 1:** Tarefa 1.
- **Onda 2 (paralelo, arquivos distintos):** Tarefas 2, 3, 4, 7, 10.
- **Onda 3 (paralelo):** Tarefa 5 (o teste usa `cenario-novo.mjs` da Tarefa 2), Tarefa 8 (prints, depois da 7) e Tarefa 9 (CDP/banco no guest).
- **Onda 4:** Tarefa 6 (skill `claudia-sandbox`; documenta os scripts prontos).
- **Onda 5:** Tarefa 11 (smoke) e Tarefa 12 (revisão + varredura).
- **Fim:** integração decidida pelo usuário (Tarefa 13).

---

### Tarefa 1: base comum e harness de teste

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/comum.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/util.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/comum.test.mjs`

**Interfaces:**
- Produz (`comum.mjs`):
  - `APP_ID = "io.github.johngabie.claudia-rh"`, `REPO_ROOT` (4 níveis acima de `scripts/`)
  - `sair(msg, code = 1)` — escreve no stderr e encerra
  - `falhaDe(err)` — encerra com 2 se o erro é de uso (`code` começando com `ERR_PARSE_ARGS`), senão 1
  - `labDir() -> string` — valida `SANDBOX_LAB_DIR` e `scripts\app-stage.ps1`; lança erro com o nome do que falta
  - `novoCenarioDir(saida, nome) -> string` — recria a pasta (padrão `%TEMP%\claudia-sandbox\<nome>`); cria `appdata\<APP_ID>` e `localappdata`
  - `escreverCenario(dir, nome, origem) -> string` — grava e devolve o JSON de `cenario.json` (`origem: null` quando ausente)
  - `desligarAutomacao(dataDir)` — `disparo.json` completo com `ativo:false` (preserva campos existentes) e `modo_autonomo.json` = `{"skip_permissions":false}`
- Produz (`test/util.mjs`): `tempDir(prefixo) -> string` (nome com espaço), `rodar(script, args, env) -> { code, out, ultima, err }`, `SCRIPTS` (pasta `scripts/`)

- [ ] **Passo 1: `test/util.mjs`**

```js
// Small helpers shared by the scenario tests.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

export const SCRIPTS = path.join(import.meta.dirname, "..");

/** Temp folder with a space in its name, so every test also covers paths with spaces. */
export function tempDir(prefixo) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `cs test ${prefixo} `));
}

export function rodar(script, args = [], env = {}) {
  const r = spawnSync(process.execPath, [script, ...args], { encoding: "utf8", env: { ...process.env, ...env } });
  const out = (r.stdout ?? "").trim().split(/\r?\n/);
  return { code: r.status, out, ultima: out.at(-1), err: r.stderr ?? "" };
}
```

- [ ] **Passo 2: teste `comum.test.mjs` (falha: `comum.mjs` não existe)**

```js
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { APP_ID, novoCenarioDir, escreverCenario, desligarAutomacao, labDir } from "../comum.mjs";
import { tempDir } from "./util.mjs";

test("saida recriada", () => {
  const saida = path.join(tempDir("comum"), "saida x");
  fs.mkdirSync(path.join(saida, "lixo"), { recursive: true });
  const dir = novoCenarioDir(saida, "t");
  assert.equal(fs.existsSync(path.join(dir, "lixo")), false);
  assert.ok(fs.existsSync(path.join(dir, "appdata", APP_ID)));
  assert.ok(fs.existsSync(path.join(dir, "localappdata")));
});

test("cenario.json gravado e devolvido", () => {
  const dir = novoCenarioDir(path.join(tempDir("comum"), "c"), "t");
  const devolvido = JSON.parse(escreverCenario(dir, "t", "origem-x"));
  const gravado = JSON.parse(fs.readFileSync(path.join(dir, "cenario.json"), "utf8"));
  assert.equal(devolvido.cenario, "t");
  assert.equal(gravado.origem, "origem-x");
  assert.ok(gravado.criadoEm);
  assert.equal(JSON.parse(escreverCenario(dir, "t")).origem, null);
});

test("automacao completa a partir de arquivo parcial", () => {
  const dir = novoCenarioDir(path.join(tempDir("comum"), "a"), "t");
  const dados = path.join(dir, "appdata", APP_ID);
  fs.writeFileSync(path.join(dados, "disparo.json"), '{"ativo":true,"limite_diario":3}');
  desligarAutomacao(dados);
  const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
  assert.deepEqual(d, { ativo: false, limiar_minutos: 15, limite_diario: 3, limite_tempo_minutos: 0, janelas: [] });
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(dados, "modo_autonomo.json"), "utf8")), { skip_permissions: false });
});

test("lab ausente ou incompleto nomeia o que falta", () => {
  const antigo = process.env.SANDBOX_LAB_DIR;
  delete process.env.SANDBOX_LAB_DIR;
  assert.throws(() => labDir(), /SANDBOX_LAB_DIR/);
  process.env.SANDBOX_LAB_DIR = tempDir("lab");
  assert.throws(() => labDir(), /app-stage\.ps1/);
  if (antigo === undefined) delete process.env.SANDBOX_LAB_DIR; else process.env.SANDBOX_LAB_DIR = antigo;
});
```

- [ ] **Passo 3: rodar e ver falhar** — `node --test .claude/skills/claudia-sandbox/scripts/test/` → erro de import (`comum.mjs` inexistente), exit ≠ 0.

- [ ] **Passo 4: `comum.mjs`**

```js
// Shared constants and helpers for the clean-machine scenario scripts.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export const APP_ID = "io.github.johngabie.claudia-rh";
export const REPO_ROOT = path.resolve(import.meta.dirname, "../../../..");

export function sair(msg, code = 1) {
  console.error(msg);
  process.exit(code);
}

/** Usage errors (bad or missing options) exit 2; everything else exits 1. */
export function falhaDe(err) {
  sair(err.message, String(err.code ?? "").startsWith("ERR_PARSE_ARGS") ? 2 : 1);
}

export function labDir() {
  const d = process.env.SANDBOX_LAB_DIR;
  if (!d) throw new Error("SANDBOX_LAB_DIR is not set: point it to the Windows Sandbox lab folder.");
  const stage = path.join(d, "scripts", "app-stage.ps1");
  if (!fs.existsSync(stage)) throw new Error(`Incomplete lab: ${stage} not found.`);
  return path.resolve(d);
}

export function novoCenarioDir(saida, nome) {
  const dir = path.resolve(saida || path.join(process.env.TEMP ?? os.tmpdir(), "claudia-sandbox", nome));
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(path.join(dir, "appdata", APP_ID), { recursive: true });
  fs.mkdirSync(path.join(dir, "localappdata"), { recursive: true });
  return dir;
}

function arquivosDe(raiz) {
  return fs.readdirSync(raiz, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => path.join(e.parentPath ?? e.path, e.name));
}

export function escreverCenario(dir, nome, origem = null) {
  const arquivos = arquivosDe(path.join(dir, "appdata")).map((f) => path.relative(dir, f));
  const json = JSON.stringify({ cenario: nome, criadoEm: new Date().toISOString(), origem: origem ?? null, arquivos });
  fs.writeFileSync(path.join(dir, "cenario.json"), json);
  return json;
}

const DISPARO_PADRAO = { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, janelas: [] };

/** The app falls back to ativo=true when disparo.json misses required fields, so always write the full shape. */
export function desligarAutomacao(dataDir) {
  const p = path.join(dataDir, "disparo.json");
  let antigo = {};
  try { antigo = JSON.parse(fs.readFileSync(p, "utf8")); } catch { /* missing or invalid: defaults */ }
  const cfg = { ...DISPARO_PADRAO };
  for (const k of ["limiar_minutos", "limite_diario", "limite_tempo_minutos", "janelas"]) {
    if (antigo[k] != null) cfg[k] = antigo[k];
  }
  cfg.ativo = false;
  fs.writeFileSync(p, JSON.stringify(cfg));
  fs.writeFileSync(path.join(dataDir, "modo_autonomo.json"), JSON.stringify({ skip_permissions: false }));
}
```

- [ ] **Passo 5: rodar e ver passar** — `node --test .claude/skills/claudia-sandbox/scripts/test/; echo "exit=$LASTEXITCODE"` → todos `ok`, `exit=0`.
- [ ] **Passo 6: prova de mutação** — trocar `cfg.ativo = false;` por `cfg.ativo = true;`; confirmar `not ok … automacao completa a partir de arquivo parcial`; restaurar e rodar de novo (verde).
- [ ] **Passo 7: commit**

```bash
git add .claude/skills/claudia-sandbox/scripts
git commit -m "feat(skill): shared helpers and test harness for clean-machine scenarios"
```

---

### Tarefa 2: cenário `novo`

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/cenario-novo.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/novo.test.mjs`

**Interfaces:**
- Consome: `comum.mjs`, `test/util.mjs` (Tarefa 1).
- Produz: `node cenario-novo.mjs [--saida <pasta>]` → exit 0; última linha do stdout = JSON de `cenario.json` com `cenario: "novo"`, `origem: null`, `arquivos: []`.

- [ ] **Passo 1: teste (falha: script inexistente)**

```js
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

test("cenario novo", () => {
  const saida = path.join(tempDir("novo"), "out dir");
  const r = rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--saida", saida]);
  assert.equal(r.code, 0, r.err);
  const j = JSON.parse(r.ultima);
  assert.equal(j.cenario, "novo");
  assert.equal(j.origem, null);
  assert.deepEqual(j.arquivos, []);
  assert.ok(fs.existsSync(path.join(saida, "appdata", "io.github.johngabie.claudia-rh")));
});

test("opcao desconhecida sai com 2", () => {
  assert.equal(rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--xyz"]).code, 2);
});
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: implementar**

```js
// Clean-machine scenario "novo": empty app data folder, so the app starts from scratch.
// Usage: node cenario-novo.mjs [--saida <folder>]   (default %TEMP%\claudia-sandbox\novo, recreated on every run)
import { parseArgs } from "node:util";
import { novoCenarioDir, escreverCenario, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { saida: { type: "string" } } });
  console.log(escreverCenario(novoCenarioDir(values.saida, "novo"), "novo"));
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 4: rodar e ver passar.**
- [ ] **Passo 5: commit** — `git commit -m "feat(skill): empty-data clean-machine scenario"`

---

### Tarefa 3: cenário `mock` e fixtures

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/fixtures/candidate_base.yaml`, `search_variants.yaml`, `strategy.md`, `seed.sql`
- Criar: `.claude/skills/claudia-sandbox/scripts/cenario-mock.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/mock.test.mjs`

**Interfaces:**
- Consome: `comum.mjs`; `src-tauri/src/db/schema.sql`; `node:sqlite`.
- Produz: `node cenario-mock.mjs [--saida <pasta>]` → pasta com `candidate_base.yaml`, `search_variants.yaml`, `strategy.md`, `claudia_rh.db`, `welcome_seen`, `disparo.json` (ativo false), `modo_autonomo.json`.

- [ ] **Passo 1: fixtures**

`candidate_base.yaml`: pessoa fictícia ("Ana Exemplo", e-mail `ana@example.com`, links `example.com`), seguindo **exatamente** o schema `PROFILE_SCHEMA` em `src-tauri/src/commands/prompts.rs` (chaves de topo: `dados_pessoais`, `experiencia`, `projetos`, `formacao`, `competencias`, `idiomas`, `fontes_usadas`, `gaps_conhecidos`, `respostas_modelo`, `ultima_atualizacao`; `links` e `fontes_usadas` como listas de objetos; `fim` sempre string; `conquistas`/`tecnologias` sempre listas). Duas experiências, um projeto, uma formação.

`search_variants.yaml`: duas variantes no formato de `prompts.rs` (seção "Mandatory schema — search_variants.yaml"), pesos somando 100, `ativa: true`.

`strategy.md`: três linhas de estratégia fictícia.

`seed.sql`:

```sql
INSERT INTO vagas (titulo, empresa, plataforma, url, localizacao, modelo_trabalho, idioma, descoberta_em, status, match_score) VALUES
 ('Backend Developer', 'Exemplo Ltda', 'linkedin', 'https://example.com/vagas/1', 'Remoto', 'remoto', 'pt', '2026-01-10T09:00:00Z', 'descoberta', 'alto'),
 ('Fullstack Engineer', 'Demo SA', 'linkedin', 'https://example.com/vagas/2', 'Lisboa', 'hibrido', 'en', '2026-01-11T09:00:00Z', 'candidatada', 'medio'),
 ('Data Analyst', 'Teste Corp', 'outra', 'https://example.com/vagas/3', 'Porto', 'presencial', 'pt', '2026-01-12T09:00:00Z', 'descartada', 'baixo');
INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES
 (2, '2026-01-11T10:00:00Z', 'candidaturas/2', 'formulario');
INSERT INTO pendencias (vaga_id, criada_em, categoria, descricao) VALUES
 (1, '2026-01-10T09:30:00Z', 'pergunta', 'Pretensao salarial nao definida no perfil');
INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, contexto) VALUES
 (1, '2026-01-10T09:31:00Z', 'Tem experiencia com Kubernetes?', 'Requisito da vaga 1');
INSERT INTO sessoes (iniciada_em, terminada_em, motivo_disparo, motivo_termino, vagas_processadas) VALUES
 ('2026-01-11T09:00:00Z', '2026-01-11T09:40:00Z', 'manual', 'concluida', 3);
```

- [ ] **Passo 2: teste (falha: script inexistente)**

```js
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

test("cenario mock", () => {
  const saida = path.join(tempDir("mock"), "out dir");
  const r = rodar(path.join(SCRIPTS, "cenario-mock.mjs"), ["--saida", saida]);
  assert.equal(r.code, 0, r.err);
  assert.equal(JSON.parse(r.ultima).cenario, "mock");
  const dados = path.join(saida, "appdata", "io.github.johngabie.claudia-rh");
  for (const f of ["candidate_base.yaml", "search_variants.yaml", "strategy.md", "claudia_rh.db", "welcome_seen", "disparo.json", "modo_autonomo.json"]) {
    assert.ok(fs.existsSync(path.join(dados, f)), `mock tem ${f}`);
  }
  const db = new DatabaseSync(path.join(dados, "claudia_rh.db"), { readOnly: true });
  assert.equal(db.prepare("SELECT count(*) AS n FROM vagas").get().n, 3);
  db.close();
  const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
  assert.equal(d.ativo, false, "mock disparo desligado");
  assert.equal(typeof d.limiar_minutos, "number", "mock disparo completo");
  const y = fs.readFileSync(path.join(dados, "candidate_base.yaml"), "utf8");
  for (const k of ["dados_pessoais", "experiencia", "projetos", "formacao", "competencias", "idiomas", "fontes_usadas", "gaps_conhecidos", "respostas_modelo", "ultima_atualizacao"]) {
    assert.match(y, new RegExp(`^${k}:`, "m"), `mock yaml tem ${k}`);
  }
});
```

- [ ] **Passo 3: rodar e ver falhar.**
- [ ] **Passo 4: implementar**

```js
// Clean-machine scenario "mock": fictional profile and a database built from the repo schema plus a seed.
// Usage: node cenario-mock.mjs [--saida <folder>]   (default %TEMP%\claudia-sandbox\mock, recreated on every run)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { DatabaseSync } from "node:sqlite";
import { APP_ID, REPO_ROOT, novoCenarioDir, escreverCenario, desligarAutomacao, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { saida: { type: "string" } } });
  const dir = novoCenarioDir(values.saida, "mock");
  const dados = path.join(dir, "appdata", APP_ID);
  const fx = path.join(import.meta.dirname, "..", "fixtures");
  for (const f of ["candidate_base.yaml", "search_variants.yaml", "strategy.md"]) {
    fs.copyFileSync(path.join(fx, f), path.join(dados, f));
  }
  const db = new DatabaseSync(path.join(dados, "claudia_rh.db"));
  db.exec(fs.readFileSync(path.join(REPO_ROOT, "src-tauri", "src", "db", "schema.sql"), "utf8"));
  db.exec(fs.readFileSync(path.join(fx, "seed.sql"), "utf8"));
  db.close();
  fs.writeFileSync(path.join(dados, "welcome_seen"), "1");
  desligarAutomacao(dados);
  console.log(escreverCenario(dir, "mock", "fixtures"));
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 5: rodar e ver passar.**
- [ ] **Passo 6: prova de mutação** — remover a linha `desligarAutomacao(dados);`; confirmar `not ok` com a mensagem `mock disparo desligado` (o arquivo não existe → falha na leitura); restaurar.
- [ ] **Passo 7: commit** — `git commit -m "feat(skill): mock-data clean-machine scenario with fictional fixtures"`

---

### Tarefa 4: cenário `copia`

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/cenario-copia.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/copia.test.mjs`

**Interfaces:**
- Consome: `comum.mjs`, `node:sqlite`.
- Produz: `node cenario-copia.mjs --autorizado [--origem <pasta>] [--saida <pasta>] [--manter-automacao]`. Sem `--autorizado`: exit 2, mensagem pedindo autorização, nada lido. `--origem` padrão: `%APPDATA%\io.github.johngabie.claudia-rh`. `%LOCALAPPDATA%` nunca é copiado.

- [ ] **Passo 1: teste (falha: script inexistente)**

```js
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { DatabaseSync } from "node:sqlite";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

const SCRIPT = path.join(SCRIPTS, "cenario-copia.mjs");
const APP = "io.github.johngabie.claudia-rh";

function hashArvore(raiz) {
  return fs.readdirSync(raiz, { recursive: true, withFileTypes: true }).filter((e) => e.isFile())
    .map((e) => path.join(e.parentPath ?? e.path, e.name)).sort()
    .map((f) => `${path.relative(raiz, f)}:${crypto.createHash("sha256").update(fs.readFileSync(f)).digest("hex")}`).join(",");
}

test("cenario copia", () => {
  const base = tempDir("copia");
  const origem = path.join(base, "origem dados");
  fs.mkdirSync(path.join(origem, "sub"), { recursive: true });
  fs.writeFileSync(path.join(origem, "candidate_base.yaml"), "dados_pessoais: {}\n");
  fs.writeFileSync(path.join(origem, "sub", "nota.txt"), "x");
  fs.writeFileSync(path.join(origem, "disparo.json"), '{"ativo":true,"limiar_minutos":5,"limite_diario":7,"limite_tempo_minutos":0,"janelas":[]}');
  // The last row lives only in the -wal file while this connection stays open (as with the app running).
  const vivo = new DatabaseSync(path.join(origem, "claudia_rh.db"));
  vivo.exec("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE t(x); INSERT INTO t VALUES (42);");
  assert.ok(fs.existsSync(path.join(origem, "claudia_rh.db-wal")), "fixture tem wal");
  const antes = hashArvore(origem);

  const sem = rodar(SCRIPT, ["--origem", origem, "--saida", path.join(base, "sem")]);
  assert.equal(sem.code, 2, "copia sem autorizacao sai com 2");
  assert.equal(fs.existsSync(path.join(base, "sem", "appdata")), false, "copia sem autorizacao nao copia nada");

  const saida = path.join(base, "out dir");
  const r = rodar(SCRIPT, ["--autorizado", "--origem", origem, "--saida", saida]);
  assert.equal(r.code, 0, r.err);
  const dados = path.join(saida, "appdata", APP);
  assert.ok(fs.existsSync(path.join(dados, "sub", "nota.txt")), "copia recursiva");
  const db = new DatabaseSync(path.join(dados, "claudia_rh.db"), { readOnly: true });
  assert.equal(db.prepare("SELECT x FROM t").get().x, 42, "copia consolida wal");
  db.close();
  assert.equal(fs.existsSync(path.join(dados, "claudia_rh.db-wal")), false, "copia sem arquivo wal");
  const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
  assert.equal(d.ativo, false, "copia desliga automacao");
  assert.equal(d.limiar_minutos, 5, "copia preserva campos");
  assert.equal(hashArvore(origem), antes, "origem intacta");
  assert.deepEqual(fs.readdirSync(path.join(saida, "localappdata")), [], "localappdata nao copiado");

  rodar(SCRIPT, ["--autorizado", "--manter-automacao", "--origem", origem, "--saida", saida]);
  assert.equal(JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8")).ativo, true, "manter-automacao preserva disparo");

  const ausente = rodar(SCRIPT, ["--autorizado", "--origem", path.join(base, "nao existe"), "--saida", saida]);
  assert.equal(ausente.code, 1);
  assert.match(ausente.err, /nao existe/, "origem ausente nomeia o caminho");
  vivo.close();
});
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: implementar**

```js
// Clean-machine scenario "copia": copy of the user's real app data. Reads the source only.
// Requires --autorizado: ask the user for permission for this round before running it.
// Usage: node cenario-copia.mjs --autorizado [--origem <folder>] [--saida <folder>] [--manter-automacao]
//   --origem            default %APPDATA%\io.github.johngabie.claudia-rh
//   --saida             default %TEMP%\claudia-sandbox\copia, recreated on every run
//   --manter-automacao  keep disparo.json / modo_autonomo.json as copied (default: turned off in the copy)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { DatabaseSync } from "node:sqlite";
import { APP_ID, novoCenarioDir, escreverCenario, desligarAutomacao, falhaDe, sair } from "./comum.mjs";

let values;
try {
  ({ values } = parseArgs({ options: {
    autorizado: { type: "boolean" }, origem: { type: "string" }, saida: { type: "string" }, "manter-automacao": { type: "boolean" },
  } }));
} catch (e) { falhaDe(e); }
if (!values.autorizado) {
  sair("Scenario 'copia' reads the user's real data: ask the user for permission for this round, then run again with --autorizado.", 2);
}
try {
  const origem = path.resolve(values.origem || path.join(process.env.APPDATA, APP_ID));
  if (!fs.existsSync(origem) || !fs.statSync(origem).isDirectory()) throw new Error(`Source data not found: ${origem}`);
  const dir = novoCenarioDir(values.saida, "copia");
  const dados = path.join(dir, "appdata", APP_ID);
  fs.cpSync(origem, dados, { recursive: true });
  // Fold each copied WAL into its database so the copy is a single consistent file.
  for (const e of fs.readdirSync(dados, { recursive: true, withFileTypes: true })) {
    if (!e.isFile() || !e.name.endsWith(".db")) continue;
    const f = path.join(e.parentPath ?? e.path, e.name);
    if (fs.existsSync(`${f}-wal`)) {
      const db = new DatabaseSync(f);
      db.exec("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;");
      db.close();
    }
    fs.rmSync(`${f}-wal`, { force: true });
    fs.rmSync(`${f}-shm`, { force: true });
  }
  if (!values["manter-automacao"]) desligarAutomacao(dados);
  console.log(escreverCenario(dir, "copia", origem));
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 4: rodar e ver passar.**
- [ ] **Passo 5: prova de mutação** — remover o bloco `if (fs.existsSync(\`${f}-wal\`)) { … }`; confirmar `not ok` com `copia consolida wal`; restaurar.
- [ ] **Passo 6: commit** — `git commit -m "feat(skill): authorized copy-of-user-data clean-machine scenario"`

---

### Tarefa 5: `publicar.mjs`

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/publicar.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/publicar.test.mjs`

**Interfaces:**
- Consome: `comum.mjs` (`labDir`, `APP_ID`); `cenario-novo.mjs` (Tarefa 2, no teste); interface do laboratório `scripts\app-stage.ps1 -App -AppData -WorkDir -Clean` (PowerShell, chamado por `pwsh`).
- Produz: `node publicar.mjs --app <exe|instalador|pasta> --cenario <pasta> [--work-dir <pasta>]` → exit 0 e JSON `{workDir, deploy, cenario}` na última linha.

- [ ] **Passo 1: teste com laboratório falso (falha: script inexistente)**

```js
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

const FAKE_STAGE = `param([string]$App, [string[]]$AppData, [string[]]$LocalAppData, [string]$WorkDir, [switch]$Clean)
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'called.json'), (@{ App = $App; AppData = @($AppData); WorkDir = $WorkDir; Clean = [bool]$Clean } | ConvertTo-Json))
exit 0
`;

test("publicar", () => {
  const base = tempDir("publicar");
  const lab = path.join(base, "lab x");
  fs.mkdirSync(path.join(lab, "scripts"), { recursive: true });
  fs.writeFileSync(path.join(lab, "scripts", "app-stage.ps1"), FAKE_STAGE);
  const app = path.join(base, "app x.exe"); fs.writeFileSync(app, "x");
  const cen = path.join(base, "cen x");
  rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--saida", cen]);
  const work = path.join(base, "work x");
  const script = path.join(SCRIPTS, "publicar.mjs");

  const r = rodar(script, ["--app", app, "--cenario", cen, "--work-dir", work], { SANDBOX_LAB_DIR: lab });
  assert.equal(r.code, 0, r.err);
  const c = JSON.parse(fs.readFileSync(path.join(lab, "scripts", "called.json"), "utf8"));
  assert.equal(c.App, app);
  assert.equal(c.WorkDir, work);
  assert.equal(c.Clean, true);
  assert.match([c.AppData].flat()[0], /appdata[\\/]io\.github\.johngabie\.claudia-rh$/, "publicar repassa a pasta de dados do app");
  assert.equal(JSON.parse(r.ultima).cenario, "novo");

  const semMeta = rodar(script, ["--app", app, "--cenario", base, "--work-dir", work], { SANDBOX_LAB_DIR: lab });
  assert.equal(semMeta.code, 1);
  assert.match(semMeta.err, /cenario\.json/, "publicar recusa pasta sem cenario.json");

  const labRuim = rodar(script, ["--app", app, "--cenario", cen, "--work-dir", work], { SANDBOX_LAB_DIR: base });
  assert.equal(labRuim.code, 1);
  assert.match(labRuim.err, /app-stage\.ps1/, "publicar com lab incompleto nomeia o arquivo");

  assert.equal(rodar(script, ["--app", app], { SANDBOX_LAB_DIR: lab }).code, 2, "sem --cenario sai com 2");
});
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: implementar**

```js
// Publishes the app plus a scenario folder to the Windows Sandbox lab (lab's scripts\app-stage.ps1).
// Usage: node publicar.mjs --app <exe|installer|folder> --cenario <scenario folder> [--work-dir <folder>]
//   --work-dir  default $env:LAB_WORK_DIR, else %TEMP%\claudia-sandbox\work
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { spawnSync } from "node:child_process";
import { APP_ID, labDir, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { app: { type: "string" }, cenario: { type: "string" }, "work-dir": { type: "string" } } });
  if (!values.app || !values.cenario) {
    throw Object.assign(new Error("Usage: node publicar.mjs --app <exe|installer|folder> --cenario <folder> [--work-dir <folder>]"), { code: "ERR_PARSE_ARGS_USAGE" });
  }
  const lab = labDir();
  const meta = path.join(values.cenario, "cenario.json");
  if (!fs.existsSync(meta)) throw new Error(`Not a scenario folder (cenario.json missing): ${values.cenario}`);
  const workDir = values["work-dir"] || process.env.LAB_WORK_DIR || path.join(process.env.TEMP, "claudia-sandbox", "work");
  const r = spawnSync("pwsh", ["-NoProfile", "-File", path.join(lab, "scripts", "app-stage.ps1"),
    "-App", values.app, "-AppData", path.join(values.cenario, "appdata", APP_ID), "-WorkDir", workDir, "-Clean"],
    { stdio: ["ignore", "inherit", "inherit"] });
  if (r.status !== 0) throw new Error(`app-stage.ps1 failed (${r.status ?? r.error?.message})`);
  const cenario = JSON.parse(fs.readFileSync(meta, "utf8")).cenario;
  console.log(JSON.stringify({ workDir, deploy: path.join(workDir, "deploy"), cenario }));
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 4: rodar e ver passar.**
- [ ] **Passo 5: rodar a suíte inteira** — `node --test .claude/skills/claudia-sandbox/scripts/test/; echo "exit=$LASTEXITCODE"` → `exit=0`.
- [ ] **Passo 6: commit** — `git commit -m "feat(skill): publish app and scenario to the sandbox lab"`

---

### Tarefa 6: skill `claudia-sandbox` (via skill-creator)

Executor: agente que carrega a skill `skill-creator` e segue o processo dela (rascunho → evals de disparo → ajuste da descrição).

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/SKILL.md`
- Criar: `.claude/skills/claudia-sandbox/references/limites.md`
- Criar: `.claude/skills/claudia-sandbox/references/pastas-de-dados.md`
- Criar: `.claude/skills/claudia-sandbox/evals/evals.json` (e runner, ver Passo 6)

**Interfaces:**
- Consome: scripts das Tarefas 1–5 e 9 (nomes e parâmetros exatos neste plano); skill `claudia-app` (Tarefas 7–8) para observar e tirar prints; laboratório via `SANDBOX_LAB_DIR` (interface na spec, seção "Contexto: interface do laboratório").

- [ ] **Passo 1:** ler a spec, `CLAUDE.md`, `.claude/skills/claudia-app/SKILL.md` (estilo e tamanho de referência) e `%SANDBOX_LAB_DIR%\.claude\skills\sandbox-lab\SKILL.md`.
- [ ] **Passo 2:** escrever `SKILL.md` em pt-BR com frontmatter `name: claudia-sandbox` e `description` ≤ 1.024 caracteres, cobrindo os 9 itens da spec (inclusive o item 8: sugerir melhorias do ferramental ao usuário, implementar só com o "sim" dele, senão registrar no Linear) ("Conteúdo da `claudia-sandbox`"). Corpo ≤ 120 linhas; detalhe vai para `references/`. A skill é um mapa: capacidades, cenários e quando usar cada um, regras; não prescreve roteiro de teste. Fluxo típico descrito como exemplo, não como obrigação: cenário → `publicar.mjs` → `test-up.ps1 -Config lab.config.app-winapp.json` → instalar ou `guest-run.ps1` → `cdp-guest.mjs` → observar com `claudia-app` (`CDP_HOST`/`CDP_PORT`, `CLAUDIA_DB` de `banco-guest.mjs`) → prints → `test-down.ps1`.
- [ ] **Passo 3:** `references/pastas-de-dados.md`: o que fica em `%APPDATA%\io.github.johngabie.claudia-rh` (YAMLs do perfil, `strategy.md`, `claudia_rh.db`, `disparo.json`, `modo_autonomo.json`, `notif.json`, `welcome_seen`, `diagnostics\`) e em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh` (browser e Activity log, quando existirem na versão em teste). Conferir cada nome no código (`grep` em `src-tauri/src`) e marcar como "não verificado" o que não for encontrado.
- [ ] **Passo 4:** `references/limites.md`: comportamentos do laboratório que afetam o teste, cada um com o comando que o demonstra (se o comando não reproduzir o comportamento, marcar como "não verificado"): winapp não alcança o guest com `networking: Disable`; app iniciado por `winapp target exec` termina quando o comando termina (iniciar com `wsb exec --id <id> -r ExistingLogin`); `wsb exec` não devolve a saída do processo; target do cargo em pasta mapeada falha com `os error 1392` (usar disco do guest); só um Sandbox por vez; `.exe` de `tauri build --no-bundle` não instala pré-requisitos (WebView2), o instalador sim; o WebView2 só aceita CDP em `127.0.0.1` do guest (daí o `portproxy` do `cdp-guest.mjs`).
- [ ] **Passo 5:** `evals/evals.json` no formato de `.claude/skills/claudia-app/evals/evals.json`: ≥ 8 prompts que devem disparar (máquina limpa, Sandbox, instalador, primeira abertura, desinstalação, PC novo, testar com meus dados sem mexer neles, build do zero) e ≥ 5 que não devem (olhar a tela do app já aberto, tirar print do Perfil, `cargo test`, registrar no Linear, mexer no CSS).
- [ ] **Passo 6:** copiar `.claude/skills/claudia-app/evals/rodar-evals.mjs` para `.claude/skills/claudia-sandbox/evals/rodar-evals.mjs` trocando só a constante `NOME` para `"claudia-sandbox"`; rodar `node .claude/skills/claudia-sandbox/evals/rodar-evals.mjs sonnet` (2 rodadas); registrar resultado em `references/limites.md` (seção "Evals de disparo"): execução inválida não conta. Ajustar a descrição até disparar em todos os "deve" sem disparar nos "não deve".
- [ ] **Passo 7:** varredura de agnosticismo nos arquivos da tarefa (Tarefa 12, Passo 2) e commit — `git commit -m "feat(skill): claudia-sandbox skill for clean-machine testing"`

---

### Tarefa 7: renomear `claudia-teste` → `claudia-app` e alvo remoto

**Arquivos:**
- Mover: `.claude/skills/claudia-teste/` → `.claude/skills/claudia-app/`
- Modificar: `claudia-app/SKILL.md`, `evals/rodar-evals.mjs`, `evals/evals.json`, `scripts/regras.mjs`, `scripts/cdp.mjs`, `scripts/subir.mjs`, `scripts/ver.mjs`, `scripts/banco.mjs`, `scripts/observar.mjs`
- Conferir: `scripts/invocar.mjs` (e `observar.mjs`) falam CDP só por `alvos()`/`ligar()` de `cdp.mjs` e herdam `HOST`; `grep -n "127.0.0.1" .claude/skills/claudia-app/scripts/*.mjs` no fim do Passo 4 só pode mostrar o teste de `:1420` em `subir.mjs`
- Modificar: `scripts/test/regras.test.mjs`
- Criar: `scripts/test/banco.test.mjs`

**Interfaces:**
- Produz: env `CDP_HOST` (padrão `127.0.0.1`), `CDP_PORT` (padrão 9333), `CLAUDIA_DB` (padrão `%APPDATA%\io.github.johngabie.claudia-rh\claudia_rh.db`); `cdp.mjs` exporta `HOST`; `regras.mjs` exporta `hostRemoto(host) -> boolean` e `decidirSubida({ appRodando, appTemCdp, remoto }) -> "subir"|"reutilizar"|"perguntar"|"abortar"`.

- [ ] **Passo 0 (referência dos evals, antes de mudar qualquer coisa):** `node .claude/skills/claudia-teste/evals/rodar-evals.mjs sonnet` (2 rodadas); guardar o resultado por caso.
- [ ] **Passo 1: renomear**

```bash
git mv .claude/skills/claudia-teste .claude/skills/claudia-app
grep -rn "claudia-teste" .claude/skills/claudia-app
```
Trocar cada ocorrência por `claudia-app` (frontmatter `name:`, constante `NOME` e comentários de `evals/rodar-evals.mjs`, comentário de cabeçalho de `scripts/regras.mjs`, texto da `SKILL.md` e de `references/`). Em `references/limites.md`, as medições antigas continuam válidas: só o nome muda.

- [ ] **Passo 2: testes que falham** — acrescentar a `scripts/test/regras.test.mjs` (e `hostRemoto` no import):

```js
test("aceita a página do build de release", () => {
  assert.equal(validarVersaoApp(versaoApp, [{ type: "page", url: "http://tauri.localhost/" }]).ok, true);
});

test("remoto sem CDP aborta, nunca sobe tauri dev", () => {
  assert.equal(decidirSubida({ appRodando: false, appTemCdp: false, remoto: true }), "abortar");
});

test("remoto com CDP reutiliza", () => {
  assert.equal(decidirSubida({ appRodando: true, appTemCdp: true, remoto: true }), "reutilizar");
});

test("hostRemoto distingue local de remoto", () => {
  assert.equal(hostRemoto(undefined), false);
  assert.equal(hostRemoto("127.0.0.1"), false);
  assert.equal(hostRemoto("localhost"), false);
  assert.equal(hostRemoto("172.20.0.5"), true);
});
```

Criar `scripts/test/banco.test.mjs`:

```js
import test from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

test("banco.mjs lê o banco apontado por CLAUDIA_DB", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "claudia app "));
  const db = path.join(dir, "x.db");
  const d = new DatabaseSync(db); d.exec("CREATE TABLE t(v); INSERT INTO t VALUES (7);"); d.close();
  const out = execFileSync(process.execPath, [path.join(import.meta.dirname, "..", "banco.mjs"), "SELECT v FROM t"],
    { env: { ...process.env, CLAUDIA_DB: db } }).toString();
  assert.equal(JSON.parse(out)[0].v, 7);
  fs.rmSync(dir, { recursive: true, force: true });
});
```

- [ ] **Passo 3: rodar e ver falhar** — `node --test .claude/skills/claudia-app/scripts/test/` → falhas nos testes novos.

- [ ] **Passo 4: implementar**

`scripts/regras.mjs`:

```js
const PAGINA_APP = /localhost:1420|tauri\.localhost/;

/** The app's WebView2 reports an Edge build and serves the page on :1420 (tauri dev) or tauri.localhost (release). */
export function validarVersaoApp(versao, paginas) {
  if (!versao || typeof versao.Browser !== "string") {
    return { ok: false, motivo: "sem campo Browser: a porta não responde como CDP" };
  }
  if (!/^Edg\//.test(versao.Browser)) {
    return { ok: false, motivo: `Browser é ${versao.Browser}, esperado Edg/ (WebView2). A porta é de outro processo` };
  }
  const temApp = Array.isArray(paginas) && paginas.some((p) => p.type === "page" && PAGINA_APP.test(p.url || ""));
  if (!temApp) {
    return { ok: false, motivo: "Edg encontrado, mas sem página do app (localhost:1420 ou tauri.localhost)" };
  }
  return { ok: true, motivo: "porta do app confirmada" };
}

/** A CDP host other than this machine means the app runs elsewhere (e.g. a sandbox guest). */
export function hostRemoto(host) {
  return !!host && !/^(127\.0\.0\.1|localhost|::1)$/i.test(host);
}

/** Never restart the user's app on our own: running without CDP means ask. Remote targets are never started from here. */
export function decidirSubida({ appRodando, appTemCdp, remoto = false }) {
  if (remoto) return appTemCdp ? "reutilizar" : "abortar";
  if (!appRodando) return "subir";
  return appTemCdp ? "reutilizar" : "perguntar";
}
```

`scripts/cdp.mjs`: acrescentar `export const HOST = process.env.CDP_HOST || "127.0.0.1";` e trocar `127.0.0.1` por `${HOST}` em `alvos` e `novaAba`.

`scripts/ver.mjs`: importar `HOST` de `./cdp.mjs` e trocar `127.0.0.1` por `${HOST}` na linha do `/json/activate/`.

`scripts/subir.mjs`: importar `hostRemoto` de `./regras.mjs` e `HOST` de `./cdp.mjs`; trocar todo `127.0.0.1` dos `fetch` de CDP por `${HOST}` (o teste de `:1420` continua em `127.0.0.1`); logo depois dos imports, inserir:

```js
if (hostRemoto(HOST)) {
  const porta = Number(process.env.CDP_PORT ?? 9333);
  const v = await ler(`http://${HOST}:${porta}/json/version`);
  const ok = v && validarVersaoApp(v, await ler(`http://${HOST}:${porta}/json/list`)).ok;
  const decisao = decidirSubida({ appRodando: ok, appTemCdp: ok, remoto: true });
  if (decisao === "reutilizar") { console.log(JSON.stringify({ acao: "reutilizar", host: HOST, porta })); process.exit(0); }
  console.log(JSON.stringify({ acao: "abortar", motivo: `sem CDP do app em ${HOST}:${porta}; no Sandbox, inicie o app com node claudia-sandbox/scripts/cdp-guest.mjs --iniciar` }));
  process.exit(3);
}
```
(mover a função `ler` para antes desse bloco, se ela estiver declarada depois.)

`scripts/banco.mjs` e `scripts/observar.mjs`: `const dados = process.env.CLAUDIA_DB || path.join(process.env.APPDATA, "io.github.johngabie.claudia-rh", "claudia_rh.db");`

- [ ] **Passo 5: rodar e ver passar** — `node --test .claude/skills/claudia-app/scripts/test/; echo "exit=$LASTEXITCODE"` → `exit=0`.
- [ ] **Passo 6: prova de mutação** — em `decidirSubida`, trocar `"abortar"` por `"subir"`; confirmar `FAILED`/`not ok` em `remoto sem CDP aborta, nunca sobe tauri dev`; restaurar.
- [ ] **Passo 7: `SKILL.md`** — `description` (≤ 1.024 caracteres) passa a dizer que a skill olha o app rodando **localmente ou no Sandbox** (com `CDP_HOST`), mantém todos os gatilhos atuais (telas, layout, UI vs banco, agentes, comandos Tauri, QUALQUER bug → reproduzir primeiro) e acrescenta "NÃO use para montar Windows limpo, escolher dados ou instalar o app no Sandbox (claudia-sandbox)". No corpo, nova seção "Alvo" com a tabela:

| Alvo | Como |
|---|---|
| Local (`tauri dev` no host) | padrão; `node scripts/subir.mjs` |
| Sandbox | `CDP_HOST`/`CDP_PORT` impressos por `node claudia-sandbox/scripts/cdp-guest.mjs`; banco: `CLAUDIA_DB` impresso por `node claudia-sandbox/scripts/banco-guest.mjs` (snapshot; repetir após cada ação) |

- [ ] **Passo 8: evals** — em `evals/evals.json`, acrescentar "deve disparar": "tira um print da tela de Perfil", "olha o app que tá rodando no Sandbox"; "não deve disparar": "monta um Windows limpo com dados mockados", "testa o instalador num PC novo", "prepara o Sandbox com cópia dos meus dados". Rodar `node .claude/skills/claudia-app/evals/rodar-evals.mjs sonnet` (2 rodadas). Esperado: casos antigos repetem a referência do Passo 0; casos novos conforme o esperado. Se algum antigo piorar, ajustar a descrição e repetir; se não convergir, reportar.
- [ ] **Passo 9: commit** — `git commit -m "refactor(skill): rename claudia-teste to claudia-app and support a remote CDP target"`

---

### Tarefa 8: prints em escada (`claudia-app`)

**Arquivos:**
- Criar: `.claude/skills/claudia-app/scripts/print-janela.ps1`
- Criar: `.claude/skills/claudia-app/scripts/test/print-janela.test.ps1`
- Modificar: `.claude/skills/claudia-app/SKILL.md` (seção "Prints")

**Interfaces:**
- Produz: `print-janela.ps1 [-Processo claudia-rh] [-TelaInteira] [-Saida <png>]` → exit 0 e JSON `{arquivo, largura, altura}`; exit 1 com motivo (sem janela visível, janela minimizada). Padrão de `-Saida`: `debug-logs\prints\<yyyyMMdd-HHmmss>.png` na raiz do repositório.

- [ ] **Passo 1: teste (falha: script inexistente)**

```powershell
$script = Join-Path $PSScriptRoot '..\print-janela.ps1'
$fail = 0
function Check([bool]$c, [string]$n) { if ($c) { "PASS $n" } else { "FAILED $n"; $script:fail++ } }
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("print test " + [guid]::NewGuid().ToString('N').Substring(0, 8) + '.png')

$out = & pwsh -NoProfile -File $script -TelaInteira -Saida $tmp
Check ($LASTEXITCODE -eq 0) 'tela inteira exit 0'
$j = $out[-1] | ConvertFrom-Json
$bytes = [IO.File]::ReadAllBytes($tmp)
Check ($bytes[0] -eq 0x89 -and $bytes[1] -eq 0x50 -and $bytes[2] -eq 0x4E -and $bytes[3] -eq 0x47) 'arquivo e PNG'
Check ($j.largura -gt 0 -and $j.altura -gt 0) 'dimensoes positivas'
Remove-Item $tmp

$errFile = [IO.Path]::GetTempFileName()
& pwsh -NoProfile -File $script -Processo 'processo-que-nao-existe-xyz' -Saida $tmp 2>$errFile | Out-Null
Check ($LASTEXITCODE -eq 1 -and (Get-Content $errFile -Raw) -match 'TelaInteira') 'sem janela sai com 1 e sugere -TelaInteira'
Remove-Item $errFile

if ($fail) { exit 1 } else { 'OK'; exit 0 }
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: implementar**

```powershell
<#
.SYNOPSIS
  Read-only screenshot of the app's main window, or of the whole screen, without CDP.
  Captures what is on screen: a window covered by another shows the one on top.
.PARAMETER Processo     Process name (default: claudia-rh).
.PARAMETER TelaInteira  Capture the whole virtual screen (use it for native dialogs or when the window is not found).
.PARAMETER Saida        PNG path (default: debug-logs\prints\<timestamp>.png under the repo root).
#>
param([string]$Processo = 'claudia-rh', [switch]$TelaInteira, [string]$Saida = '')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsIconic(System.IntPtr h);
public struct RECT { public int Left, Top, Right, Bottom; }
'@
[void][W.U]::SetProcessDPIAware()
try {
  if (-not $Saida) {
    $root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
    $Saida = Join-Path $root ('debug-logs\prints\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.png')
  }
  New-Item -ItemType Directory -Force (Split-Path $Saida) | Out-Null
  if ($TelaInteira) {
    $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $x = $b.Left; $y = $b.Top; $w = $b.Width; $h = $b.Height
  } else {
    $p = Get-Process -Name $Processo -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if (-not $p) { throw "No visible window for process '$Processo'. Use -TelaInteira." }
    if ([W.U]::IsIconic($p.MainWindowHandle)) { throw "Window of '$Processo' is minimized: restore it or use -TelaInteira." }
    $r = New-Object W.U+RECT
    [void][W.U]::GetWindowRect($p.MainWindowHandle, [ref]$r)
    $x = $r.Left; $y = $r.Top; $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  }
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
  $bmp.Save($Saida, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output ([ordered]@{ arquivo = $Saida; largura = $w; altura = $h } | ConvertTo-Json -Compress)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 4: rodar e ver passar** — `pwsh -NoProfile -File .claude/skills/claudia-app/scripts/test/print-janela.test.ps1; echo "exit=$LASTEXITCODE"` → `exit=0`. Abrir o PNG gerado num passo manual (Read) para confirmar que mostra a tela.
- [ ] **Passo 5: prova de mutação** — remover o `throw` de "No visible window"; confirmar `FAILED sem janela sai com 1 e sugere -TelaInteira`; restaurar.
- [ ] **Passo 6: `SKILL.md`, seção "Prints"** — a tabela de escada da spec (seção "Prints (`claudia-app`)"), a regra "toda afirmação sobre a UI vem com print; se nenhum degrau funcionar, dizer quais falharam e por quê" e o aviso de dados pessoais. Pasta padrão: `debug-logs/prints/` (já ignorada pelo git).
- [ ] **Passo 7: commit** — `git commit -m "feat(skill): screenshot ladder with a native-window fallback"`

---

### Tarefa 9: CDP e banco do guest (`claudia-sandbox`)

**Arquivos:**
- Modificar: `.claude/skills/claudia-sandbox/scripts/comum.mjs` (funções de Sandbox)
- Criar: `.claude/skills/claudia-sandbox/scripts/cdp-guest.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/banco-guest.mjs`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/sandbox.test.mjs`

**Interfaces:**
- Consome: `comum.mjs`; laboratório (`C:\labsandbox\app\guest-run.ps1`, `C:\lab\tools\sqlite\sqlite3.exe`, `C:\labwork`); `wsb` no PATH do host.
- Produz em `comum.mjs`: `idsDeWsbList(texto) -> string[]`, `sandboxId(id) -> string`, `sandboxIp(id) -> string`, `noGuest(id, como, ps) -> number` (`como` = `"System"` | `"ExistingLogin"`; envia o PowerShell com `-EncodedCommand`, sem problemas de aspas; `wsb exec` não devolve a saída do processo).
- Produz: `node cdp-guest.mjs [--id] [--porta 9333] [--porta-host 19333] [--iniciar] [--exe-path <exe no guest>] [--timeout 90]` → JSON `{host, porta, browser}`; `node banco-guest.mjs [--id] [--work-dir] [--timeout 30]` → JSON `{db}` (caminho no host).

- [ ] **Passo 1: teste do parser (falha)**

```js
import test from "node:test";
import assert from "node:assert/strict";
import { idsDeWsbList } from "../comum.mjs";

test("extrai um id da saida do wsb list", () => {
  const texto = "Id                                   Status   Uptime\n6f446955-fba4-455f-af60-4da1c64f8b3b Running  00:03:10\n";
  assert.deepEqual(idsDeWsbList(texto), ["6f446955-fba4-455f-af60-4da1c64f8b3b"]);
});

test("sem sandbox, nenhum id", () => {
  assert.deepEqual(idsDeWsbList("Nenhum ambiente"), []);
  assert.deepEqual(idsDeWsbList(undefined), []);
});
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: acrescentar a `comum.mjs`** (e `import { spawnSync } from "node:child_process";` no topo)

```js
const GUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi;

export function idsDeWsbList(texto) {
  return [...new Set(String(texto ?? "").match(GUID) ?? [])];
}

function wsb(args) {
  const r = spawnSync("wsb", args, { encoding: "utf8" });
  if (r.error) throw new Error(`wsb is not available: ${r.error.message}`);
  return r;
}

export function sandboxId(id) {
  if (id) return id;
  const ids = idsDeWsbList(wsb(["list"]).stdout);
  if (ids.length !== 1) throw new Error(`Expected exactly one running sandbox, found ${ids.length}. Start one with the lab's test-up.ps1 or pass --id.`);
  return ids[0];
}

export function sandboxIp(id) {
  const m = (wsb(["ip", "--id", id]).stdout ?? "").match(/\d{1,3}(\.\d{1,3}){3}/);
  if (!m) throw new Error(`Could not read the sandbox IP (wsb ip --id ${id}). The profile needs networking enabled.`);
  return m[0];
}

/** Runs a PowerShell snippet in the guest. -EncodedCommand avoids quoting issues; wsb exec returns no process output. */
export function noGuest(id, como, ps) {
  const enc = Buffer.from(ps, "utf16le").toString("base64");
  return wsb(["exec", "--id", id, "-r", como, "-c", `powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${enc}`]).status;
}
```

- [ ] **Passo 4: `cdp-guest.mjs`**

```js
// Exposes the app's WebView2 DevTools port (inside the sandbox guest) to the host, optionally starting the app with CDP on.
// Prints {host, porta, browser}: use them as CDP_HOST / CDP_PORT with the claudia-app scripts.
// Usage: node cdp-guest.mjs [--id <sandbox>] [--porta 9333] [--porta-host 19333] [--iniciar] [--exe-path <exe in guest>] [--timeout 90]
//   --iniciar   start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta>
//   --exe-path  app exe inside the guest (e.g. the installed copy); without it, the lab's guest-run.ps1 starts the published exe
import { parseArgs } from "node:util";
import { sandboxId, sandboxIp, noGuest, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: {
    id: { type: "string" }, porta: { type: "string", default: "9333" }, "porta-host": { type: "string", default: "19333" },
    iniciar: { type: "boolean" }, "exe-path": { type: "string" }, timeout: { type: "string", default: "90" },
  } });
  const porta = Number(values.porta), portaHost = Number(values["porta-host"]), timeout = Number(values.timeout);
  const id = sandboxId(values.id);
  const ip = sandboxIp(id);
  noGuest(id, "System", [
    `netsh interface portproxy delete v4tov4 listenaddress=0.0.0.0 listenport=${portaHost} | Out-Null`,
    `netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=${portaHost} connectaddress=127.0.0.1 connectport=${porta} | Out-Null`,
    "Remove-NetFirewallRule -DisplayName 'claudia-cdp' -ErrorAction SilentlyContinue",
    `New-NetFirewallRule -DisplayName 'claudia-cdp' -Direction Inbound -Action Allow -Protocol TCP -LocalPort ${portaHost} | Out-Null`,
  ].join("\n"));
  if (values.iniciar) {
    const exe = values["exe-path"];
    const start = exe ? `Start-Process '${exe.replace(/'/g, "''")}'` : "& C:\\labsandbox\\app\\guest-run.ps1";
    noGuest(id, "ExistingLogin", `$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=${porta}'; ${start}`);
  }
  const url = `http://${ip}:${portaHost}/json/version`;
  const fim = Date.now() + timeout * 1000;
  let v = null;
  while (!v && Date.now() < fim) {
    try { v = await (await fetch(url, { signal: AbortSignal.timeout(3000) })).json(); }
    catch { await new Promise((r) => setTimeout(r, 2000)); }
  }
  if (!v) throw new Error(`CDP did not answer at ${url} within ${timeout}s. Check that WebView2 exists in the guest and the app is running (the claudia-app screenshot ladder shows why).`);
  console.log(JSON.stringify({ host: ip, porta: portaHost, browser: v.Browser }));
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 5: `banco-guest.mjs`**

```js
// Copies a consistent snapshot of the app database from the sandbox guest to the host.
// Prints {db}: use it as CLAUDIA_DB with the claudia-app scripts. Repeat after each action that changes data.
// Usage: node banco-guest.mjs [--id <sandbox>] [--work-dir <folder>] [--timeout 30]   (--work-dir default $env:LAB_WORK_DIR)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { APP_ID, sandboxId, noGuest, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { id: { type: "string" }, "work-dir": { type: "string" }, timeout: { type: "string", default: "30" } } });
  const id = sandboxId(values.id);
  const workDir = values["work-dir"] || process.env.LAB_WORK_DIR;
  if (!workDir) throw new Error("LAB_WORK_DIR is not set: pass --work-dir (the folder used by publicar.mjs).");
  const hostDb = path.join(workDir, "guest-rw", "db", "claudia_rh.db");
  fs.rmSync(hostDb, { force: true });
  noGuest(id, "ExistingLogin", [
    "New-Item -ItemType Directory -Force C:\\labwork\\db | Out-Null",
    `& C:\\lab\\tools\\sqlite\\sqlite3.exe (Join-Path $env:APPDATA '${APP_ID}\\claudia_rh.db') '.backup C:/labwork/db/claudia_rh.db'`,
  ].join("\n"));
  const fim = Date.now() + Number(values.timeout) * 1000;
  while (!fs.existsSync(hostDb) && Date.now() < fim) await new Promise((r) => setTimeout(r, 1000));
  if (!fs.existsSync(hostDb)) throw new Error(`Snapshot not found at ${hostDb} (does the app database exist in the guest yet?).`);
  console.log(JSON.stringify({ db: hostDb }));
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 6: rodar a suíte** — `node --test .claude/skills/claudia-sandbox/scripts/test/; echo "exit=$LASTEXITCODE"` → `exit=0`. `cdp-guest.mjs` e `banco-guest.mjs` são verificados no smoke (Tarefa 11), porque dependem de um Sandbox rodando.
- [ ] **Passo 7: commit** — `git commit -m "feat(skill): expose the guest app's CDP and database to the host"`

---

### Tarefa 10: `CLAUDE.md`

**Arquivos:**
- Modificar: `CLAUDE.md`

- [ ] **Passo 1:** na seção "## Superpowers", trocar `claudia-teste` por `claudia-app`.
- [ ] **Passo 2:** inserir logo depois de "## Build":

```markdown
## Teste em máquina limpa

| Quero… | Skill |
|---|---|
| ver, clicar ou conferir o app rodando, tirar print, reproduzir bug | `claudia-app` (local ou no Sandbox) |
| montar um Windows limpo, escolher os dados, publicar ou instalar o app, ligar o CDP no guest | `claudia-sandbox` |

A `claudia-sandbox` usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável `SANDBOX_LAB_DIR`. Cenários de dados: `novo`, `mock` e `copia` (este só com autorização do usuário na rodada). Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.
```

- [ ] **Passo 3:** `git diff CLAUDE.md` mostra só essas duas mudanças; `grep -n "claudia-teste" CLAUDE.md` não encontra nada.
- [ ] **Passo 4:** commit — `git commit -m "docs: route app observation and clean-machine testing to their skills"`

---

### Tarefa 11: smoke

Executor: agente Opus. Pré-requisitos: `SANDBOX_LAB_DIR` definido; nenhum Sandbox rodando (`wsb list` sem ids). Evidências em `%TEMP%\claudia-sandbox\evidencias\` (fora do repositório).

- [ ] **Passo 1: build** com target fora do repo: `$env:CARGO_TARGET_DIR="$env:TEMP\claudia-sandbox\target"; npm ci; npx tauri build --bundles nsis`. Registrar tempo, o `.exe` do app (`$env:CARGO_TARGET_DIR\release\claudia-rh.exe`) e o instalador (`$env:CARGO_TARGET_DIR\release\bundle\nsis\*.exe`).
- [ ] **Passo 2: cenários `novo` e `mock` com o exe solto.** Para cada `<c>`:
  1. `node .claude/skills/claudia-sandbox/scripts/cenario-<c>.mjs`
  2. `node .claude/skills/claudia-sandbox/scripts/publicar.mjs --app <exe> --cenario %TEMP%\claudia-sandbox\<c>`
  3. `$env:LAB_APP_PROJECT=<raiz do repo>; $env:LAB_WORK_DIR=<workDir impresso no passo 2>; & "$env:SANDBOX_LAB_DIR\scripts\test-up.ps1" -Config lab.config.app-winapp.json`
  4. (`<id>` = id impresso pelo `test-up.ps1`, também em `wsb list`) `wsb exec --id <id> -r System -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-firewall.ps1"`
  5. `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1"`
  6. Prints pela escada do Sandbox (degraus 2 e 3; o 1 depende de CDP): `winapp ui screenshot --on sandbox -a claudia-rh -o …\<c>-janela.png`, `winapp target screenshot sandbox -o …\<c>-tela.png`; ler os PNGs e descrever o que mostram.
  7. `& "$env:SANDBOX_LAB_DIR\scripts\test-down.ps1"`
- [ ] **Passo 3: instalador + CDP (cenário `mock`).**
  1. Cenário `mock`, `publicar.mjs --app <instalador>`, `test-up.ps1 -Config lab.config.app-winapp.json` (sem firewall ainda: o instalador pode baixar o WebView2).
  2. `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1 -NoStart"` (copia os dados) e `wsb exec --id <id> -r ExistingLogin -c "C:\appdeploy\app\<instalador>.exe /S"`; aguardar; localizar o exe instalado (`winapp target exec sandbox -- powershell -c "Get-ChildItem $env:LOCALAPPDATA,$env:ProgramFiles -Recurse -Filter claudia-rh.exe -ErrorAction SilentlyContinue | Select -Expand FullName"`).
  3. Firewall (`guest-firewall.ps1` como System).
  4. `node .claude/skills/claudia-sandbox/scripts/cdp-guest.mjs --iniciar --exe-path <exe instalado>` → `{host, porta}`.
  5. `$env:CDP_HOST=<host>; $env:CDP_PORT=<porta>; node .claude/skills/claudia-app/scripts/subir.mjs` (esperado `reutilizar`), `ver.mjs arvore`, `ver.mjs tela …\mock-cdp.png` (degrau 1).
  6. `node .claude/skills/claudia-sandbox/scripts/banco-guest.mjs` → `$env:CLAUDIA_DB=<db>; node .claude/skills/claudia-app/scripts/banco.mjs "SELECT count(*) AS n FROM vagas"` (esperado `3`).
  7. Registrar a origem real da página do app (`/json/list`) e confrontar com a regra `tauri.localhost`; divergência → corrigir a regra e o teste (Tarefa 7) antes de seguir.
  8. `test-down.ps1`.
- [ ] **Passo 4: local** — com o app do usuário fechado ou com autorização para subir um `tauri dev`: `node .claude/skills/claudia-app/scripts/subir.mjs`, `ver.mjs arvore`, `pwsh .claude/skills/claudia-app/scripts/print-janela.ps1`. Se o app do usuário estiver aberto sem CDP, `subir.mjs` devolve `perguntar`: perguntar antes.
- [ ] **Passo 5:** cenário `copia` **só** se o usuário autorizar nesta rodada; sem autorização, registrar "não verificado: aguarda autorização".
- [ ] **Passo 6: relatório** em `%TEMP%\claudia-sandbox\evidencias\relatorio.md`: por passo, comandos, exit codes, o que cada print mostrou, qual degrau da escada funcionou. Falha do app é achado (nome, passo, print) para virar task no Linear; não é contornada.

---

### Tarefa 12: revisão e varredura

Executor: agente Opus, independente dos implementadores.

- [ ] **Passo 1:** revisão de código da branch inteira contra a spec (`requesting-code-review`): contrato dos cenários, exit codes, origem só lida, `disparo.json` completo, nenhuma escrita fora de `--saida`/`--work-dir`, `subir.mjs` nunca sobe `tauri dev` com alvo remoto, regras de segurança da `claudia-app` intactas.
- [ ] **Passo 2:** varredura de agnosticismo em `git diff dev --name-only`: procurar referências a sessões de trabalho, pessoas, modelos de IA usados na construção, nomes de máquina, IPs, usuários do Windows, caminhos absolutos pessoais ou pastas externas além das variáveis permitidas nas restrições globais. Cada ocorrência: arquivo:linha e correção.
- [ ] **Passo 3:** rodar de novo `node --test .claude/skills/claudia-sandbox/scripts/test/`, `node --test .claude/skills/claudia-app/scripts/test/` e `pwsh -NoProfile -File .claude/skills/claudia-app/scripts/test/print-janela.test.ps1`; conferir `exit=0` em cada um.

---

### Tarefa 13: integração (decisão do usuário)

- [ ] **Passo 1:** apresentar ao usuário o resumo: commits, resultado dos testes, evals, smoke e revisão. Aguardar a decisão.
- [ ] **Passo 2 (com aprovação):** `git checkout dev && git merge --no-ff feat/clean-machine-testing`.
- [ ] **Passo 3 (com aprovação):** `git checkout feat/browser-cdp-mcp && git rebase dev`. Conflito esperado em `CLAUDE.md` (conferir antes com `git diff dev feat/browser-cdp-mcp -- CLAUDE.md`): manter o conteúdo da branch, reinserir a seção "Teste em máquina limpa" depois de "## Build" e trocar toda referência `claudia-teste`/`/claudia-teste` por `claudia-app`/`/claudia-app` (`grep -n "claudia-teste" CLAUDE.md` vazio no fim). Arquivos da branch que citem `.claude/skills/claudia-teste/` fora de `docs/superpowers/` também passam ao caminho novo. Depois do rebase, rodar as rodadas de teste definidas no `CLAUDE.md` resultante (unitários: `cargo test --lib` em `src-tauri/` e `npx tsc --noEmit`) e as três suítes da Tarefa 12, Passo 3; conferir exit codes.
