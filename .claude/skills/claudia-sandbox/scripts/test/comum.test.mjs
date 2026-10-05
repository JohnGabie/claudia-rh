import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { APP_ID, REPO_ROOT, novoCenarioDir, escreverCenario, desligarAutomacao, labDir, dentroDe, erroDeExec, erroBancoAusente } from "../comum.mjs";
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

test("novoCenarioDir: recreates folder with .cenario marker (failed run recovery)", () => {
  const temp = tempDir("marker");
  try {
    const target = path.join(temp, "with-marker");
    // Simulate a failed run that left .cenario marker
    fs.mkdirSync(path.join(target, "appdata", APP_ID), { recursive: true });
    fs.writeFileSync(path.join(target, ".cenario"), "");
    fs.writeFileSync(path.join(target, "garbage.txt"), "leftover from failed run");

    // Second run should succeed and clean up the garbage
    const dir = novoCenarioDir(target, "t");
    assert.ok(fs.existsSync(path.join(dir, "appdata", APP_ID)));
    assert.ok(fs.existsSync(path.join(dir, ".cenario")), "marker recreated");
    assert.equal(fs.existsSync(path.join(dir, "garbage.txt")), false, "garbage cleaned up");
  } finally {
    cleanup(temp);
  }
});

test("novoCenarioDir: sibling path is no longer refused as ancestor", () => {
  // A sibling like "<repo_parent>/<repo_name_prefix>" should not be refused
  // dentroDe-based ancestor check works correctly for non-ancestry cases
  const repoParent = path.dirname(REPO_ROOT);
  const testDir = path.join(repoParent, "test-sibling-scenario");

  assert.equal(dentroDe(testDir, REPO_ROOT), false, "sibling is not ancestor of REPO_ROOT");
  assert.equal(dentroDe(REPO_ROOT, testDir), false, "REPO_ROOT is not ancestor of sibling");
});

test("erroDeExec carries the guest exit status", () => {
  assert.equal(erroDeExec({ status: 3, stderr: "" }, "id", "ExistingLogin").status, 3);
});

test("erroBancoAusente: status 3 becomes the 'start the app first' error, others pass through", () => {
  const e3 = erroDeExec({ status: 3, stderr: "" }, "id", "ExistingLogin");
  const m = erroBancoAusente(e3, "GUEST/claudia_rh.db");
  assert.equal(m.message, "the app database does not exist in the guest yet (GUEST/claudia_rh.db): start the app first");
  const e1 = erroDeExec({ status: 1, stderr: "boom" }, "id", "ExistingLogin");
  assert.equal(erroBancoAusente(e1, "x"), e1);
  const outro = new Error("x");
  assert.equal(erroBancoAusente(outro, "x"), outro);
});

test("erroDeExec reads the guest exit code from the localized stdout line", () => {
  const ok = erroDeExec({ status: 0, stdout: "Processo encerrado com o código: 0\r\n" }, "id", "System");
  assert.equal(ok, null);
  const pt = erroDeExec({ status: 0, stdout: "x\r\nProcesso encerrado com o código: 3\r\n" }, "id", "System");
  assert.equal(pt.status, 3);
  assert.equal(pt.code, "ERR_EXEC_FAILED");
  assert.match(pt.message, /código: 3/);
  const en = erroDeExec({ status: 0, stdout: "Process exited with code: 1" }, "id", "System");
  assert.equal(en.status, 1);
  assert.equal(erroDeExec({ status: 0, stdout: "nothing useful here" }, "id", "System"), null);
  assert.equal(erroDeExec({ status: 0, stdout: "" }, "id", "System"), null);
  assert.equal(erroDeExec({ status: 1, stdout: "Process exited with code: 0" }, "id", "System").status, 1);
  // last match wins
  assert.equal(erroDeExec({ status: 0, stdout: "exit code: 5\nProcess exited with code: 0" }, "id", "System"), null);
});

test("erroBancoAusente works with a guest code parsed from stdout", () => {
  const e = erroDeExec({ status: 0, stdout: "Processo encerrado com o código: 3" }, "id", "ExistingLogin");
  assert.match(erroBancoAusente(e, "G").message, /does not exist in the guest yet/);
});
