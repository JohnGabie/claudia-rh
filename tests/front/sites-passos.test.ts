import { test } from "node:test";
import assert from "node:assert/strict";
import type { Site } from "../../src/components/sites/logica.ts";
import {
  estadoVisual, mostraJaEntrei, passo1, podeAlternarSemLogin, pronto, semLogin, situacaoFaixa,
} from "../../src/components/sites/estado.ts";
import { pt } from "../../src/i18n/pt.ts";
import { en } from "../../src/i18n/en.ts";

const site = (extra: Partial<Site> = {}): Site => ({
  chave: "x", nome: "X", url_login: "https://x.com/", dominios: ["x.com"], dominios_ats: [], caminho_ats: null,
  tipo: "geral", regiao: "GLOBAL", pais: null, papel: "candidatura", regra_login: "manual", ia_pode_usar: false,
  embutida: true, padrao_atual: true, estado: "desconhecido", verificada_em: null, ...extra,
});

test("pronto: true for no-login sites in any state, true for manual + logada, false otherwise", () => {
  assert.equal(pronto(site({ regra_login: "nenhuma", estado: "desconhecido" })), true);
  assert.equal(pronto(site({ regra_login: "nenhuma", estado: "precisa_login" })), true);
  assert.equal(pronto(site({ regra_login: "manual", estado: "logada" })), true);
  assert.equal(pronto(site({ regra_login: "manual", estado: "precisa_login" })), false);
  assert.equal(pronto(site({ regra_login: "manual", estado: "desconhecido" })), false);
});

test("semLogin is true only for regra_login nenhuma", () => {
  assert.equal(semLogin(site({ regra_login: "nenhuma" })), true);
  assert.equal(semLogin(site({ regra_login: "manual" })), false);
});

test("situacaoFaixa gives falta, fechada and ligada", () => {
  assert.equal(situacaoFaixa(site({ regra_login: "manual", estado: "desconhecido", ia_pode_usar: false })), "falta");
  assert.equal(situacaoFaixa(site({ regra_login: "manual", estado: "logada", ia_pode_usar: false })), "fechada");
  assert.equal(situacaoFaixa(site({ regra_login: "manual", estado: "logada", ia_pode_usar: true })), "ligada");
});

test("passo1 gives semLogin, logado and pendente", () => {
  assert.equal(passo1(site({ regra_login: "nenhuma" })), "semLogin");
  assert.equal(passo1(site({ regra_login: "manual", estado: "logada" })), "logado");
  assert.equal(passo1(site({ regra_login: "manual", estado: "precisa_login" })), "pendente");
});

test("podeAlternarSemLogin is false only for cookie_sessao", () => {
  assert.equal(podeAlternarSemLogin(site({ regra_login: "cookie_sessao" })), false);
  assert.equal(podeAlternarSemLogin(site({ regra_login: "manual" })), true);
  assert.equal(podeAlternarSemLogin(site({ regra_login: "nenhuma" })), true);
});

test("mostraJaEntrei only after opening and while step 1 is pending", () => {
  const pendente = site({ regra_login: "manual", estado: "desconhecido" });
  const logado = site({ regra_login: "manual", estado: "logada" });
  assert.equal(mostraJaEntrei(false, pendente), false);
  assert.equal(mostraJaEntrei(true, pendente), true);
  assert.equal(mostraJaEntrei(true, logado), false);
});

test("estadoVisual of a no-login site is semlogin with the noLogin text", () => {
  const v = estadoVisual(site({ regra_login: "nenhuma" }), new Date(), pt);
  assert.equal(v.cor, "semlogin");
  assert.equal(v.texto, pt.sites.noLogin);
});

test("type labels: all 7 keys exist in pt and en, pt matches the table", () => {
  const tabela: Record<string, string> = {
    geral: "Portal de vagas", regional: "Portal regional", tecnologia: "Vagas de tecnologia", remoto: "Vagas remotas",
    freelance: "Plataforma freelance", empresa: "Site da empresa", nicho: "Portal especializado",
  };
  for (const k of Object.keys(tabela)) {
    const p = (pt.sites.types as Record<string, string>)[k];
    const e = (en.sites.types as Record<string, string>)[k];
    assert.ok(typeof p === "string" && p.length > 0, `pt ${k}`);
    assert.ok(typeof e === "string" && e.length > 0, `en ${k}`);
    assert.equal(p, tabela[k]);
  }
});
