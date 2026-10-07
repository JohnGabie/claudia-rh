// Pure tests: lstat/stat are fakes and the paths are invented. Nothing is read, created or deleted.
import test from "node:test";
import assert from "node:assert/strict";
import { motivoLinkNaOrigem, filtroSemLinks } from "../copia-filtro.mjs";

const ORIGEM = "Y:\\inventado\\origem";
// links: Map path -> { pasta: boolean } (a missing entry is a regular file or folder)
const lstatFalso = (links) => (p) => ({ isSymbolicLink: () => links.has(p) });
const statFalso = (links) => (p) => ({ isDirectory: () => links.get(p)?.pasta ?? false });

test("motivoLinkNaOrigem: null for a regular entry", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\app.log", { ehLink: false, ehPasta: false }), null);
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: false, ehPasta: false }), null);
});

test("motivoLinkNaOrigem: only diagnostics\\latest as a FILE link is skipped", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: true, ehPasta: false }), "pular");
  assert.equal(motivoLinkNaOrigem("Diagnostics/LATEST", { ehLink: true, ehPasta: false }), "pular");
});

test("motivoLinkNaOrigem: the same name as a folder link or junction, at another level, or another name refuses", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: true, ehPasta: true }), "recusar");
  assert.equal(motivoLinkNaOrigem("x\\diagnostics\\latest", { ehLink: true, ehPasta: false }), "recusar");
  assert.equal(motivoLinkNaOrigem("latest", { ehLink: true, ehPasta: false }), "recusar");
  assert.equal(motivoLinkNaOrigem("diagnostics\\other", { ehLink: true, ehPasta: false }), "recusar");
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest\\sub", { ehLink: true, ehPasta: false }), "recusar");
  assert.equal(motivoLinkNaOrigem("", { ehLink: true, ehPasta: false }), "recusar");
});

test("filtroSemLinks: allows regular entries (returns true)", () => {
  const links = new Map();
  const f = filtroSemLinks(ORIGEM, lstatFalso(links), statFalso(links));
  assert.equal(f(ORIGEM), true);
  assert.equal(f(`${ORIGEM}\\app.db`), true);
  assert.equal(f(`${ORIGEM}\\sub\\x.json`), true);
  assert.equal(f(`${ORIGEM}\\diagnostics\\claude-startup.log`), true);
});

test("filtroSemLinks: the file link diagnostics\\latest is skipped (false), the real log is kept", () => {
  const links = new Map([[`${ORIGEM}\\diagnostics\\latest`, { pasta: false }]]);
  const f = filtroSemLinks(ORIGEM, lstatFalso(links), statFalso(links));
  assert.equal(f(`${ORIGEM}\\diagnostics`), true);
  assert.equal(f(`${ORIGEM}\\diagnostics\\latest`), false);
  assert.equal(f(`${ORIGEM}\\diagnostics\\claude-startup.log`), true);
});

test("filtroSemLinks: any other link throws, naming it", () => {
  for (const [rel, pasta] of [["diagnostics\\latest", true], ["x\\diagnostics\\latest", false], ["latest", false], ["outro", false]]) {
    const links = new Map([[`${ORIGEM}\\${rel}`, { pasta }]]);
    const f = filtroSemLinks(ORIGEM, lstatFalso(links), statFalso(links));
    assert.throws(() => f(`${ORIGEM}\\${rel}`), new RegExp(`link .*${rel.replace(/\\/g, "\\\\")}`));
  }
});

test("filtroSemLinks: without a stat, or when stat fails (dangling), the link counts as a folder and refuses", () => {
  const links = new Map([[`${ORIGEM}\\diagnostics\\latest`, { pasta: false }]]);
  assert.throws(() => filtroSemLinks(ORIGEM, lstatFalso(links))(`${ORIGEM}\\diagnostics\\latest`), /link/);
  const quebrado = () => { throw new Error("ENOENT"); };
  assert.throws(() => filtroSemLinks(ORIGEM, lstatFalso(links), quebrado)(`${ORIGEM}\\diagnostics\\latest`), /link/);
});

test("filtroSemLinks: the source folder itself being a link throws", () => {
  const links = new Map([[ORIGEM, { pasta: false }]]);
  const f = filtroSemLinks(ORIGEM, lstatFalso(links), statFalso(links));
  assert.throws(() => f(ORIGEM), /source folder itself/);
});
