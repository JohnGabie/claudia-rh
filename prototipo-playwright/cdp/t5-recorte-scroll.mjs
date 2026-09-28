import { alvos, ligar, sleep } from "./cdp.mjs";
import fs from "node:fs";
const PORT = 9889;
const BASE = "C:/Users/joaog/AppData/Local/Temp/claude/C--Users-joaog-PycharmProjects-curriculum-apply/5ac6932b-7941-4e8f-9186-2bb5314ffd89/scratchpad/";
const lista = await alvos(PORT);
const p = lista.find((t) => t.type === "page" && /linkedin\.com/.test(t.url));
const { cdp, fechar } = await ligar(p.webSocketDebuggerUrl);
await cdp("Page.enable"); await cdp("DOM.enable"); await cdp("Runtime.enable"); await cdp("Accessibility.enable");
await cdp("Page.bringToFront");
const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
const lerAX = async () => (await cdp("Accessibility.getFullAXTree")).result?.nodes ?? [];

await ev(`(() => { const b=[...document.querySelectorAll('button.jobs-apply-button, button')].find(x=>{const t=((x.getAttribute('aria-label')||'')+' '+(x.innerText||'')).trim(); return /candidatura simplificada|easy apply/i.test(t)&&!/filtro|filter/i.test(t);}); if(b){b.scrollIntoView({block:'center'});b.click();} return !!b; })()`);
for (let i=0;i<8;i++){ await sleep(1800); if (await ev("document.querySelectorAll('[role=dialog]').length")>0) break; }
console.log("dialogs:", await ev("document.querySelectorAll('[role=dialog]').length"));

// ---- Page.captureScreenshot com clip, sobre o modal real ----
const cx = await ev(`(() => { const d=document.querySelector('[role=dialog]'); if(!d) return null; const r=d.getBoundingClientRect(); return JSON.stringify([Math.round(r.x),Math.round(r.y),Math.round(r.width),Math.round(r.height)]); })()`);
const [dx,dy,dw,dh] = JSON.parse(cx);
console.log(`dialog rect = ${dx},${dy} ${dw}x${dh}`);
const clip = { x: dx, y: dy, width: Math.min(dw, 700), height: Math.min(dh, 420), scale: 1 };
const s = await cdp("Page.captureScreenshot", { format: "png", clip });
const buf = Buffer.from(s.result.data, "base64");
fs.writeFileSync(BASE + "recorte-linkedin.png", buf);
console.log(`clip pedido ${clip.width}x${clip.height} -> PNG ${buf.readUInt32BE(16)}x${buf.readUInt32BE(20)} (${buf.length} bytes)`);

// ---- elemento fora do viewport: getBoxModel vs DOM.scrollIntoViewIfNeeded ----
console.log("\n=== fora do viewport ===");
const vh = await ev("window.innerHeight");
const btn = (await lerAX()).find((n) => n.role?.value === "button" && /enviar candidatura/i.test(String(n.name?.value ?? "")));
if (btn) {
  const b1 = await cdp("DOM.getBoxModel", { backendNodeId: btn.backendDOMNodeId });
  const c1 = b1.result?.model?.content;
  const y1 = c1 ? Math.round((c1[1]+c1[5])/2) : null;
  console.log(`  innerHeight=${vh}  centro y ANTES do scroll = ${y1}  -> ${y1 > vh ? "FORA do viewport (coordenada invalida)" : "dentro"}`);
  const pt1 = await ev(`(() => { const e=document.elementFromPoint(${Math.round((c1[0]+c1[4])/2)}, ${y1}); return e? e.tagName : "NADA"; })()`);
  console.log(`  elementFromPoint antes = ${pt1}`);
  await cdp("DOM.scrollIntoViewIfNeeded", { backendNodeId: btn.backendDOMNodeId });
  await sleep(900);
  const b2 = await cdp("DOM.getBoxModel", { backendNodeId: btn.backendDOMNodeId });
  const c2 = b2.result.model.content;
  const x2 = Math.round((c2[0]+c2[4])/2), y2 = Math.round((c2[1]+c2[5])/2);
  const pt2 = await ev(`(() => { const e=document.elementFromPoint(${x2}, ${y2}); if(!e) return "NADA"; const c=e.closest('button,a,input,label')||e; return c.tagName+"|"+((c.getAttribute('aria-label')||c.innerText||'').trim().slice(0,50)); })()`);
  console.log(`  centro y DEPOIS do scroll = ${y2}`);
  console.log(`  elementFromPoint depois = ${pt2}`);
  console.log(`  >>> guarda AGORA recusa corretamente pelo rotulo: ${/enviar|submit/i.test(pt2) ? "SIM (nao clicado)" : "NAO — PERIGO"}`);
}

// fechar
for (const rot of ["fechar|close|dismiss","descartar|discard"]) {
  await ev(`(() => { const b=[...document.querySelectorAll('button')].find(x=>new RegExp("${rot}","i").test((x.getAttribute('aria-label')||x.innerText||'').trim())); if(b)b.click(); return !!b; })()`);
  await sleep(2200);
}
const fim = await lerAX();
console.log(`\ndialogs finais: ${fim.filter(n=>n.role?.value==='dialog').length}  |  nada submetido: ${fim.some(n=>/candidatura simplificada/i.test(String(n.name?.value??"")))?"SIM":"VERIFICAR"}`);
fechar();
