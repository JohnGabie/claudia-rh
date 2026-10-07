// PURE (no disk access of its own; lstat and stat are injected). Decision of the `copia` scenario about links in the source.
// The copy never follows a link (fs.cpSync dereference:false) and never reproduces one. A symlink or junction in the source
// makes the whole copy fail, with the relative path in the message, with ONE exception: the FILE link diagnostics\latest
// (made by the Claude Code debug logger, pointing to claude-startup.log in the same folder). That one is skipped, not copied.
import path from "node:path";

/**
 * PURE. "pular" (leave the entry out of the copy), "recusar" (fail), or null (a regular entry).
 * `ehLink` comes from lstat (symlinks and junctions are both links); `ehPasta` says whether the link is to a folder
 * (or a junction, or could not be told: then it is true, the conservative answer).
 */
export function motivoLinkNaOrigem(rel, { ehLink, ehPasta }) {
  if (!ehLink) return null;
  const norm = String(rel).replace(/\//g, "\\").toLowerCase();
  if (norm === "diagnostics\\latest" && !ehPasta) return "pular";
  return "recusar";
}

export function mensagemLinkNaOrigem(rel) {
  return `link (symlink/junction) in the source data: ${rel === "" ? "(the source folder itself)" : rel}. The copy scenario refuses links instead of following them.`;
}

/**
 * The `filter` option of fs.cpSync: returns false for the tolerated link, throws on any other link, true otherwise.
 * `lstat` and `stat` are injected (fs.lstatSync / fs.statSync in the scenario, fakes in the pure test). `stat` is only
 * called for a link, to learn if its target is a folder; if it throws (dangling), the link counts as a folder.
 */
export function filtroSemLinks(origem, lstat, stat = () => ({ isDirectory: () => true })) {
  return (src) => {
    const rel = path.win32.relative(origem, src); // Windows path semantics on every platform
    const ehLink = lstat(src).isSymbolicLink();
    let ehPasta = true;
    if (ehLink) {
      try { ehPasta = stat(src).isDirectory(); } catch { ehPasta = true; }
    }
    const decisao = motivoLinkNaOrigem(rel, { ehLink, ehPasta });
    if (decisao === "pular") return false;
    if (decisao) throw new Error(mensagemLinkNaOrigem(rel));
    return true;
  };
}
