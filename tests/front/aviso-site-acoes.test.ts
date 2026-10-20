import { test } from "node:test";
import assert from "node:assert/strict";
import { acoesAvisoSite } from "../../src/components/avisoSiteAcoes.ts";

test("I logged in is hidden until the site was opened", () => {
  const a = acoesAvisoSite({ abriu: false, ocupado: false });
  assert.equal(a.mostrarJaEntrei, false);
  assert.equal(a.podeJaEntrei, false);
  assert.equal(a.podeAbrir, true);
});

test("after opening, I logged in is available", () => {
  const a = acoesAvisoSite({ abriu: true, ocupado: false });
  assert.equal(a.mostrarJaEntrei, true);
  assert.equal(a.podeJaEntrei, true);
});

test("everything is disabled while a call is in flight", () => {
  const a = acoesAvisoSite({ abriu: true, ocupado: true });
  assert.equal(a.podeAbrir, false);
  assert.equal(a.podeJaEntrei, false);
});
