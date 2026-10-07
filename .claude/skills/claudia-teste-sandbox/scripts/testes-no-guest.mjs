// Runs Sandbox-only tests inside the running guest through the lab's guest-build.ps1 of the `testes` environment
// (compose.json), which uses the Node the guest installed; `exit /b 97` stops everything if the Z: disk is not mounted.
// The guest copies C:\labproject (the project passed to test-up -Project)
// to C:\labdata\project and runs the command there. The host only reads <work folder>\guest-rw\build\result.json.
// Usage: node testes-no-guest.mjs [--id <id>] (--node <file.test.mjs> ... | --ps sandbox-lab/scripts/test/<x>.sandbox.ps1) [--timeout 900]
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { onde, sandboxId, noGuest, validarInt, falhaDe } from "./comum.mjs";

const ARQ_NODE = /^\.claude\/skills\/[A-Za-z0-9_\-/.]+\.test\.mjs$/;
const ARQ_PS = /^sandbox-lab\/scripts\/test\/[A-Za-z0-9_\-.]+\.sandbox\.ps1$/;

/** PURE. The cmd.exe line guest-build.ps1 runs as its "build" step. No quotes, so nothing needs escaping. */
export function comandoDeTesteNoGuest({ node = [], ps = null, nonce }) {
  if (!/^[a-f0-9]{8,64}$/.test(String(nonce))) throw new Error("nonce must be 8-64 hex characters");
  if ((node.length > 0) === Boolean(ps)) throw new Error("pass either --node files or one --ps file");
  for (const f of [...node, ...(ps ? [ps] : [])]) if (String(f).split("/").includes("..")) throw new Error(`'..' is not allowed: ${f}`);
  for (const f of node) if (!ARQ_NODE.test(f)) throw new Error(`not a skill test file: ${f}`);
  if (ps && !ARQ_PS.test(ps)) throw new Error(`not a lab Sandbox test: ${ps}`);
  const prefixo = `set LAB_NONCE=${nonce}&& (if not exist Z:\\claudia-teste-sandbox exit /b 97)&& set TEMP=Z:\\tmp&& set TMP=Z:\\tmp&& (if not exist Z:\\tmp mkdir Z:\\tmp)`;
  const corpo = ps ? `powershell -NoProfile -ExecutionPolicy Bypass -File ${ps.replace(/\//g, "\\")}` : `node --test ${node.join(" ")}`;
  return `${prefixo}&& ${corpo}`;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: { id: { type: "string" }, node: { type: "string", multiple: true }, ps: { type: "string" }, timeout: { type: "string", default: "900" } } });
    const timeout = validarInt(values.timeout, 1, 7200, "--timeout");
    const nonce = crypto.randomBytes(8).toString("hex");
    const build = comandoDeTesteNoGuest({ node: values.node ?? [], ps: values.ps ?? null, nonce });
    const id = sandboxId(values.id);
    const result = path.join(onde().guestRw, "build", "result.json");
    noGuest(id, "ExistingLogin", `& C:\\labsandbox\\app\\guest-build.ps1 -Install '' -Precisa node -Build '${build}'`);
    const fim = Date.now() + timeout * 1000;
    let j = null;
    while (Date.now() < fim) {
      try {
        const t = JSON.parse(fs.readFileSync(result, "utf8"));
        if (t.end && String(t.steps?.build?.cmd ?? "").includes(nonce)) { j = t; break; }
      } catch { /* not written yet */ }
      await new Promise((r) => setTimeout(r, 2000));
    }
    if (!j) throw new Error(`no result for this run in ${result} after ${timeout}s`);
    const exit = j.steps.build.exit;
    console.log(JSON.stringify({ exit, log: path.join(path.dirname(result), "guest-build.log") }));
    process.exit(exit === 0 ? 0 : 1);
  } catch (e) { falhaDe(e); }
}
