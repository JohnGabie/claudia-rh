// Shared constants and helpers for the clean-machine scenario scripts.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { raizPrincipalDe, caminhosDoLab, ambienteGitLimpo } from "./caminhos-lab.mjs";

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

function gitLer(args, cwd) {
  // GIT_DIR & co. would override -C when this runs inside a git hook or worktree command.
  const r = spawnSync("git", ["-C", cwd, ...args], { encoding: "utf8", env: ambienteGitLimpo(process.env) });
  if (r.status !== 0) throw new Error(`git ${args.join(" ")} failed: ${(r.stderr || r.error?.message || "").trim()}`);
  return r.stdout.trim();
}

/**
 * Main checkout (lab + work folder) and current checkout (project under test). Read-only: only `git rev-parse`.
 * `git` is injectable so the logic is tested without spawning git.
 */
export function onde(cwd = REPO_ROOT, git = gitLer) {
  const raizPrincipal = raizPrincipalDe(git(["rev-parse", "--path-format=absolute", "--git-common-dir"], cwd));
  const projeto = path.win32.normalize(git(["rev-parse", "--show-toplevel"], cwd));
  return { raizPrincipal, projeto, ...caminhosDoLab(raizPrincipal) };
}

/** sandbox-lab of the main checkout; throws naming the missing file. Never reads an environment variable. */
export function labDir({ cwd = REPO_ROOT, git = gitLer, existe = fs.existsSync } = {}) {
  const o = onde(cwd, git);
  if (!existe(o.appStage)) {
    throw new Error(`Lab not found: ${o.appStage} is missing in the main checkout. The lab lives in sandbox-lab/ of the main checkout (SKILL.md, section 1).`);
  }
  return o.lab;
}

const NOME_BASE = "claudia-teste-sandbox";
const minusculas = (p) => (process.platform === "win32" ? p.toLowerCase() : p);

/** Same as dentroDe, but case-insensitive on Windows. */
function dentroDeCI(a, b) {
  return dentroDe(minusculas(a), minusculas(b));
}

/**
 * PURE (never touches the disk). Allow-list for deletable scenario folders.
 * Returns null when `dir` may be deleted, otherwise the reason it is refused.
 * Only accepts an absolute, normalized path with no "..", strictly inside <tempRoot>/claudia-teste-sandbox.
 */
export function motivoRecusa(dir, { repoRoot, tempRoot, ancestoresComGit = [] }) {
  if (typeof dir !== "string" || dir.trim() === "") return "empty path";
  if (dir.startsWith("\\\\") || dir.startsWith("//")) return "UNC or extended-length path";
  if (!path.isAbsolute(dir)) return "relative path";
  const segmentos = dir.split(/[\\/]+/);
  if (segmentos.includes("..")) return "path contains '..'";
  // Win32 strips trailing dots/spaces ("<base>\..." resolves to "<base>"), so a lexical check would be fooled.
  if (segmentos.some((s) => /[. ]$/.test(s))) return "path has a segment ending in '.' or space";
  if (path.normalize(dir) !== dir) return "path is not normalized";
  if (path.parse(dir).root === dir) return "path is a filesystem root";
  if (typeof repoRoot !== "string" || repoRoot === "" || typeof tempRoot !== "string" || tempRoot === "") {
    return "repoRoot and tempRoot are required";
  }
  if (dentroDeCI(dir, repoRoot)) return "path is the repository or an ancestor of it";
  if (dentroDeCI(repoRoot, dir)) return "path is inside the repository";
  if (ancestoresComGit.some((g) => dentroDeCI(g, dir))) return "path is inside a git checkout";
  const base = path.join(tempRoot, NOME_BASE);
  if (minusculas(dir) === minusculas(base)) return "path is the sandbox base folder itself";
  if (!dentroDeCI(base, dir)) return `path is outside the sandbox base folder (${base})`;
  return null;
}

/**
 * `dir` and each of its ancestors up to the root that contain a ".git" entry.
 * `existe` is injectable so the logic can be tested without a disk.
 */
export function ancestoresComGit(dir, existe = fs.existsSync) {
  const achados = [];
  let atual = dir;
  for (;;) {
    if (existe(path.join(atual, ".git"))) achados.push(atual);
    const pai = path.dirname(atual);
    if (pai === atual) break;
    atual = pai;
  }
  return achados;
}

export { MARCADOR_DEPLOY, podeLimparDeploy } from "./caminhos-lab.mjs";

export function novoCenarioDir(saida, nome) {
  const tempRoot = fs.realpathSync.native(os.tmpdir());
  const dir = path.resolve(saida || path.join(tempRoot, NOME_BASE, nome));

  const motivo = motivoRecusa(dir, { repoRoot: REPO_ROOT, tempRoot, ancestoresComGit: ancestoresComGit(dir) });
  if (motivo) throw new Error(`novoCenarioDir: refused (${motivo}): ${dir}`);

  // Symlink/junction guard: never delete through a link.
  let st = null;
  try { st = fs.lstatSync(dir); } catch (e) { if (e.code !== "ENOENT") throw e; }
  if (st) {
    if (st.isSymbolicLink()) throw new Error(`novoCenarioDir: refused (target is a symlink/junction): ${dir}`);
    const real = fs.realpathSync.native(dir);
    if (minusculas(real) !== minusculas(dir) || motivoRecusa(real, { repoRoot: REPO_ROOT, tempRoot, ancestoresComGit: ancestoresComGit(real) })) {
      throw new Error(`novoCenarioDir: refused (real path differs: ${real}): ${dir}`);
    }
    const entries = fs.readdirSync(dir);
    const hasCenarioJson = fs.existsSync(path.join(dir, "cenario.json"));
    const hasMarker = fs.existsSync(path.join(dir, ".cenario"));
    if (entries.length > 0 && !hasCenarioJson && !hasMarker) {
      throw new Error(`novoCenarioDir: target directory must not have existing files without cenario.json: ${dir}`);
    }
  }

  // Only now (marker rule passed) delete.
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

/**
 * wsb exec always exits 0 on the host; the guest exit code only shows up as a localized text line
 * (pt-BR "Processo encerrado com o código: 3", English "... code: 3"). Returns the last one, or null.
 */
export function codigoDoGuest(stdout) {
  const m = [...String(stdout ?? "").matchAll(/(?:c\S{0,2}digo|code)[^\r\n:]*:\s*(-?\d+)\s*$/gim)];
  return m.length ? { code: Number(m.at(-1)[1]), line: m.at(-1)[0].trim() } : null;
}

/** Returns an Error if wsb exec failed (own status !== 0) or the guest reported a non-zero code, null otherwise. */
export function erroDeExec(r, id, como) {
  if (r.status === 0) {
    const g = codigoDoGuest(r.stdout);
    if (!g || g.code === 0) return null;
    const err = new Error(`wsb exec: guest process failed (${g.line}) in sandbox ${id} as ${como}`);
    err.code = "ERR_EXEC_FAILED";
    err.status = g.code;
    return err;
  }
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
