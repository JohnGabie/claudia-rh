import { alvos, ligar, sleep } from "./cdp.mjs";

const PORT = 9889;
const PROIBIDO = /enviar|submit|revisar|pr[oó]ximo|continuar|apply|avan[çc]ar/i;
const lista = await alvos(PORT);
const p = lista.find((t) => t.type === "page" && /linkedin\.com/.test(t.url));
const { cdp, fechar } = await ligar(p.webSocketDebuggerUrl);
await cdp("Page.enable"); await cdp("DOM.enable"); await cdp("Runtime.enable"); await cdp("Accessibility.enable");
await cdp("Page.bringToFront");

const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
const R = [];
const reg = (n, ok, nota) => { R.push({ n, ok }); console.log(`${ok ? "OK    " : "FALHOU"}  ${n}  ${nota ?? ""}`); };

// helper: centro a partir de um seletor CSS
const centroSel = async (sel) => {
  const doc = await cdp("DOM.getDocument", { depth: 1 });
  const nid = (await cdp("DOM.querySelector", { nodeId: doc.result.root.nodeId, selector: sel })).result?.nodeId;
  if (!nid) return null;
  const bm = await cdp("DOM.getBoxModel", { nodeId: nid });
  const c = bm.result?.model?.content;
  if (!c) return null;
  return { x: Math.round((c[0] + c[4]) / 2), y: Math.round((c[1] + c[5]) / 2) };
};

// guarda dura: nunca clicar num ponto cujo rotulo seja proibido
const cliqueGuardado = async (x, y, etiqueta) => {
  const alvo = await ev(`(() => {
    const e = document.elementFromPoint(${x}, ${y});
    if (!e) return "NADA";
    const clic = e.closest('button,a,input,label,[role=button],[role=radio],[role=checkbox]') || e;
    return clic.tagName + "|" + (clic.getAttribute('aria-label') || clic.innerText || clic.value || '').trim().slice(0,80);
  })()`);
  console.log(`  ponto (${x},${y}) -> ${alvo}`);
  if (alvo === "NADA") { console.log("  RECUSADO: nada no ponto"); return { ok: false, alvo }; }
  if (PROIBIDO.test(alvo)) { console.log(`  RECUSADO pela guarda: "${alvo}"`); return { ok: false, alvo, recusado: true }; }
  await cdp("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none", buttons: 0 });
  await cdp("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1 });
  await cdp("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1 });
  return { ok: true, alvo };
};

// ---------- A. dispatchMouseEvent no LinkedIn: foco num campo de busca ----------
console.log("\n=== A. dispatchMouseEvent no LinkedIn (campo de busca de palavra-chave) ===");
const sel = "input.jobs-search-box__text-input[id*='keyword'], #jobs-search-box-keyword-id-ember, input[aria-label*='Pesquisar cargo'], input[aria-label*='Search by title']";
const c = await centroSel(sel);
console.log(`  centro do campo: ${JSON.stringify(c)}`);
await ev(`document.activeElement && document.activeElement.blur()`);
const antesFoco = await ev("document.activeElement.tagName + '/' + (document.activeElement.getAttribute('aria-label')||'').slice(0,40)");
const r1 = c ? await cliqueGuardado(c.x, c.y, "campo busca") : { ok: false };
await sleep(800);
const depoisFoco = await ev("document.activeElement.tagName + '/' + (document.activeElement.getAttribute('aria-label')||'').slice(0,40)");
reg("Input.dispatchMouseEvent no LinkedIn (muda foco)", antesFoco !== depoisFoco && /INPUT/.test(depoisFoco), `${antesFoco} -> ${depoisFoco}`);

// ---------- B. insertText + dispatchKeyEvent no LinkedIn ----------
console.log("\n=== B. Input.insertText / dispatchKeyEvent no LinkedIn ===");
const valAntes = await ev(`(document.activeElement.value ?? '')`);
await cdp("Input.insertText", { text: "CDP-teste" });
await sleep(500);
const valDepois = await ev(`(document.activeElement.value ?? '')`);
reg("Input.insertText no LinkedIn", valDepois.includes("CDP-teste"), `"${valAntes}" -> "${valDepois}"`);

for (let i = 0; i < 9; i++) {
  await cdp("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8, nativeVirtualKeyCode: 8 });
  await cdp("Input.dispatchKeyEvent", { type: "keyUp", key: "Backspace", code: "Backspace", windowsVirtualKeyCode: 8, nativeVirtualKeyCode: 8 });
}
await sleep(500);
const valLimpo = await ev(`(document.activeElement.value ?? '')`);
reg("Input.dispatchKeyEvent no LinkedIn (Backspace apaga)", !valLimpo.includes("CDP-teste"), `-> "${valLimpo}"`);
await ev("document.activeElement.blur()");
await cdp("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
await cdp("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
await sleep(600);

// ---------- C. abrir o modal Easy Apply ----------
console.log("\n=== C. modal Easy Apply ===");
const abriu = await ev(`(() => {
  const escopo = document.querySelector('.jobs-details, .job-view-layout, main') || document;
  const b = [...escopo.querySelectorAll('button.jobs-apply-button, button')].find(x => {
    const t = ((x.getAttribute('aria-label')||'') + ' ' + (x.innerText||'')).trim();
    return /candidatura simplificada|easy apply/i.test(t) && !/filtro|filter/i.test(t);
  });
  if (!b) return 'NAO ENCONTRADO';
  b.scrollIntoView({block:'center'});
  b.click();
  return 'clicou: ' + ((b.getAttribute('aria-label')||b.innerText)||'').trim().slice(0,70);
})()`);
console.log(`  ${abriu}`);
let dialogos = 0;
for (let i = 0; i < 8; i++) {
  await sleep(1800);
  dialogos = await ev("document.querySelectorAll('[role=dialog]').length");
  if (dialogos > 0) break;
}
console.log(`  dialogs abertos: ${dialogos}`);

if (dialogos > 0) {
  const nos = (await cdp("Accessibility.getFullAXTree")).result?.nodes ?? [];
  // subarvore do dialog
  const porId = new Map(nos.map((n) => [n.nodeId, n]));
  const dlg = nos.find((n) => n.role?.value === "dialog");
  const dentro = [];
  const anda = (id) => { const n = porId.get(id); if (!n) return; dentro.push(n); (n.childIds ?? []).forEach(anda); };
  if (dlg) anda(dlg.nodeId);
  const campos = dentro.filter((n) => ["textbox","combobox","radio","checkbox","button","listbox","spinbutton"].includes(n.role?.value));
  const comNome = campos.filter((n) => (n.name?.value ?? "").trim().length > 0);
  const serialD = JSON.stringify(dentro.map((n) => ({ r: n.role?.value, n: n.name?.value, v: n.value?.value })));
  console.log(`\n  ARVORE DO MODAL (CDP puro, desktop)`);
  console.log(`    nos no dialog     : ${dentro.length}`);
  console.log(`    chars             : ${serialD.length}`);
  console.log(`    tokens (chars/3.5): ${Math.round(serialD.length / 3.5)}`);
  console.log(`    campos            : ${campos.length}`);
  console.log(`    campos COM nome   : ${comNome.length} de ${campos.length}`);
  console.log(`\n  campos:`);
  campos.forEach((n, i) => console.log(`    [${i}] ${(n.role.value + "        ").slice(0,10)} "${String(n.name?.value ?? "").slice(0,62)}"  bdn=${n.backendDOMNodeId}`));
  reg("Accessibility.getFullAXTree nomeia campos do modal (desktop, CDP puro)", comNome.length > campos.length * 0.5, `${comNome.length}/${campos.length}`);

  // ---------- D. clique por coordenada num alvo seguro do modal ----------
  console.log("\n=== D. cadeia AX -> getBoxModel -> dispatchMouseEvent DENTRO do modal ===");
  const seguro = campos.find((n) => {
    const nm = String(n.name?.value ?? "");
    return (n.role.value === "radio" || n.role.value === "checkbox") && nm && !PROIBIDO.test(nm);
  });
  if (!seguro) {
    console.log("  nenhum radio/checkbox seguro no passo actual — a saltar");
  } else {
    const estado = (n) => n?.properties?.find((q) => q.name === "checked")?.value?.value;
    console.log(`  alvo: ${seguro.role.value} "${String(seguro.name.value).slice(0,60)}" checked=${estado(seguro)}`);
    const bm = await cdp("DOM.getBoxModel", { backendNodeId: seguro.backendDOMNodeId });
    const cc = bm.result?.model?.content;
    if (!cc) { console.log("  getBoxModel falhou:", JSON.stringify(bm.error)); }
    else {
      const x = Math.round((cc[0] + cc[4]) / 2), y = Math.round((cc[1] + cc[5]) / 2);
      const res = await cliqueGuardado(x, y, "campo do modal");
      await sleep(2500);
      const nos2 = (await cdp("Accessibility.getFullAXTree")).result?.nodes ?? [];
      const dep = nos2.find((n) => n.backendDOMNodeId === seguro.backendDOMNodeId)
               ?? nos2.find((n) => String(n.name?.value ?? "") === String(seguro.name.value));
      const mudou = estado(seguro) !== estado(dep) || String(dep?.name?.value ?? "") !== String(seguro.name.value);
      reg("clique por coordenada dentro do modal do LinkedIn", res.ok && mudou,
        `checked ${estado(seguro)} -> ${estado(dep)} | nome "${String(dep?.name?.value??"?").slice(0,40)}"`);
    }
  }

  // ---------- fechar sem submeter ----------
  console.log("\n=== fechar o modal sem submeter ===");
  for (const rot of ["fechar|close|dismiss", "descartar|discard"]) {
    await ev(`(() => { const b=[...document.querySelectorAll('button')].find(x=>new RegExp("${rot}","i").test((x.getAttribute('aria-label')||x.innerText||'').trim())); if(b)b.click(); return !!b; })()`);
    await sleep(2200);
  }
  console.log(`  dialogs finais: ${await ev("document.querySelectorAll('[role=dialog]').length")}`);
}

console.log("\n=== RESUMO LINKEDIN ===");
R.forEach((r) => console.log(`${r.ok ? "OK    " : "FALHOU"}  ${r.n}`));
fechar();
