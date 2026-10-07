// Clean-machine scenario "copia": copy of the user's real app data. Reads the source only.
// Requires --autorizado: ask the user for permission for this round before running it.
// Usage: node cenario-copia.mjs --autorizado [--origem <folder>] [--saida <folder>] [--manter-automacao]
// Runs only inside the guest (D1); from the PC use cenario-no-guest.mjs.
//   --origem            default C:\labdadosreais (mounted read-only by test-up -DadosReais)
//   --saida             default Z:\claudia-teste-sandbox\copia, recreated on every run
//   --manter-automacao  keep disparo.json / modo_autonomo.json as copied (default: turned off in the copy)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { DatabaseSync } from "node:sqlite";
import { filtroSemLinks } from "./copia-filtro.mjs";
import { APP_ID, ORIGEM_DADOS_REAIS, raizCenariosAtual, saidaPadraoCenario, novoCenarioDir, escreverCenario, desligarAutomacao, dentroDe, falhaDe, sair } from "./comum.mjs";

let values;
try {
  ({ values } = parseArgs({ options: {
    autorizado: { type: "boolean" }, origem: { type: "string" }, saida: { type: "string" }, "manter-automacao": { type: "boolean" },
  } }));
} catch (e) { falhaDe(e); }
if (!values.autorizado) {
  sair("Scenario 'copia' reads the user's real data: ask the user for permission for this round, then run again with --autorizado.", 2);
}

// A torn copy (source written mid-copy) surfaces as SQLITE_CORRUPT (11) / SQLITE_NOTADB (26): retry those, throw the rest.
function dbCorrompido(err) {
  return err?.errcode === 11 || err?.errcode === 26 || /database disk image is malformed|file is not a database/i.test(err?.message ?? "");
}

// Helper function: perform one copy cycle (cpSync + fold + integrity check).
// Returns { ok: true } on success, or { ok: false, db: <path> } if integrity check fails (does not throw).
// Throws on cpSync errors, database open errors, etc. — those propagate to top-level catch.
function copiarUmaVez(origem, dados) {
  // Never follow a link and never reproduce one: the filter throws on a symlink or junction anywhere in the source.
  fs.cpSync(origem, dados, { recursive: true, dereference: false, filter: filtroSemLinks(origem, fs.lstatSync, fs.statSync) });

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
      if (dbCorrompido(err)) return { ok: false, db: f };
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
        // Integrity check failed: return without throwing so it can be retried.
        return { ok: false, db: f };
      }
    } catch (err) {
      if (db) try { db.close(); } catch {}
      if (dbCorrompido(err)) return { ok: false, db: f };
      throw err;
    }
  }

  // Cleanup -wal/-shm files after successful integrity check.
  for (const e of fs.readdirSync(dados, { recursive: true, withFileTypes: true })) {
    if (!e.isFile() || !e.name.endsWith(".db")) continue;
    const f = path.join(e.parentPath ?? e.path, e.name);
    fs.rmSync(`${f}-wal`, { force: true });
    fs.rmSync(`${f}-shm`, { force: true });
  }

  return { ok: true };
}

try {
  const tempRoot = raizCenariosAtual(); // throws on the PC before anything is read (D1)
  const origem = path.resolve(values.origem || ORIGEM_DADOS_REAIS);
  if (!fs.existsSync(origem) || !fs.statSync(origem).isDirectory()) {
    throw new Error(`Source data not found: ${origem}. The real data is mounted only by test-up -DadosReais, with the user's authorization for this round.`);
  }

  // Same default folder novoCenarioDir uses.
  const saidaPath = path.resolve(values.saida || saidaPadraoCenario("copia", tempRoot));

  // Check for overlap: --saida must not be inside --origem or vice versa.
  if (dentroDe(origem, saidaPath) || dentroDe(saidaPath, origem)) {
    throw new Error(`--saida must not overlap --origem: origem=${origem}, saida=${saidaPath}`);
  }

  // Call novoCenarioDir once before retry loop (guards apply, errors exit 1 immediately).
  const dir = novoCenarioDir(values.saida, "copia");
  const dados = path.join(dir, "appdata", APP_ID);

  // Copy with retries only for integrity failures.
  let lastFailedDb;
  for (let attempt = 1; attempt <= 3; attempt++) {
    const result = copiarUmaVez(origem, dados);
    if (result.ok) {
      // Successful copy: run post-processing outside the retry loop.
      if (!values["manter-automacao"]) desligarAutomacao(dados);
      console.log(escreverCenario(dir, "copia", origem));
      process.exit(0);
    }
    // Integrity check failed: remember the db path for retry.
    lastFailedDb = result.db;
    if (attempt < 3) {
      try {
        fs.rmSync(dados, { recursive: true, force: true });
      } catch {}
    }
  }

  // After 3 failed attempts:
  throw new Error(`copied database ${lastFailedDb} failed integrity_check after 3 attempts: close the app and retry`);
} catch (e) { falhaDe(e); }
