// Clean-machine scenario "copia": copy of the user's real app data. Reads the source only.
// Requires --autorizado: ask the user for permission for this round before running it.
// Usage: node cenario-copia.mjs --autorizado [--origem <folder>] [--saida <folder>] [--manter-automacao]
//   --origem            default %APPDATA%\io.github.johngabie.claudia-rh
//   --saida             default %TEMP%\claudia-sandbox\copia, recreated on every run
//   --manter-automacao  keep disparo.json / modo_autonomo.json as copied (default: turned off in the copy)
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { parseArgs } from "node:util";
import { DatabaseSync } from "node:sqlite";
import { APP_ID, novoCenarioDir, escreverCenario, desligarAutomacao, dentroDe, falhaDe, sair } from "./comum.mjs";

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

  // Resolve saida the same way novoCenarioDir does.
  const saidaPath = path.resolve(values.saida || path.join(process.env.TEMP ?? os.tmpdir(), "claudia-sandbox", "copia"));

  // Check for overlap: --saida must not be inside --origem or vice versa.
  if (dentroDe(origem, saidaPath) || dentroDe(saidaPath, origem)) {
    throw new Error(`--saida must not overlap --origem: origem=${origem}, saida=${saidaPath}`);
  }

  // Call novoCenarioDir once before retry loop (guards apply, errors exit 1 immediately).
  const dir = novoCenarioDir(values.saida, "copia");
  const dados = path.join(dir, "appdata", APP_ID);

  // Copy with retries to handle transient state from live databases.
  let lastError;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      fs.cpSync(origem, dados, { recursive: true, dereference: true });

      // Fold each copied database: checkpoint WAL, change journal mode.
      for (const e of fs.readdirSync(dados, { recursive: true, withFileTypes: true })) {
        if (!e.isFile() || !e.name.endsWith(".db")) continue;
        const f = path.join(e.parentPath ?? e.path, e.name);
        let db;
        try {
          db = new DatabaseSync(f);
          db.exec("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;");
          db.close();
        } catch (err) {
          if (db) try { db.close(); } catch {}
          throw err;
        }
      }

      // Verify integrity of all databases.
      for (const e of fs.readdirSync(dados, { recursive: true, withFileTypes: true })) {
        if (!e.isFile() || !e.name.endsWith(".db")) continue;
        const f = path.join(e.parentPath ?? e.path, e.name);
        let db;
        try {
          db = new DatabaseSync(f, { readOnly: true });
          const result = db.prepare("PRAGMA integrity_check").get();
          if (db) db.close();
          if (result.integrity_check !== "ok") {
            throw new Error(`integrity_check failed: ${f}`);
          }
        } catch (err) {
          if (db) try { db.close(); } catch {}
          throw new Error(`copied database ${f}: ${err.message}`);
        }
      }

      // Cleanup -wal/-shm files after successful integrity check.
      for (const e of fs.readdirSync(dados, { recursive: true, withFileTypes: true })) {
        if (!e.isFile() || !e.name.endsWith(".db")) continue;
        const f = path.join(e.parentPath ?? e.path, e.name);
        fs.rmSync(`${f}-wal`, { force: true });
        fs.rmSync(`${f}-shm`, { force: true });
      }

      if (!values["manter-automacao"]) desligarAutomacao(dados);
      console.log(escreverCenario(dir, "copia", origem));
      process.exit(0);
    } catch (err) {
      lastError = err;
      if (attempt < 3) {
        try {
          fs.rmSync(dados, { recursive: true, force: true });
        } catch {}
      }
    }
  }

  throw new Error(`copied database failed integrity_check after 3 attempts: close the app and retry`);
} catch (e) { falhaDe(e); }
