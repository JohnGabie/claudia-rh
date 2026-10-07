// Pure tests: invented Windows paths (Z:\...), injected predicates, no disk. Runs on any OS (path.win32 inside).
import test from "node:test";
import assert from "node:assert/strict";
import {
  raizPrincipalDe, caminhosDoLab, linksAte, podeLimparDeploy, motivoRecusaLimparDeploy, estaNoSandbox, MARCADOR_DEPLOY,
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

test("estaNoSandbox is true only for the Sandbox user", () => {
  assert.equal(estaNoSandbox("WDAGUtilityAccount"), true);
  assert.equal(estaNoSandbox("wdagutilityaccount"), true);
  for (const u of ["", undefined, null, "fulano", "WDAGUtilityAccount2", "Administrator"]) assert.equal(estaNoSandbox(u), false, String(u));
});
