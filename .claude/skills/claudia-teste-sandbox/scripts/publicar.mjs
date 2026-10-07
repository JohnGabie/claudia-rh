// Publishes the app plus a scenario folder to the Windows Sandbox lab (lab's scripts\app-stage.ps1).
// Usage: node publicar.mjs --app <exe|installer|folder> --cenario <scenario folder> [--work-dir <folder>]
//   --work-dir  default $env:LAB_WORK_DIR, else %TEMP%\claudia-teste-sandbox\work
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { parseArgs } from "node:util";
import { spawnSync } from "node:child_process";
import { APP_ID, REPO_ROOT, labDir, falhaDe, podeLimparDeploy, MARCADOR_DEPLOY, motivoRecusa, ancestoresComGit } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { app: { type: "string" }, cenario: { type: "string" }, "work-dir": { type: "string" } } });
  if (!values.app || !values.cenario) {
    throw Object.assign(new Error("Usage: node publicar.mjs --app <exe|installer|folder> --cenario <folder> [--work-dir <folder>]"), { code: "ERR_PARSE_ARGS_USAGE" });
  }
  const lab = labDir();
  const meta = path.join(values.cenario, "cenario.json");
  if (!fs.existsSync(meta)) throw new Error(`Not a scenario folder (cenario.json missing): ${values.cenario}`);
  const tempRoot = fs.realpathSync.native(os.tmpdir());
  const workDir = path.resolve(values["work-dir"] || process.env.LAB_WORK_DIR || path.join(tempRoot, "claudia-teste-sandbox", "work"));
  // Same allow-list as the scenario folders: the work dir must live under <tmp>/claudia-teste-sandbox.
  for (const alvo of [workDir, fs.existsSync(workDir) ? fs.realpathSync.native(workDir) : null]) {
    if (alvo === null) continue;
    const motivoWork = motivoRecusa(alvo, { repoRoot: REPO_ROOT, tempRoot, ancestoresComGit: ancestoresComGit(alvo) });
    if (motivoWork) throw new Error(`Refusing work-dir (${motivoWork}): ${alvo}`);
  }
  // -Clean wipes <work-dir>/deploy: only allow it when deploy is absent or ours (marker) and not a link.
  const deploy = path.join(workDir, "deploy");
  let st = null;
  try { st = fs.lstatSync(deploy); } catch (e) { if (e.code !== "ENOENT") throw e; }
  const motivoDeploy = podeLimparDeploy({
    existe: st !== null,
    temMarcador: st !== null && fs.existsSync(path.join(deploy, MARCADOR_DEPLOY)),
    ehLink: st !== null && st.isSymbolicLink(),
  });
  if (motivoDeploy) throw new Error(`Refusing -Clean: ${motivoDeploy}: ${deploy}`);
  const r = spawnSync("pwsh", ["-NoProfile", "-File", path.join(lab, "scripts", "app-stage.ps1"),
    "-App", values.app, "-AppData", path.join(values.cenario, "appdata", APP_ID), "-WorkDir", workDir, "-Clean"],
    { stdio: ["ignore", "inherit", "inherit"] });
  if (r.status !== 0) throw new Error(`app-stage.ps1 failed (${r.status ?? r.error?.message})`);
  fs.mkdirSync(deploy, { recursive: true });
  fs.writeFileSync(path.join(deploy, MARCADOR_DEPLOY), "");
  const cenario = JSON.parse(fs.readFileSync(meta, "utf8")).cenario;
  console.log(JSON.stringify({ workDir, deploy: path.join(workDir, "deploy"), cenario }));
} catch (e) { falhaDe(e); }
