// Small helpers shared by the scenario tests.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { estaNoSandbox, MARCADOR_GUEST, RAIZ_TESTE_GUEST, motivoRecusaRaizTeste } from "../caminhos-lab.mjs";

export const SCRIPTS = path.join(import.meta.dirname, "..");

/** Tests that create or delete folders run only inside Windows Sandbox (CLAUDE.md, hard rules). Throws elsewhere. */
export function exigirSandbox() {
  const usuario = os.userInfo().username; // OS call, not an environment variable
  if (!estaNoSandbox({ identidade: usuario, temBootstrap: fs.existsSync(MARCADOR_GUEST) })) {
    throw new Error(`SANDBOX ONLY: this test creates and deletes folders; it runs only inside Windows Sandbox (user is ${usuario}).`);
  }
}

/** Temp folder with a space in its name, on the guest-only Z: drive (spec A6). Sandbox only. */
export function tempDir(prefixo) {
  exigirSandbox(); // before any folder is created
  fs.mkdirSync(RAIZ_TESTE_GUEST, { recursive: true });
  const raiz = fs.realpathSync.native(RAIZ_TESTE_GUEST);
  const m1 = motivoRecusaRaizTeste(raiz);
  if (m1) throw new Error(`tempDir: refused (${m1}): ${raiz}`);
  const dir = fs.mkdtempSync(path.join(raiz, `cs test ${prefixo} `));
  const m2 = motivoRecusaRaizTeste(fs.realpathSync.native(dir));
  if (m2) throw new Error(`tempDir: refused (${m2}): ${dir}`);
  return dir;
}

export function rodar(script, args = [], env = {}) {
  const r = spawnSync(process.execPath, [script, ...args], { encoding: "utf8", env: { ...process.env, ...env } });
  const out = (r.stdout ?? "").trim().split(/\r?\n/);
  return { code: r.status, out, ultima: out.at(-1), err: r.stderr ?? "" };
}
