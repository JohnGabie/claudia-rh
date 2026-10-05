import { test } from "node:test";
import assert from "node:assert/strict";
import { filtrar, valoresDeTag, ehPreferido, avisoAoApagar, preferidoDaVariante, comTag, semTag } from "../../src/components/documentos/logica.ts";

const doc = (id: number, extra: Partial<Record<string, unknown>> = {}) => ({
  id, tipo: "curriculo", nome: `CV ${id}`, origem: "gerado", formato: "pdf", idioma: "pt", tags: [], tamanho: 1,
  criado_em: "2026-10-04", vaga_id: null, preferido_global: false, preferido_variantes: [], tem_preview: true, ...extra,
}) as any;

const be = { tipo: "variante", valor: "backend" } as const;
const ats = { tipo: "template", valor: "classic-ats" } as const;

test("filters by every chosen tag kind at once", () => {
  const docs = [doc(1, { tags: [be, ats] }), doc(2, { tags: [be] }), doc(3, { tags: [ats] })];
  assert.deepEqual(filtrar(docs, { variante: "backend", template: "classic-ats" }).map((d: any) => d.id), [1]);
  assert.deepEqual(filtrar(docs, {}).map((d: any) => d.id), [1, 2, 3], "no filter, everything");
});

test("tag values for a filter are distinct and sorted", () => {
  const docs = [doc(1, { tags: [be] }), doc(2, { tags: [{ tipo: "variante", valor: "devops" }, be] })];
  assert.deepEqual(valoresDeTag(docs, "variante"), ["backend", "devops"]);
});

test("the star comes from the backend fields, never from local state", () => {
  assert.equal(ehPreferido(doc(1, { preferido_global: true }), null), true);
  assert.equal(ehPreferido(doc(1, { preferido_variantes: ["backend"] }), "backend"), true);
  assert.equal(ehPreferido(doc(1, { preferido_variantes: ["backend"] }), null), false);
});

test("deleting a preferred document warns, the general mark first", () => {
  assert.equal(avisoAoApagar(doc(1)), null);
  assert.equal(avisoAoApagar(doc(1, { preferido_variantes: ["x"] })), "preferido_variante");
  assert.equal(avisoAoApagar(doc(1, { preferido_global: true, preferido_variantes: ["x"] })), "preferido_global");
});

test("the variant picker shows the document marked for that variant", () => {
  const docs = [doc(1), doc(2, { preferido_variantes: ["backend"] })];
  assert.equal(preferidoDaVariante(docs, "backend")?.id, 2);
  assert.equal(preferidoDaVariante(docs, "devops"), null);
});

test("editing tags trims, refuses empty and never duplicates", () => {
  assert.deepEqual(comTag([be], { tipo: "variante", valor: " backend " }), [be]);
  assert.deepEqual(comTag([], { tipo: "livre", valor: "  " }), []);
  assert.deepEqual(semTag([be, ats], be), [ats]);
});
