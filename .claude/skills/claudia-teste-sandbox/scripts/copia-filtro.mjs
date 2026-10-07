// PURE (no disk access of its own; the lstat is injected). Decision of the `copia` scenario about links in the source.
// The copy never follows a link (fs.cpSync dereference:false) and never reproduces one: a symlink or junction anywhere in
// the source makes the whole copy fail, with the relative path in the message.
import path from "node:path";

/** PURE. Reason to refuse an entry of the source, or null. `ehLink` comes from lstat (symlinks and junctions are both links). */
export function motivoLinkNaOrigem(rel, ehLink) {
  if (!ehLink) return null;
  return `link (symlink/junction) in the source data: ${rel === "" ? "(the source folder itself)" : rel}. The copy scenario refuses links instead of following them.`;
}

/**
 * The `filter` option of fs.cpSync: refuses (throws) on a link, allows everything else.
 * `lstat` is injected (fs.lstatSync in the scenario, a fake in the pure test).
 */
export function filtroSemLinks(origem, lstat) {
  return (src) => {
    const rel = path.win32.relative(origem, src); // Windows path semantics on every platform
    const motivo = motivoLinkNaOrigem(rel, lstat(src).isSymbolicLink());
    if (motivo) throw new Error(motivo);
    return true;
  };
}
