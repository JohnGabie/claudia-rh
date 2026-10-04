// End-to-end smoke test over the REAL MCP protocol (stdio, JSON-RPC): the app binary, a real Chrome,
// the fixture page served over HTTP. Exits non-zero on any failed expectation.
//
//   CLAUDIA_TEST_CHROME=C:/path/chrome.exe node tests/mcp-browser-smoke.mjs [path/to/claudia-rh.exe]
//
// Uses only Node built-ins (Node 22+): no install step.
import { spawn, execSync } from "node:child_process";
import { createServer } from "node:http";
import { mkdtempSync, readFileSync, rmSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const exe = process.argv[2] ?? "target/debug/claudia-rh.exe";
// With CLAUDIA_APP_LOCAL (the real <dados-locais>) the test drives the Chrome the running APP started
// (its state.json), visible, instead of launching a headless one. The Activity log is always on.
const appLocal = process.env.CLAUDIA_APP_LOCAL;
const chrome = process.env.CLAUDIA_TEST_CHROME;
if (!chrome && !appLocal) { console.error("set CLAUDIA_TEST_CHROME to chrome.exe, or CLAUDIA_APP_LOCAL to the app local data dir"); process.exit(2); }

let failed = 0;
const check = (ok, msg) => { console.log((ok ? "OK    " : "FALHOU") + " " + msg); if (!ok) failed++; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── a throwaway app data dir and local data dir ──
const base = mkdtempSync(join(tmpdir(), "claudia-smoke-"));
const dataDir = join(base, "data");
const localDir = appLocal ?? join(base, "local");
mkdirSync(dataDir, { recursive: true });
mkdirSync(join(localDir, "browser"), { recursive: true });
const statePath = join(localDir, "browser", "state.json");

// ── the fixture page over HTTP (file:// URLs are not allowed in production) ──
const page = readFileSync(new URL("./fixtures/apply-form.html", import.meta.url));
const web = createServer((_q, s) => { s.setHeader("Content-Type", "text/html; charset=utf-8"); s.end(page); });
await new Promise((r) => web.listen(0, "127.0.0.1", r));
const pageUrl = `http://127.0.0.1:${web.address().port}/`;

// ── a real headless Chrome with its state.json, as the app would leave it ──
let chromeProc = null;
if (!appLocal) {
  const port = 9840 + Math.floor(Math.random() * 50);
  chromeProc = spawn(chrome, [`--remote-debugging-port=${port}`, `--user-data-dir=${join(base, "profile")}`,
    "--no-first-run", "--no-default-browser-check", "--headless=new"], { stdio: "ignore" });
  for (let i = 0; i < 60; i++) { try { await (await fetch(`http://127.0.0.1:${port}/json/version`)).json(); break; } catch { await sleep(500); } }
  writeFileSync(statePath, JSON.stringify({ port, pid: chromeProc.pid, version: "" }));
}

// ── the MCP server, exactly as the AI CLI would start it ──
const child = spawn(exe, ["--mcp-serve", "--data-dir", dataDir, "--local-data-dir", localDir,
  "--browser-state", statePath, "--session-kind", "autonomous"], { stdio: ["pipe", "pipe", "inherit"] });
let id = 0; const waiting = new Map(); let buf = "";
child.stdout.on("data", (d) => {
  buf += d; let i;
  while ((i = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, i); buf = buf.slice(i + 1);
    if (!line.trim()) continue;
    const m = JSON.parse(line);
    if (m.id && waiting.has(m.id)) { waiting.get(m.id)(m); waiting.delete(m.id); }
  }
});
const send = (method, params) => new Promise((res, rej) => {
  const i = ++id; waiting.set(i, res);
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: i, method, params }) + "\n");
  setTimeout(() => { if (waiting.has(i)) { waiting.delete(i); rej(new Error("timeout " + method)); } }, 30000);
});
const call = async (name, args = {}) => (await send("tools/call", { name, arguments: args })).result;
const text = (r) => r?.content?.find((c) => c.type === "text")?.text ?? "";

try {
  await send("initialize", { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "smoke", version: "1" } });
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized", params: {} }) + "\n");

  const tools = (await send("tools/list", {})).result.tools.map((t) => t.name);
  for (const need of ["browser_open", "browser_read", "browser_text", "browser_click", "browser_type", "browser_key", "browser_scroll", "browser_crop", "browser_click_at"]) {
    check(tools.includes(need), `tool ${need} is listed`);
  }
  check(tools.includes("update_vaga_status"), "the existing tools are still there");
  check(!tools.some((t) => /activity|dev_?mode|trace/i.test(t)), "no tool exposes the Activity log to the AI");

  const opened = await call("browser_open", { url: pageUrl });
  check(!opened.isError && /Abriu/.test(text(opened)), "browser_open: " + text(opened).slice(0, 60));
  const read = await call("browser_read");
  const listing = text(read);
  check(/button "Avançar"/.test(listing) && /textbox "Nome"/.test(listing), "browser_read lists named elements");
  const scoped = text(await call("browser_read", { scope: "dialog" }));
  check(/Avançar/.test(scoped) && !/Fora do modal/.test(scoped), "scope=dialog excludes what is outside the modal");

  const refOf = (role, name) => listing.split("\n").find((l) => l.includes(`${role} "${name}"`))?.split(" ")[0];
  const adv = refOf("button", "Avançar");
  const clicked = await call("browser_click", { reference: adv });
  check(!clicked.isError, "browser_click on an ordinary button: " + text(clicked));
  const typed = await call("browser_type", { reference: refOf("textbox", "Nome"), text: "Ana" });
  check(!typed.isError, "browser_type: " + text(typed));

  const crop = await call("browser_crop", { x: 0, y: 0, width: 200, height: 100 });
  const img = crop?.content?.find((c) => c.type === "image");
  check(!!img && img.mimeType === "image/png" && img.data.length > 100, "browser_crop returns a PNG image content block");

  check(/\nestado: ok$/.test(listing.trimEnd()), "browser_read ends with the estado line");
  const pageText = text(await call("browser_text"));
  check(pageText.startsWith("[conteúdo da página") && /total_chars: \d+/.test(pageText) && /estado: ok$/.test(pageText.trimEnd()), "browser_text: untrusted header, total_chars and the estado line");
  check(!pageText.includes("segredo"), "browser_text never carries what was typed into a field");
  const scrolled = await call("browser_scroll", { direction: "down" });
  check(!scrolled.isError && /Rolou down/.test(text(scrolled)), "browser_scroll: " + text(scrolled).slice(0, 70));
  const badScroll = await call("browser_scroll", { direction: "sideways" });
  check(badScroll.isError === true && /direção inválida/.test(text(badScroll)), "browser_scroll rejects a bad direction");

  // walk the button to its final label and try to submit WITHOUT confirmation: the guard must refuse
  for (let step = 0; step < 2; step++) {
    const l = text(await call("browser_read"));
    const r = l.split("\n").find((x) => /button "(Avançar|Avaliar)"/.test(x))?.split(" ")[0];
    if (r) await call("browser_click", { reference: r });
  }
  const l2 = text(await call("browser_read"));
  const submit = l2.split("\n").filter((x) => x.includes('button "Enviar candidatura"'))[0]?.split(" ")[0];
  const refused = await call("browser_click", { reference: submit });
  check(refused.isError === true && /RECUSADO/.test(text(refused)), "submit without confirm_submit is refused: " + text(refused).slice(0, 70));
  const wrongVaga = await call("browser_click", { reference: submit, confirm_submit: 12345 });
  check(wrongVaga.isError === true && /RECUSADO/.test(text(wrongVaga)), "submit with an unknown vaga is refused: " + text(wrongVaga).slice(0, 70));

  const bad = await call("browser_open", { url: "file:///c:/windows/win.ini" });
  check(bad.isError === true && /http/.test(text(bad)), "a file:// URL is refused: " + text(bad).slice(0, 60));
} catch (e) {
  check(false, "unexpected error: " + e.message);
} finally {
  child.kill();
  if (chromeProc) { try { execSync(`taskkill /PID ${chromeProc.pid} /T /F`, { stdio: "ignore" }); } catch {} }
  web.close();
  await sleep(1000);
  try { rmSync(base, { recursive: true, force: true }); } catch {}
}
console.log(failed === 0 ? "\nTUDO OK" : `\n${failed} VERIFICAÇÕES FALHARAM`);
process.exit(failed === 0 ? 0 : 1);
