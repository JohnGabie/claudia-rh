// Pure tests: the lstat is a fake and the paths are invented. Nothing is read, created or deleted.
import test from "node:test";
import assert from "node:assert/strict";
import { motivoLinkNaOrigem, filtroSemLinks } from "../copia-filtro.mjs";

const ORIGEM = "Y:\\inventado\\origem";
const lstatFalso = (links) => (p) => ({ isSymbolicLink: () => links.has(p) });

test("motivoLinkNaOrigem: null for a regular entry, a reason naming the entry for a link", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\app.log", false), null);
  assert.match(motivoLinkNaOrigem("diagnostics\\latest", true), /link .*diagnostics\\latest/);
  assert.match(motivoLinkNaOrigem("", true), /source folder itself/);
});

test("filtroSemLinks: allows regular entries (returns true)", () => {
  const f = filtroSemLinks(ORIGEM, lstatFalso(new Set()));
  assert.equal(f(ORIGEM), true);
  assert.equal(f(`${ORIGEM}\\app.db`), true);
  assert.equal(f(`${ORIGEM}\\sub\\x.json`), true);
});

test("filtroSemLinks: a link anywhere in the source throws, naming it", () => {
  const f = filtroSemLinks(ORIGEM, lstatFalso(new Set([`${ORIGEM}\\diagnostics\\latest`])));
  assert.equal(f(`${ORIGEM}\\diagnostics`), true);
  assert.throws(() => f(`${ORIGEM}\\diagnostics\\latest`), /link .*diagnostics\\latest/);
});

test("filtroSemLinks: the source folder itself being a link throws", () => {
  const f = filtroSemLinks(ORIGEM, lstatFalso(new Set([ORIGEM])));
  assert.throws(() => f(ORIGEM), /source folder itself/);
});
