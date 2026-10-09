// Pure tests of the guest command builder for scenarios (D1): strings only, no disk, no wsb.
import test from "node:test";
import assert from "node:assert/strict";
import { comandoDeCenarioNoGuest } from "../cenario-no-guest.mjs";

const NONCE = "0123abcd0123abcd";

test("novo and mock: fixed guest script, nonce, nothing else", () => {
  assert.equal(comandoDeCenarioNoGuest({ cenario: "novo", nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario novo -Nonce ${NONCE}`);
  assert.equal(comandoDeCenarioNoGuest({ cenario: "mock", nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario mock -Nonce ${NONCE}`);
});

test("copia: needs --autorizado; --manter-automacao passes through", () => {
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "copia", nonce: NONCE }), /autorizado/);
  assert.equal(comandoDeCenarioNoGuest({ cenario: "copia", autorizado: true, nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario copia -Nonce ${NONCE} -Autorizado`);
  assert.equal(comandoDeCenarioNoGuest({ cenario: "copia", autorizado: true, manterAutomacao: true, nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario copia -Nonce ${NONCE} -Autorizado -ManterAutomacao`);
});

test("refuses unknown scenarios, injected text, flags on the wrong scenario and a bad nonce", () => {
  for (const c of ["", "Novo", "copia; del x", "../copia", "outro"]) assert.throws(() => comandoDeCenarioNoGuest({ cenario: c, nonce: NONCE }), /unknown scenario/, c);
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "novo", autorizado: true, nonce: NONCE }), /only apply to copia/);
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "mock", manterAutomacao: true, nonce: NONCE }), /only apply to copia/);
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "novo", nonce: "x&del" }), /nonce/);
});
