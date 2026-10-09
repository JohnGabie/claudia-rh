// See and click the running app.
// Usage: node ver.mjs arvore | tela [arquivo.png] | clicar "<rótulo>"   (env CDP_PORT, default 9333)
import fs from "node:fs";
import path from "node:path";
import { HOST, alvos, ligar, sleep } from "./cdp.mjs";
import { rotuloPerigoso, destinoPrintValido, pastaPrintsSegura } from "./regras.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");

const PORTA = Number(process.env.CDP_PORT ?? 9333);
const [cmd, arg] = process.argv.slice(2);

const lista = await alvos(PORTA);
const alvo = lista.find((t) => t.type === "page" && /1420|tauri\.localhost/.test(t.url));
if (!alvo) { console.error("sem página do app na porta " + PORTA + ". Rode subir.mjs"); process.exit(1); }

// A frozen/hidden tab swallows input events silently, so activate it first.
await fetch(`http://${HOST}:${PORTA}/json/activate/${alvo.id}`).catch(() => {});
const { cdp, fechar } = await ligar(alvo.webSocketDebuggerUrl);
await cdp("Runtime.enable"); await cdp("Accessibility.enable"); await cdp("DOM.enable"); await cdp("Page.bringToFront");
const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;
// Closing the socket before exiting avoids a libuv assertion on Windows.
const sair = (codigo) => { fechar(); setTimeout(() => process.exit(codigo), 50); };
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
  // R9: prints only go to debug-logs/prints/ and never overwrite.
  const alvoPrint = arg ?? `debug-logs/prints/tela-${Date.now()}.png`;
  const v = destinoPrintValido(alvoPrint, RAIZ, (d) => fs.existsSync(d));
  if (!v.ok) { console.error(`recusado: ${v.motivo}`); sair(2); await new Promise(() => {}); }
  // M4: debug-logs and debug-logs/prints must be real directories (no symlink/junction) inside the real repo root.
  const dLogs = path.join(RAIZ, "debug-logs");
  const ehLink = (p) => { try { return fs.lstatSync(p).isSymbolicLink(); } catch { return false; } };
  if (ehLink(dLogs)) { console.error("recusado: debug-logs é link/junction"); sair(2); await new Promise(() => {}); }
  const pastaPrints = path.dirname(v.destino);
  fs.mkdirSync(pastaPrints, { recursive: true });
  const seg = pastaPrintsSegura({
    raizReal: fs.realpathSync(RAIZ), pastaReal: fs.realpathSync(pastaPrints), ehLink: ehLink(pastaPrints) || ehLink(dLogs),
  });
  if (!seg.ok) { console.error(`recusado: ${seg.motivo}`); sair(2); await new Promise(() => {}); }
  const dados = (await cdp("Page.captureScreenshot", { format: "png" })).result.data;
  fs.writeFileSync(v.destino, Buffer.from(dados, "base64"), { flag: "wx" });
  console.log(path.relative(RAIZ, v.destino));
} else if (cmd === "clicar") {
  if (rotuloPerigoso(arg)) { console.error(`recusado: "${arg}" é destrutivo, de envio ou sem nome`); sair(2); await new Promise(() => {}); }
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
  if (pos.erro) { console.error(pos.erro); sair(1); await new Promise(() => {}); }
  // Guard: what is physically at that point must be the button we named.
  if (!pos.confere || rotuloPerigoso(pos.rotuloSob)) {
    console.error(`recusado: no ponto (${pos.x},${pos.y}) está "${pos.rotuloSob}", não "${arg}"`); sair(2); await new Promise(() => {});
  }
  // Real mouse events: limites.md records that both mechanisms work, and this is closest to a user.
  for (const type of ["mousePressed", "mouseReleased"]) {
    await cdp("Input.dispatchMouseEvent", { type, x: pos.x, y: pos.y, button: "left", clickCount: 1 });
  }
  await sleep(1500);
  const depois = await arvore();
  console.log(JSON.stringify({ clicou: arg, arvoreMudou: antes.join("|") !== depois.join("|") }));
} else {
  console.error("uso: ver.mjs arvore | tela [arquivo] | clicar <rótulo>"); sair(1); await new Promise(() => {});
}
fechar();
