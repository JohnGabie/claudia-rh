// Cross-checks the Activity log against its files and the rules of the spec.
//   node tests/activity-log-validate.mjs <local-data-dir> [--expect-min N] [--expect-passwords-of-names senha,password]
// Reads <local-data-dir>/dev/activity-log.db (read-only). Node 22+ (node:sqlite). Exit code 1 on any failure.
import { DatabaseSync } from "node:sqlite";
import { existsSync, statSync, readdirSync } from "node:fs";
import { join } from "node:path";

const [local, ...rest] = process.argv.slice(2);
const flag = (name) => { const i = rest.indexOf(name); return i >= 0 ? rest[i + 1] : null; };
const expectMin = flag("--expect-min") ? Number(flag("--expect-min")) : 1;
const dev = join(local, "dev");
let failed = 0;
const check = (ok, msg) => { console.log((ok ? "OK    " : "FALHOU") + " " + msg); if (!ok) failed++; };
const nonEmptyFile = (rel) => { const p = rel && join(dev, rel); return !!p && existsSync(p) && statSync(p).size > 0; };

const db = new DatabaseSync(join(dev, "activity-log.db"), { readOnly: true });
const rows = db.prepare("SELECT * FROM events ORDER BY id").all();
check(rows.length >= expectMin, `há ${rows.length} registros (mínimo ${expectMin})`);

const actions = rows.filter((r) => r.kind === "action" || r.kind === "failure");

// 1. every successful mutating action has a readable 'after'; a click also has 'before'
for (const r of actions.filter((x) => x.result === "ok" && ["browser_click", "browser_type"].includes(x.tool))) {
  check(nonEmptyFile(r.shot_after), `#${r.id} ${r.tool} tem shot_after legível`);
}
for (const r of actions.filter((x) => x.result === "ok" && x.tool === "browser_click")) {
  check(nonEmptyFile(r.shot_before), `#${r.id} clique tem shot_before legível`);
}

// 2. failures and refusals keep an error and a screenshot
for (const r of rows.filter((x) => x.kind === "failure")) {
  check(!!r.error, `#${r.id} falha tem erro`);
  // A failure that happens BEFORE any page exists (browser not ready, URL refused up front) has nothing to capture.
  if (r.url) check(nonEmptyFile(r.shot_after), `#${r.id} falha com página tem print`);
  else check(r.shot_after === null, `#${r.id} falha sem página não tem print`);
  check(r.result === "error" || r.result === "refused", `#${r.id} resultado classificado (${r.result})`);
}

// 3. passwords: neither text nor 'before' crop for a password-like target
for (const r of rows.filter((x) => x.tool === "browser_type" && /senha|password/i.test(x.target ?? ""))) {
  check(r.text === null, `#${r.id} campo de senha sem texto gravado`);
  check(r.shot_before === null, `#${r.id} campo de senha sem recorte 'antes'`);
}

// 4. navigation rows carry a redirect list and an initiator
for (const r of rows.filter((x) => x.kind === "navigation")) {
  let e = {};
  try { e = JSON.parse(r.extra ?? "{}"); } catch { /* checked below */ }
  check(Array.isArray(e.redirects) && ["agent", "outside-tool"].includes(e.initiator), `#${r.id} navegação com iniciador e redirects`);
}

// 5. the referenced paths stay inside capturas/ and nothing on disk is orphaned
const referenced = new Set();
for (const r of rows) for (const p of [r.shot_before, r.shot_after]) {
  if (!p) continue;
  check(p.startsWith("capturas/") && !p.includes(".."), `caminho ${p} fica em capturas/`);
  referenced.add(p.replace(/\\/g, "/"));
}
const walk = (d) => readdirSync(d, { withFileTypes: true }).flatMap((e) => e.isDirectory() ? walk(join(d, e.name)) : [join(d, e.name)]);
const files = existsSync(join(dev, "capturas")) ? walk(join(dev, "capturas")) : [];
const orphans = files.filter((f) => !referenced.has(f.slice(dev.length + 1).replace(/\\/g, "/")));
check(orphans.length === 0, `sem arquivos órfãos (${orphans.length})`);

// 6. retention budget (1 GB) and age (30 days)
const bytes = files.reduce((s, f) => s + statSync(f).size, 0);
check(bytes <= 1_073_741_824, `capturas ocupam ${(bytes / 1e6).toFixed(1)} MB (teto 1024 MB)`);
const oldest = rows[0]?.ts;
check(!oldest || Date.now() - Date.parse(oldest) <= 31 * 86400e3, `registro mais antigo: ${oldest ?? "-"} (máx. 30 dias)`);

console.log(failed === 0 ? "\nTUDO OK" : `\n${failed} VERIFICAÇÕES FALHARAM`);
process.exit(failed === 0 ? 0 : 1);
