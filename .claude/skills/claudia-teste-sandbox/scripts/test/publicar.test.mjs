import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { tempDir, rodar, SCRIPTS } from "./util.mjs";

const FAKE_STAGE = `param([string]$App, [string[]]$AppData, [string[]]$LocalAppData, [string]$WorkDir, [switch]$Clean)
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'called.json'), (@{ App = $App; AppData = @($AppData); WorkDir = $WorkDir; Clean = [bool]$Clean } | ConvertTo-Json))
exit 0
`;

test("publicar", () => {
  const base = tempDir("publicar");
  const lab = path.join(base, "lab x");
  fs.mkdirSync(path.join(lab, "scripts"), { recursive: true });
  fs.writeFileSync(path.join(lab, "scripts", "app-stage.ps1"), FAKE_STAGE);
  const app = path.join(base, "app x.exe"); fs.writeFileSync(app, "x");
  const cen = path.join(base, "cen x");
  rodar(path.join(SCRIPTS, "cenario-novo.mjs"), ["--saida", cen]);
  const work = path.join(base, "work x");
  const script = path.join(SCRIPTS, "publicar.mjs");

  const r = rodar(script, ["--app", app, "--cenario", cen, "--work-dir", work], { SANDBOX_LAB_DIR: lab });
  assert.equal(r.code, 0, r.err);
  const c = JSON.parse(fs.readFileSync(path.join(lab, "scripts", "called.json"), "utf8"));
  assert.equal(c.App, app);
  assert.equal(c.WorkDir, work);
  assert.equal(c.Clean, true);
  assert.match([c.AppData].flat()[0], /appdata[\\/]io\.github\.johngabie\.claudia-rh$/, "publicar repassa a pasta de dados do app");
  assert.equal(JSON.parse(r.ultima).cenario, "novo");

  const semMeta = rodar(script, ["--app", app, "--cenario", base, "--work-dir", work], { SANDBOX_LAB_DIR: lab });
  assert.equal(semMeta.code, 1);
  assert.match(semMeta.err, /cenario\.json/, "publicar recusa pasta sem cenario.json");

  const labRuim = rodar(script, ["--app", app, "--cenario", cen, "--work-dir", work], { SANDBOX_LAB_DIR: base });
  assert.equal(labRuim.code, 1);
  assert.match(labRuim.err, /app-stage\.ps1/, "publicar com lab incompleto nomeia o arquivo");

  assert.equal(rodar(script, ["--app", app], { SANDBOX_LAB_DIR: lab }).code, 2, "sem --cenario sai com 2");
});
