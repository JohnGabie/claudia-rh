// Writes the tools manifest for sandbox-lab/shared/tools. Reads `--raiz`; writes only `--saida`, which must not exist
// yet and must end in .sha256. Never overwrites, moves or deletes anything.
// Usage: node manifesto-ferramentas.mjs --raiz <tools folder> --saida <new file.sha256>
import fs from "node:fs";
import { parseArgs } from "node:util";
import { escreverManifesto } from "./manifesto.mjs";
import { listarArquivos } from "./conferir-copia.mjs";

try {
  const { values } = parseArgs({ options: { raiz: { type: "string" }, saida: { type: "string" } } });
  if (!values.raiz || !values.saida || !values.saida.endsWith(".sha256")) {
    console.error("Usage: node manifesto-ferramentas.mjs --raiz <folder> --saida <new file.sha256>");
    process.exit(2);
  }
  const entradas = listarArquivos(values.raiz);
  if (entradas.length === 0) throw new Error(`no files under ${values.raiz}`);
  fs.writeFileSync(values.saida, escreverManifesto(entradas), { flag: "wx" }); // "wx": fails if the file exists
  console.log(JSON.stringify({ arquivos: entradas.length, bytes: entradas.reduce((s, e) => s + e.tamanho, 0) }));
} catch (e) {
  console.error(e.message);
  process.exit(1);
}
