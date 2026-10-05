// Small helpers shared by the scenario tests.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

export const SCRIPTS = path.join(import.meta.dirname, "..");

/** Temp folder with a space in its name, so every test also covers paths with spaces. */
export function tempDir(prefixo) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `cs test ${prefixo} `));
}

export function rodar(script, args = [], env = {}) {
  const r = spawnSync(process.execPath, [script, ...args], { encoding: "utf8", env: { ...process.env, ...env } });
  const out = (r.stdout ?? "").trim().split(/\r?\n/);
  return { code: r.status, out, ultima: out.at(-1), err: r.stderr ?? "" };
}
