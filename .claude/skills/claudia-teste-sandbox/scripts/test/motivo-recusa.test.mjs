// Pure string tests: no disk access, no folder creation. Windows path syntax (the skill targets Windows).
import test from "node:test";
import assert from "node:assert/strict";
import { motivoRecusa, podeLimparDeploy } from "../comum.mjs";

const TEMP = "Z:\\tmp-inventado";
const BASE = `${TEMP}\\claudia-teste-sandbox`;
const REPO = "Z:\\repo\\x";
const ctx = { repoRoot: REPO, tempRoot: TEMP };
// A repo that (hypothetically) lives inside the allowed area, to isolate the repo rules from the allow-list.
const REPO_DENTRO = `${BASE}\\a\\repo`;
const ctxDentro = { repoRoot: REPO_DENTRO, tempRoot: TEMP };

test("accepts a folder inside the sandbox base", () => {
  assert.equal(motivoRecusa(`${BASE}\\novo`, ctx), null);
  assert.equal(motivoRecusa(`${BASE}\\novo\\fundo\\mais`, ctx), null);
});

test("accepts with different letter case on Windows", () => {
  assert.equal(motivoRecusa(`z:\\TMP-INVENTADO\\CLAUDIA-TESTE-SANDBOX\\Novo`, ctx), null);
});

test("refuses the sandbox base folder itself", () => {
  assert.match(motivoRecusa(BASE, ctx), /base folder itself/);
  assert.match(motivoRecusa(BASE.toUpperCase(), ctx), /base folder itself/);
});

test("refuses a disk root", () => {
  assert.match(motivoRecusa("Z:\\", ctx), /root/);
});

test("refuses the repo, its parent and its descendants", () => {
  assert.match(motivoRecusa(REPO, ctx), /repository/);
  assert.match(motivoRecusa("Z:\\repo", ctx), /repository/);
  assert.match(motivoRecusa(`${REPO}\\sub`, ctx), /repository/);
});

test("repo rules hold even when the repo is inside the allowed area", () => {
  assert.match(motivoRecusa(REPO_DENTRO, ctxDentro), /repository or an ancestor/);
  assert.match(motivoRecusa(`${BASE}\\a`, ctxDentro), /repository or an ancestor/);
  assert.match(motivoRecusa(`${REPO_DENTRO}\\sub`, ctxDentro), /inside the repository/);
});

test("refuses relative, empty and non-string input", () => {
  assert.match(motivoRecusa("relativo\\pasta", ctx), /relative/);
  assert.match(motivoRecusa(".\\x", ctx), /relative/);
  assert.match(motivoRecusa("", ctx), /empty/);
  assert.match(motivoRecusa("   ", ctx), /empty/);
  assert.match(motivoRecusa(undefined, ctx), /empty/);
});

test("refuses UNC and extended-length paths", () => {
  assert.match(motivoRecusa("\\\\?\\Z:\\tmp-inventado\\claudia-teste-sandbox\\x", ctx), /UNC/);
  assert.match(motivoRecusa("\\\\servidor\\share\\x", ctx), /UNC/);
});

test("refuses '..' segments and non-normalized paths", () => {
  assert.match(motivoRecusa(`${BASE}\\a\\..\\..\\..`, ctx), /\.\./);
  assert.match(motivoRecusa(`${BASE}\\a\\.\\b`, ctx), /not normalized/);
  assert.match(motivoRecusa(`${BASE}\\\\a`, ctx), /not normalized/);
});

test("refuses folders outside the sandbox base", () => {
  assert.match(motivoRecusa("Z:\\outro\\lugar", ctx), /outside/);
  assert.match(motivoRecusa(TEMP, ctx), /outside/);
  assert.match(motivoRecusa(`${TEMP}\\claudia-teste-sandbox-irmao\\x`, ctx), /outside/);
});

test("refuses when repoRoot or tempRoot is missing", () => {
  assert.match(motivoRecusa(`${BASE}\\x`, { repoRoot: "", tempRoot: TEMP }), /required/);
  assert.match(motivoRecusa(`${BASE}\\x`, { repoRoot: REPO, tempRoot: "" }), /required/);
});

test("podeLimparDeploy: only an absent deploy or one with the marker, never a link", () => {
  assert.equal(podeLimparDeploy({ existe: false, temMarcador: false, ehLink: false }), null);
  assert.equal(podeLimparDeploy({ existe: true, temMarcador: true, ehLink: false }), null);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: false, ehLink: false }), /marker/);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: true, ehLink: true }), /link/);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: false, ehLink: true }), /link/);
});
