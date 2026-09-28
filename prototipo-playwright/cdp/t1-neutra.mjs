import { alvos, ligar, sleep } from "./cdp.mjs";
import fs from "node:fs";

const PORT = 9888;
const lista = await alvos(PORT);
const p = lista.find((t) => t.type === "page" && /neutra\.html/.test(t.url));
if (!p) { console.log("pagina neutra nao encontrada"); process.exit(1); }
const { cdp, fechar } = await ligar(p.webSocketDebuggerUrl);

await cdp("Page.enable");
await cdp("DOM.enable");
await cdp("Runtime.enable");
await cdp("Accessibility.enable");
await cdp("Page.bringToFront");
await sleep(500);

const ev = async (expr) =>
  (await cdp("Runtime.evaluate", { expression: expr, returnByValue: true })).result?.result?.value;

const doc = await cdp("DOM.getDocument", { depth: -1 });
const raiz = doc.result.root.nodeId;
const achar = async (sel) =>
  (await cdp("DOM.querySelector", { nodeId: raiz, selector: sel })).result?.nodeId;

const centro = async (nodeId) => {
  const bm = await cdp("DOM.getBoxModel", { nodeId });
  const c = bm.result?.model?.content;
  if (!c) return null;
  return { x: Math.round((c[0] + c[4]) / 2), y: Math.round((c[1] + c[5]) / 2), bm: bm.result.model };
};

const R = [];
const reg = (nome, passou, nota) => { R.push({ nome, passou, nota }); console.log(`${passou ? "OK    " : "FALHOU"}  ${nome}  ${nota ?? ""}`); };

// ---------- 1. Input.dispatchMouseEvent ----------
console.log("\n=== 1. Input.dispatchMouseEvent ===");
const nAlvo = await achar("#alvo");
const cAlvo = await centro(nAlvo);
console.log(`  #alvo centro = (${cAlvo.x}, ${cAlvo.y})`);
await ev("window.__cliques = 0; window.__eventos = []; document.getElementById('alvo').className=''; document.getElementById('alvo').textContent='NAO CLICADO';");

await cdp("Input.dispatchMouseEvent", { type: "mouseMoved", x: cAlvo.x, y: cAlvo.y, button: "none", buttons: 0 });
await cdp("Input.dispatchMouseEvent", { type: "mousePressed", x: cAlvo.x, y: cAlvo.y, button: "left", buttons: 1, clickCount: 1 });
await cdp("Input.dispatchMouseEvent", { type: "mouseReleased", x: cAlvo.x, y: cAlvo.y, button: "left", buttons: 0, clickCount: 1 });
await sleep(400);
const cliques = await ev("window.__cliques");
const eventos = await ev("JSON.stringify(window.__eventos)");
reg("Input.dispatchMouseEvent (mousePressed/Released)", cliques >= 1, `cliques=${cliques} eventos=${eventos}`);

// ---------- 2. Input.dispatchTouchEvent ----------
console.log("\n=== 2. Input.dispatchTouchEvent ===");
await ev("window.__cliques = 0; window.__eventos = [];");
const t1 = await cdp("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: cAlvo.x, y: cAlvo.y }] });
const t2 = await cdp("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
await sleep(400);
const cliquesT = await ev("window.__cliques");
const eventosT = await ev("JSON.stringify(window.__eventos)");
reg("Input.dispatchTouchEvent", cliquesT >= 1, `cliques=${cliquesT} eventos=${eventosT} err=${JSON.stringify(t1.error ?? t2.error ?? null)}`);

// ---------- 3. Input.insertText ----------
console.log("\n=== 3. Input.insertText ===");
const nTexto = await achar("#texto");
await cdp("DOM.focus", { nodeId: nTexto });
const ins = await cdp("Input.insertText", { text: "Joao Gabriel 12345" });
await sleep(300);
const vTexto = await ev("document.getElementById('texto').value");
reg("Input.insertText", vTexto === "Joao Gabriel 12345", `valor="${vTexto}" err=${JSON.stringify(ins.error ?? null)}`);

// ---------- 4. Input.dispatchKeyEvent ----------
console.log("\n=== 4. Input.dispatchKeyEvent ===");
const nTexto2 = await achar("#texto2");
await cdp("DOM.focus", { nodeId: nTexto2 });
for (const ch of "ABC-789") {
  const code = ch === "-" ? "Minus" : /[0-9]/.test(ch) ? `Digit${ch}` : `Key${ch}`;
  await cdp("Input.dispatchKeyEvent", { type: "keyDown", text: ch, unmodifiedText: ch, key: ch, code, windowsVirtualKeyCode: ch.charCodeAt(0) });
  await cdp("Input.dispatchKeyEvent", { type: "keyUp", key: ch, code, windowsVirtualKeyCode: ch.charCodeAt(0) });
}
await sleep(300);
const vTexto2 = await ev("document.getElementById('texto2').value");
reg("Input.dispatchKeyEvent (texto)", vTexto2 === "ABC-789", `valor="${vTexto2}"`);

// Backspace, para provar teclas nao-imprimiveis
await cdp("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8, nativeVirtualKeyCode: 8 });
await cdp("Input.dispatchKeyEvent", { type: "keyUp", key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8, nativeVirtualKeyCode: 8 });
await sleep(250);
const vBs = await ev("document.getElementById('texto2').value");
reg("Input.dispatchKeyEvent (Backspace)", vBs === "ABC-78", `valor="${vBs}"`);

// ---------- 5. Cadeia AX -> backendDOMNodeId -> getBoxModel -> clique -> reler AX ----------
console.log("\n=== 5. Cadeia AX -> DOM.getBoxModel -> dispatchMouseEvent -> reler AX ===");
const lerAX = async () => {
  const ax = await cdp("Accessibility.getFullAXTree");
  return ax.result?.nodes ?? [];
};
const nosA = await lerAX();
const chkA = nosA.find((n) => n.role?.value === "checkbox");
const estado = (n) => n?.properties?.find((p) => p.name === "checked")?.value?.value;
console.log(`  checkbox AX: nome="${chkA?.name?.value}" checked=${estado(chkA)} backendDOMNodeId=${chkA?.backendDOMNodeId}`);

const desc = await cdp("DOM.describeNode", { backendNodeId: chkA.backendDOMNodeId });
const nodeIdChk = (await cdp("DOM.requestNode", { objectId: (await cdp("DOM.resolveNode", { backendNodeId: chkA.backendDOMNodeId })).result.object.objectId })).result?.nodeId;
const bmChk = await cdp("DOM.getBoxModel", { backendNodeId: chkA.backendDOMNodeId });
const cc = bmChk.result?.model?.content;
const cx = Math.round((cc[0] + cc[4]) / 2), cy = Math.round((cc[1] + cc[5]) / 2);
console.log(`  DOM.getBoxModel(backendNodeId) -> centro (${cx}, ${cy})  [${desc.result?.node?.nodeName}]`);

// guarda: le o que esta no ponto antes de clicar
const noPonto = await ev(`(() => { const e = document.elementFromPoint(${cx}, ${cy}); return e ? e.tagName + "|" + (e.getAttribute("aria-label")||e.innerText||"").slice(0,60) : "nada"; })()`);
console.log(`  elementFromPoint(${cx},${cy}) = ${noPonto}`);

await cdp("Input.dispatchMouseEvent", { type: "mouseMoved", x: cx, y: cy, button: "none", buttons: 0 });
await cdp("Input.dispatchMouseEvent", { type: "mousePressed", x: cx, y: cy, button: "left", buttons: 1, clickCount: 1 });
await cdp("Input.dispatchMouseEvent", { type: "mouseReleased", x: cx, y: cy, button: "left", buttons: 0, clickCount: 1 });
await sleep(500);
const nosB = await lerAX();
const chkB = nosB.find((n) => n.role?.value === "checkbox");
console.log(`  depois: checked=${estado(chkB)}`);
reg("Cadeia AX -> getBoxModel -> clique -> AX mudou", estado(chkA) !== estado(chkB), `${estado(chkA)} -> ${estado(chkB)}`);

// ---------- 6. Page.captureScreenshot com clip ----------
console.log("\n=== 6. Page.captureScreenshot com clip ===");
const clip = { x: Math.max(0, cAlvo.x - 160), y: Math.max(0, cAlvo.y - 60), width: 320, height: 120, scale: 1 };
const shot = await cdp("Page.captureScreenshot", { format: "png", clip, captureBeyondViewport: false });
const b64 = shot.result?.data;
const caminho = "C:/Users/joaog/AppData/Local/Temp/claude/C--Users-joaog-PycharmProjects-curriculum-apply/5ac6932b-7941-4e8f-9186-2bb5314ffd89/scratchpad/recorte.png";
let dims = "sem imagem";
if (b64) {
  const buf = Buffer.from(b64, "base64");
  fs.writeFileSync(caminho, buf);
  // le largura/altura do cabecalho IHDR do PNG
  dims = `${buf.readUInt32BE(16)}x${buf.readUInt32BE(20)}  (${buf.length} bytes)`;
}
reg("Page.captureScreenshot com clip", b64 && dims.startsWith("320x120"), `pedido 320x120 -> saiu ${dims}`);

// tambem o ecra inteiro, para referencia
const full = await cdp("Page.captureScreenshot", { format: "png" });
if (full.result?.data) fs.writeFileSync(caminho.replace("recorte", "inteiro"), Buffer.from(full.result.data, "base64"));

console.log("\n=== RESUMO ===");
R.forEach((r) => console.log(`${r.passou ? "OK    " : "FALHOU"}  ${r.nome}`));
fechar();
