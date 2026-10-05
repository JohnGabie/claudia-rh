import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { APP_ID, REPO_ROOT, novoCenarioDir, escreverCenario, desligarAutomacao, labDir, dentroDe } from "../comum.mjs";
import { tempDir, SCRIPTS } from "./util.mjs";

function cleanup(dir) {
  if (dir && fs.existsSync(dir)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

test("saida recriada", () => {
  const temp = tempDir("comum");
  try {
    const saida = path.join(temp, "saida x");
    // Create scenario.json first so directory can be safely deleted
    fs.mkdirSync(path.join(saida, "appdata", APP_ID), { recursive: true });
    fs.writeFileSync(path.join(saida, "cenario.json"), "{}");
    fs.mkdirSync(path.join(saida, "lixo"), { recursive: true });
    const dir = novoCenarioDir(saida, "t");
    assert.equal(fs.existsSync(path.join(dir, "lixo")), false);
    assert.ok(fs.existsSync(path.join(dir, "appdata", APP_ID)));
    assert.ok(fs.existsSync(path.join(dir, "localappdata")));
  } finally {
    cleanup(temp);
  }
});

test("cenario.json gravado e devolvido", () => {
  const temp = tempDir("comum");
  try {
    const dir = novoCenarioDir(path.join(temp, "c"), "t");
    const devolvido = JSON.parse(escreverCenario(dir, "t", "origem-x"));
    const gravado = JSON.parse(fs.readFileSync(path.join(dir, "cenario.json"), "utf8"));
    assert.equal(devolvido.cenario, "t");
    assert.equal(gravado.origem, "origem-x");
    assert.ok(gravado.criadoEm);
    assert.equal(JSON.parse(escreverCenario(dir, "t")).origem, null);
  } finally {
    cleanup(temp);
  }
});

test("automacao completa a partir de arquivo parcial", () => {
  const temp = tempDir("comum");
  try {
    const dir = novoCenarioDir(path.join(temp, "a"), "t");
    const dados = path.join(dir, "appdata", APP_ID);
    fs.writeFileSync(path.join(dados, "disparo.json"), '{"ativo":true,"limite_diario":3}');
    desligarAutomacao(dados);
    const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
    assert.deepEqual(d, { ativo: false, limiar_minutos: 15, limite_diario: 3, limite_tempo_minutos: 0, janelas: [] });
    assert.deepEqual(JSON.parse(fs.readFileSync(path.join(dados, "modo_autonomo.json"), "utf8")), { skip_permissions: false });
  } finally {
    cleanup(temp);
  }
});

test("lab ausente ou incompleto nomeia o que falta", () => {
  const antigo = process.env.SANDBOX_LAB_DIR;
  try {
    delete process.env.SANDBOX_LAB_DIR;
    assert.throws(() => labDir(), /SANDBOX_LAB_DIR/);
    const labTemp = tempDir("lab");
    try {
      process.env.SANDBOX_LAB_DIR = labTemp;
      assert.throws(() => labDir(), /app-stage\.ps1/);
    } finally {
      cleanup(labTemp);
    }
  } finally {
    if (antigo === undefined) delete process.env.SANDBOX_LAB_DIR; else process.env.SANDBOX_LAB_DIR = antigo;
  }
});

test("desligarAutomacao: null file treated as defaults", () => {
  const temp = tempDir("comum");
  try {
    const dir = novoCenarioDir(path.join(temp, "n"), "t");
    const dados = path.join(dir, "appdata", APP_ID);
    fs.writeFileSync(path.join(dados, "disparo.json"), "null");
    desligarAutomacao(dados);
    const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
    assert.deepEqual(d, { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, janelas: [] });
  } finally {
    cleanup(temp);
  }
});

test("desligarAutomacao: bad janela and invalid limite_diario use defaults", () => {
  const temp = tempDir("comum");
  try {
    const dir = novoCenarioDir(path.join(temp, "j"), "t");
    const dados = path.join(dir, "appdata", APP_ID);
    fs.writeFileSync(path.join(dados, "disparo.json"), JSON.stringify({
      ativo: true,
      limite_diario: "not-a-number",
      janelas: [{ dia_semana: "invalid" }]
    }));
    desligarAutomacao(dados);
    const d = JSON.parse(fs.readFileSync(path.join(dados, "disparo.json"), "utf8"));
    assert.deepEqual(d, { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, janelas: [] });
  } finally {
    cleanup(temp);
  }
});

test("novoCenarioDir: refuses non-empty dir without cenario.json", () => {
  const temp = tempDir("comum");
  try {
    const target = path.join(temp, "refuse");
    fs.mkdirSync(path.join(target, "stuff"), { recursive: true });
    assert.throws(() => novoCenarioDir(target, "t"), /must not have existing files/);
  } finally {
    cleanup(temp);
  }
});

test("novoCenarioDir: refuses if target is REPO_ROOT", () => {
  assert.throws(() => novoCenarioDir(REPO_ROOT, "t"), /REPO_ROOT/);
});

test("novoCenarioDir: refuses if target is ancestor of REPO_ROOT", () => {
  const parent = path.dirname(REPO_ROOT);
  assert.throws(() => novoCenarioDir(parent, "t"), /ancestor of REPO_ROOT/);
});

test("novoCenarioDir: refuses if target is filesystem root", () => {
  const root = path.parse(REPO_ROOT).root;
  assert.throws(() => novoCenarioDir(root, "t"), /filesystem root/);
});

test("novoCenarioDir: recreates existing scenario folder", () => {
  const temp = tempDir("comum");
  try {
    const target = path.join(temp, "existing");
    // Create first scenario
    let dir = novoCenarioDir(target, "t");
    escreverCenario(dir, "t");
    fs.writeFileSync(path.join(dir, "appdata", APP_ID, "old-file.txt"), "old");
    // Recreate should clean it
    dir = novoCenarioDir(target, "t");
    assert.equal(fs.existsSync(path.join(dir, "appdata", APP_ID, "old-file.txt")), false);
    assert.ok(fs.existsSync(path.join(dir, "appdata", APP_ID)));
    assert.ok(fs.existsSync(path.join(dir, "localappdata")));
  } finally {
    cleanup(temp);
  }
});

test("falhaDe exits 2 for ERR_PARSE_ARGS", () => {
  const tempScript = tempDir("falha");
  const script = path.join(tempScript, "test.mjs");
  try {
    const commonPath = path.resolve(SCRIPTS, "comum.mjs").replace(/\\/g, "/");
    fs.writeFileSync(script, `import { falhaDe } from "file:///${commonPath}";
const e = new Error("bad arg");
e.code = "ERR_PARSE_ARGS";
falhaDe(e);
`);
    const r = spawnSync(process.execPath, [script], { encoding: "utf8" });
    assert.equal(r.status, 2);
  } finally {
    cleanup(tempScript);
  }
});

test("falhaDe exits 1 for other errors", () => {
  const tempScript = tempDir("falha-other");
  const script = path.join(tempScript, "test.mjs");
  try {
    const commonPath = path.resolve(SCRIPTS, "comum.mjs").replace(/\\/g, "/");
    fs.writeFileSync(script, `import { falhaDe } from "file:///${commonPath}";
const e = new Error("other error");
falhaDe(e);
`);
    const r = spawnSync(process.execPath, [script], { encoding: "utf8" });
    assert.equal(r.status, 1);
  } finally {
    cleanup(tempScript);
  }
});

test("dentroDe: equal paths", () => {
  assert.equal(dentroDe("C:\\a", "C:\\a"), true, "same path is inside");
});

test("dentroDe: child path", () => {
  assert.equal(dentroDe("C:\\a", "C:\\a\\b"), true, "child is inside parent");
  assert.equal(dentroDe("C:\\a", "C:\\a\\b\\c"), true, "deep child is inside parent");
});

test("dentroDe: parent path", () => {
  assert.equal(dentroDe("C:\\a\\b", "C:\\a"), false, "parent is not inside child");
});

test("dentroDe: sibling paths", () => {
  assert.equal(dentroDe("C:\\a", "C:\\b"), false, "sibling is not inside");
  assert.equal(dentroDe("C:\\a\\x", "C:\\a\\y"), false, "sibling subdirs are not inside");
});

test("dentroDe: different drives", () => {
  assert.equal(dentroDe("C:\\a", "D:\\b"), false, "different drive is not inside");
});

test("dentroDe: folder named ..foo", () => {
  assert.equal(dentroDe("C:\\a", "C:\\a\\..foo"), true, "folder named ..foo is a child, not parent");
});
