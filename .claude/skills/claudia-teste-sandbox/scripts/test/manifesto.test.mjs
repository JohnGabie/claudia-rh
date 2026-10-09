// Pure tests: in-memory lists only.
import test from "node:test";
import assert from "node:assert/strict";
import { lerManifesto, escreverManifesto, compararListas, excluido } from "../manifesto.mjs";

const H1 = "a".repeat(64);
const H2 = "b".repeat(64);

test("manifest round-trip sorts by path, lowercases the hash and uses forward slashes", () => {
  const texto = escreverManifesto([
    { caminho: "uv\\uv.exe", tamanho: 10, sha256: H2 },
    { caminho: "chrome/chrome-win64/chrome.exe", tamanho: 5, sha256: H1.toUpperCase() },
  ]);
  assert.equal(texto, `${H1} 5 chrome/chrome-win64/chrome.exe\n${H2} 10 uv/uv.exe\n`);
  assert.deepEqual(lerManifesto(texto), [
    { sha256: H1, tamanho: 5, caminho: "chrome/chrome-win64/chrome.exe" },
    { sha256: H2, tamanho: 10, caminho: "uv/uv.exe" },
  ]);
});

test("paths with spaces survive the round-trip", () => {
  const e = [{ caminho: "python/Lib/site packages/x y.py", tamanho: 0, sha256: H1 }];
  assert.deepEqual(lerManifesto(escreverManifesto(e)), [{ sha256: H1, tamanho: 0, caminho: "python/Lib/site packages/x y.py" }]);
});

test("manifest with a malformed line is refused with its line number", () => {
  assert.throws(() => lerManifesto(`${H1} 5 a.txt\nlixo\n`), /line 2/);
  assert.throws(() => lerManifesto(`${H1} -5 a.txt\n`), /line 1/);
  assert.throws(() => lerManifesto(`${"g".repeat(64)} 5 a.txt\n`), /line 1/);
});

test("blank lines and CRLF are accepted", () => {
  assert.equal(lerManifesto(`\r\n${H1} 1 a.txt\r\n\r\n`).length, 1);
});

test("compararListas: identical lists give no differences", () => {
  const l = [{ caminho: "a.txt", tamanho: 1, sha256: H1 }];
  assert.deepEqual(compararListas(l, l), { faltando: [], diferentes: [], sobrando: [] });
});

test("compararListas: missing, extra, size and hash differences", () => {
  const esperado = [
    { caminho: "a.txt", tamanho: 1, sha256: H1 },
    { caminho: "b.txt", tamanho: 2, sha256: H1 },
    { caminho: "c.txt", tamanho: 3, sha256: H1 },
  ];
  const achado = [
    { caminho: "a.txt", tamanho: 1, sha256: H2 },
    { caminho: "b.txt", tamanho: 9, sha256: H1 },
    { caminho: "d.txt", tamanho: 4, sha256: H1 },
  ];
  assert.deepEqual(compararListas(esperado, achado), {
    faltando: ["c.txt"],
    diferentes: [{ caminho: "a.txt", motivo: "sha256 differs" }, { caminho: "b.txt", motivo: "size 9 != 2" }],
    sobrando: ["d.txt"],
  });
});

test("compararListas: case and separators do not matter on Windows", () => {
  const r = compararListas([{ caminho: "Chrome/Chrome.exe", tamanho: 1, sha256: H1 }], [{ caminho: "chrome\\chrome.exe", tamanho: 1, sha256: H1.toUpperCase() }]);
  assert.deepEqual(r, { faltando: [], diferentes: [], sobrando: [] });
});

test("compararListas: without sha256 on one side, size alone decides", () => {
  assert.deepEqual(compararListas([{ caminho: "a", tamanho: 1, sha256: H1 }], [{ caminho: "a", tamanho: 1 }]).diferentes, []);
  assert.equal(compararListas([{ caminho: "a", tamanho: 1, sha256: H1 }], [{ caminho: "a", tamanho: 2 }]).diferentes.length, 1);
});

test("excluido: folder prefix and exact file, never a sibling with the same start", () => {
  const ex = ["shared/tools", "sandbox/guest.config.json"];
  assert.equal(excluido("shared/tools/uv/uv.exe", ex), true);
  assert.equal(excluido("shared/tools", ex), true);
  assert.equal(excluido("shared/tools.sha256", ex), false);
  assert.equal(excluido("Sandbox\\Guest.Config.json", ex), true);
  assert.equal(excluido("sandbox/guest.config.json.bak", ex), false);
  assert.equal(excluido("qualquer", []), false);
});
