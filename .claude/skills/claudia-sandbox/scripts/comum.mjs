// Shared constants and helpers for the clean-machine scenario scripts.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export const APP_ID = "io.github.johngabie.claudia-rh";
export const REPO_ROOT = path.resolve(import.meta.dirname, "../../../..");

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
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(path.join(dir, "appdata", APP_ID), { recursive: true });
  fs.mkdirSync(path.join(dir, "localappdata"), { recursive: true });
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

/** The app falls back to ativo=true when disparo.json misses required fields, so always write the full shape. */
export function desligarAutomacao(dataDir) {
  const p = path.join(dataDir, "disparo.json");
  let antigo = {};
  try { antigo = JSON.parse(fs.readFileSync(p, "utf8")); } catch { /* missing or invalid: defaults */ }
  const cfg = { ...DISPARO_PADRAO };
  for (const k of ["limiar_minutos", "limite_diario", "limite_tempo_minutos", "janelas"]) {
    if (antigo[k] != null) cfg[k] = antigo[k];
  }
  cfg.ativo = false;
  fs.writeFileSync(p, JSON.stringify(cfg));
  fs.writeFileSync(path.join(dataDir, "modo_autonomo.json"), JSON.stringify({ skip_permissions: false }));
}
