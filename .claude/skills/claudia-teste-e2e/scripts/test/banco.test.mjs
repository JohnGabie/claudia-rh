import test from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

test("banco.mjs lê o banco apontado por CLAUDIA_DB", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "claudia app "));
  const db = path.join(dir, "x.db");
  const d = new DatabaseSync(db); d.exec("CREATE TABLE t(v); INSERT INTO t VALUES (7);"); d.close();
  const out = execFileSync(process.execPath, [path.join(import.meta.dirname, "..", "banco.mjs"), "SELECT v FROM t"],
    { env: { ...process.env, CLAUDIA_DB: db } }).toString();
  assert.equal(JSON.parse(out)[0].v, 7);
  fs.rmSync(dir, { recursive: true, force: true });
});

const BANCO = path.join(import.meta.dirname, "..", "banco.mjs");
const OBSERVAR = path.join(import.meta.dirname, "..", "observar.mjs");

function rodarSemDb(script, args) {
  // Fake APPDATA so a regression can never reach the real app folder; 192.0.2.1 is the RFC 5737 test address.
  const fake = fs.mkdtempSync(path.join(os.tmpdir(), "claudia fake appdata "));
  const env = { ...process.env, APPDATA: fake, TEMP: fake, CDP_HOST: "192.0.2.1" };
  delete env.CLAUDIA_DB;
  const r = spawnSync(process.execPath, [script, ...args], { env, encoding: "utf8", timeout: 20000 });
  fs.rmSync(fake, { recursive: true, force: true });
  return r;
}

test("banco.mjs recusa alvo remoto sem CLAUDIA_DB", () => {
  const r = rodarSemDb(BANCO, ["SELECT 1"]);
  assert.equal(r.status, 2);
  assert.match(r.stderr, /CLAUDIA_DB/);
  assert.match(r.stderr, /CDP_HOST=192\.0\.2\.1/);
});

test("banco.mjs com alvo remoto e CLAUDIA_DB funciona", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "claudia app "));
  const db = path.join(dir, "x.db");
  const d = new DatabaseSync(db); d.exec("CREATE TABLE t(v); INSERT INTO t VALUES (9);"); d.close();
  const out = execFileSync(process.execPath, [BANCO, "SELECT v FROM t"],
    { env: { ...process.env, CDP_HOST: "192.0.2.1", CLAUDIA_DB: db } }).toString();
  assert.equal(JSON.parse(out)[0].v, 9);
  fs.rmSync(dir, { recursive: true, force: true });
});

test("observar.mjs recusa alvo remoto sem CLAUDIA_DB", () => {
  const r = rodarSemDb(OBSERVAR, []);
  assert.equal(r.status, 2);
  assert.match(r.stderr, /CLAUDIA_DB/);
});
