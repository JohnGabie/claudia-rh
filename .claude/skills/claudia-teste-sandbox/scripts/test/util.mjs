// Small helpers shared by the scenario tests.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { estaNoSandbox } from "../caminhos-lab.mjs";

export const SCRIPTS = path.join(import.meta.dirname, "..");

/** Tests that create or delete folders run only inside Windows Sandbox (CLAUDE.md, hard rules). Throws elsewhere. */
export function exigirSandbox() {
  const usuario = os.userInfo().username;
  if (!estaNoSandbox(usuario)) {
    throw new Error(`SANDBOX ONLY: this test creates and deletes folders; it runs only inside Windows Sandbox (user is ${usuario}).`);
  }
}

/** Temp folder with a space in its name, so every test also covers paths with spaces. Sandbox only. */
export function tempDir(prefixo) {
  exigirSandbox(); // before any folder is created
  // Must live under <tmp>/claudia-teste-sandbox: the scenario scripts refuse anything else (allow-list).
  const base = path.join(fs.realpathSync.native(os.tmpdir()), "claudia-teste-sandbox");
  fs.mkdirSync(base, { recursive: true });
  return fs.mkdtempSync(path.join(base, `cs test ${prefixo} `));
}

export function rodar(script, args = [], env = {}) {
  const r = spawnSync(process.execPath, [script, ...args], { encoding: "utf8", env: { ...process.env, ...env } });
  const out = (r.stdout ?? "").trim().split(/\r?\n/);
  return { code: r.status, out, ultima: out.at(-1), err: r.stderr ?? "" };
}
