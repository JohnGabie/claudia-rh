// Copies a consistent snapshot of the app database from the sandbox guest to the host.
// Prints {db}: use it as CLAUDIA_DB with the claudia-app scripts. Repeat after each action that changes data.
// Usage: node banco-guest.mjs [--id <sandbox>] [--work-dir <folder>] [--timeout 30]   (--work-dir default $env:LAB_WORK_DIR)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { APP_ID, sandboxId, noGuest, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { id: { type: "string" }, "work-dir": { type: "string" }, timeout: { type: "string", default: "30" } } });
  const id = sandboxId(values.id);
  const workDir = values["work-dir"] || process.env.LAB_WORK_DIR;
  if (!workDir) throw new Error("LAB_WORK_DIR is not set: pass --work-dir (the folder used by publicar.mjs).");
  const hostDb = path.join(workDir, "guest-rw", "db", "claudia_rh.db");
  fs.rmSync(hostDb, { force: true });
  noGuest(id, "ExistingLogin", [
    "New-Item -ItemType Directory -Force C:\\labwork\\db | Out-Null",
    `& C:\\lab\\tools\\sqlite\\sqlite3.exe (Join-Path $env:APPDATA '${APP_ID}\\claudia_rh.db') '.backup C:/labwork/db/claudia_rh.db'`,
  ].join("\n"));
  const fim = Date.now() + Number(values.timeout) * 1000;
  while (!fs.existsSync(hostDb) && Date.now() < fim) await new Promise((r) => setTimeout(r, 1000));
  if (!fs.existsSync(hostDb)) throw new Error(`Snapshot not found at ${hostDb} (does the app database exist in the guest yet?).`);
  console.log(JSON.stringify({ db: hostDb }));
} catch (e) { falhaDe(e); }
