import test from "node:test";
import assert from "node:assert/strict";
import { idsDeWsbList, erroDeExec } from "../comum.mjs";

test("extrai um id da saida do wsb list", () => {
  const texto = "Id                                   Status   Uptime\n6f446955-fba4-455f-af60-4da1c64f8b3b Running  00:03:10\n";
  assert.deepEqual(idsDeWsbList(texto), ["6f446955-fba4-455f-af60-4da1c64f8b3b"]);
});

test("sem sandbox, nenhum id", () => {
  assert.deepEqual(idsDeWsbList("Nenhum ambiente"), []);
  assert.deepEqual(idsDeWsbList(undefined), []);
});

test("erroDeExec: status 0 returns null", () => {
  const r = { status: 0, stdout: "ok", stderr: "" };
  assert.strictEqual(erroDeExec(r, "test-id", "System"), null);
});

test("erroDeExec: non-zero status returns error", () => {
  const r = { status: 1, stdout: "", stderr: "permission denied" };
  const err = erroDeExec(r, "abc-123", "ExistingLogin");
  assert.ok(err instanceof Error);
  assert.match(err.message, /wsb exec failed.*status 1.*abc-123.*ExistingLogin.*permission denied/);
});

test("erroDeExec: uses stdout if stderr empty", () => {
  const r = { status: 2, stdout: "error output", stderr: "" };
  const err = erroDeExec(r, "id", "System");
  assert.match(err.message, /error output/);
});

test("erroDeExec: null status returns error", () => {
  const r = { status: null, stdout: "", stderr: "no status" };
  const err = erroDeExec(r, "id", "System");
  assert.ok(err instanceof Error);
  assert.match(err.message, /wsb exec failed/);
});
