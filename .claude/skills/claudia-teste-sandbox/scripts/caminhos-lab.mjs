// PURE (no disk, no git). Where the lab and the work folder live, and whether the work folder's deploy may be wiped.
// Windows path semantics on every platform (path.win32), so the tests also run in CI on Linux.
import path from "node:path";

const w = path.win32;

export const MARCADOR_DEPLOY = ".lab-deploy";
export const USUARIO_SANDBOX = "WDAGUtilityAccount";

/** `git rev-parse --path-format=absolute --git-common-dir` output -> main checkout root. Throws on anything else. */
export function raizPrincipalDe(gitCommonDir) {
  const d = String(gitCommonDir ?? "").trim();
  if (d === "") throw new Error("git common dir is empty");
  if (d.startsWith("\\\\") || d.startsWith("//")) throw new Error(`git common dir must be an absolute local path: ${d}`);
  const n = w.normalize(d).replace(/\\+$/, "");
  if (!/^[a-z]:\\/i.test(n)) throw new Error(`git common dir must be an absolute local path: ${d}`);
  if (w.basename(n).toLowerCase() !== ".git") throw new Error(`unexpected git common dir (expected <checkout>\\.git): ${d}`);
  const raiz = w.dirname(n);
  if (w.parse(raiz).root === raiz) throw new Error(`refusing a main checkout at a drive root: ${raiz}`);
  return raiz;
}

export function caminhosDoLab(raizPrincipal) {
  const lab = w.join(raizPrincipal, "sandbox-lab");
  const trabalho = w.join(raizPrincipal, ".sandbox-work");
  return {
    lab,
    trabalho,
    deploy: w.join(trabalho, "deploy"),
    guestRw: w.join(trabalho, "guest-rw"),
    appStage: w.join(lab, "scripts", "app-stage.ps1"),
    testUp: w.join(lab, "scripts", "test-up.ps1"),
  };
}

/** Folders from `dir` up to, not including, `raiz` for which `ehLink(p)` is true. `ehLink` is injected. */
export function linksAte(dir, raiz, ehLink) {
  const fim = raiz.replace(/\\+$/, "").toLowerCase();
  const achados = [];
  let atual = dir;
  while (atual.replace(/\\+$/, "").toLowerCase() !== fim) {
    if (ehLink(atual)) achados.push(atual);
    const pai = w.dirname(atual);
    if (pai === atual) throw new Error(`${dir} is not inside ${raiz}`);
    atual = pai;
  }
  return achados;
}

/** Old contract (B3), kept for callers: null when allowed, otherwise the reason. */
export function podeLimparDeploy({ existe, temMarcador, ehLink }) {
  if (!existe) return null;
  if (ehLink) return "deploy is a link (symlink/junction/reparse point)";
  if (!temMarcador) return `deploy exists without the ${MARCADOR_DEPLOY} marker (not created by this script)`;
  return null;
}

/**
 * Whether publicar/app-stage may wipe `deploy`. null = allowed, otherwise the reason.
 * Allow-list: `deploy` must be exactly <raizPrincipal>\.sandbox-work\deploy (same case-insensitive string, normalized),
 * with no link in it or in .sandbox-work, and, when it exists, with the marker.
 */
export function motivoRecusaLimparDeploy({ deploy, raizPrincipal, existe, temMarcador, linksNoCaminho = [] }) {
  if (typeof deploy !== "string" || deploy.trim() === "" || typeof raizPrincipal !== "string" || raizPrincipal.trim() === "") {
    return "deploy and raizPrincipal are required";
  }
  if (raizPrincipal.startsWith("\\\\") || !/^[a-z]:\\/i.test(raizPrincipal)) return "raizPrincipal must be an absolute local path";
  if (w.parse(raizPrincipal).root === raizPrincipal) return "raizPrincipal is a drive root";
  const esperado = w.join(raizPrincipal, ".sandbox-work", "deploy");
  if (deploy !== w.normalize(deploy) || deploy.toLowerCase() !== esperado.toLowerCase()) return `deploy is not exactly ${esperado}`;
  if (linksNoCaminho.length > 0) return `link (symlink/junction) in the path: ${linksNoCaminho[0]}`;
  return podeLimparDeploy({ existe, temMarcador, ehLink: false });
}

/**
 * PURE. Copy of `env` without the variables that redirect git to another repository (git sets them inside hooks and
 * worktree commands). Used for the child process of every read-only git call, so `-C <dir>` is what decides the repo.
 */
export function ambienteGitLimpo(env) {
  const limpo = {};
  for (const [k, v] of Object.entries(env ?? {})) {
    if (!/^GIT_(DIR|COMMON_DIR|WORK_TREE)$/i.test(k)) limpo[k] = v;
  }
  return limpo;
}

/**
 * Only exists inside the guest: the lab's sandbox\ folder is mapped there (sandbox.template.wsb). The same literal lives in
 * travas.ps1 ($script:LabGuestMarker); a pure test on each side pins it.
 * Why the guards differ: PowerShell reads the full DOMAIN\user from WindowsIdentity and requires "\WDAGUtilityAccount".
 * Node gets the identity from os.userInfo(), which may come without a domain, so estaNoSandbox also accepts the bare user
 * name; both need this marker file as the second factor.
 */
export const MARCADOR_GUEST = "C:\\labsandbox\\bootstrap.ps1";

/**
 * PURE. True only with BOTH factors: the process identity ends in WDAGUtilityAccount (DOMAIN\user or bare user) and the
 * guest-only marker file exists. Inputs are injected (identity from the OS, `temBootstrap` from a disk check made by
 * the caller); an environment variable is never read.
 */
export function estaNoSandbox({ identidade, temBootstrap } = {}) {
  const partes = String(identidade ?? "").split("\\");
  const usuario = partes[partes.length - 1];
  return usuario.toLowerCase() === USUARIO_SANDBOX.toLowerCase() && temBootstrap === true;
}

/**
 * Root of every folder the Sandbox tests create (spec A6): a VHD mounted as Z: inside the guest only (sandbox\disco-z.ps1).
 * travas.ps1 keeps the same literal ($script:LabGuestTestRoot); a pure test on each side pins it.
 */
export const RAIZ_TESTE_GUEST = "Z:\\claudia-teste-sandbox";

/** PURE. null when `real` (from fs.realpathSync.native) is the guest test root or inside it; otherwise the reason. */
export function motivoRecusaRaizTeste(real) {
  const r = String(real ?? "");
  if (r.trim() === "") return "path is required";
  if (r.split(/[\\/]+/).some((s) => s === "." || s === "..")) return "path has '.' or '..' segments";
  if (w.normalize(r) !== r) return "path is not normalized";
  const k = r.replace(/\\+$/, "").toLowerCase();
  const raiz = RAIZ_TESTE_GUEST.toLowerCase();
  if (k === raiz || k.startsWith(raiz + "\\")) return null;
  return `real path is not under ${RAIZ_TESTE_GUEST}`;
}

/** PURE. tempRoot for scenario folders (D1, spec addendum 8.1): Z:\ inside the guest; null on the PC, where scenarios never run. */
export function raizDeCenarios({ noSandbox } = {}) {
  return noSandbox === true ? "Z:\\" : null;
}
