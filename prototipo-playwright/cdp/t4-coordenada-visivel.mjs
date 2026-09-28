// A coordenada do input[type=radio] cru cai por baixo do botao de download.
// Teste: resolver o elemento VISIVEL (label/wrapper) e clicar nessa coordenada.
import { alvos, ligar, sleep } from "./cdp.mjs";

const PORT = 9889;
const PROIBIDO = /enviar|submit|revisar|pr[oó]ximo|continuar|avan[çc]ar/i;
const lista = await alvos(PORT);
const p = lista.find((t) => t.type === "page" && /linkedin\.com/.test(t.url));
const { cdp, fechar } = await ligar(p.webSocketDebuggerUrl);
await cdp("Page.enable"); await cdp("DOM.enable"); await cdp("Runtime.enable"); await cdp("Accessibility.enable");
await cdp("Page.bringToFront");

const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
const R = [];
const reg = (n, ok, nota) => { R.push({ n, ok }); console.log(`${ok ? "OK    " : "FALHOU"}  ${n}  ${nota ?? ""}`); };

const cliqueGuardado = async (x, y) => {
  const alvo = await ev(`(() => {
    const e = document.elementFromPoint(${x}, ${y});
    if (!e) return "NADA";
    const c = e.closest('button,a,input,label,[role=button],[role=radio],[role=checkbox]') || e;
    return c.tagName + "|" + (c.getAttribute('aria-label') || c.innerText || c.value || '').trim().slice(0,80);
  })()`);
  console.log(`  ponto (${x},${y}) -> ${alvo}`);
  if (alvo === "NADA" || PROIBIDO.test(alvo)) { console.log("  RECUSADO pela guarda"); return { ok: false, alvo }; }
  await cdp("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none", buttons: 0 });
  await cdp("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1 });
  await cdp("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1 });
  return { ok: true, alvo };
};

const lerAX = async () => (await cdp("Accessibility.getFullAXTree")).result?.nodes ?? [];
const estado = (n) => n?.properties?.find((q) => q.name === "checked")?.value?.value;

// reabrir o modal
console.log("=== reabrir modal ===");
console.log(await ev(`(() => {
  const b = [...document.querySelectorAll('button.jobs-apply-button, button')].find(x => {
    const t = ((x.getAttribute('aria-label')||'') + ' ' + (x.innerText||'')).trim();
    return /candidatura simplificada|easy apply/i.test(t) && !/filtro|filter/i.test(t);
  });
  if (!b) return 'NAO ENCONTRADO'; b.scrollIntoView({block:'center'}); b.click(); return 'clicou';
})()`));
for (let i = 0; i < 8; i++) { await sleep(1800); if (await ev("document.querySelectorAll('[role=dialog]').length") > 0) break; }
console.log(`dialogs: ${await ev("document.querySelectorAll('[role=dialog]').length")}`);

// ---------- D2a: checkbox "Siga a empresa" por coordenada ----------
console.log("\n=== D2a. checkbox visivel por coordenada (dispatchMouseEvent) ===");
let nos = await lerAX();
const chkA = nos.find((n) => n.role?.value === "checkbox" && /siga a empresa|follow/i.test(String(n.name?.value ?? "")));
if (!chkA) console.log("  checkbox nao encontrado");
else {
  console.log(`  "${String(chkA.name.value).slice(0,60)}" checked=${estado(chkA)} bdn=${chkA.backendDOMNodeId}`);
  const bm = await cdp("DOM.getBoxModel", { backendNodeId: chkA.backendDOMNodeId });
  const cc = bm.result.model.content;
  const x = Math.round((cc[0]+cc[4])/2), y = Math.round((cc[1]+cc[5])/2);
  const r = await cliqueGuardado(x, y);
  await sleep(1800);
  const chkB = (await lerAX()).find((n) => n.backendDOMNodeId === chkA.backendDOMNodeId);
  reg("dispatchMouseEvent muda checkbox no modal do LinkedIn", r.ok && estado(chkA) !== estado(chkB), `${estado(chkA)} -> ${estado(chkB)}`);
  // reverter
  if (estado(chkA) !== estado(chkB)) { await cliqueGuardado(x, y); await sleep(1500); console.log(`  revertido -> ${estado((await lerAX()).find(n=>n.backendDOMNodeId===chkA.backendDOMNodeId))}`); }
}

// ---------- D2b: radio via elemento VISIVEL ----------
console.log("\n=== D2b. radio: coordenada do elemento VISIVEL, nao do input cru ===");
const lerRadios = async () => (await lerAX()).filter((n) => n.role?.value === "radio" && n.name?.value)
  .map((n) => ({ bdn: n.backendDOMNodeId, nome: String(n.name.value), sel: /^Desmarcar/i.test(String(n.name.value)),
                 arq: String(n.name.value).replace(/^(Desmarcar seleção de|Selecionar)\s*/i, "") }));
let rs = await lerRadios();
rs.forEach((r,i)=>console.log(`  [${i}] ${r.sel?"●":"○"} ${r.arq.slice(0,44)} bdn=${r.bdn}`));
const antes = rs.find((r) => r.sel), alvo = rs.find((r) => !r.sel);

if (!alvo || !antes) console.log("  precisa de 2 curriculos — a saltar");
else {
  // resolve o input -> sobe ate ao elemento com caixa visivel (label ou container clicavel)
  const oid = (await cdp("DOM.resolveNode", { backendNodeId: alvo.bdn })).result?.object?.objectId;
  const info = await cdp("Runtime.callFunctionOn", {
    objectId: oid, returnByValue: true,
    functionDeclaration: `function() {
      const cru = this.getBoundingClientRect();
      let el = this;
      // sobe ate encontrar um ancestral com area razoavel
      let visivel = null;
      for (let n = this; n && n !== document.body; n = n.parentElement) {
        const r = n.getBoundingClientRect();
        if (r.width > 40 && r.height > 20) { visivel = n; break; }
      }
      const lbl = this.id ? document.querySelector('label[for="' + CSS.escape(this.id) + '"]') : null;
      const escolha = lbl && lbl.getBoundingClientRect().width > 10 ? lbl : visivel;
      const r = escolha.getBoundingClientRect();
      return JSON.stringify({
        cruRect: [Math.round(cru.x), Math.round(cru.y), Math.round(cru.width), Math.round(cru.height)],
        cruOpacity: getComputedStyle(this).opacity,
        escolhaTag: escolha.tagName + (escolha.className ? '.' + String(escolha.className).split(' ')[0] : ''),
        rect: [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]
      });
    }`
  });
  const d = JSON.parse(info.result?.result?.value ?? "{}");
  console.log(`  input cru: rect=${JSON.stringify(d.cruRect)} opacity=${d.cruOpacity}`);
  console.log(`  elemento visivel: ${d.escolhaTag} rect=${JSON.stringify(d.rect)}`);
  const x = d.rect[0] + Math.round(d.rect[2] / 2);
  const y = d.rect[1] + Math.round(d.rect[3] / 2);
  const r = await cliqueGuardado(x, y);
  await sleep(2500);
  const dp = await lerRadios();
  dp.forEach((q,i)=>console.log(`  [${i}] ${q.sel?"●":"○"} ${q.arq.slice(0,44)}`));
  const novo = dp.find((q) => q.sel);
  const mudou = novo && novo.arq === alvo.arq;
  reg("dispatchMouseEvent no radio via coordenada do elemento visivel", r.ok && mudou,
      `${antes.arq.slice(0,24)} -> ${novo?.arq.slice(0,24) ?? "nenhum"}`);

  // reverter para o curriculo original
  if (mudou) {
    const volta = (await lerRadios()).find((q) => q.arq === antes.arq);
    const oid2 = (await cdp("DOM.resolveNode", { backendNodeId: volta.bdn })).result?.object?.objectId;
    const i2 = await cdp("Runtime.callFunctionOn", { objectId: oid2, returnByValue: true, functionDeclaration: `function(){
      let v=null; for(let n=this;n&&n!==document.body;n=n.parentElement){const r=n.getBoundingClientRect(); if(r.width>40&&r.height>20){v=n;break;}}
      const l=this.id?document.querySelector('label[for="'+CSS.escape(this.id)+'"]'):null;
      const e=(l&&l.getBoundingClientRect().width>10)?l:v; const r=e.getBoundingClientRect();
      return JSON.stringify([Math.round(r.x),Math.round(r.y),Math.round(r.width),Math.round(r.height)]); }` });
    const rr = JSON.parse(i2.result.result.value);
    await cliqueGuardado(rr[0]+Math.round(rr[2]/2), rr[1]+Math.round(rr[3]/2));
    await sleep(2000);
    console.log(`  restaurado -> ${(await lerRadios()).find(q=>q.sel)?.arq.slice(0,40)}`);
  }
}

// ---------- guarda: recusa explicita no botao de submeter ----------
console.log("\n=== guarda: tentativa deliberada sobre 'Enviar candidatura' ===");
const btn = (await lerAX()).find((n) => n.role?.value === "button" && /enviar candidatura/i.test(String(n.name?.value ?? "")));
if (btn) {
  const bm = await cdp("DOM.getBoxModel", { backendNodeId: btn.backendDOMNodeId });
  const cc = bm.result.model.content;
  const r = await cliqueGuardado(Math.round((cc[0]+cc[4])/2), Math.round((cc[1]+cc[5])/2));
  reg("guarda elementFromPoint recusa o botao de submeter", r.ok === false, `alvo lido: "${r.alvo}"`);
}

// ---------- fechar ----------
console.log("\n=== fechar sem submeter ===");
for (const rot of ["fechar|close|dismiss", "descartar|discard"]) {
  await ev(`(() => { const b=[...document.querySelectorAll('button')].find(x=>new RegExp("${rot}","i").test((x.getAttribute('aria-label')||x.innerText||'').trim())); if(b)b.click(); return !!b; })()`);
  await sleep(2200);
}
const fim = await lerAX();
console.log(`  dialogs finais: ${fim.filter(n=>n.role?.value==='dialog').length}`);
console.log(`  botao 'Candidatura simplificada' de volta na pagina: ${fim.some(n=>/candidatura simplificada/i.test(String(n.name?.value??""))) ? "SIM — nada submetido" : "VERIFICAR"}`);

console.log("\n=== RESUMO ===");
R.forEach((r) => console.log(`${r.ok ? "OK    " : "FALHOU"}  ${r.n}`));
fechar();
