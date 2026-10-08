// PURE (no disk access of its own; lstat is injected). Decision of the `copia` scenario about links in the source.
// The copy never follows a link (fs.cpSync dereference:false) and never reproduces one. A symlink or junction in the source
// makes the whole copy fail, with the relative path in the message, with ONE exception: the FILE link diagnostics\latest
// (made by the Claude Code debug logger, pointing to claude-startup.log). That one is skipped, not copied.
//
// Who decides that it is a FILE link: the PC, not the guest. The real link holds the ABSOLUTE PC path of its target, which
// does not exist in the guest, and Node offers no way to tell a file symlink from a folder symlink/junction without following
// it (fs.lstatSync only says "symbolic link"; statSync follows and fails on a dangling target). So test-up checks the link
// on the PC (file link, exact target) and writes only the boolean to guest.config.json; the guest gets it as the flag
// --latest-link-aprovado and skips exactly this one name. The guest never calls stat or readlink on any link.
import path from "node:path";

/**
 * PURE. "pular" (leave the entry out of the copy), "recusar" (fail), or null (a regular entry).
 * `ehLink` comes from lstat (symlinks and junctions are both links); `latestAprovado` is the PC's decision (see above):
 * only a literal true counts.
 */
export function motivoLinkNaOrigem(rel, { ehLink, latestAprovado = false }) {
  if (!ehLink) return null;
  const norm = String(rel).replace(/\//g, "\\").toLowerCase();
  if (norm === "diagnostics\\latest" && latestAprovado === true) return "pular";
  return "recusar";
}

export function mensagemLinkNaOrigem(rel) {
  return `link (symlink/junction) in the source data: ${rel === "" ? "(the source folder itself)" : rel}. The copy scenario refuses links instead of following them.`;
}

/**
 * The `filter` option of fs.cpSync: returns false for the approved link, throws on any other link, true otherwise.
 * `lstat` is injected (fs.lstatSync in the scenario, a fake in the pure test). Nothing else is called: a link is never
 * followed, read or measured.
 */
export function filtroSemLinks(origem, lstat, { latestAprovado = false } = {}) {
  return (src) => {
    const rel = path.win32.relative(origem, src); // Windows path semantics on every platform
    const ehLink = lstat(src).isSymbolicLink();
    const decisao = motivoLinkNaOrigem(rel, { ehLink, latestAprovado });
    if (decisao === "pular") return false;
    if (decisao) throw new Error(mensagemLinkNaOrigem(rel));
    return true;
  };
}
