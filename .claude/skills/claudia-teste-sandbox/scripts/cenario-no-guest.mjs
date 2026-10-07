// Creates a data scenario INSIDE the guest (D1: never on the PC) through C:\labsandbox\app\guest-cenario.ps1, then waits
// for C:\labwork\run\cenario.json (= <work folder>\guest-rw\run\cenario.json on the host) carrying this run's nonce.
// Usage: node cenario-no-guest.mjs --cenario novo|mock|copia [--autorizado] [--manter-automacao] [--id <id>] [--timeout 600]
// copia needs --autorizado (ask the user in this round) and a lab started with test-up -DadosReais.
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { onde, sandboxId, noGuest, validarInt, falhaDe } from "./comum.mjs";

const CENARIOS = ["novo", "mock", "copia"];
const uso = (msg) => Object.assign(new Error(msg), { code: "ERR_PARSE_ARGS_USAGE" });

/** PURE. The PowerShell line run in the guest. The scenario comes only from a fixed list, so nothing needs quoting. */
export function comandoDeCenarioNoGuest({ cenario, autorizado = false, manterAutomacao = false, nonce }) {
  if (!/^[a-f0-9]{8,64}$/.test(String(nonce))) throw new Error("nonce must be 8-64 hex characters");
  if (!CENARIOS.includes(cenario)) throw uso(`unknown scenario: ${cenario} (novo, mock, copia)`);
  if (cenario !== "copia" && (autorizado || manterAutomacao)) throw uso("--autorizado and --manter-automacao only apply to copia");
  if (cenario === "copia" && !autorizado) throw uso("copia needs --autorizado (ask the user in this round)");
  return `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario ${cenario} -Nonce ${nonce}${autorizado ? " -Autorizado" : ""}${manterAutomacao ? " -ManterAutomacao" : ""}`;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: {
      cenario: { type: "string" }, autorizado: { type: "boolean" }, "manter-automacao": { type: "boolean" },
      id: { type: "string" }, timeout: { type: "string", default: "600" },
    } });
    const timeout = validarInt(values.timeout, 1, 7200, "--timeout");
    const nonce = crypto.randomBytes(8).toString("hex");
    const cmd = comandoDeCenarioNoGuest({ cenario: values.cenario, autorizado: Boolean(values.autorizado), manterAutomacao: Boolean(values["manter-automacao"]), nonce });
    const id = sandboxId(values.id);
    const result = path.join(onde().guestRw, "run", "cenario.json");
    noGuest(id, "ExistingLogin", cmd);
    const fim = Date.now() + timeout * 1000;
    let j = null;
    while (Date.now() < fim) {
      try { const t = JSON.parse(fs.readFileSync(result, "utf8")); if (t.nonce === nonce && t.exit !== null && t.exit !== undefined) { j = t; break; } } catch { /* not written yet */ }
      await new Promise((r) => setTimeout(r, 2000));
    }
    if (!j) throw new Error(`no result for this run in ${result} after ${timeout}s`);
    console.log(JSON.stringify({ exit: j.exit, cenario: j.cenario, saida: j.saida, erro: j.erro ?? null }));
    process.exit(j.exit === 0 ? 0 : 1);
  } catch (e) { falhaDe(e); }
}
