// SANDBOX ONLY: creates folders and junctions. tempDir() refuses to run outside Windows Sandbox.
// Run through: node testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs
// Nothing is cleaned up on purpose: the guest is discarded by test-down.
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";
import { publicar } from "../publicar.mjs";
import { MARCADOR_DEPLOY } from "../caminhos-lab.mjs";

function montar(nome) {
  const base = tempDir(`publicar ${nome}`);
  const raiz = path.join(base, "repo x");
  fs.mkdirSync(path.join(raiz, "sandbox-lab", "scripts"), { recursive: true });
  const app = path.join(base, "app x.exe");
  fs.writeFileSync(app, "x");
  const chamadas = [];
  const deps = { raizPrincipal: raiz, rodarStage: (ps1, args) => { chamadas.push({ ps1, args }); return 0; } };
  const sentinela = (dir) => { fs.mkdirSync(dir, { recursive: true }); const f = path.join(dir, "keep.txt"); fs.writeFileSync(f, "keep"); return f; };
  return { base, raiz, app, chamadas, deps, sentinela, deploy: path.join(raiz, ".sandbox-work", "deploy") };
}

test("first publish: app-stage gets only -App, -WorkDir <root>\\.sandbox-work and -Clean; marker written", () => {
  const m = montar("a");
  publicar({ app: m.app }, m.deps);
  assert.equal(m.chamadas.length, 1);
  assert.equal(m.chamadas[0].ps1, path.join(m.raiz, "sandbox-lab", "scripts", "app-stage.ps1"));
  assert.deepEqual(m.chamadas[0].args, ["-App", m.app, "-WorkDir", path.join(m.raiz, ".sandbox-work"), "-Clean"]);
  assert.ok(fs.existsSync(path.join(m.deploy, MARCADOR_DEPLOY)));
});

test("second publish with the marker is allowed", () => {
  const m = montar("b");
  publicar({ app: m.app }, m.deps);
  publicar({ app: m.app }, m.deps);
  assert.equal(m.chamadas.length, 2);
});

test("deploy without the marker: refused, app-stage not called, files kept", () => {
  const m = montar("c");
  const keep = m.sentinela(m.deploy);
  assert.throws(() => publicar({ app: m.app }, m.deps), /marker/);
  assert.equal(m.chamadas.length, 0);
  assert.ok(fs.existsSync(keep));
});

test("deploy as a junction (even with the marker inside): refused, target kept", () => {
  const m = montar("d");
  const alvo = path.join(m.base, "alvo");
  const keep = m.sentinela(alvo);
  fs.writeFileSync(path.join(alvo, MARCADOR_DEPLOY), "");
  fs.mkdirSync(path.dirname(m.deploy), { recursive: true });
  fs.symlinkSync(alvo, m.deploy, "junction");
  assert.throws(() => publicar({ app: m.app }, m.deps), /link/);
  assert.equal(m.chamadas.length, 0);
  assert.ok(fs.existsSync(keep));
});

test(".sandbox-work as a junction: refused, target kept", () => {
  const m = montar("e");
  const alvo = path.join(m.base, "alvo-work");
  const keep = m.sentinela(path.join(alvo, "deploy"));
  fs.writeFileSync(path.join(alvo, "deploy", MARCADOR_DEPLOY), "");
  fs.symlinkSync(alvo, path.join(m.raiz, ".sandbox-work"), "junction");
  assert.throws(() => publicar({ app: m.app }, m.deps), /link/);
  assert.equal(m.chamadas.length, 0);
  assert.ok(fs.existsSync(keep));
});

test("the old --cenario option is gone", () => {
  assert.equal(rodar(path.join(SCRIPTS, "publicar.mjs"), ["--app", "x", "--cenario", "y"]).code, 2);
});

test("CLI: --work-dir and --cenario no longer exist and a missing --app exits 2", () => {
  const script = path.join(SCRIPTS, "publicar.mjs");
  assert.equal(rodar(script, ["--app", "x", "--cenario", "y", "--work-dir", "z"]).code, 2);
  assert.equal(rodar(script, []).code, 2);
});
