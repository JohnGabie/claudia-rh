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

const alvo = (await alvos(PORTA)).find((t) => t.type === "page" && /1420|tauri\.localhost/.test(t.url));
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
