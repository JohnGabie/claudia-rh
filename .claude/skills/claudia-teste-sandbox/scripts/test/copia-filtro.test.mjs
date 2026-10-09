// Pure tests: lstat is a fake and the paths are invented. Nothing is read, created or deleted.
import test from "node:test";
import assert from "node:assert/strict";
import { motivoLinkNaOrigem, filtroSemLinks } from "../copia-filtro.mjs";

const ORIGEM = "Y:\\inventado\\origem";
// links: Set of paths that are links. The fake lstat is the ONLY call the filter may make.
const lstatFalso = (links) => (p) => ({ isSymbolicLink: () => links.has(p) });

test("motivoLinkNaOrigem: null for a regular entry", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\app.log", { ehLink: false }), null);
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: false, latestAprovado: true }), null);
});

test("motivoLinkNaOrigem: only diagnostics\\latest, with the PC's approval, is skipped", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: true, latestAprovado: true }), "pular");
  assert.equal(motivoLinkNaOrigem("Diagnostics/LATEST", { ehLink: true, latestAprovado: true }), "pular");
});

test("motivoLinkNaOrigem: without the approval (absent, false or not a literal true) the link refuses", () => {
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: true }), "recusar");
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: true, latestAprovado: false }), "recusar");
  for (const v of ["true", 1, "sim", {}, [], null]) assert.equal(motivoLinkNaOrigem("diagnostics\\latest", { ehLink: true, latestAprovado: v }), "recusar", String(v));
});

test("motivoLinkNaOrigem: even approved, another level or another name refuses", () => {
  const ap = { ehLink: true, latestAprovado: true };
  assert.equal(motivoLinkNaOrigem("x\\diagnostics\\latest", ap), "recusar");
  assert.equal(motivoLinkNaOrigem("latest", ap), "recusar");
  assert.equal(motivoLinkNaOrigem("diagnostics\\other", ap), "recusar");
  assert.equal(motivoLinkNaOrigem("diagnostics\\latest\\sub", ap), "recusar");
  assert.equal(motivoLinkNaOrigem("", ap), "recusar");
});

test("filtroSemLinks: allows regular entries (returns true)", () => {
  const f = filtroSemLinks(ORIGEM, lstatFalso(new Set()), { latestAprovado: true });
  assert.equal(f(ORIGEM), true);
  assert.equal(f(`${ORIGEM}\\app.db`), true);
  assert.equal(f(`${ORIGEM}\\sub\\x.json`), true);
  assert.equal(f(`${ORIGEM}\\diagnostics\\claude-startup.log`), true);
});

test("filtroSemLinks: the approved link diagnostics\\latest is skipped (false) with only lstat, the real log is kept", () => {
  const links = new Set([`${ORIGEM}\\diagnostics\\latest`]);
  const f = filtroSemLinks(ORIGEM, lstatFalso(links), { latestAprovado: true });
  assert.equal(f(`${ORIGEM}\\diagnostics`), true);
  assert.equal(f(`${ORIGEM}\\diagnostics\\latest`), false);
  assert.equal(f(`${ORIGEM}\\diagnostics\\claude-startup.log`), true);
});

test("filtroSemLinks: the link is never followed or measured (the fake exposes nothing but isSymbolicLink)", () => {
  const links = new Set([`${ORIGEM}\\diagnostics\\latest`]);
  const estrito = (p) => new Proxy({ isSymbolicLink: () => links.has(p) }, { get(t, k) { if (k in t) return t[k]; throw new Error(`the filter touched ${String(k)}`); } });
  assert.equal(filtroSemLinks(ORIGEM, estrito, { latestAprovado: true })(`${ORIGEM}\\diagnostics\\latest`), false);
  assert.throws(() => filtroSemLinks(ORIGEM, estrito)(`${ORIGEM}\\diagnostics\\latest`), /link/);
});

test("filtroSemLinks: without the approval, diagnostics\\latest throws like any link", () => {
  const links = new Set([`${ORIGEM}\\diagnostics\\latest`]);
  assert.throws(() => filtroSemLinks(ORIGEM, lstatFalso(links))(`${ORIGEM}\\diagnostics\\latest`), /link .*diagnostics\\latest/);
});

test("filtroSemLinks: any other link throws, naming it, even with the approval", () => {
  for (const rel of ["x\\diagnostics\\latest", "latest", "outro"]) {
    const links = new Set([`${ORIGEM}\\${rel}`]);
    const f = filtroSemLinks(ORIGEM, lstatFalso(links), { latestAprovado: true });
    assert.throws(() => f(`${ORIGEM}\\${rel}`), new RegExp(`link .*${rel.replace(/\\/g, "\\\\")}`));
  }
});

test("filtroSemLinks: the source folder itself being a link throws", () => {
  const links = new Set([ORIGEM]);
  const f = filtroSemLinks(ORIGEM, lstatFalso(links), { latestAprovado: true });
  assert.throws(() => f(ORIGEM), /source folder itself/);
});

// Static order check on the scenario script's TEXT (read only; the script is not run): cenario.json is the last thing written,
// so a copy that fails half way never leaves one, and the approval flag reaches the filter.
import fs from "node:fs";
test("cenario-copia.mjs: cenario.json is written only after the copy, the fold and the automation switch-off", () => {
  const src = fs.readFileSync(new URL("../cenario-copia.mjs", import.meta.url), "utf8");
  const copia = src.indexOf("copiarUmaVez(origem, dados)");
  const desliga = src.indexOf("desligarAutomacao(dados)");
  const escreve = src.indexOf("escreverCenario(");
  assert.ok(copia > 0 && desliga > copia && escreve > desliga, `order: copy ${copia}, switch-off ${desliga}, cenario.json ${escreve}`);
  assert.equal(src.split("escreverCenario(").length - 1, 1, "cenario.json must be written in exactly one place");
});

test("cenario-copia.mjs: the approval flag reaches the filter and the filter gets no stat", () => {
  const src = fs.readFileSync(new URL("../cenario-copia.mjs", import.meta.url), "utf8");
  assert.match(src, /"latest-link-aprovado":\s*\{\s*type:\s*"boolean"/);
  assert.match(src, /filtroSemLinks\(origem, fs\.lstatSync, \{ latestAprovado: values\["latest-link-aprovado"\] === true \}\)/);
  assert.doesNotMatch(src, /filtroSemLinks\([^)]*fs\.statSync/);
});
