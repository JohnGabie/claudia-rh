// Shared constants and helpers for the clean-machine scenario scripts.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

export const APP_ID = "io.github.johngabie.claudia-rh";
export const REPO_ROOT = path.resolve(import.meta.dirname, "../../../..");

/** Check if path b is inside path a. Works across different drives. */
export function dentroDe(a, b) {
  const rel = path.relative(a, b);
  return rel === "" || (!path.isAbsolute(rel) && rel !== ".." && !rel.startsWith(".." + path.sep));
}

export function sair(msg, code = 1) {
  console.error(msg);
  process.exit(code);
}

/** Usage errors (bad or missing options) exit 2; everything else exits 1. */
export function falhaDe(err) {
  sair(err.message, String(err.code ?? "").startsWith("ERR_PARSE_ARGS") ? 2 : 1);
}

export function labDir() {
  const d = process.env.SANDBOX_LAB_DIR;
  if (!d) throw new Error("SANDBOX_LAB_DIR is not set: point it to the Windows Sandbox lab folder.");
  const stage = path.join(d, "scripts", "app-stage.ps1");
  if (!fs.existsSync(stage)) throw new Error(`Incomplete lab: ${stage} not found.`);
  return path.resolve(d);
}

export function novoCenarioDir(saida, nome) {
  const dir = path.resolve(saida || path.join(process.env.TEMP ?? os.tmpdir(), "claudia-sandbox", nome));

  // Safety checks before deletion
  const parsed = path.parse(dir);
  if (parsed.root === dir) {
    throw new Error("novoCenarioDir: target is a filesystem root");
  }
  if (dir === REPO_ROOT) {
    throw new Error("novoCenarioDir: target is REPO_ROOT");
  }
  if (dentroDe(dir, REPO_ROOT)) {
    throw new Error("novoCenarioDir: target is an ancestor of REPO_ROOT");
  }

  // Check if non-empty directory exists without cenario.json or .cenario marker
  if (fs.existsSync(dir)) {
    const entries = fs.readdirSync(dir);
    const hasCenarioJson = fs.existsSync(path.join(dir, "cenario.json"));
    const hasMarker = fs.existsSync(path.join(dir, ".cenario"));
    if (entries.length > 0 && !hasCenarioJson && !hasMarker) {
      throw new Error(`novoCenarioDir: target directory must not have existing files without cenario.json: ${dir}`);
    }
  }

  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(path.join(dir, "appdata", APP_ID), { recursive: true });
  fs.mkdirSync(path.join(dir, "localappdata"), { recursive: true });
  fs.writeFileSync(path.join(dir, ".cenario"), "");
  return dir;
}

function arquivosDe(raiz) {
  return fs.readdirSync(raiz, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => path.join(e.parentPath ?? e.path, e.name));
}

export function escreverCenario(dir, nome, origem = null) {
  const arquivos = arquivosDe(path.join(dir, "appdata")).map((f) => path.relative(dir, f));
  const json = JSON.stringify({ cenario: nome, criadoEm: new Date().toISOString(), origem: origem ?? null, arquivos });
  fs.writeFileSync(path.join(dir, "cenario.json"), json);
  return json;
}

const DISPARO_PADRAO = { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, janelas: [] };

/** Validate a janela entry: dia_semana 0..6, inicio/fim strings, ativo boolean. */
function janelaValida(janela) {
  return janela && typeof janela === "object" &&
    Number.isInteger(janela.dia_semana) && janela.dia_semana >= 0 && janela.dia_semana <= 6 &&
    typeof janela.inicio === "string" && typeof janela.fim === "string" &&
    typeof janela.ativo === "boolean";
}

/** The app falls back to ativo=true when disparo.json misses required fields, so always write the full shape. */
export function desligarAutomacao(dataDir) {
  const p = path.join(dataDir, "disparo.json");
  let antigo = {};
  try {
    const parsed = JSON.parse(fs.readFileSync(p, "utf8"));
    if (parsed && typeof parsed === "object") antigo = parsed;
  } catch { /* missing or invalid: defaults */ }

  const cfg = { ...DISPARO_PADRAO };

  // Copy numeric fields only when valid
  for (const k of ["limiar_minutos", "limite_diario", "limite_tempo_minutos"]) {
    const v = antigo[k];
    if (Number.isInteger(v) && v >= 0 && v <= 0xFFFFFFFF) cfg[k] = v;
  }

  // Copy janelas only when array of valid entries
  if (Array.isArray(antigo.janelas) && antigo.janelas.every(janelaValida)) {
    cfg.janelas = antigo.janelas;
  }

  cfg.ativo = false;
  fs.writeFileSync(p, JSON.stringify(cfg));
  fs.writeFileSync(path.join(dataDir, "modo_autonomo.json"), JSON.stringify({ skip_permissions: false }));
}

const GUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi;

export function idsDeWsbList(texto) {
  return [...new Set(String(texto ?? "").match(GUID) ?? [])];
}

function wsb(args) {
  const r = spawnSync("wsb", args, { encoding: "utf8" });
  if (r.error) throw new Error(`wsb is not available: ${r.error.message}`);
  return r;
}

/** Returns an Error if wsb exec failed (status !== 0), null otherwise. */
export function erroDeExec(r, id, como) {
  if (r.status === 0) return null;
  const msg = (r.stderr || r.stdout || "").trim();
  const detail = msg ? `: ${msg}` : "";
  const err = new Error(`wsb exec failed (status ${r.status}) in sandbox ${id} as ${como}${detail}`);
  err.code = "ERR_EXEC_FAILED";
  err.status = r.status;
  return err;
}

/** Guest exit status 3 means the app database does not exist yet; map it to an actionable error, pass others through. */
export function erroBancoAusente(err, guestPath) {
  if (err?.code === "ERR_EXEC_FAILED" && err.status === 3) {
    return new Error(`the app database does not exist in the guest yet (${guestPath}): start the app first`);
  }
  return err;
}

export function sandboxId(id) {
  if (id) return id;
  const ids = idsDeWsbList(wsb(["list"]).stdout);
  if (ids.length !== 1) throw new Error(`Expected exactly one running sandbox, found ${ids.length}. Start one with the lab's test-up.ps1 or pass --id.`);
  return ids[0];
}

export function sandboxIp(id) {
  const m = (wsb(["ip", "--id", id]).stdout ?? "").match(/\d{1,3}(\.\d{1,3}){3}/);
  if (!m) throw new Error(`Could not read the sandbox IP (wsb ip --id ${id}). The profile needs networking enabled.`);
  return m[0];
}

/** Runs a PowerShell snippet in the guest. -EncodedCommand avoids quoting issues; wsb exec returns no process output. Throws if execution fails. */
export function noGuest(id, como, ps) {
  const enc = Buffer.from(ps, "utf16le").toString("base64");
  const r = wsb(["exec", "--id", id, "-r", como, "-c", `powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${enc}`]);
  const err = erroDeExec(r, id, como);
  if (err) throw err;
  return r.status;
}

/** Validate a number is an integer in the given range. Throws with code ERR_PARSE_ARGS_USAGE. */
export function validarInt(value, min, max, name) {
  const n = Number(value);
  if (!Number.isInteger(n) || n < min || n > max) {
    const err = new Error(`${name}: must be an integer between ${min} and ${max}, got ${value}`);
    err.code = "ERR_PARSE_ARGS_USAGE";
    throw err;
  }
  return n;
}
