// Read-only repo guard for CI and local use. Lists versioned files with `git ls-files -z` and fails when
// (1) a file that must never be versioned is there, or (2) a versioned text file under sandbox-lab/ or .claude/skills/
// holds a machine path (also docs/ and CLAUDE.md), or (3) any versioned file holds an Anthropic token (file and line only,
// never the text). Never writes, moves or deletes anything.
// Usage (from any folder of the checkout): node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { arquivosProibidos, achaCaminhoPessoal, achaToken, textoParaConferir } from "./proibidos.mjs";
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
// Tokens: every versioned file, binaries included (a token in a .sqlite shows up in ASCII). Only file and line are printed.
const tokens = [];
for (const a of arquivos) {
  const abs = path.join(RAIZ, a);
  if (!fs.existsSync(abs)) continue; // staged deletion
  for (const t of achaToken(fs.readFileSync(abs, "latin1"))) tokens.push({ arquivo: a, linha: t.linha });
}
for (const p of proibidos) console.error(`FORBIDDEN ${p.arquivo}: ${p.motivo}`);
for (const c of caminhos) console.error(`MACHINE PATH ${c.arquivo}:${c.linha}: ${c.trecho}`);
for (const t of tokens) console.error(`TOKEN ${t.arquivo}:${t.linha}`);
console.log(JSON.stringify({ arquivos: arquivos.length, proibidos: proibidos.length, caminhos: caminhos.length, tokens: tokens.length }));
process.exit(proibidos.length || caminhos.length || tokens.length ? 1 : 0);
