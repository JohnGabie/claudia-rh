import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

test("cenario mock", () => {
  const base = tempDir("mock");
  try {
    const saida = path.join(base, "out dir");
    const r = rodar(path.join(SCRIPTS, "cenario-mock.mjs"), ["--saida", saida]);
    assert.equal(r.code, 0, r.err);
    assert.equal(JSON.parse(r.ultima).cenario, "mock");
    const dados = path.join(saida, "appdata", "io.github.johngabie.claudia-rh");
    for (const f of ["candidate_base.yaml", "search_variants.yaml", "strategy.md", "claudia_rh.db", "welcome_seen", "disparo.json", "modo_autonomo.json"]) {
      assert.ok(fs.existsSync(path.join(dados, f)), `mock tem ${f}`);
    }
    const db = new DatabaseSync(path.join(dados, "claudia_rh.db"), { readOnly: true });
    assert.equal(db.prepare("SELECT count(*) AS n FROM vagas").get().n, 3);
    db.close();
    const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
    assert.equal(d.ativo, false, "mock disparo desligado");
    assert.equal(typeof d.limiar_minutos, "number", "mock disparo completo");
    const y = fs.readFileSync(path.join(dados, "candidate_base.yaml"), "utf8");
    for (const k of ["dados_pessoais", "experiencia", "projetos", "formacao", "competencias", "idiomas", "fontes_usadas", "gaps_conhecidos", "respostas_modelo", "ultima_atualizacao"]) {
      assert.match(y, new RegExp(`^${k}:`, "m"), `mock yaml tem ${k}`);
    }
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});
