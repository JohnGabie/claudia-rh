import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

test("cenario novo", () => {
  const saida = path.join(tempDir("novo"), "out dir");
  const r = rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--saida", saida]);
  assert.equal(r.code, 0, r.err);
  const j = JSON.parse(r.ultima);
  assert.equal(j.cenario, "novo");
  assert.equal(j.origem, null);
  assert.deepEqual(j.arquivos, []);
  assert.ok(fs.existsSync(path.join(saida, "appdata", "io.github.johngabie.claudia-rh")));
});

test("opcao desconhecida sai com 2", () => {
  assert.equal(rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--xyz"]).code, 2);
});
