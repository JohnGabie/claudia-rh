// Regenerates references/comandos.md from the Rust sources, so the list never
// goes stale. Run from claudia-rh/.
import fs from "node:fs";
import path from "node:path";
import { classificarComando } from "./regras.mjs";

const RAIZ = path.resolve(import.meta.dirname, "../../../..");
const src = path.join(RAIZ, "src-tauri/src");
const achados = [];

function varrer(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) { varrer(p); continue; }
    if (!e.name.endsWith(".rs")) continue;
    const t = fs.readFileSync(p, "utf8");
    for (const m of t.matchAll(/#\[tauri::command\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)/g)) {
      achados.push({ nome: m[1], arquivo: path.relative(RAIZ, p).split(path.sep).join("/") });
    }
  }
}
varrer(src);
achados.sort((a, b) => a.nome.localeCompare(b.nome));

const corpo = achados.map((c) => `| \`${c.nome}\` | ${classificarComando(c.nome)} | ${c.arquivo} |`).join("\n");
fs.writeFileSync(
  path.join(import.meta.dirname, "../references/comandos.md"),
  `<!-- GERADO por scripts/gerar-comandos.mjs — não editar à mão -->\n# Comandos Tauri (${achados.length})\n\n` +
  `"escrita" inclui tudo cujo prefixo não é de leitura reconhecida. Esses exigem \`invocar.mjs --escrever\`.\n\n` +
  `| comando | tipo | arquivo |\n|---|---|---|\n${corpo}\n`);
console.log(`${achados.length} comandos`);
