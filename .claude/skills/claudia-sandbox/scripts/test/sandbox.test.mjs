import test from "node:test";
import assert from "node:assert/strict";
import { idsDeWsbList } from "../comum.mjs";

test("extrai um id da saida do wsb list", () => {
  const texto = "Id                                   Status   Uptime\n6f446955-fba4-455f-af60-4da1c64f8b3b Running  00:03:10\n";
  assert.deepEqual(idsDeWsbList(texto), ["6f446955-fba4-455f-af60-4da1c64f8b3b"]);
});

test("sem sandbox, nenhum id", () => {
  assert.deepEqual(idsDeWsbList("Nenhum ambiente"), []);
  assert.deepEqual(idsDeWsbList(undefined), []);
});
