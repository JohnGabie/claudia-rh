// Clean-machine scenario "copia": copy of the user's real app data. Reads the source only.
// Requires --autorizado: ask the user for permission for this round before running it.
// Usage: node cenario-copia.mjs --autorizado [--origem <folder>] [--saida <folder>] [--manter-automacao]
//   --origem            default %APPDATA%\io.github.johngabie.claudia-rh
//   --saida             default %TEMP%\claudia-sandbox\copia, recreated on every run
//   --manter-automacao  keep disparo.json / modo_autonomo.json as copied (default: turned off in the copy)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { DatabaseSync } from "node:sqlite";
import { APP_ID, novoCenarioDir, escreverCenario, desligarAutomacao, falhaDe, sair } from "./comum.mjs";

let values;
try {
  ({ values } = parseArgs({ options: {
    autorizado: { type: "boolean" }, origem: { type: "string" }, saida: { type: "string" }, "manter-automacao": { type: "boolean" },
  } }));
} catch (e) { falhaDe(e); }
if (!values.autorizado) {
  sair("Scenario 'copia' reads the user's real data: ask the user for permission for this round, then run again with --autorizado.", 2);
}
try {
  const origem = path.resolve(values.origem || path.join(process.env.APPDATA, APP_ID));
  if (!fs.existsSync(origem) || !fs.statSync(origem).isDirectory()) throw new Error(`Source data not found: ${origem}`);
  const dir = novoCenarioDir(values.saida, "copia");
  const dados = path.join(dir, "appdata", APP_ID);
  fs.cpSync(origem, dados, { recursive: true });
  // Fold each copied WAL into its database so the copy is a single consistent file.
  for (const e of fs.readdirSync(dados, { recursive: true, withFileTypes: true })) {
    if (!e.isFile() || !e.name.endsWith(".db")) continue;
    const f = path.join(e.parentPath ?? e.path, e.name);
    if (fs.existsSync(`${f}-wal`)) {
      const db = new DatabaseSync(f);
      db.exec("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;");
      db.close();
    }
    fs.rmSync(`${f}-wal`, { force: true });
    fs.rmSync(`${f}-shm`, { force: true });
  }
  if (!values["manter-automacao"]) desligarAutomacao(dados);
  console.log(escreverCenario(dir, "copia", origem));
} catch (e) { falhaDe(e); }
