// Publishes the app plus a scenario folder to the Windows Sandbox lab (lab's scripts\app-stage.ps1).
// Usage: node publicar.mjs --app <exe|installer|folder> --cenario <scenario folder> [--work-dir <folder>]
//   --work-dir  default $env:LAB_WORK_DIR, else %TEMP%\claudia-teste-sandbox\work
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { spawnSync } from "node:child_process";
import { APP_ID, labDir, falhaDe, podeLimparDeploy, MARCADOR_DEPLOY } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { app: { type: "string" }, cenario: { type: "string" }, "work-dir": { type: "string" } } });
  if (!values.app || !values.cenario) {
    throw Object.assign(new Error("Usage: node publicar.mjs --app <exe|installer|folder> --cenario <folder> [--work-dir <folder>]"), { code: "ERR_PARSE_ARGS_USAGE" });
  }
  const lab = labDir();
  const meta = path.join(values.cenario, "cenario.json");
  if (!fs.existsSync(meta)) throw new Error(`Not a scenario folder (cenario.json missing): ${values.cenario}`);
  const workDir = values["work-dir"] || process.env.LAB_WORK_DIR || path.join(process.env.TEMP, "claudia-teste-sandbox", "work");
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
  fs.writeFileSync(path.join(deploy, MARCADOR_DEPLOY), "");
  const cenario = JSON.parse(fs.readFileSync(meta, "utf8")).cenario;
  console.log(JSON.stringify({ workDir, deploy: path.join(workDir, "deploy"), cenario }));
} catch (e) { falhaDe(e); }
