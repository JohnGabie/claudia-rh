import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { DatabaseSync } from "node:sqlite";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

const SCRIPT = path.join(SCRIPTS, "cenario-copia.mjs");
const APP = "io.github.johngabie.claudia-rh";

function hashArvore(raiz) {
  return fs.readdirSync(raiz, { recursive: true, withFileTypes: true }).filter((e) => e.isFile())
    .map((e) => path.join(e.parentPath ?? e.path, e.name)).sort()
    .map((f) => `${path.relative(raiz, f)}:${crypto.createHash("sha256").update(fs.readFileSync(f)).digest("hex")}`).join(",");
}

test("cenario copia", () => {
  const base = tempDir("copia");
  const origem = path.join(base, "origem dados");
  fs.mkdirSync(path.join(origem, "sub"), { recursive: true });
  fs.writeFileSync(path.join(origem, "candidate_base.yaml"), "dados_pessoais: {}\n");
  fs.writeFileSync(path.join(origem, "sub", "nota.txt"), "x");
  fs.writeFileSync(path.join(origem, "disparo.json"), '{"ativo":true,"limiar_minutos":5,"limite_diario":7,"limite_tempo_minutos":0,"janelas":[]}');
  // The last row lives only in the -wal file while this connection stays open (as with the app running).
  const vivo = new DatabaseSync(path.join(origem, "claudia_rh.db"));
  vivo.exec("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE t(x); INSERT INTO t VALUES (42);");
  assert.ok(fs.existsSync(path.join(origem, "claudia_rh.db-wal")), "fixture tem wal");
  const antes = hashArvore(origem);

  const sem = rodar(SCRIPT, ["--origem", origem, "--saida", path.join(base, "sem")]);
  assert.equal(sem.code, 2, "copia sem autorizacao sai com 2");
  assert.equal(fs.existsSync(path.join(base, "sem", "appdata")), false, "copia sem autorizacao nao copia nada");

  const saida = path.join(base, "out dir");
  const r = rodar(SCRIPT, ["--autorizado", "--origem", origem, "--saida", saida]);
  assert.equal(r.code, 0, r.err);
  const dados = path.join(saida, "appdata", APP);
  assert.ok(fs.existsSync(path.join(dados, "sub", "nota.txt")), "copia recursiva");
  const db = new DatabaseSync(path.join(dados, "claudia_rh.db"), { readOnly: true });
  assert.equal(db.prepare("SELECT x FROM t").get().x, 42, "copia consolida wal");
  db.close();
  assert.equal(fs.existsSync(path.join(dados, "claudia_rh.db-wal")), false, "copia sem arquivo wal");
  assert.equal(fs.existsSync(path.join(dados, "claudia_rh.db-shm")), false, "copia sem arquivo shm");
  const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
  assert.equal(d.ativo, false, "copia desliga automacao");
  assert.equal(d.limiar_minutos, 5, "copia preserva campos");
  assert.equal(hashArvore(origem), antes, "origem intacta");
  assert.deepEqual(fs.readdirSync(path.join(saida, "localappdata")), [], "localappdata nao copiado");

  rodar(SCRIPT, ["--autorizado", "--manter-automacao", "--origem", origem, "--saida", saida]);
  assert.equal(JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8")).ativo, true, "manter-automacao preserva disparo");

  const ausente = rodar(SCRIPT, ["--autorizado", "--origem", path.join(base, "nao existe"), "--saida", saida]);
  assert.equal(ausente.code, 1);
  assert.match(ausente.err, /nao existe/, "origem ausente nomeia o caminho");

  // Minor: assert exit 0 for --manter-automacao run
  const mantAuto = rodar(SCRIPT, ["--autorizado", "--manter-automacao", "--origem", origem, "--saida", saida]);
  assert.equal(mantAuto.code, 0, "manter-automacao run exits 0");

  // Minor: assert stderr matches for authorization check
  const semAuto = rodar(SCRIPT, ["--origem", origem, "--saida", path.join(base, "sem-auto")]);
  assert.match(semAuto.err, /--autorizado/, "stderr mentions --autorizado when missing");

  vivo.close();
});

test("cenario copia: saida dentro de origem", () => {
  const base = tempDir("copia-overlap");
  const origem = path.join(base, "origem");
  fs.mkdirSync(origem, { recursive: true });
  fs.writeFileSync(path.join(origem, "file.txt"), "data");
  const antesOrigem = hashArvore(origem);

  // Test: --saida is a subdirectory of --origem
  const saidaDentro = path.join(origem, "x");
  const r = rodar(SCRIPT, ["--autorizado", "--origem", origem, "--saida", saidaDentro]);
  assert.equal(r.code, 1, "saida dentro de origem sai com 1");
  assert.match(r.err, /overlap/, "erro menciona overlap");
  assert.equal(fs.existsSync(saidaDentro), false, "saida nao criada");
  assert.equal(hashArvore(origem), antesOrigem, "origem intacta");
});

test("cenario copia: database corrompido", () => {
  const base = tempDir("copia-corrupt");
  const origem = path.join(base, "origem");
  fs.mkdirSync(origem, { recursive: true });
  fs.writeFileSync(path.join(origem, "candidate_base.yaml"), "dados: {}\n");
  // Write garbage instead of valid SQLite database
  fs.writeFileSync(path.join(origem, "claudia_rh.db"), "not a database");

  const saida = path.join(base, "saida");
  const r = rodar(SCRIPT, ["--autorizado", "--origem", origem, "--saida", saida]);
  assert.equal(r.code, 1, "db corrompido sai com 1");
  assert.match(r.err, /integrity|database|claudia_rh\.db/, "erro menciona db ou integrity");
});

test("cenario copia: saida folder guard (non-empty without cenario.json)", () => {
  const base = tempDir("copia-guard");
  const origem = path.join(base, "origem");
  fs.mkdirSync(origem, { recursive: true });
  fs.writeFileSync(path.join(origem, "importante.txt"), "critical data");
  const antes = hashArvore(origem);

  // Create a non-empty directory without cenario.json to be used as --saida
  const saidaDir = path.join(base, "saida folder");
  fs.mkdirSync(saidaDir, { recursive: true });
  fs.writeFileSync(path.join(saidaDir, "importante.txt"), "critical data");
  const antesDir = hashArvore(saidaDir);

  const r = rodar(SCRIPT, ["--autorizado", "--origem", origem, "--saida", saidaDir]);
  assert.equal(r.code, 1, "folder guard sai com 1");
  assert.match(r.err, /must not have existing files|cenario.json/, "erro menciona guard");
  assert.equal(hashArvore(origem), antes, "origem intacta");
  assert.equal(hashArvore(saidaDir), antesDir, "saida folder intacta");
});

test("cenario copia: failed run recovery with .cenario marker", () => {
  const base = tempDir("copia-recovery");
  // First run: garbage db (will fail)
  const origemBad = path.join(base, "origem-bad");
  fs.mkdirSync(origemBad, { recursive: true });
  fs.writeFileSync(path.join(origemBad, "candidate_base.yaml"), "dados: {}\n");
  fs.writeFileSync(path.join(origemBad, "claudia_rh.db"), "not a database");

  const saida = path.join(base, "saida");
  const r1 = rodar(SCRIPT, ["--autorizado", "--origem", origemBad, "--saida", saida]);
  assert.equal(r1.code, 1, "primeira tentativa com db ruim falha");
  // The .cenario marker should now exist, allowing the second run
  assert.ok(fs.existsSync(path.join(saida, ".cenario")), "marker exists after failed run");

  // Second run: good data to same --saida (should succeed because of .cenario marker)
  const origemGood = path.join(base, "origem-good");
  fs.mkdirSync(origemGood, { recursive: true });
  fs.writeFileSync(path.join(origemGood, "candidate_base.yaml"), "dados: {}\n");
  const db = new DatabaseSync(path.join(origemGood, "claudia_rh.db"));
  db.exec("PRAGMA journal_mode=WAL; CREATE TABLE t(x); INSERT INTO t VALUES (42);");
  db.close();

  const r2 = rodar(SCRIPT, ["--autorizado", "--origem", origemGood, "--saida", saida]);
  assert.equal(r2.code, 0, "segunda tentativa com dados bons sucede");
  const dados = path.join(saida, "appdata", "io.github.johngabie.claudia-rh");
  const dbCopy = new DatabaseSync(path.join(dados, "claudia_rh.db"), { readOnly: true });
  assert.equal(dbCopy.prepare("SELECT x FROM t").get().x, 42, "dados copiados corretamente");
  dbCopy.close();
});
