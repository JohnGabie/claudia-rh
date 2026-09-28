// End-to-end Easy Apply, by CDP alone. This is the reference implementation:
// everything the Rust side has to reproduce is here, in one file.
//
// Consolidates what five throwaway attempts taught:
//   - activate the tab before anything (a frozen renderer swallows input)
//   - LinkedIn's modal does NOT expose role=dialog in the DOM, only in the tree
//   - the advance button renames itself between steps: Avançar → Avaliar
//   - detect a stuck step by comparing the field signature, or it loops silently
//
// SAFETY: submission is gated on `--submeter`. Without it, the run stops at the
// last screen and reports what would be sent.
import { alvos, ligar, sleep } from "./cdp.mjs";

const PORT = Number(process.env.CDP_PORT ?? 9888);
const SUBMETER = process.argv.includes("--submeter");
const AVANCO = /^(avançar|avaliar|revisar|next|review)/i;
const ENVIO = /^(enviar candidatura|submit application)/i;

export async function candidatar({ jobId, telefone, curriculo, submeter = false }) {
  const lista = await alvos(PORT);
  const alvo = lista.find((t) => t.type === "page" && /linkedin/.test(t.url || ""));
  if (!alvo) throw new Error("sem aba do LinkedIn");

  // A frozen tab accepts no input events. Activate, then wait for the renderer.
  await fetch(`http://127.0.0.1:${PORT}/json/activate/${alvo.id}`).catch(() => {});
  const { cdp, ws } = await ligar(alvo.webSocketDebuggerUrl);
  await cdp("Page.enable"); await cdp("Runtime.enable"); await cdp("Accessibility.enable");
  await cdp("Page.bringToFront");

  const js = async (e) => (await cdp("Runtime.evaluate", { expression: e, returnByValue: true })).result?.result?.value;
  const arv = async () => (await cdp("Accessibility.getFullAXTree")).result?.nodes?.filter((n) => n.name?.value && n.role?.value) ?? [];
  const clicar = async (rot) => js(`(() => { const b=[...document.querySelectorAll("button")]
    .find(x=>(x.getAttribute("aria-label")||x.innerText||"").trim().startsWith(${JSON.stringify(rot.slice(0, 14))}));
    if(b){b.scrollIntoView({block:"center"});b.click();return "ok";} return "não"; })()`);

  for (let i = 0; i < 10; i++) {
    if (await js("document.visibilityState") === "visible") break;
    await sleep(800);
  }

  await cdp("Page.navigate", { url: `https://www.linkedin.com/jobs/view/${jobId}/` });
  let prev = -1, est = 0;
  for (let i = 0; i < 14; i++) {
    await sleep(1500);
    const n = (await arv()).length;
    if (n === prev && n > 50) { if (++est >= 2) break; } else { est = 0; prev = n; }
  }

  // Open the modal. Scoping to [role=dialog] fails — LinkedIn does not set it.
  await js(`(() => { const b=[...document.querySelectorAll("button")]
    .find(x=>/candidatura simplificada|easy apply/i.test(x.getAttribute("aria-label")||""));
    if(b){b.scrollIntoView({block:"center"});b.click();} return !!b; })()`);

  let aberto = false;
  for (let i = 0; i < 10 && !aberto; i++) { await sleep(1800); aberto = (await arv()).some((n) => n.role.value === "dialog"); }
  if (!aberto) { ws.close(); return { ok: false, motivo: "modal não abriu" }; }

  const passos = [];
  let anterior = "";
  for (let passo = 1; passo <= 8; passo++) {
    const nos = await arv();
    const campos = nos.filter((n) => ["textbox", "combobox", "radio", "checkbox"].includes(n.role.value));
    const botoes = [...new Set(nos.filter((n) => n.role.value === "button").map((n) => String(n.name.value)))]
      .filter((b) => !/pular para|fechar menu|^fechar$|^baixar/i.test(b));
    passos.push({ passo, campos: campos.map((c) => ({ papel: c.role.value, nome: String(c.name.value), valor: c.value?.value ?? null })), botoes });

    // Required empty text fields get the phone.
    const vazio = campos.find((c) => c.role.value === "textbox" && !c.value?.value);
    if (vazio && telefone) {
      await js(`(() => { const t=[...document.querySelectorAll('input[type=text],input[type=tel]')]
        .find(i => !i.value && i.offsetParent !== null);
        if(!t) return "nenhum"; const set=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,"value").set;
        t.focus(); set.call(t, ${JSON.stringify(telefone)});
        t.dispatchEvent(new Event("input",{bubbles:true})); t.dispatchEvent(new Event("change",{bubbles:true})); t.blur();
        return "ok"; })()`);
      await sleep(1500);
    }

    // Résumé step: click the label, since the input itself is opacity:0.
    const radios = campos.filter((c) => c.role.value === "radio" && /\.pdf/i.test(String(c.name.value)));
    if (radios.length > 1 && curriculo) {
      await js(`(() => { const l=[...document.querySelectorAll('label')].find(x=>(x.innerText||"").includes(${JSON.stringify(curriculo)}));
        if(l){l.click();return "ok";} return "não"; })()`);
      await sleep(2500);
    }

    const assin = campos.map((c) => String(c.name.value).slice(0, 20)).join("|");
    if (passo > 1 && assin === anterior && radios.length === 0) { ws.close(); return { ok: false, motivo: "passo preso — validação recusou", passos }; }
    anterior = assin;

    const env = botoes.find((b) => ENVIO.test(b));
    if (env) {
      if (!submeter) { ws.close(); return { ok: true, submetido: false, motivo: "parado antes de enviar", passos }; }
      await clicar(env);
      await sleep(7000);
      const fim = await arv();
      const conf = fim.filter((n) => /enviada|sent/i.test(String(n.name.value))).map((n) => String(n.name.value));
      ws.close();
      return { ok: true, submetido: conf.length > 0, confirmacao: conf.slice(0, 2), passos };
    }

    const av = botoes.find((b) => AVANCO.test(b));
    if (!av) { ws.close(); return { ok: false, motivo: "sem avanço nem envio", passos }; }
    await clicar(av);
    await sleep(4000);
  }
  ws.close();
  return { ok: false, motivo: "excedeu 8 passos", passos };
}

if (import.meta.url === `file://${process.argv[1]?.replace(/\/g, "/")}`) {
  const jobId = process.argv[2];
  if (!jobId) { console.error("uso: node candidatura-completa.mjs <jobId> [--submeter]"); process.exit(1); }
  const fs = await import("fs");
  const yaml = fs.readFileSync(`${process.env.APPDATA}/io.github.johngabie.claudia-rh/candidate_base.yaml`, "utf8");
  const telefone = (yaml.match(/telefone:\s*"?([^"\n]+)"?/) || [])[1]?.trim().replace(/^55\s*/, "").replace(/\s/g, "");
  const r = await candidatar({ jobId, telefone, curriculo: "cv_fullstack_engineer.pdf", submeter: SUBMETER });
  console.log(JSON.stringify(r, null, 1));
}
