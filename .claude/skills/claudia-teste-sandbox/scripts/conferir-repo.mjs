// Read-only repo guard for CI and local use. Lists versioned files with `git ls-files -z` and fails when
// (1) a file that must never be versioned is there, or (2) a versioned text file under sandbox-lab/ or .claude/skills/
// holds a machine path (also docs/ and CLAUDE.md). Never writes, moves or deletes anything.
// Usage (from any folder of the checkout): node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { arquivosProibidos, achaCaminhoPessoal, textoParaConferir } from "./proibidos.mjs";
import { ambienteGitLimpo } from "./caminhos-lab.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const r = spawnSync("git", ["-C", RAIZ, "ls-files", "-z"], { encoding: "utf8", env: ambienteGitLimpo(process.env) });
if (r.status !== 0) {
  console.error(`git ls-files failed: ${(r.stderr || "").trim()}`);
  process.exit(1);
}
const arquivos = r.stdout.split("\0").filter(Boolean);
const proibidos = arquivosProibidos(arquivos);
const caminhos = [];
for (const a of arquivos) {
  if (!textoParaConferir(a)) continue;
  const abs = path.join(RAIZ, a);
  if (!fs.existsSync(abs)) continue; // staged deletion
  const buf = fs.readFileSync(abs);
  if (buf.subarray(0, 8000).includes(0)) continue; // binary (e.g. shared/db/backup.sqlite)
  for (const c of achaCaminhoPessoal(buf.toString("utf8"))) caminhos.push({ arquivo: a, ...c });
}
for (const p of proibidos) console.error(`FORBIDDEN ${p.arquivo}: ${p.motivo}`);
for (const c of caminhos) console.error(`MACHINE PATH ${c.arquivo}:${c.linha}: ${c.trecho}`);
console.log(JSON.stringify({ arquivos: arquivos.length, proibidos: proibidos.length, caminhos: caminhos.length }));
process.exit(proibidos.length || caminhos.length ? 1 : 0);
