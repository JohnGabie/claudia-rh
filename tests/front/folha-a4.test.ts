import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("the A4 sheet renders the preview in a sandbox that runs no scripts", () => {
  const src = readFileSync(new URL("../../src/components/documentos/FolhaA4.tsx", import.meta.url), "utf8");
  assert.match(src, /sandbox=""/, "an empty sandbox: no scripts, no same-origin");
  assert.doesNotMatch(src, /allow-scripts|allow-same-origin|dangerouslySetInnerHTML/);
  assert.match(src, /srcDoc=/);
});
