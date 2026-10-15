import { test } from "node:test";
import assert from "node:assert/strict";
import { localDoSite, nomePais } from "../../src/components/sites/paises.ts";

const regioes = { GLOBAL: "Global", EU: "Europa" };
const site = (pais: string | null, regiao = "GLOBAL") => ({ pais, regiao });

test("country codes become full pt-BR names", () => {
  assert.equal(localDoSite(site("BR"), "pt", regioes), "Brasil");
  assert.equal(localDoSite(site("DK"), "pt", regioes), "Dinamarca");
  assert.equal(localDoSite(site("GB"), "pt", regioes), "Reino Unido");
  assert.equal(localDoSite(site("US"), "pt", regioes), "Estados Unidos");
});

test("English names are used for the en locale", () => {
  assert.equal(localDoSite(site("DK"), "en", regioes), "Denmark");
});

test("a site without country shows its region label", () => {
  assert.equal(localDoSite(site(null, "GLOBAL"), "pt", regioes), "Global");
  assert.equal(localDoSite(site(null, "EU"), "pt", regioes), "Europa");
  assert.equal(localDoSite(site(null, "XX"), "pt", regioes), "XX");
});

test("unknown or malformed codes fall back to the code without throwing", () => {
  assert.equal(nomePais("ZZ", "pt"), "ZZ");
  assert.doesNotThrow(() => nomePais("not a code", "pt"));
  assert.equal(nomePais("not a code", "en"), "not a code");
});
