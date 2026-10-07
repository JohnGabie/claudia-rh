# Skill `claudia-teste` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Criar a skill `claudia-teste`, que conecta ao Claudia RH rodando (Tauri) por CDP para ver a tela, chamar comandos do backend e observar agentes, e torná-la passo obrigatório do ciclo de desenvolvimento.

**Architecture:** Skill única em `claudia-rh/.claude/skills/claudia-teste/`. Um módulo de regras puras (`regras.mjs`, testável sem app) é importado pelos scripts de I/O (`subir`, `ver`, `invocar`, `banco`, `observar`). Os dois limites ainda não verificados (ler a aba Terminal; clicar na UI do app) são testados **primeiro**, e o resultado decide o que `observar.mjs` promete.

**Tech Stack:** Node ≥ 22 (`WebSocket` global, `node:sqlite`, `node:test`), zero dependências npm. CDP no WebView2 via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta>`.

**Spec:** `docs/superpowers/specs/2026-10-01-skill-claudia-teste-design.md`

## Global Constraints

- **Zero dependências.** Nenhum `npm install`. Só APIs nativas do Node ≥ 22. (regra "nenhum passo de instalação novo", `CLAUDE.md`)
- **A porta de debug só existe no `tauri dev`.** Nunca no instalador nem em build de release.
- **Comentários de código e mensagens de commit: inglês.** Docs e `SKILL.md`: pt-BR. (`CLAUDE.md`)
- **Nunca mudar o `identifier` Tauri** em `src-tauri/tauri.conf.json`.
- **Não editar `src-tauri/src/prompt_sistema_runtime.md`.**
- **A skill nunca interage com sites de vagas** e nunca clica em algo que submeta candidatura.
- **Escrita no banco só por `invoke`.** `banco.mjs` abre o SQLite em modo somente leitura.
- **Não reiniciar o app do usuário sem perguntar**, e só encerrar processo que a própria skill subiu.
- **A alteração do `CLAUDE.md` (Tarefa 8) é a única mudança fora de `.claude/skills/claudia-teste/`** e de `docs/`.
- Atribuição nos commits: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus

Modos de falha que a spec implica e que os testes de cada tarefa fixam:

1. **A porta é de outro processo** (já aconteceu: a 9222 era o Chrome pessoal). `validarVersaoApp` tem de rejeitar uma resposta que não seja `Edg/…` com página `localhost:1420`. Pinado na Tarefa 3.
2. **O app já roda sem CDP.** A skill tem de decidir "perguntar", nunca reiniciar sozinha. Pinado na Tarefa 3.
3. **SQL de escrita chegando ao `banco.mjs`.** `INSERT`, `UPDATE`, `DELETE`, `DROP`, `ATTACH`, `PRAGMA` com escrita e múltiplos statements têm de ser recusados. Pinado na Tarefa 5.
4. **Comando Tauri que escreve chamado sem querer.** Prefixo desconhecido conta como escrita e exige `--escrever`. Pinado na Tarefa 5.
5. **Clique em rótulo destrutivo** (`apagar`, `excluir`, `remover`, `enviar`, `submit`). A guarda lê o rótulo antes de clicar. Pinado na Tarefa 4.

---

## File Structure

| Arquivo | Responsabilidade |
|---|---|
| `.claude/skills/claudia-teste/SKILL.md` | quando usar, fluxo, o que nunca fazer (curto) |
| `.../scripts/cdp.mjs` | cliente WebSocket CDP (copiado do protótipo) |
| `.../scripts/regras.mjs` | **regras puras**, sem I/O: validar porta, classificar comando, SQL, rótulo, decisão de subida |
| `.../scripts/subir.mjs` | sobe o `tauri dev` com CDP, escolhe porta livre, libera a 1420 |
| `.../scripts/ver.mjs` | árvore, screenshot, clique por rótulo |
| `.../scripts/invocar.mjs` | `invoke("comando", args)` |
| `.../scripts/banco.mjs` | consulta somente leitura em `claudia_rh.db` |
| `.../scripts/observar.mjs` | terminal da sessão, log, tabelas de sessão |
| `.../scripts/gerar-comandos.mjs` | gera `references/comandos.md` por grep de `#[tauri::command]` |
| `.../scripts/test/regras.test.mjs` | testes de `regras.mjs` |
| `.../references/limites.md` | resultado dos dois limites testados (Tarefas 1–2) |
| `.../references/padroes-cdp.md` | armadilhas do CDP |
| `.../references/comandos.md` | gerado, marcado como derivado |
| `.../evals/evals.json` | prompts que devem / não devem disparar |

Rodar todos os testes: `node --test .claude/skills/claudia-teste/scripts/test/` (a partir de `claudia-rh/`).

---

### Task 1: Limite 1 — ler a aba Terminal por CDP

**Files:**
- Create: `.claude/skills/claudia-teste/scripts/cdp.mjs` (copiado)
- Create: `.claude/skills/claudia-teste/references/limites.md`

**Interfaces:**
- Consumes: nada.
- Produces: `cdp.mjs` exporta `sleep(ms)`, `alvos(port)`, `ligar(url)` → `{ ws, cdp(method, params), eventos, fechar() }`. E o **veredito do limite 1** em `limites.md`, do qual a Tarefa 6 depende.

Esta tarefa é uma **medição**, não TDD: o "teste" é uma observação com resultado sim/não registrado em `limites.md`.

- [ ] **Step 1: Trazer o cliente CDP do protótipo**

```bash
cd claudia-rh
mkdir -p .claude/skills/claudia-teste/scripts .claude/skills/claudia-teste/references
git show feat/exec-prototipo-playwright:prototipo-playwright/cdp/cdp.mjs > .claude/skills/claudia-teste/scripts/cdp.mjs
node -e "import('./.claude/skills/claudia-teste/scripts/cdp.mjs').then(m=>console.log(Object.keys(m).join(',')))"
```
Expected: `sleep,alvos,ligar,novaAba,ok`

- [ ] **Step 2: Subir o app com CDP, à mão**

A porta 9222 costuma ser do Chrome pessoal; usar 9333. Se a 1420 estiver ocupada por um vite antigo, liberá-la primeiro.

```bash
P=$(netstat -ano | grep ":1420.*LISTENING" | awk '{print $5}' | head -1); [ -n "$P" ] && taskkill //PID $P //T //F
export WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9333"
nohup npm run tauri dev > /tmp/tauri-cdp.log 2>&1 &
for i in $(seq 1 40); do curl -s -m 2 http://127.0.0.1:9333/json/version | grep -q Edg && break; sleep 3; done
curl -s http://127.0.0.1:9333/json/version | grep Browser
```
Expected: `"Browser": "Edg/…"`.

- [ ] **Step 3: Sondar onde o texto do Terminal está**

Criar `/tmp/sonda-terminal.mjs`:

```js
// Probe: is the xterm text readable from the DOM, from the AX tree, or neither?
import { alvos, ligar, sleep } from ".claude/skills/claudia-teste/scripts/cdp.mjs";

const lista = await alvos(9333);
const alvo = lista.find((t) => t.type === "page" && /1420/.test(t.url));
const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
await cdp("Runtime.enable"); await cdp("Accessibility.enable");
const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;

// Open the Terminal tab through the DOM: the sidebar buttons are named.
await ev(`[...document.querySelectorAll("button")].find(b => /terminal/i.test(b.getAttribute("aria-label")||b.innerText||""))?.click()`);
await sleep(2000);

console.log("xterm presente:", await ev(`!!document.querySelector(".xterm")`));
console.log("renderer DOM (.xterm-rows):", await ev(`!!document.querySelector(".xterm-rows")`));
console.log("renderer canvas:", await ev(`document.querySelectorAll(".xterm canvas").length`));
console.log("linhas no DOM:", await ev(`document.querySelectorAll(".xterm-rows > div").length`));
console.log("texto (200 chars):", JSON.stringify((await ev(`document.querySelector(".xterm-rows")?.innerText||""`)).slice(0, 200)));
const nos = (await cdp("Accessibility.getFullAXTree")).result.nodes.filter((n) => n.name?.value && /terminal/i.test(n.role?.value + n.name.value));
console.log("nós de AX com 'terminal':", nos.length);
fechar();
```

- [ ] **Step 4: Rodar a sonda e interpretar**

Run: `node /tmp/sonda-terminal.mjs`

Interpretação (anotar exatamente uma):
- `.xterm-rows` existe **e** `linhas no DOM > 0` **e** o texto não é vazio → **DOM legível**. Tarefa 6 lê por `.xterm-rows`.
- `canvas > 0` e `.xterm-rows` ausente → **canvas, texto ilegível pelo DOM**. Tarefa 6 só observa por log e banco, e a limitação é registrada.
- Terminal vazio porque nenhuma sessão rodou → repetir o Step 4 depois de iniciar uma sessão **pela UI**. Não conta como veredito.

- [ ] **Step 5: Registrar o veredito**

Criar `references/limites.md`:

```markdown
# Limites testados da skill

Cada limite aqui foi **medido** contra o app rodando, não deduzido. Data e versão abaixo.

## 1. Ler a aba Terminal por CDP

- **Veredito:** <DOM legível | canvas ilegível | inconclusivo>
- **Evidência:** <saída da sonda: xterm presente, renderer, nº de linhas, 200 chars de texto>
- **Consequência:** <o que `observar.mjs` faz por causa disto>
- **Data / app:** 2026-10-01, tauri dev, Edg/<versão>
```

Substituir cada `<…>` pelo valor real da sonda. Nenhum `<…>` pode sobrar.

- [ ] **Step 6: Commit**

```bash
git add .claude/skills/claudia-teste/scripts/cdp.mjs .claude/skills/claudia-teste/references/limites.md
git commit -m "feat(skill): add CDP client and record whether the Terminal tab is readable

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Limite 2 — clicar na UI do app

**Files:**
- Modify: `.claude/skills/claudia-teste/references/limites.md`

**Interfaces:**
- Consumes: `cdp.mjs` da Tarefa 1; app rodando com CDP na porta 9333.
- Produces: o **veredito do limite 2** em `limites.md`, que define como `ver.mjs` (Tarefa 4) clica: por DOM `.click()` ou por `Input.dispatchMouseEvent`.

Medição, não TDD.

- [ ] **Step 1: Escrever a sonda de clique**

Criar `/tmp/sonda-clique.mjs`:

```js
// Probe: does a click on the app UI change state, and which mechanism works?
import { alvos, ligar, sleep } from ".claude/skills/claudia-teste/scripts/cdp.mjs";

const lista = await alvos(9333);
const alvo = lista.find((t) => t.type === "page" && /1420/.test(t.url));
await fetch(`http://127.0.0.1:9333/json/activate/${alvo.id}`).catch(() => {});
const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
await cdp("Runtime.enable"); await cdp("DOM.enable"); await cdp("Accessibility.enable"); await cdp("Page.bringToFront");
const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;
const titulo = () => ev(`document.querySelector("h1,h2")?.innerText`);

// Safe, reversible navigation target: the History tab. It only changes the view.
const antes = await titulo();

// Mechanism A: DOM click on a named button.
await ev(`[...document.querySelectorAll("button")].find(b => /history/i.test(b.getAttribute("aria-label")||b.innerText||""))?.click()`);
await sleep(1500);
const depoisA = await titulo();
console.log(`A) DOM click: "${antes}" -> "${depoisA}"  ${antes !== depoisA ? "MUDOU" : "nada"}`);

// Back to Dashboard, then Mechanism B: real mouse event at the button's centre.
await ev(`[...document.querySelectorAll("button")].find(b => /dashboard/i.test(b.getAttribute("aria-label")||b.innerText||""))?.click()`);
await sleep(1500);
const base = await titulo();
const r = JSON.parse(await ev(`(() => { const b=[...document.querySelectorAll("button")].find(b=>/history/i.test(b.getAttribute("aria-label")||b.innerText||"")); b.scrollIntoView({block:"center"}); const q=b.getBoundingClientRect(); return JSON.stringify({x:Math.round(q.x+q.width/2), y:Math.round(q.y+q.height/2), sob: document.elementFromPoint(q.x+q.width/2,q.y+q.height/2)?.closest("button")===b}); })()`));
console.log("   centro:", r.x, r.y, "elemento sob o ponto é o botão:", r.sob);
for (const type of ["mousePressed", "mouseReleased"]) await cdp("Input.dispatchMouseEvent", { type, x: r.x, y: r.y, button: "left", clickCount: 1 });
await sleep(1500);
const depoisB = await titulo();
console.log(`B) mouse CDP: "${base}" -> "${depoisB}"  ${base !== depoisB ? "MUDOU" : "nada"}`);
fechar();
```

- [ ] **Step 2: Rodar**

Run: `node /tmp/sonda-clique.mjs`
Expected: duas linhas `A)` e `B)`, cada uma `MUDOU` ou `nada`. O destino (History) só troca a vista, não escreve nada.

- [ ] **Step 3: Interpretar**

- A `MUDOU` e B `MUDOU` → ambos os mecanismos funcionam; `ver.mjs` usa o mouse CDP com guarda `elementFromPoint` (mais fiel a um usuário).
- Só A `MUDOU` → `ver.mjs` usa DOM `.click()`.
- Só B `MUDOU` → `ver.mjs` usa mouse CDP.
- Nenhum `MUDOU` → limite real; registrar e parar. Não inventar contorno.

- [ ] **Step 4: Registrar**

Acrescentar a `references/limites.md`:

```markdown
## 2. Clicar na UI do app

- **Veredito:** <ambos | só DOM | só mouse CDP | nenhum>
- **Evidência:** <as linhas A) e B) da sonda, com o centro e `elementFromPoint`>
- **Consequência:** <mecanismo que `ver.mjs` usa>
- **Data / app:** 2026-10-01, tauri dev, Edg/<versão>
```

Nenhum `<…>` pode sobrar.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/claudia-teste/references/limites.md
git commit -m "docs(skill): record which click mechanism works on the app UI

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Regras puras e `subir.mjs`

**Files:**
- Create: `.claude/skills/claudia-teste/scripts/regras.mjs`
- Create: `.claude/skills/claudia-teste/scripts/test/regras.test.mjs`
- Create: `.claude/skills/claudia-teste/scripts/subir.mjs`

**Interfaces:**
- Consumes: nada.
- Produces (`regras.mjs`):
  - `validarVersaoApp(versao, paginas) -> { ok: boolean, motivo: string }` — `versao` é o JSON de `/json/version`, `paginas` o de `/json/list`.
  - `decidirSubida({ appRodando: boolean, appTemCdp: boolean }) -> "subir" | "reutilizar" | "perguntar"`.
  - `podeMatarDonoDaPorta(cmd: string) -> boolean`.

- [ ] **Step 1: Escrever os testes que falham**

`scripts/test/regras.test.mjs`:

```js
import test from "node:test";
import assert from "node:assert/strict";
import { validarVersaoApp, decidirSubida, podeMatarDonoDaPorta } from "../regras.mjs";

const versaoApp = { Browser: "Edg/154.0.4258.48", "Protocol-Version": "1.3" };
const paginaApp = [{ type: "page", url: "http://localhost:1420/" }];

test("aceita a porta quando é Edge com a página do app", () => {
  assert.equal(validarVersaoApp(versaoApp, paginaApp).ok, true);
});

test("rejeita Chrome comum na porta (caso real: 9222 era o Chrome pessoal)", () => {
  const r = validarVersaoApp({ Browser: "Chrome/153.0.0.0" }, paginaApp);
  assert.equal(r.ok, false);
  assert.match(r.motivo, /Edg/);
});

test("rejeita Edge sem a página do app", () => {
  const r = validarVersaoApp(versaoApp, [{ type: "page", url: "https://example.com/" }]);
  assert.equal(r.ok, false);
  assert.match(r.motivo, /1420/);
});

test("rejeita resposta vazia ou sem Browser", () => {
  assert.equal(validarVersaoApp({}, []).ok, false);
  assert.equal(validarVersaoApp(null, null).ok, false);
});

test("app rodando sem CDP: pergunta, nunca reinicia sozinha", () => {
  assert.equal(decidirSubida({ appRodando: true, appTemCdp: false }), "perguntar");
});

test("app rodando com CDP: reutiliza", () => {
  assert.equal(decidirSubida({ appRodando: true, appTemCdp: true }), "reutilizar");
});

test("app parado: sobe", () => {
  assert.equal(decidirSubida({ appRodando: false, appTemCdp: false }), "subir");
});

test("só mata dono da porta que seja vite ou tauri", () => {
  assert.equal(podeMatarDonoDaPorta("node.exe ... vite --port 1420"), true);
  assert.equal(podeMatarDonoDaPorta("claudia-rh.exe"), true);
  assert.equal(podeMatarDonoDaPorta('"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"'), false);
  assert.equal(podeMatarDonoDaPorta(""), false);
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: FAIL com `Cannot find module '../regras.mjs'`.

- [ ] **Step 3: Implementar `regras.mjs` (parte 1)**

```js
// Pure rules for the claudia-teste skill. No I/O here, so every rule that
// protects the user can be pinned by a test that needs no running app.

/** The app's WebView2 reports an Edge build and serves the page on :1420. */
export function validarVersaoApp(versao, paginas) {
  if (!versao || typeof versao.Browser !== "string") {
    return { ok: false, motivo: "sem campo Browser: a porta não responde como CDP" };
  }
  if (!/^Edg\//.test(versao.Browser)) {
    return { ok: false, motivo: `Browser é ${versao.Browser}, esperado Edg/ (WebView2). A porta é de outro processo` };
  }
  const temApp = Array.isArray(paginas) && paginas.some((p) => p.type === "page" && /localhost:1420/.test(p.url || ""));
  if (!temApp) {
    return { ok: false, motivo: "Edg encontrado, mas sem página em localhost:1420" };
  }
  return { ok: true, motivo: "porta do app confirmada" };
}

/** Never restart the user's app on our own: running without CDP means ask. */
export function decidirSubida({ appRodando, appTemCdp }) {
  if (!appRodando) return "subir";
  return appTemCdp ? "reutilizar" : "perguntar";
}

/** Only free :1420 when its owner is something we recognise as ours. */
export function podeMatarDonoDaPorta(cmd) {
  return /vite|tauri|claudia-rh/i.test(cmd || "");
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: `# pass 8`, `# fail 0`.

- [ ] **Step 5: Implementar `subir.mjs`**

```js
// Starts `tauri dev` with a CDP port, or reuses one already running.
// Usage: node subir.mjs [--porta 9333]
import { spawn, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { validarVersaoApp, decidirSubida, podeMatarDonoDaPorta } from "./regras.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const CANDIDATAS = [9333, 9334, 9335, 9336];

async function ler(url) {
  try { return await (await fetch(url, { signal: AbortSignal.timeout(2000) })).json(); }
  catch { return null; }
}

/** A port is free only if nothing answers at all — an answer, even a 404, means it is taken. */
async function portaLivre(porta) {
  try { await fetch(`http://127.0.0.1:${porta}/json/version`, { signal: AbortSignal.timeout(1500) }); return false; }
  catch { return true; }
}

async function acharAppComCdp() {
  for (const p of CANDIDATAS) {
    const v = await ler(`http://127.0.0.1:${p}/json/version`);
    if (!v) continue;
    const r = validarVersaoApp(v, await ler(`http://127.0.0.1:${p}/json/list`));
    if (r.ok) return p;
  }
  return null;
}

function donoDaPorta(porta) {
  const linha = execSync(`netstat -ano | findstr :${porta} | findstr LISTENING`, { encoding: "utf8" }).split("\n")[0] || "";
  const pid = linha.trim().split(/\s+/).pop();
  if (!pid) return null;
  const cmd = execSync(
    `powershell -NoProfile -Command "(Get-CimInstance Win32_Process -Filter 'ProcessId=${pid}').CommandLine"`,
    { encoding: "utf8" }).trim();
  return { pid, cmd };
}

const portaApp = await acharAppComCdp();
const appRodando = portaApp !== null || (await ler("http://127.0.0.1:1420/")) !== null;
const decisao = decidirSubida({ appRodando, appTemCdp: portaApp !== null });

if (decisao === "reutilizar") { console.log(JSON.stringify({ acao: "reutilizar", porta: portaApp })); process.exit(0); }
if (decisao === "perguntar") {
  console.log(JSON.stringify({ acao: "perguntar", motivo: "o app já roda sem CDP; reiniciar fecha a sessão do usuário" }));
  process.exit(2);
}

// Free :1420 only if a stale vite of ours holds it.
const dono = (() => { try { return donoDaPorta(1420); } catch { return null; } })();
if (dono) {
  if (!podeMatarDonoDaPorta(dono.cmd)) {
    console.log(JSON.stringify({ acao: "abortar", motivo: `:1420 é de outro processo (${dono.cmd.slice(0, 60)})` }));
    process.exit(3);
  }
  execSync(`taskkill /PID ${dono.pid} /T /F`);
}

let porta = null;
for (const p of CANDIDATAS) { if (await portaLivre(p)) { porta = p; break; } }
if (!porta) { console.log(JSON.stringify({ acao: "abortar", motivo: "nenhuma porta livre entre 9333 e 9336" })); process.exit(3); }

const log = path.join(process.env.TEMP || ".", "tauri-cdp.log");
const fd = fs.openSync(log, "w");
const filho = spawn("npm", ["run", "tauri", "dev"], {
  cwd: RAIZ, shell: true, detached: true, stdio: ["ignore", fd, fd],
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${porta}` },
});
filho.unref();

for (let i = 0; i < 60; i++) {
  await new Promise((r) => setTimeout(r, 3000));
  const v = await ler(`http://127.0.0.1:${porta}/json/version`);
  if (v && validarVersaoApp(v, await ler(`http://127.0.0.1:${porta}/json/list`)).ok) {
    console.log(JSON.stringify({ acao: "subiu", porta, pid: filho.pid, log }));
    process.exit(0);
  }
}
console.log(JSON.stringify({ acao: "falhou", motivo: "CDP não respondeu em 180 s", log }));
process.exit(4);
```

- [ ] **Step 6: Verificar contra o app real**

Com o app parado: `node .claude/skills/claudia-teste/scripts/subir.mjs`
Expected: linha JSON com `"acao":"subiu"` e uma `porta` entre 9333 e 9336.

Rodar de novo, com o app já rodando: `node .claude/skills/claudia-teste/scripts/subir.mjs`
Expected: `"acao":"reutilizar"` com a mesma porta.

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/claudia-teste/scripts/regras.mjs .claude/skills/claudia-teste/scripts/subir.mjs .claude/skills/claudia-teste/scripts/test/regras.test.mjs
git commit -m "feat(skill): add port validation rules and the app launcher

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `ver.mjs` — árvore, screenshot e clique por rótulo

**Files:**
- Modify: `.claude/skills/claudia-teste/scripts/regras.mjs`
- Modify: `.claude/skills/claudia-teste/scripts/test/regras.test.mjs`
- Create: `.claude/skills/claudia-teste/scripts/ver.mjs`

**Interfaces:**
- Consumes: `ligar`, `alvos`, `sleep` de `cdp.mjs`; o veredito do limite 2 em `references/limites.md` (decide o mecanismo de clique).
- Produces: `rotuloPerigoso(rotulo: string) -> boolean` em `regras.mjs`; CLI `ver.mjs arvore | tela [arquivo] | clicar "<rótulo>"`.

- [ ] **Step 1: Escrever o teste que falha**

Acrescentar a `regras.test.mjs`:

```js
import { rotuloPerigoso } from "../regras.mjs";

test("rótulos destrutivos ou de envio são perigosos", () => {
  for (const r of ["Apagar perfil", "Excluir", "Remover vaga", "Enviar candidatura", "Submit application"]) {
    assert.equal(rotuloPerigoso(r), true, r);
  }
});

test("rótulos de navegação não são perigosos", () => {
  for (const r of ["Dashboard", "Profile", "History", "Notifications 9+", "Terminal"]) {
    assert.equal(rotuloPerigoso(r), false, r);
  }
});

test("rótulo vazio é tratado como perigoso: sem nome não há como julgar", () => {
  assert.equal(rotuloPerigoso(""), true);
  assert.equal(rotuloPerigoso(undefined), true);
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: FAIL, `rotuloPerigoso is not a function` (ou erro de import).

- [ ] **Step 3: Implementar**

Acrescentar a `regras.mjs`:

```js
/** A button we cannot name, or that deletes/sends, is never clicked blind. */
export function rotuloPerigoso(rotulo) {
  if (!rotulo || !String(rotulo).trim()) return true;
  return /apagar|excluir|remover|delete|enviar|submit|disparar/i.test(rotulo);
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: `# fail 0`.

- [ ] **Step 5: Implementar `ver.mjs`**

Usar o mecanismo que o veredito do limite 2 indicar. O código abaixo implementa o mouse CDP com guarda `elementFromPoint`; se o veredito foi "só DOM", trocar o bloco `clicar` pelo `.click()` marcado em comentário.

```js
// See and click the running app.
// Usage: node ver.mjs arvore | tela [arquivo.png] | clicar "<rótulo>"   (env CDP_PORT, default 9333)
import fs from "node:fs";
import { alvos, ligar, sleep } from "./cdp.mjs";
import { rotuloPerigoso } from "./regras.mjs";

const PORTA = Number(process.env.CDP_PORT ?? 9333);
const [cmd, arg] = process.argv.slice(2);

const lista = await alvos(PORTA);
const alvo = lista.find((t) => t.type === "page" && /1420/.test(t.url));
if (!alvo) { console.error("sem página do app na porta " + PORTA + ". Rode subir.mjs"); process.exit(1); }

// A frozen/hidden tab swallows input events silently, so activate it first.
await fetch(`http://127.0.0.1:${PORTA}/json/activate/${alvo.id}`).catch(() => {});
const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
await cdp("Runtime.enable"); await cdp("Accessibility.enable"); await cdp("DOM.enable"); await cdp("Page.bringToFront");
const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;
const IGNORADOS = ["none", "generic", "InlineTextBox", "StaticText"];

async function arvore() {
  const nos = (await cdp("Accessibility.getFullAXTree")).result.nodes
    .filter((n) => n.name?.value && !IGNORADOS.includes(n.role?.value));
  return nos.map((n) => `${n.role.value} "${n.name.value}"`);
}

if (cmd === "arvore") {
  const linhas = await arvore();
  console.log(linhas.join("\n"));
  console.error(`# ${linhas.length} nós úteis ≈ ${Math.ceil(linhas.join("\n").length / 3.5)} tokens`);
} else if (cmd === "tela") {
  const dados = (await cdp("Page.captureScreenshot", { format: "png" })).result.data;
  const destino = arg ?? "tela.png";
  fs.writeFileSync(destino, Buffer.from(dados, "base64"));
  console.log(destino);
} else if (cmd === "clicar") {
  if (rotuloPerigoso(arg)) { console.error(`recusado: "${arg}" é destrutivo, de envio ou sem nome`); process.exit(2); }
  const antes = await arvore();
  const pos = JSON.parse(await ev(`(() => {
    const b = [...document.querySelectorAll("button,a,[role=button]")]
      .find(x => (x.getAttribute("aria-label")||x.innerText||"").trim().startsWith(${JSON.stringify(arg)}));
    if (!b) return JSON.stringify({ erro: "rótulo não encontrado" });
    b.scrollIntoView({ block: "center" });
    const q = b.getBoundingClientRect();
    const x = Math.round(q.x + q.width/2), y = Math.round(q.y + q.height/2);
    const sob = document.elementFromPoint(x, y)?.closest("button,a,[role=button]");
    return JSON.stringify({ x, y, confere: sob === b, rotuloSob: (sob?.getAttribute("aria-label")||sob?.innerText||"").trim() });
  })()`));
  if (pos.erro) { console.error(pos.erro); process.exit(1); }
  // Guard: what is physically at that point must be the button we named.
  if (!pos.confere || rotuloPerigoso(pos.rotuloSob)) {
    console.error(`recusado: no ponto (${pos.x},${pos.y}) está "${pos.rotuloSob}", não "${arg}"`); process.exit(2);
  }
  // If limites.md says only DOM click works, replace the two events below with:
  //   await ev(`[...document.querySelectorAll("button,a,[role=button]")].find(x=>(x.getAttribute("aria-label")||x.innerText||"").trim().startsWith(${JSON.stringify(arg)})).click()`);
  for (const type of ["mousePressed", "mouseReleased"]) {
    await cdp("Input.dispatchMouseEvent", { type, x: pos.x, y: pos.y, button: "left", clickCount: 1 });
  }
  await sleep(1500);
  const depois = await arvore();
  console.log(JSON.stringify({ clicou: arg, arvoreMudou: antes.join("|") !== depois.join("|") }));
} else {
  console.error("uso: ver.mjs arvore | tela [arquivo] | clicar <rótulo>"); process.exit(1);
}
fechar();
```

- [ ] **Step 6: Verificar contra o app real**

```bash
export CDP_PORT=9333
node .claude/skills/claudia-teste/scripts/ver.mjs arvore | head -8
node .claude/skills/claudia-teste/scripts/ver.mjs clicar "History"
node .claude/skills/claudia-teste/scripts/ver.mjs clicar "Apagar"
```
Expected: a primeira lista `button "Dashboard"`, `button "Profile"`…; a segunda imprime `{"clicou":"History","arvoreMudou":true}`; a terceira sai com código 2 e `recusado: "Apagar" é destrutivo…`.

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/claudia-teste/scripts/regras.mjs .claude/skills/claudia-teste/scripts/ver.mjs .claude/skills/claudia-teste/scripts/test/regras.test.mjs
git commit -m "feat(skill): add tree, screenshot and guarded click by label

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: `invocar.mjs` e `banco.mjs`

**Files:**
- Modify: `.claude/skills/claudia-teste/scripts/regras.mjs`
- Modify: `.claude/skills/claudia-teste/scripts/test/regras.test.mjs`
- Create: `.claude/skills/claudia-teste/scripts/invocar.mjs`
- Create: `.claude/skills/claudia-teste/scripts/banco.mjs`

**Interfaces:**
- Consumes: `ligar`, `alvos` de `cdp.mjs`.
- Produces: `classificarComando(nome: string) -> "leitura" | "escrita"` e `sqlSomenteLeitura(sql: string) -> { ok: boolean, motivo: string }` em `regras.mjs`; CLIs `invocar.mjs <comando> [json-args] [--escrever]` e `banco.mjs "<SELECT …>"`.

- [ ] **Step 1: Escrever os testes que falham**

Acrescentar a `regras.test.mjs`:

```js
import { classificarComando, sqlSomenteLeitura } from "../regras.mjs";

test("comandos com prefixo de leitura são leitura", () => {
  for (const c of ["obter_modo_autonomo", "obter_config_disparo", "listar_pendencias", "ler_candidato_base", "verificar_setup"]) {
    assert.equal(classificarComando(c), "leitura", c);
  }
});

test("comandos que gravam são escrita", () => {
  for (const c of ["configurar_modo_autonomo", "guardar_pesos_variantes", "apagar_curriculo", "iniciar_pty", "escrever_pty"]) {
    assert.equal(classificarComando(c), "escrita", c);
  }
});

test("prefixo desconhecido é escrita: na dúvida, exige --escrever", () => {
  assert.equal(classificarComando("fazer_algo_novo"), "escrita");
  assert.equal(classificarComando(""), "escrita");
});

test("aceita SELECT simples", () => {
  assert.equal(sqlSomenteLeitura("SELECT COUNT(*) FROM vagas").ok, true);
  assert.equal(sqlSomenteLeitura("  select id from sessoes order by id desc limit 3;").ok, true);
});

test("recusa escrita e DDL", () => {
  for (const s of ["INSERT INTO vagas VALUES (1)", "UPDATE vagas SET status='x'", "DELETE FROM vagas",
                   "DROP TABLE vagas", "ATTACH DATABASE 'x' AS y", "PRAGMA writable_schema=1", "CREATE TABLE t(a)"]) {
    assert.equal(sqlSomenteLeitura(s).ok, false, s);
  }
});

test("recusa múltiplos statements, mesmo que o primeiro seja SELECT", () => {
  assert.equal(sqlSomenteLeitura("SELECT 1; DELETE FROM vagas").ok, false);
});

test("recusa SQL vazio", () => {
  assert.equal(sqlSomenteLeitura("").ok, false);
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: FAIL, `classificarComando is not a function`.

- [ ] **Step 3: Implementar**

Acrescentar a `regras.mjs`:

```js
const PREFIXOS_LEITURA = /^(obter|listar|ler|verificar|consultar|contar|abrir_leitura)_/;

/** Unknown prefixes count as writes: calling a mutating command by accident is the costly mistake. */
export function classificarComando(nome) {
  return PREFIXOS_LEITURA.test(nome || "") ? "leitura" : "escrita";
}

/** The DB is opened read-only as well; this rule just fails early with a clear reason. */
export function sqlSomenteLeitura(sql) {
  const s = String(sql ?? "").trim().replace(/;+\s*$/, "");
  if (!s) return { ok: false, motivo: "SQL vazio" };
  if (s.includes(";")) return { ok: false, motivo: "um único statement por chamada" };
  if (!/^select\b/i.test(s)) return { ok: false, motivo: "só SELECT; escrita passa por invocar.mjs --escrever" };
  if (/\b(insert|update|delete|drop|alter|create|attach|pragma|replace)\b/i.test(s)) {
    return { ok: false, motivo: "palavra de escrita dentro do SELECT" };
  }
  return { ok: true, motivo: "ok" };
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: `# fail 0`.

- [ ] **Step 5: Implementar `invocar.mjs`**

```js
// Call a Tauri command through the running window.
// Usage: node invocar.mjs <comando> ['{"arg":1}'] [--escrever]   (env CDP_PORT, default 9333)
import { alvos, ligar } from "./cdp.mjs";
import { classificarComando } from "./regras.mjs";

const PORTA = Number(process.env.CDP_PORT ?? 9333);
const args = process.argv.slice(2).filter((a) => a !== "--escrever");
const [comando, json] = args;
const podeEscrever = process.argv.includes("--escrever");

if (!comando) { console.error("uso: invocar.mjs <comando> [json-args] [--escrever]"); process.exit(1); }
if (classificarComando(comando) === "escrita" && !podeEscrever) {
  console.error(`recusado: "${comando}" não é de leitura reconhecida. Se grava de propósito, repita com --escrever`);
  process.exit(2);
}

const alvo = (await alvos(PORTA)).find((t) => t.type === "page" && /1420/.test(t.url));
if (!alvo) { console.error("sem página do app. Rode subir.mjs"); process.exit(1); }
const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
await cdp("Runtime.enable");

const r = await cdp("Runtime.evaluate", {
  awaitPromise: true, returnByValue: true,
  expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(comando)}, ${json ?? "{}"})
    .then(v => JSON.stringify({ ok: true, valor: v }))
    .catch(e => JSON.stringify({ ok: false, erro: String(e) }))`,
});
console.log(r.result?.result?.value ?? JSON.stringify({ ok: false, erro: "sem resposta do CDP" }));
fechar();
```

- [ ] **Step 6: Implementar `banco.mjs`**

```js
// Read-only query against the app's SQLite database.
// Usage: node banco.mjs "SELECT ..."
import { DatabaseSync } from "node:sqlite";
import path from "node:path";
import { sqlSomenteLeitura } from "./regras.mjs";

const sql = process.argv[2];
const v = sqlSomenteLeitura(sql);
if (!v.ok) { console.error(`recusado: ${v.motivo}`); process.exit(2); }

const dados = path.join(process.env.APPDATA, "io.github.johngabie.claudia-rh", "claudia_rh.db");
// readOnly at the engine level too: the rule above is the early, readable failure.
const db = new DatabaseSync(dados, { readOnly: true });
console.log(JSON.stringify(db.prepare(sql).all(), null, 1));
db.close();
```

- [ ] **Step 7: Verificar contra o app real**

```bash
export CDP_PORT=9333
node .claude/skills/claudia-teste/scripts/invocar.mjs obter_modo_autonomo
node .claude/skills/claudia-teste/scripts/invocar.mjs configurar_modo_autonomo '{"ativo":true}'
node .claude/skills/claudia-teste/scripts/banco.mjs "SELECT COUNT(*) AS n FROM vagas"
node .claude/skills/claudia-teste/scripts/banco.mjs "DELETE FROM vagas"
```
Expected, em ordem: `{"ok":true,"valor":true}`; recusa com código 2 (`não é de leitura reconhecida`); `[{"n": <número>}]`; recusa com código 2 (`só SELECT`). **Nenhum dado pode ter mudado**: repetir a contagem e comparar.

- [ ] **Step 8: Commit**

```bash
git add .claude/skills/claudia-teste/scripts/regras.mjs .claude/skills/claudia-teste/scripts/invocar.mjs .claude/skills/claudia-teste/scripts/banco.mjs .claude/skills/claudia-teste/scripts/test/regras.test.mjs
git commit -m "feat(skill): add guarded invoke and a read-only database query

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 6: `observar.mjs`

**Files:**
- Create: `.claude/skills/claudia-teste/scripts/observar.mjs`

**Interfaces:**
- Consumes: `cdp.mjs`; `node:sqlite`; o veredito do limite 1 em `references/limites.md`.
- Produces: CLI `observar.mjs` que imprime JSON `{ terminal, log, sessoes }`. `terminal` é `{ texto }` quando legível ou `{ indisponivel: "<motivo>" }` quando o Terminal não pode ser lido. **Nunca finge ler.**

- [ ] **Step 1: Implementar**

```js
// Observe what the agent is doing inside the app: terminal text, the dev log,
// and the sessions table. It only reads; starting a session is done by pressing
// the start button in the app's own UI.
// Usage: node observar.mjs [--linhas 60]   (env CDP_PORT, default 9333)
import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { alvos, ligar, sleep } from "./cdp.mjs";

const PORTA = Number(process.env.CDP_PORT ?? 9333);
const i = process.argv.indexOf("--linhas");
const LINHAS = i > 0 ? Number(process.argv[i + 1]) : 60;

async function lerTerminal() {
  const alvo = (await alvos(PORTA)).find((t) => t.type === "page" && /1420/.test(t.url));
  if (!alvo) return { indisponivel: "sem página do app" };
  const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
  await cdp("Runtime.enable");
  const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;
  await ev(`[...document.querySelectorAll("button")].find(b => /terminal/i.test(b.getAttribute("aria-label")||b.innerText||""))?.click()`);
  await sleep(1500);
  const temDom = await ev(`!!document.querySelector(".xterm-rows")`);
  const texto = temDom ? await ev(`[...document.querySelectorAll(".xterm-rows > div")].map(d => d.innerText).join("\\n")`) : "";
  fechar();
  if (!temDom) return { indisponivel: "renderer sem .xterm-rows (canvas): texto não legível pelo DOM" };
  if (!texto.trim()) return { indisponivel: "terminal vazio: nenhuma sessão rodou. Inicie uma pela UI do app" };
  return { texto: texto.split("\n").slice(-LINHAS).join("\n") };
}

function lerLog() {
  const f = path.join(process.env.TEMP || ".", "tauri-cdp.log");
  if (!fs.existsSync(f)) return { indisponivel: "sem log do subir.mjs" };
  return { texto: fs.readFileSync(f, "utf8").split("\n").slice(-LINHAS).join("\n") };
}

function lerSessoes() {
  const dados = path.join(process.env.APPDATA, "io.github.johngabie.claudia-rh", "claudia_rh.db");
  const db = new DatabaseSync(dados, { readOnly: true });
  const linhas = db.prepare("SELECT * FROM sessoes ORDER BY id DESC LIMIT 5").all();
  db.close();
  return linhas;
}

console.log(JSON.stringify({ terminal: await lerTerminal(), log: lerLog(), sessoes: lerSessoes() }, null, 1));
```

- [ ] **Step 2: Verificar contra o app real**

Run: `CDP_PORT=9333 node .claude/skills/claudia-teste/scripts/observar.mjs --linhas 10`
Expected: JSON com as três chaves. `terminal` é `{texto}` ou `{indisponivel}` **de acordo com o veredito do limite 1**. Se o veredito foi "DOM legível" e veio `indisponivel` por canvas, há contradição: voltar à Tarefa 1 e reverificar.

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/claudia-teste/scripts/observar.mjs
git commit -m "feat(skill): observe terminal, dev log and sessions without faking reads

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 7: `SKILL.md`, referências, comandos gerados e evals

**Files:**
- Create: `.claude/skills/claudia-teste/SKILL.md`
- Create: `.claude/skills/claudia-teste/scripts/gerar-comandos.mjs`
- Create: `.claude/skills/claudia-teste/references/comandos.md` (gerado)
- Create: `.claude/skills/claudia-teste/references/padroes-cdp.md`
- Create: `.claude/skills/claudia-teste/evals/evals.json`

**Interfaces:**
- Consumes: todos os scripts anteriores e `references/limites.md`.
- Produces: a skill carregável pelo harness.

- [ ] **Step 1: Gerador de `comandos.md`**

`scripts/gerar-comandos.mjs`:

```js
// Regenerates references/comandos.md from the Rust sources, so the list never
// goes stale. Run from claudia-rh/.
import fs from "node:fs";
import path from "node:path";
import { classificarComando } from "./regras.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const src = path.join(RAIZ, "src-tauri/src");
const achados = [];

function varrer(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) { varrer(p); continue; }
    if (!e.name.endsWith(".rs")) continue;
    const t = fs.readFileSync(p, "utf8");
    for (const m of t.matchAll(/#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/g)) {
      achados.push({ nome: m[1], arquivo: path.relative(RAIZ, p).replace(/\\/g, "/") });
    }
  }
}
varrer(src);
achados.sort((a, b) => a.nome.localeCompare(b.nome));

const corpo = achados.map((c) => `| \`${c.nome}\` | ${classificarComando(c.nome)} | ${c.arquivo} |`).join("\n");
fs.writeFileSync(
  path.join(import.meta.dirname, "../references/comandos.md"),
  `<!-- GERADO por scripts/gerar-comandos.mjs — não editar à mão -->\n# Comandos Tauri (${achados.length})\n\n` +
  `"escrita" inclui tudo cujo prefixo não é de leitura reconhecida. Esses exigem \`invocar.mjs --escrever\`.\n\n` +
  `| comando | tipo | arquivo |\n|---|---|---|\n${corpo}\n`);
console.log(`${achados.length} comandos`);
```

- [ ] **Step 2: Gerar e conferir contra a contagem medida**

Run: `node .claude/skills/claudia-teste/scripts/gerar-comandos.mjs`
Expected: a contagem impressa bate com `grep -rc '#\[tauri::command\]' src-tauri/src` somado (77 em 2026-10-01). Se divergir, o regex do gerador perde comandos com atributos entre `#[tauri::command]` e `fn`; corrigir antes de seguir.

- [ ] **Step 3: `padroes-cdp.md`**

```markdown
# Padrões e armadilhas do CDP

Todas falharam em silêncio quando descobertas. Não dão erro; devolvem algo que parece resposta.

1. **A porta é de outro processo.** A 9222 era o Chrome pessoal. Confirmar sempre com `validarVersaoApp` (`Edg/` + página `localhost:1420`).
2. **A 1420 presa por um vite antigo** derruba o `tauri dev`. Só liberar se o dono for vite/tauri.
3. **Aba escondida engole eventos de input** sem erro. `/json/activate/<id>` e `Page.bringToFront` antes de clicar.
4. **`getBoxModel` sem `scrollIntoViewIfNeeded`** devolve coordenada fora do ecrã.
5. **Clique por coordenada não tem elemento para recusar.** Ler `elementFromPoint` antes e comparar com o rótulo pretendido.
6. **A árvore de acessibilidade calcula o nome pelo algoritmo completo** (`label`, `aria-labelledby`, texto), não só `aria-label`.

Referência validada: `prototipo-playwright/cdp/` na branch `feat/exec-prototipo-playwright`.
```

- [ ] **Step 4: `evals/evals.json`**

```json
{
  "deve_disparar": [
    "testa o Dashboard rodando",
    "o badge de notificações bate com o banco?",
    "tem um bug no Perfil, reproduz",
    "chama o comando obter_modo_autonomo e vê o retorno",
    "o agente parou onde devia?",
    "compara Claude e Grok na mesma tarefa",
    "não gostei do layout do Dashboard",
    "o bug do Perfil tá acontecendo de novo",
    "olha o que tá errado nessa tela",
    "só mexi no CSS, confere se ficou bom",
    "corrige este bug no Perfil"
  ],
  "nao_deve_disparar": [
    "roda cargo test",
    "candidata nessa vaga do LinkedIn",
    "registra uma issue no Linear",
    "explica o pty_manager.rs",
    "qual a cor do tema?"
  ]
}
```

- [ ] **Step 5: `SKILL.md`**

```markdown
---
name: claudia-teste
description: Vê e controla o Claudia RH (Tauri) rodando, por CDP no WebView2, para testar frontend e backend e observar o que os agentes fazem dentro dele. Use SEMPRE que o usuário pedir para testar o app rodando, ver uma tela, conferir se a UI bate com o banco, chamar um comando Tauri, reproduzir um bug, ou reclamar do layout ("não gostei do X", "o bug tá acontecendo", "tá estranho aqui"). Use também antes de dar por concluída qualquer mudança em UI ou em comandos Tauri, e quando um bug for citado no código — reproduzir antes de propor correção. NÃO use para cargo test puro, automação de sites de vagas, nem registrar no Linear.
---

# claudia-teste

Dá a quem desenvolve o olhar de um dev sobre o app rodando. **Nunca interage com sites de vagas** e nunca clica em algo que submeta candidatura: quem interage com sites são os agentes dentro da Claudia.

## Fluxo

1. **Conectar:** `node scripts/subir.mjs`. Devolve JSON com `acao`: `subiu`, `reutilizar`, `perguntar` ou `abortar`. Se `perguntar`, o app já roda sem CDP: peça ao usuário antes de reiniciar.
2. **Ver:** `node scripts/ver.mjs arvore | tela [arq] | clicar "<rótulo>"`.
3. **Backend:** `node scripts/invocar.mjs <comando> [json]` e `node scripts/banco.mjs "SELECT …"`. Compare o que a UI mostra com o banco.
4. **Observar agentes:** `node scripts/observar.mjs`. Iniciar uma sessão é apertar o botão na UI do app, nunca rodar um agente por fora.

Defina `CDP_PORT` com a porta devolvida por `subir.mjs` (padrão 9333).

## Modo reconhecimento

Quando a queixa é vaga, não suponha. Conecte, abra a tela citada, leia a árvore, tire screenshot e **descreva o que vê**. Só depois pergunte ou proponha. Nesse modo nada é alterado. Se o bug depende de dados ou sequência, pergunte como chegar lá.

## Bug citado

Primeiro reproduza no app rodando. Só feche depois de reproduzido antes e confirmado depois.

## O que nunca fazer

- Escrever direto no banco ou nos YAMLs do perfil. Escrita só por `invocar.mjs --escrever`, avisando antes.
- Editar `prompt_sistema_runtime.md` ou o `identifier` Tauri.
- Reiniciar o app do usuário sem perguntar, ou matar processo que não subiu.
- Clicar em rótulo destrutivo, de envio, ou sem nome (o `ver.mjs` recusa).
- Dar teste por passado porque o `cargo test` ficou verde.

## Saída

Cada teste devolve: o que foi feito, o que foi visto (árvore ou screenshot) e o veredito, com evidência.

## Mais detalhe

- `references/limites.md` — o que foi medido e o que não funciona
- `references/padroes-cdp.md` — armadilhas
- `references/comandos.md` — comandos Tauri (gerado)
```

- [ ] **Step 6: Validar o frontmatter e rodar os evals**

Run, com o `skill-creator`: validar `SKILL.md` e rodar `evals/evals.json` contra a descrição.
Expected: todos os `deve_disparar` disparam e nenhum `nao_deve_disparar` dispara. Se algum errar, ajustar **a descrição**, não o eval, e repetir.

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/claudia-teste/SKILL.md .claude/skills/claudia-teste/scripts/gerar-comandos.mjs .claude/skills/claudia-teste/references .claude/skills/claudia-teste/evals
git commit -m "feat(skill): add SKILL.md, generated command list and trigger evals

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Passo de teste no ciclo (`CLAUDE.md`)

**Files:**
- Modify: `claudia-rh/CLAUDE.md` (seção `## Superpowers`)

**Interfaces:**
- Consumes: a skill pronta.
- Produces: o ciclo oficial com o passo de teste.

- [ ] **Step 1: Editar a seção**

Em `claudia-rh/CLAUDE.md`, substituir a linha:

```
Um ciclo = spec aprovada → plano → branch a partir de `dev` → implementação → review → merge em `dev`.
```

por:

```
Um ciclo = spec aprovada → plano → branch a partir de `dev` → implementação → **teste no app rodando (`claudia-teste`)** → review → merge em `dev`.

O passo de teste é obrigatório:

- Roda o app de verdade (`tauri dev` com CDP) e exercita o que mudou. `cargo test` e `tsc` não bastam.
- O resultado entra no ledger ou na descrição do PR, com evidência: o que foi aberto, o que foi chamado, o que a tela mostrou.
- Mudança só de backend, sem efeito visível: chama o comando por `invoke` e confere o banco.
- Bug corrigido só fecha depois de reproduzido antes e confirmado depois.
```

- [ ] **Step 2: Conferir que só essa seção mudou**

Run: `git diff --stat CLAUDE.md`
Expected: uma única seção alterada; a regra dos passos de instalação e o `identifier` intactos.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: make a running-app test a required step of the dev cycle

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Verificação contra casos reais

**Files:**
- Modify: `.claude/skills/claudia-teste/references/limites.md`

**Interfaces:**
- Consumes: toda a skill.
- Produces: evidência de que a skill fecha o buraco que a motivou.

- [ ] **Step 1: Conectar, ver e navegar**

```bash
node .claude/skills/claudia-teste/scripts/subir.mjs
export CDP_PORT=<porta devolvida>
node .claude/skills/claudia-teste/scripts/ver.mjs arvore | head -12
node .claude/skills/claudia-teste/scripts/ver.mjs clicar "Notifications"
```
Expected: `acao` `subiu` ou `reutilizar`; a árvore lista Dashboard, Profile, History…; o clique devolve `"arvoreMudou":true`.

- [ ] **Step 2: Backend contra arquivo**

```bash
node .claude/skills/claudia-teste/scripts/invocar.mjs obter_modo_autonomo
cat "$APPDATA/io.github.johngabie.claudia-rh/modo_autonomo.json"
```
Expected: o `valor` devolvido bate com `skip_permissions` do arquivo.

- [ ] **Step 3: Reproduzir a CLA-25 pela UI**

Abrir o Dashboard, mover um slider de modo de busca pelo `ver.mjs`, e depois ler o estado persistido:

```bash
node .claude/skills/claudia-teste/scripts/ver.mjs arvore | grep -i -E "slider|search|linkedin"
node .claude/skills/claudia-teste/scripts/invocar.mjs obter_config_disparo
```
Expected: a UI aceita o movimento, **mas nenhum valor de modo de busca aparece persistido** em `obter_config_disparo` nem em arquivo de dados. Isso reproduz a CLA-25 sem depender dos olhos do usuário. Registrar o que foi observado, incluindo se a reprodução **falhou** (se o bug já foi corrigido, dizer isso).

- [ ] **Step 4: Observar**

Run: `node .claude/skills/claudia-teste/scripts/observar.mjs --linhas 20`
Expected: coerente com o veredito do limite 1 (Tarefa 1).

- [ ] **Step 5: Registrar e fechar**

Acrescentar a `references/limites.md` uma seção `## Verificação` com, para cada um dos Steps 1–4: o comando, o que voltou e o veredito, sem `<…>` restantes.

- [ ] **Step 6: Rodar tudo**

Run: `node --test .claude/skills/claudia-teste/scripts/test/`
Expected: `# fail 0`.

- [ ] **Step 7: Commit**

```bash
git add .claude/skills/claudia-teste/references/limites.md
git commit -m "docs(skill): record verification against the running app, including CLA-25

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

## Notas de execução

- **Tarefas 1 e 2 vêm primeiro de propósito.** Se o Terminal for canvas ou o clique não pegar, `observar.mjs` e `ver.mjs` mudam; descobrir isso na Tarefa 6 custaria retrabalho.
- **O app precisa estar rodando com CDP** nas Tarefas 1, 2, 4, 5, 6 e 9. A Tarefa 3 constrói o launcher que automatiza isso; até lá, subir à mão como no Step 2 da Tarefa 1.
- **`cdp.mjs` é cópia** do protótipo. Quando `feat/exec-prototipo-playwright` for integrada e o protótipo apagado, esta cópia passa a ser a única.
