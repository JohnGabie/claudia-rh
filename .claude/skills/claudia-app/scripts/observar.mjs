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
  const alvo = (await alvos(PORTA)).find((t) => t.type === "page" && /1420|tauri\.localhost/.test(t.url));
  if (!alvo) return { indisponivel: "sem página do app" };
  const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
  await cdp("Runtime.enable");
  const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;
  await ev(`[...document.querySelectorAll("button")].find(b => /terminal/i.test(b.getAttribute("aria-label")||b.innerText||""))?.click()`);
  await sleep(1500);
  const temDom = await ev(`!!document.querySelector(".xterm-rows")`);
  const texto = temDom ? ((await ev(`JSON.stringify([...document.querySelectorAll(".xterm-rows > div")].map(d => d.innerText))`)) ?? "[]") : "[]";
  const linhas = JSON.parse(texto);
  fechar();
  if (!temDom) return { indisponivel: "renderer sem .xterm-rows (canvas): texto não legível pelo DOM" };
  if (!linhas.join("").trim()) return { indisponivel: "terminal vazio: nenhuma sessão rodou. Inicie uma pela UI do app" };
  return { texto: linhas.slice(-LINHAS).join("\n") };
}

function lerLog() {
  const f = path.join(process.env.TEMP || ".", "tauri-cdp.log");
  if (!fs.existsSync(f)) return { indisponivel: "sem log do subir.mjs" };
  return { texto: fs.readFileSync(f, "utf8").split("\n").slice(-LINHAS).join("\n") };
}

function lerSessoes() {
  const dados = process.env.CLAUDIA_DB || path.join(process.env.APPDATA, "io.github.johngabie.claudia-rh", "claudia_rh.db");
  const db = new DatabaseSync(dados, { readOnly: true });
  const linhas = db.prepare("SELECT * FROM sessoes ORDER BY id DESC LIMIT 5").all();
  db.close();
  return linhas;
}

const saida = { terminal: await lerTerminal(), log: lerLog(), sessoes: lerSessoes() };
console.log(JSON.stringify(saida, null, 1));
process.exit(0);
