// Clean-machine scenario "mock": fictional profile and a database built from the repo schema plus a seed.
// Usage: node cenario-mock.mjs [--saida <folder>]   (default %TEMP%\claudia-sandbox\mock, recreated on every run)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { DatabaseSync } from "node:sqlite";
import { APP_ID, REPO_ROOT, novoCenarioDir, escreverCenario, desligarAutomacao, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { saida: { type: "string" } } });
  const dir = novoCenarioDir(values.saida, "mock");
  const dados = path.join(dir, "appdata", APP_ID);
  const fx = path.join(import.meta.dirname, "..", "fixtures");
  for (const f of ["candidate_base.yaml", "search_variants.yaml", "strategy.md"]) {
    fs.copyFileSync(path.join(fx, f), path.join(dados, f));
  }
  const db = new DatabaseSync(path.join(dados, "claudia_rh.db"));
  db.exec(fs.readFileSync(path.join(REPO_ROOT, "src-tauri", "src", "db", "schema.sql"), "utf8"));
  db.exec(fs.readFileSync(path.join(fx, "seed.sql"), "utf8"));
  db.close();
  fs.writeFileSync(path.join(dados, "welcome_seen"), "1");
  desligarAutomacao(dados);
  console.log(escreverCenario(dir, "mock", "fixtures"));
} catch (e) { falhaDe(e); }
