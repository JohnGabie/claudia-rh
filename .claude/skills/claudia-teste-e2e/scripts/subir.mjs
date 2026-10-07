// Starts `tauri dev` with a CDP port, or reuses one already running.
// Usage: node subir.mjs
import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { validarVersaoApp, decidirSubida, hostRemoto, decidirComAppInstalado, ambienteSubida } from "./regras.mjs";
import { HOST } from "./cdp.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const CANDIDATAS = [9333, 9334, 9335, 9336];

async function ler(url) {
  try { return await (await fetch(url, { signal: AbortSignal.timeout(2000) })).json(); }
  catch { return null; }
}

if (hostRemoto(HOST)) {
  const porta = Number(process.env.CDP_PORT ?? 9333);
  const v = await ler(`http://${HOST}:${porta}/json/version`);
  const ok = v && validarVersaoApp(v, await ler(`http://${HOST}:${porta}/json/list`)).ok;
  const decisao = decidirSubida({ appRodando: ok, appTemCdp: ok, remoto: true });
  if (decisao === "reutilizar") { console.log(JSON.stringify({ acao: "reutilizar", host: HOST, porta })); process.exit(0); }
  console.log(JSON.stringify({ acao: "abortar", motivo: `sem CDP do app em ${HOST}:${porta}; no Sandbox, inicie o app com node .claude/skills/claudia-teste-sandbox/scripts/cdp-guest.mjs --iniciar` }));
  process.exit(3);
}

/** A port is free only if nothing answers at all — an answer, even a 404, means it is taken. */
async function portaLivre(porta) {
  try { await fetch(`http://${HOST}:${porta}/json/version`, { signal: AbortSignal.timeout(1500) }); return false; }
  catch { return true; }
}

async function acharAppComCdp() {
  for (const p of CANDIDATAS) {
    const v = await ler(`http://${HOST}:${p}/json/version`);
    if (!v) continue;
    const r = validarVersaoApp(v, await ler(`http://${HOST}:${p}/json/list`));
    if (r.ok) return p;
  }
  return null;
}

const portaApp = await acharAppComCdp();
// :1420 answering means a vite (and probably the app) is up even without CDP.
let vite1420 = false;
try { await fetch("http://127.0.0.1:1420/", { signal: AbortSignal.timeout(1500) }); vite1420 = true; } catch { /* nothing there */ }
const appRodando = portaApp !== null || vite1420;

// R12: a claudia-rh.exe running without our dev CDP is probably the user's installed app.
let processos = [];
try {
  const saida = execFileSync("tasklist", ["/FI", "IMAGENAME eq claudia-rh.exe", "/FO", "CSV", "/NH"], { encoding: "utf8", timeout: 5000 });
  processos = saida.split(/\r?\n/).map((l) => l.split('","')[0].replace(/^"/, "")).filter(Boolean);
} catch { /* tasklist unavailable: fall back to the port checks below */ }
const instalado = decidirComAppInstalado({ processos, cdpResponde: portaApp !== null });
if (instalado.acao === "perguntar") {
  console.log(JSON.stringify({ acao: "perguntar", motivo: instalado.motivo }));
  process.exit(2);
}

const decisao = decidirSubida({ appRodando, appTemCdp: portaApp !== null });

if (decisao === "reutilizar") { console.log(JSON.stringify({ acao: "reutilizar", porta: portaApp })); process.exit(0); }
if (decisao === "perguntar") {
  console.log(JSON.stringify({ acao: "perguntar", motivo: "o app já roda sem CDP; reiniciar fecha a sessão do usuário" }));
  process.exit(2);
}

let porta = null;
for (const p of CANDIDATAS) { if (await portaLivre(p)) { porta = p; break; } }
if (!porta) { console.log(JSON.stringify({ acao: "abortar", motivo: "nenhuma porta livre entre 9333 e 9336" })); process.exit(3); }

const log = path.join(process.env.TEMP || ".", "tauri-cdp.log");
const fd = fs.openSync(log, "w");
const filho = spawn("npm", ["run", "tauri", "dev"], {
  cwd: RAIZ, shell: true, detached: true, stdio: ["ignore", fd, fd],
  // B4: CLAUDIA_SEM_AUTOMACAO=1 turns idle dispatch and autonomous mode off for this run only (dev build); user files stay untouched.
  env: ambienteSubida(process.env, porta),
});
filho.unref();

for (let i = 0; i < 60; i++) {
  await new Promise((r) => setTimeout(r, 3000));
  const v = await ler(`http://${HOST}:${porta}/json/version`);
  if (v && validarVersaoApp(v, await ler(`http://${HOST}:${porta}/json/list`)).ok) {
    console.log(JSON.stringify({ acao: "subiu", porta, pid: filho.pid, log, automacao: "desligada nesta execução" }));
    process.exit(0);
  }
}
console.log(JSON.stringify({ acao: "falhou", motivo: "CDP não respondeu em 180 s", log }));
process.exit(4);
