import { test } from "node:test";
import assert from "node:assert/strict";
import { rotuloAnexo, rotuloDocumentoCandidatura, ehSemDocumento, lerLinhaUpload, alvoDoUpload, formatarTamanho } from "../../src/components/dashboard/anexos.ts";

test("an attachment reads as name and uppercase format", () => {
  assert.equal(rotuloAnexo({ tipo: "curriculo", documento_id: 3, nome: "CV Backend", formato: "pdf" }, "apagado"), "CV Backend (PDF)");
  assert.equal(rotuloAnexo({ tipo: "carta", documento_id: 4, nome: "Carta", formato: null }, "apagado"), "Carta");
});

test("a deleted attachment shows the given text, never a blank", () => {
  assert.equal(rotuloAnexo({ tipo: "curriculo", documento_id: null, nome: null, formato: null }, "documento apagado"), "documento apagado");
});

test("the history shows the document name, or a dash when there is none", () => {
  assert.equal(rotuloDocumentoCandidatura({ documento_id: 3, documento_nome: "CV Backend" }), "CV Backend");
  assert.equal(rotuloDocumentoCandidatura({ documento_id: null, documento_nome: null }), "—");
});

test("both missing-document categories are recognised, others are not", () => {
  assert.equal(ehSemDocumento("sem_curriculo"), true);
  assert.equal(ehSemDocumento("sem_carta"), true);
  assert.equal(ehSemDocumento("captcha"), false);
});

test("the upload log line is parsed into its parts", () => {
  const u = lerLinhaUpload("documento 7 | CV Backend | pdf | 84213 bytes | sha256 a1b2c3d4e5f6");
  assert.deepEqual(u, { id: 7, nome: "CV Backend", formato: "pdf", bytes: 84213, sha: "a1b2c3d4e5f6" });
  assert.equal(alvoDoUpload("documento 7 | CV Backend | pdf | 84213 bytes | sha256 a1b2c3d4e5f6"), "CV Backend (PDF)");
});

test("a name that contains the separator survives", () => {
  const u = lerLinhaUpload("documento 9 | A | B | pdf | 10 bytes | sha256 abcdef012345");
  assert.equal(u?.nome, "A | B");
  assert.equal(u?.formato, "pdf");
});

test("text that is not an upload line parses to null", () => {
  assert.equal(lerLinhaUpload(null), null);
  assert.equal(lerLinhaUpload("clicou no botão"), null);
  assert.equal(alvoDoUpload("clicou no botão"), null);
});

test("sizes are human readable", () => {
  assert.equal(formatarTamanho(500), "500 B");
  assert.equal(formatarTamanho(84213), "82 KB");
  assert.equal(formatarTamanho(2 * 1048576), "2.0 MB");
});
