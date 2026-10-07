// Read-only check of the lab copy. Both sides are only read; nothing is written, moved or deleted.
// Prints counts as JSON and the differing paths on stderr. Never prints file contents or hashes (the MCP key is hashed too).
// Usage: node conferir-copia.mjs --copia <dir> (--origem <dir> | --manifesto <file.sha256>) [--excluir a,b/c]
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { lerManifesto, compararListas, excluido } from "./manifesto.mjs";

/** Every regular file under `raiz` (relative path with /, size, sha256). Refuses links instead of following them. */
export function listarArquivos(raiz, excluir = []) {
  const saida = [];
  for (const e of fs.readdirSync(raiz, { recursive: true, withFileTypes: true })) {
    const abs = path.join(e.parentPath ?? e.path, e.name);
    const rel = path.relative(raiz, abs).split(path.sep).join("/");
    if (excluido(rel, excluir)) continue;
    if (e.isSymbolicLink()) throw new Error(`link found, refusing to follow it: ${rel}`);
    if (!e.isFile()) continue;
    const buf = fs.readFileSync(abs);
    saida.push({ caminho: rel, tamanho: buf.length, sha256: crypto.createHash("sha256").update(buf).digest("hex") });
  }
  return saida;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: { origem: { type: "string" }, copia: { type: "string" }, manifesto: { type: "string" }, excluir: { type: "string", default: "" } } });
    if (!values.copia || Boolean(values.origem) === Boolean(values.manifesto)) {
      console.error("Usage: node conferir-copia.mjs --copia <dir> (--origem <dir> | --manifesto <file>) [--excluir a,b/c]");
      process.exit(2);
    }
    const excluir = values.excluir.split(",").map((s) => s.trim()).filter(Boolean);
    const esperado = values.manifesto ? lerManifesto(fs.readFileSync(values.manifesto, "utf8")) : listarArquivos(values.origem, excluir);
    const achado = listarArquivos(values.copia, excluir);
    const r = compararListas(esperado, achado);
    for (const f of r.faltando) console.error(`MISSING ${f}`);
    for (const d of r.diferentes) console.error(`DIFFERS ${d.caminho}: ${d.motivo}`);
    for (const s of r.sobrando) console.error(`EXTRA ${s}`);
    if (esperado.length === 0) console.error("the expected side has 0 files: wrong origin?");
    console.log(JSON.stringify({ origem: esperado.length, copia: achado.length, faltando: r.faltando.length, diferentes: r.diferentes.length, sobrando: r.sobrando.length }));
    process.exit(esperado.length === 0 || r.faltando.length || r.diferentes.length || r.sobrando.length ? 1 : 0);
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
