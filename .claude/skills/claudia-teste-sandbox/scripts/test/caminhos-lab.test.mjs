// Pure tests: invented Windows paths (Z:\...), injected predicates, no disk. Runs on any OS (path.win32 inside).
import test from "node:test";
import assert from "node:assert/strict";
import {
  raizPrincipalDe, caminhosDoLab, linksAte, podeLimparDeploy, motivoRecusaLimparDeploy, estaNoSandbox, MARCADOR_DEPLOY, MARCADOR_GUEST, ambienteGitLimpo,
  RAIZ_TESTE_GUEST, motivoRecusaRaizTeste, raizDeCenarios,
} from "../caminhos-lab.mjs";

const RAIZ = "Z:\\proj x\\claudia-rh";
const DEPLOY = `${RAIZ}\\.sandbox-work\\deploy`;
const ok = { deploy: DEPLOY, raizPrincipal: RAIZ, existe: true, temMarcador: true, linksNoCaminho: [] };

test("raizPrincipalDe: git's forward slashes, a trailing slash and backslashes", () => {
  assert.equal(raizPrincipalDe("Z:/proj x/claudia-rh/.git"), RAIZ);
  assert.equal(raizPrincipalDe("Z:/proj x/claudia-rh/.git/\n"), RAIZ);
  assert.equal(raizPrincipalDe(`${RAIZ}\\.GIT`), RAIZ);
});

test("raizPrincipalDe: refuses empty, relative, UNC, submodule and drive-root answers", () => {
  assert.throws(() => raizPrincipalDe(""), /empty/);
  assert.throws(() => raizPrincipalDe(".git"), /absolute/);
  assert.throws(() => raizPrincipalDe("\\\\srv\\share\\r\\.git"), /absolute local/);
  assert.throws(() => raizPrincipalDe("Z:/r/.git/modules/sub"), /unexpected/);
  assert.throws(() => raizPrincipalDe("Z:/.git"), /drive root/);
});

test("caminhosDoLab: everything under the main checkout", () => {
  assert.deepEqual(caminhosDoLab(RAIZ), {
    lab: `${RAIZ}\\sandbox-lab`,
    trabalho: `${RAIZ}\\.sandbox-work`,
    deploy: DEPLOY,
    guestRw: `${RAIZ}\\.sandbox-work\\guest-rw`,
    appStage: `${RAIZ}\\sandbox-lab\\scripts\\app-stage.ps1`,
    testUp: `${RAIZ}\\sandbox-lab\\scripts\\test-up.ps1`,
  });
});

test("linksAte: checks deploy and .sandbox-work, stops before the root", () => {
  const vistos = [];
  const r = linksAte(DEPLOY, RAIZ, (p) => { vistos.push(p); return p.endsWith(".sandbox-work"); });
  assert.deepEqual(vistos, [DEPLOY, `${RAIZ}\\.sandbox-work`]);
  assert.deepEqual(r, [`${RAIZ}\\.sandbox-work`]);
  assert.throws(() => linksAte("Z:\\outro\\deploy", RAIZ, () => false), /not inside/);
});

test("motivoRecusaLimparDeploy: the exact deploy with the marker or absent is allowed", () => {
  assert.equal(motivoRecusaLimparDeploy(ok), null);
  assert.equal(motivoRecusaLimparDeploy({ ...ok, existe: false, temMarcador: false }), null);
  assert.equal(motivoRecusaLimparDeploy({ ...ok, deploy: DEPLOY.toUpperCase() }), null);
});

test("motivoRecusaLimparDeploy: any other path is refused", () => {
  for (const deploy of [
    `${RAIZ}\\.sandbox-work`, `${RAIZ}\\.sandbox-work\\deploy2`, `${RAIZ}\\.sandbox-work\\deploy\\app`,
    `${RAIZ}\\.sandbox-work\\deploy\\..\\..`, `${RAIZ}\\.sandbox-work\\deploy.`, DEPLOY.replace(/\\/g, "/"),
    "Z:\\", "Z:\\outro\\.sandbox-work\\deploy", "relativo\\deploy",
  ]) assert.match(motivoRecusaLimparDeploy({ ...ok, deploy }), /not exactly|required/, deploy);
});

test("motivoRecusaLimparDeploy: bad roots are refused", () => {
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "" }), /required/);
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "claudia-rh" }), /absolute/);
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "\\\\srv\\share" }), /absolute/);
  assert.match(motivoRecusaLimparDeploy({ ...ok, raizPrincipal: "Z:\\", deploy: "Z:\\.sandbox-work\\deploy" }), /drive root/);
});

test("motivoRecusaLimparDeploy: a link in the path wins over the marker", () => {
  assert.match(motivoRecusaLimparDeploy({ ...ok, linksNoCaminho: [`${RAIZ}\\.sandbox-work`] }), /link/);
});

test("motivoRecusaLimparDeploy: an existing deploy without the marker is refused", () => {
  assert.match(motivoRecusaLimparDeploy({ ...ok, temMarcador: false }), new RegExp(MARCADOR_DEPLOY.replace(".", "\\.")));
});

test("podeLimparDeploy keeps its old contract", () => {
  assert.equal(podeLimparDeploy({ existe: false, temMarcador: false, ehLink: false }), null);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: true, ehLink: true }), /link/);
  assert.match(podeLimparDeploy({ existe: true, temMarcador: false, ehLink: false }), /marker/);
});

test("MARCADOR_GUEST keeps the literal that travas.ps1 pins ($script:LabGuestMarker)", () => {
  assert.equal(MARCADOR_GUEST, "C:\\labsandbox\\bootstrap.ps1");
});

test("estaNoSandbox needs BOTH the Sandbox identity and the guest-only folder", () => {
  const WD = "WDAGUtilityAccount";
  assert.equal(estaNoSandbox({ identidade: WD, temBootstrap: true }), true);
  assert.equal(estaNoSandbox({ identidade: `PC-NOME\\${WD}`, temBootstrap: true }), true);
  assert.equal(estaNoSandbox({ identidade: "wdagutilityaccount", temBootstrap: true }), true);
  // identity alone (a spoofed user name) is not enough
  assert.equal(estaNoSandbox({ identidade: WD, temBootstrap: false }), false);
  // the guest folder alone is not enough
  assert.equal(estaNoSandbox({ identidade: "PC-NOME\\fulano", temBootstrap: true }), false);
  for (const u of ["", undefined, null, "fulano", `${WD}2`, `x${WD}`, "Administrator", `${WD}\\fulano`]) {
    assert.equal(estaNoSandbox({ identidade: u, temBootstrap: true }), false, String(u));
  }
  assert.equal(estaNoSandbox({ identidade: WD }), false);
  assert.equal(estaNoSandbox(), false);
  assert.equal(estaNoSandbox(WD), false);
});

test("ambienteGitLimpo drops GIT_DIR, GIT_COMMON_DIR and GIT_WORK_TREE and keeps the rest", () => {
  const env = { PATH: "p", GIT_DIR: "x", git_common_dir: "y", GIT_WORK_TREE: "z", GIT_INDEX_FILE: "i", HOME: "h" };
  const limpo = ambienteGitLimpo(env);
  assert.deepEqual(Object.keys(limpo).sort(), ["GIT_INDEX_FILE", "HOME", "PATH"]);
  assert.equal(env.GIT_DIR, "x", "input is not mutated");
  assert.deepEqual(ambienteGitLimpo(undefined), {});
});

test("MARCADOR_GUEST is the bootstrap mapped by the .wsb template", () => {
  assert.equal(MARCADOR_GUEST, "C:\\labsandbox\\bootstrap.ps1");
});

test("guest test root literal is pinned (travas.ps1 pins the same literal)", () => {
  assert.equal(RAIZ_TESTE_GUEST, "Z:\\claudia-teste-sandbox");
});

test("motivoRecusaRaizTeste: the root and anything under it, any letter case", () => {
  for (const p of ["Z:\\claudia-teste-sandbox", "z:\\CLAUDIA-TESTE-SANDBOX\\cs test a 1", "Z:\\claudia-teste-sandbox\\a\\b"]) {
    assert.equal(motivoRecusaRaizTeste(p), null, p);
  }
});

test("motivoRecusaRaizTeste: C:, other folders on Z:, siblings, dotted, relative, UNC and empty are refused", () => {
  for (const p of ["C:\\labdata\\tmp\\x", "Z:\\", "Z:\\tmp\\x", "Z:\\claudia-teste-sandbox-irmao\\x", "Z:\\claudia-teste-sandbox\\..\\x",
    "Z:\\claudia-teste-sandbox\\.\\x", "claudia-teste-sandbox\\x", "\\\\srv\\s\\claudia-teste-sandbox", "", null]) {
    assert.notEqual(motivoRecusaRaizTeste(p), null, String(p));
  }
});

test("raizDeCenarios: Z:\\ in the guest, nothing on the PC (D1: scenarios only in the Sandbox)", () => {
  assert.equal(raizDeCenarios({ noSandbox: true }), "Z:\\");
  assert.equal(raizDeCenarios({ noSandbox: false }), null);
  assert.equal(raizDeCenarios({}), null);
});
