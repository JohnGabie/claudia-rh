import { alvos, ligar, sleep } from "./cdp.mjs";

const PORT = 9889;
const lista = await alvos(PORT);
const p = lista.find((t) => t.type === "page" && /linkedin\.com/.test(t.url));
const { cdp, fechar } = await ligar(p.webSocketDebuggerUrl);
await cdp("Page.enable"); await cdp("DOM.enable"); await cdp("Runtime.enable"); await cdp("Accessibility.enable");

const ev = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;

// busca com filtro de candidatura simplificada
await cdp("Page.navigate", { url: "https://www.linkedin.com/jobs/search/?keywords=software%20engineer&location=Denmark&f_AL=true" });
await sleep(9000);
console.log("url:", await ev("location.href.slice(0,110)"));
console.log("logado:", await ev("!!document.querySelector('.global-nav__me, [data-control-name=nav_settings]') || /feed|jobs/.test(location.pathname)"));

// escolhe a primeira vaga da lista
const vaga = await ev(`(() => {
  const a = [...document.querySelectorAll('a.job-card-container__link, a.job-card-list__title--link, .job-card-container a[href*="/jobs/view/"]')];
  if (!a.length) return null;
  a[0].scrollIntoView({block:'center'});
  a[0].click();
  return a[0].href.slice(0,90) + " :: " + (a[0].innerText||"").trim().split("\\n")[0].slice(0,60);
})()`);
console.log("vaga:", vaga);
await sleep(7000);

const lerAX = async () => (await cdp("Accessibility.getFullAXTree")).result?.nodes ?? [];
const nos = await lerAX();
const serial = JSON.stringify(nos.map((n) => ({ r: n.role?.value, n: n.name?.value, v: n.value?.value })).filter((x) => x.r && x.r !== "none" && x.r !== "generic"));
console.log(`\nARVORE DA VAGA (CDP puro, desktop)`);
console.log(`  nos totais          : ${nos.length}`);
console.log(`  chars (serializada) : ${serial.length}`);
console.log(`  tokens (chars/3.5)  : ${Math.round(serial.length / 3.5)}`);

const botao = await ev(`(() => {
  const b = [...document.querySelectorAll('button')].find(x => /candidatura simplificada|easy apply/i.test((x.getAttribute('aria-label')||x.innerText||'')));
  return b ? (b.getAttribute('aria-label')||b.innerText).trim().slice(0,70) : 'NAO ENCONTRADO';
})()`);
console.log(`  botao Easy Apply    : ${botao}`);
fechar();
