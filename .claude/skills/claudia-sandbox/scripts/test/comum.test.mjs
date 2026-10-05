import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { APP_ID, novoCenarioDir, escreverCenario, desligarAutomacao, labDir } from "../comum.mjs";
import { tempDir } from "./util.mjs";

test("saida recriada", () => {
  const saida = path.join(tempDir("comum"), "saida x");
  fs.mkdirSync(path.join(saida, "lixo"), { recursive: true });
  const dir = novoCenarioDir(saida, "t");
  assert.equal(fs.existsSync(path.join(dir, "lixo")), false);
  assert.ok(fs.existsSync(path.join(dir, "appdata", APP_ID)));
  assert.ok(fs.existsSync(path.join(dir, "localappdata")));
});

test("cenario.json gravado e devolvido", () => {
  const dir = novoCenarioDir(path.join(tempDir("comum"), "c"), "t");
  const devolvido = JSON.parse(escreverCenario(dir, "t", "origem-x"));
  const gravado = JSON.parse(fs.readFileSync(path.join(dir, "cenario.json"), "utf8"));
  assert.equal(devolvido.cenario, "t");
  assert.equal(gravado.origem, "origem-x");
  assert.ok(gravado.criadoEm);
  assert.equal(JSON.parse(escreverCenario(dir, "t")).origem, null);
});

test("automacao completa a partir de arquivo parcial", () => {
  const dir = novoCenarioDir(path.join(tempDir("comum"), "a"), "t");
  const dados = path.join(dir, "appdata", APP_ID);
  fs.writeFileSync(path.join(dados, "disparo.json"), '{"ativo":true,"limite_diario":3}');
  desligarAutomacao(dados);
  const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
  assert.deepEqual(d, { ativo: false, limiar_minutos: 15, limite_diario: 3, limite_tempo_minutos: 0, janelas: [] });
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(dados, "modo_autonomo.json"), "utf8")), { skip_permissions: false });
});

test("lab ausente ou incompleto nomeia o que falta", () => {
  const antigo = process.env.SANDBOX_LAB_DIR;
  delete process.env.SANDBOX_LAB_DIR;
  assert.throws(() => labDir(), /SANDBOX_LAB_DIR/);
  process.env.SANDBOX_LAB_DIR = tempDir("lab");
  assert.throws(() => labDir(), /app-stage\.ps1/);
  if (antigo === undefined) delete process.env.SANDBOX_LAB_DIR; else process.env.SANDBOX_LAB_DIR = antigo;
});
