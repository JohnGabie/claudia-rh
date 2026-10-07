// Read-only repo guard for CI and local use. Lists versioned files with `git ls-files -z` and fails when a file that
// must never be versioned is there. Never writes, moves or deletes anything.
// Usage (from any folder of the checkout): node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs
import path from "node:path";
import { spawnSync } from "node:child_process";
import { arquivosProibidos } from "./proibidos.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const r = spawnSync("git", ["-C", RAIZ, "ls-files", "-z"], { encoding: "utf8" });
if (r.status !== 0) {
  console.error(`git ls-files failed: ${(r.stderr || "").trim()}`);
  process.exit(1);
}
const arquivos = r.stdout.split("\0").filter(Boolean);
const proibidos = arquivosProibidos(arquivos);
for (const p of proibidos) console.error(`FORBIDDEN ${p.arquivo}: ${p.motivo}`);
console.log(JSON.stringify({ arquivos: arquivos.length, proibidos: proibidos.length, caminhos: 0 }));
process.exit(proibidos.length ? 1 : 0);
