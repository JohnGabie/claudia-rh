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
  vivo.close();
});
