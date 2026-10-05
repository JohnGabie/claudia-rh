import test from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { execFileSync } from "node:child_process";
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
