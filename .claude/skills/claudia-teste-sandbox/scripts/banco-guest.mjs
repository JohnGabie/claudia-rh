// Copies a consistent snapshot of the app database from the sandbox guest to the host.
// Prints {db}: use it as CLAUDIA_DB with the claudia-teste-e2e scripts. Repeat after each action that changes data.
// Usage: node banco-guest.mjs [--id <sandbox>] [--work-dir <folder>] [--timeout 30]   (--work-dir default $env:LAB_WORK_DIR)
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { APP_ID, sandboxId, noGuest, validarInt, falhaDe, erroBancoAusente } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { id: { type: "string" }, "work-dir": { type: "string" }, timeout: { type: "string", default: "30" } } });
  const id = sandboxId(values.id);
  const timeout = validarInt(values.timeout, 1, 0xFFFFFFFF, "--timeout");
  const workDir = values["work-dir"] || process.env.LAB_WORK_DIR;
  if (!workDir) throw new Error("LAB_WORK_DIR is not set: pass --work-dir (the folder used by publicar.mjs).");
  const hostDb = path.join(workDir, "guest-rw", "db", "claudia_rh.db");
  const hostDbTmp = hostDb + ".tmp";
  fs.rmSync(hostDb, { force: true });
  fs.rmSync(hostDbTmp, { force: true });
  const guestDb = `%APPDATA%\\${APP_ID}\\claudia_rh.db`;
  try {
    noGuest(id, "ExistingLogin", [
      // Never let sqlite3 create an empty database when the app has not made one yet.
      `$src = Join-Path $env:APPDATA '${APP_ID}\\claudia_rh.db'; if (-not (Test-Path $src)) { exit 3 }`,
      "New-Item -ItemType Directory -Force C:\\labwork\\db | Out-Null",
      "Remove-Item -Force C:\\labwork\\db\\claudia_rh.db.tmp -ErrorAction SilentlyContinue",
      "& C:\\lab\\tools\\sqlite\\sqlite3.exe $src '.backup C:/labwork/db/claudia_rh.db.tmp'",
      "if ($LASTEXITCODE -eq 0) { Move-Item -Force C:\\labwork\\db\\claudia_rh.db.tmp C:\\labwork\\db\\claudia_rh.db } else { exit $LASTEXITCODE }",
    ].join("\n"));
  } catch (e) { throw erroBancoAusente(e, guestDb); }
  const fim = Date.now() + timeout * 1000;
  while (!fs.existsSync(hostDb) && Date.now() < fim) await new Promise((r) => setTimeout(r, 1000));
  if (!fs.existsSync(hostDb)) throw new Error(`Snapshot not found at ${hostDb} (does the app database exist in the guest yet?).`);
  console.log(JSON.stringify({ db: hostDb }));
} catch (e) { falhaDe(e); }
