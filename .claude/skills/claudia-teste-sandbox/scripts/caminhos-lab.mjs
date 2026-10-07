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

export function estaNoSandbox(usuario) {
  return String(usuario ?? "").toLowerCase() === USUARIO_SANDBOX.toLowerCase();
}
