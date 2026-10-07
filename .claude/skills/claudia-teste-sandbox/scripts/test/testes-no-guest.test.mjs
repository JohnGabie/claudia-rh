// Pure tests of the guest command builder: strings only.
import test from "node:test";
import assert from "node:assert/strict";
import { comandoDeTesteNoGuest } from "../testes-no-guest.mjs";

const NONCE = "0123abcd0123abcd";

test("node tests: explicit files, guest temp under C:\\labdata, nonce first", () => {
  const c = comandoDeTesteNoGuest({ node: [".claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs"], nonce: NONCE });
  assert.equal(c, `set LAB_NONCE=${NONCE}&& set TEMP=C:\\labdata\\tmp&& set TMP=C:\\labdata\\tmp&& (if not exist C:\\labdata\\tmp mkdir C:\\labdata\\tmp)&& node --test .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs`);
});

test("ps test: only sandbox-lab/scripts/test/*.sandbox.ps1, with backslashes", () => {
  const c = comandoDeTesteNoGuest({ ps: "sandbox-lab/scripts/test/travas.sandbox.ps1", nonce: NONCE });
  assert.match(c, /&& powershell -NoProfile -ExecutionPolicy Bypass -File sandbox-lab\\scripts\\test\\travas\.sandbox\.ps1$/);
});

test("refuses globs, folders, '..', spaces, quotes and other files", () => {
  for (const f of ["*.test.mjs", ".claude/skills/x/", ".claude/skills/../../x.test.mjs", ".claude/skills/a b.test.mjs", ".claude/skills/a\".test.mjs", "src/x.test.mjs"]) {
    assert.throws(() => comandoDeTesteNoGuest({ node: [f], nonce: NONCE }), /not a skill test file|'\.\.'/, f);
  }
  assert.throws(() => comandoDeTesteNoGuest({ ps: "sandbox-lab/scripts/test-up.ps1", nonce: NONCE }), /not a lab Sandbox test/);
});

test("needs exactly one kind of test and a hex nonce", () => {
  assert.throws(() => comandoDeTesteNoGuest({ nonce: NONCE }), /either/);
  assert.throws(() => comandoDeTesteNoGuest({ node: [".claude/skills/a/x.test.mjs"], ps: "sandbox-lab/scripts/test/t.sandbox.ps1", nonce: NONCE }), /either/);
  assert.throws(() => comandoDeTesteNoGuest({ node: [".claude/skills/a/x.test.mjs"], nonce: "x&del" }), /nonce/);
});
