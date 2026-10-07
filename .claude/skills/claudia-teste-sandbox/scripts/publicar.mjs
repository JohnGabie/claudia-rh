// Publishes the app plus a scenario folder to the lab's work folder (<main checkout>\.sandbox-work) via app-stage.ps1 -Clean.
// Usage: node publicar.mjs --app <exe|installer|folder> --cenario <scenario folder>
// The work folder is fixed (no --work-dir, no LAB_WORK_DIR). -Clean only runs when <main>\.sandbox-work\deploy is
// absent or carries the .lab-deploy marker, with no link on the way (app-stage.ps1 checks the same again).
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { APP_ID, falhaDe, labDir, onde } from "./comum.mjs";
import { caminhosDoLab, linksAte, motivoRecusaLimparDeploy, MARCADOR_DEPLOY } from "./caminhos-lab.mjs";

function ehLinkNoDisco(p) {
  try { return fs.lstatSync(p).isSymbolicLink(); } catch (e) { if (e.code === "ENOENT") return false; throw e; }
}

/** `raizPrincipal` and `rodarStage` are injected so the Sandbox test uses a fake checkout and a fake app-stage. */
export function publicar({ app, cenario }, { raizPrincipal, rodarStage }) {
  const meta = path.join(cenario, "cenario.json");
  if (!fs.existsSync(meta)) throw new Error(`Not a scenario folder (cenario.json missing): ${cenario}`);
  const c = caminhosDoLab(raizPrincipal);
  let st = null;
  try { st = fs.lstatSync(c.deploy); } catch (e) { if (e.code !== "ENOENT") throw e; }
  const motivo = motivoRecusaLimparDeploy({
    deploy: c.deploy,
    raizPrincipal,
    existe: st !== null,
    temMarcador: st !== null && fs.existsSync(path.join(c.deploy, MARCADOR_DEPLOY)),
    linksNoCaminho: linksAte(c.deploy, raizPrincipal, ehLinkNoDisco),
  });
  if (motivo) throw new Error(`Refusing -Clean: ${motivo}: ${c.deploy}`);
  const status = rodarStage(c.appStage, ["-App", app, "-AppData", path.join(cenario, "appdata", APP_ID), "-WorkDir", c.trabalho, "-Clean"]);
  if (status !== 0) throw new Error(`app-stage.ps1 failed (${status})`);
  fs.mkdirSync(c.deploy, { recursive: true });
  fs.writeFileSync(path.join(c.deploy, MARCADOR_DEPLOY), "");
  return { workDir: c.trabalho, deploy: c.deploy, cenario: JSON.parse(fs.readFileSync(meta, "utf8")).cenario };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: { app: { type: "string" }, cenario: { type: "string" } } });
    if (!values.app || !values.cenario) {
      throw Object.assign(new Error("Usage: node publicar.mjs --app <exe|installer|folder> --cenario <folder>"), { code: "ERR_PARSE_ARGS_USAGE" });
    }
    labDir();
    const { raizPrincipal } = onde();
    const rodarStage = (ps1, args) => {
      const r = spawnSync("pwsh", ["-NoProfile", "-File", ps1, ...args], { stdio: ["ignore", "inherit", "inherit"] });
      return r.status ?? r.error?.message;
    };
    console.log(JSON.stringify(publicar(values, { raizPrincipal, rodarStage })));
  } catch (e) { falhaDe(e); }
}
