// Pure tests: git and the disk check are injected; nothing is spawned or read.
import test from "node:test";
import assert from "node:assert/strict";
import { onde, labDir } from "../comum.mjs";

const MAIN = "Z:\\proj x\\claudia-rh";
const WT = `${MAIN}\\.claude\\worktrees\\agent-x`;
const gitFalso = (topo) => (args) => {
  const k = args.join(" ");
  if (k === "rev-parse --path-format=absolute --git-common-dir") return "Z:/proj x/claudia-rh/.git";
  if (k === "rev-parse --show-toplevel") return topo.replace(/\\/g, "/");
  throw new Error(`unexpected git ${k}`);
};

test("from a worktree: lab and work folder from the main checkout, project = the worktree", () => {
  const o = onde(WT, gitFalso(WT));
  assert.equal(o.raizPrincipal, MAIN);
  assert.equal(o.projeto, WT);
  assert.equal(o.lab, `${MAIN}\\sandbox-lab`);
  assert.equal(o.trabalho, `${MAIN}\\.sandbox-work`);
});

test("from the main checkout: project = the main checkout", () => {
  const o = onde(MAIN, gitFalso(MAIN));
  assert.equal(o.projeto, MAIN);
  assert.equal(o.raizPrincipal, MAIN);
});

test("labDir names the missing app-stage.ps1 and ignores SANDBOX_LAB_DIR", () => {
  const antigo = process.env.SANDBOX_LAB_DIR;
  process.env.SANDBOX_LAB_DIR = "Z:\\outro-lab";
  try {
    assert.throws(() => labDir({ cwd: WT, git: gitFalso(WT), existe: () => false }), /sandbox-lab\\scripts\\app-stage\.ps1/);
    const alvo = `${MAIN}\\sandbox-lab\\scripts\\app-stage.ps1`;
    assert.equal(labDir({ cwd: WT, git: gitFalso(WT), existe: (p) => p === alvo }), `${MAIN}\\sandbox-lab`);
  } finally {
    if (antigo === undefined) delete process.env.SANDBOX_LAB_DIR; else process.env.SANDBOX_LAB_DIR = antigo;
  }
});
