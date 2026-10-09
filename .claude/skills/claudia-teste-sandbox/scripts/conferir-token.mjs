// Read-only scan of folders for Anthropic tokens (spec A4/A5). Prints only file and line, never the matched text.
// Usage: node .claude/skills/claudia-teste-sandbox/scripts/conferir-token.mjs --pasta <folder> [--pasta <folder>]
// Does not follow links. Never writes, moves or deletes anything.
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { achaToken } from "./proibidos.mjs";
import { falhaDe } from "./comum.mjs";

const LIMITE = 256 * 1024 * 1024;

function* arquivos(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isSymbolicLink()) continue; // junctions are reported as links on Windows
    if (e.isDirectory()) yield* arquivos(p);
    else if (e.isFile()) yield p;
  }
}

try {
  const { values } = parseArgs({ options: { pasta: { type: "string", multiple: true } } });
  if (!values.pasta?.length) throw Object.assign(new Error("Usage: node conferir-token.mjs --pasta <folder> [--pasta <folder>]"), { code: "ERR_PARSE_ARGS_USAGE" });
  let total = 0;
  const achados = [];
  const pulados = [];
  for (const pasta of values.pasta) {
    const raiz = path.resolve(pasta);
    if (!fs.existsSync(raiz)) continue;
    for (const f of arquivos(raiz)) {
      total++;
      if (fs.statSync(f).size > LIMITE) { pulados.push(f); continue; }
      for (const a of achaToken(fs.readFileSync(f, "latin1"))) achados.push({ arquivo: f, linha: a.linha });
    }
  }
  for (const a of achados) console.error(`TOKEN ${a.arquivo}:${a.linha}`);
  console.log(JSON.stringify({ arquivos: total, achados, pulados }));
  process.exit(achados.length ? 1 : 0);
} catch (e) { falhaDe(e); }
