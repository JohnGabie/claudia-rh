// Call a Tauri command through the running window.
// Usage: node invocar.mjs <comando> ['{"arg":1}'] [--escrever]   (env CDP_PORT, default 9333)
import { alvos, ligar } from "./cdp.mjs";
import { classificarInvocacao } from "./regras.mjs";

const PORTA = Number(process.env.CDP_PORT ?? 9333);
const brutos = process.argv.slice(2);
const args = brutos.filter((a) => a !== "--escrever" && !a.startsWith("--confirmo="));
const [comando, json] = args;
const podeEscrever = brutos.includes("--escrever");
const confirmo = brutos.find((a) => a.startsWith("--confirmo="))?.slice("--confirmo=".length);

if (!comando) { console.error("uso: invocar.mjs <comando> [json-args] [--escrever] [--confirmo=<comando>]"); process.exit(1); }
const veredito = classificarInvocacao(comando, { escrever: podeEscrever, confirmo });
if (!veredito.ok) { console.error(`recusado: ${veredito.motivo}`); process.exit(2); }

// B5: the argument must be valid JSON and enters the expression only through JSON.stringify.
let argumentos = {};
if (json !== undefined) {
  try { argumentos = JSON.parse(json); }
  catch (e) { console.error(`recusado: argumento não é JSON válido (${e.message})`); process.exit(2); }
}

const alvo = (await alvos(PORTA)).find((t) => t.type === "page" && /1420|tauri\.localhost/.test(t.url));
if (!alvo) { console.error("sem página do app. Rode subir.mjs"); process.exit(1); }
const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
await cdp("Runtime.enable");

const r = await cdp("Runtime.evaluate", {
  awaitPromise: true, returnByValue: true,
  expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(comando)}, ${JSON.stringify(argumentos)})
    .then(v => JSON.stringify({ ok: true, valor: v }))
    .catch(e => JSON.stringify({ ok: false, erro: String(e) }))`,
});
console.log(r.result?.result?.value ?? JSON.stringify({ ok: false, erro: "sem resposta do CDP" }));
fechar();
