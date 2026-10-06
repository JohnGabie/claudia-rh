// Read-only query against the app's SQLite database.
// Usage: node banco.mjs "SELECT ..."
import { DatabaseSync } from "node:sqlite";
import path from "node:path";
import { sqlSomenteLeitura, hostRemoto } from "./regras.mjs";
import { HOST } from "./cdp.mjs";

const sql = process.argv[2];
const v = sqlSomenteLeitura(sql);
if (!v.ok) { console.error(`recusado: ${v.motivo}`); process.exit(2); }

// A remote target has its own database: never fall back to the host's app folder.
if (hostRemoto(HOST) && !process.env.CLAUDIA_DB) {
  console.error(`recusado: remote target (CDP_HOST=${HOST}): set CLAUDIA_DB from node .claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs`);
  process.exit(2);
}

const dados = process.env.CLAUDIA_DB || path.join(process.env.APPDATA, "io.github.johngabie.claudia-rh", "claudia_rh.db");
// readOnly at the engine level too: the rule above is the early, readable failure.
const db = new DatabaseSync(dados, { readOnly: true });
console.log(JSON.stringify(db.prepare(sql).all(), null, 1));
db.close();
