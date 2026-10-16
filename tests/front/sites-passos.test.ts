import { test } from "node:test";
import assert from "node:assert/strict";
import type { Site } from "../../src/components/sites/logica.ts";
import {
  bloqueado, estadoVisual, mostraJaEntrei, passo1, podeAlternarSemLogin, pronto, semLogin, situacaoFaixa,
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
  const v = estadoVisual(site({ regra_login: "nenhuma" }), new Date(), pt)!;
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

test("cookie_sessao follows the same release rule as manual: ready only when logged in", () => {
  const nao = site({ regra_login: "cookie_sessao", estado: "precisa_login" });
  const sim = site({ regra_login: "cookie_sessao", estado: "logada" });
  assert.equal(pronto(nao), false);
  assert.equal(bloqueado(nao), true);
  assert.equal(situacaoFaixa(nao), "falta");
  assert.equal(pronto(sim), true);
  assert.equal(bloqueado(sim), false);
});

test("estadoVisual of a not logged cookie_sessao site: red only for precisa_login, nothing for desconhecido", () => {
  const agora = new Date();
  const a = estadoVisual(site({ regra_login: "cookie_sessao", estado: "precisa_login" }), agora, pt)!;
  assert.equal(a.cor, "precisa");
  assert.equal(a.texto, pt.sites.state.precisa_login);
  assert.equal(estadoVisual(site({ regra_login: "cookie_sessao", estado: "desconhecido" }), agora, pt), null);
  const c = estadoVisual(site({ regra_login: "cookie_sessao", estado: "logada", verificada_em: null }), agora, pt)!;
  assert.equal(c.cor, "logada");
});

test("estadoVisual table: precisa_login red, desconhecido nothing, no-login green, logada green", () => {
  const agora = new Date();
  for (const regra of ["manual", "cookie_sessao"] as const) {
    const r = estadoVisual(site({ regra_login: regra, estado: "precisa_login" }), agora, pt)!;
    assert.equal(r.cor, "precisa");
    assert.equal(r.texto, "Precisa de login");
    assert.equal(estadoVisual(site({ regra_login: regra, estado: "precisa_login" }), agora, en)!.texto, "Login required");
    assert.equal(estadoVisual(site({ regra_login: regra, estado: "desconhecido" }), agora, pt), null);
    assert.equal(estadoVisual(site({ regra_login: regra, estado: "desconhecido" }), agora, en), null);
    assert.equal(estadoVisual(site({ regra_login: regra, estado: "logada" }), agora, pt)!.cor, "logada");
  }
  assert.equal(estadoVisual(site({ regra_login: "nenhuma", estado: "desconhecido" }), agora, pt)!.cor, "semlogin");
  assert.equal(estadoVisual(site({ regra_login: "nenhuma", estado: "precisa_login" }), agora, pt)!.cor, "semlogin");
  assert.equal(estadoVisual(site({ regra_login: "nenhuma", estado: "logada" }), agora, pt)!.cor, "logada");
});

test("login-required strings exist in both dictionaries, old wording is gone", () => {
  assert.equal(pt.sites.state.precisa_login, "Precisa de login");
  assert.equal(en.sites.state.precisa_login, "Login required");
});

test("mock definir_site_sem_login(false) mirrors the backend: red state, AI closed, one open notice", async () => {
  const { mockInvoke } = await import("../../src/lib/sites.mock.ts");
  type S = { regra_login: string; estado: string; ia_pode_usar: boolean };
  const get = async (chave: string) => (await mockInvoke<S[] & { chave: string }[]>("listar_sites", {})).find((x) => x.chave === chave) as unknown as S;
  const avisos = async () => (await mockInvoke<{ chave: string }[]>("listar_avisos_sites", {})).filter((a) => a.chave === "jobindex");
  // jobindex starts as a no-login site; going to "asks for login" while logged in must still redden it.
  await mockInvoke("definir_site_sem_login", { chave: "jobindex", valor: false });
  let x = await get("jobindex");
  assert.equal(x.regra_login, "manual");
  assert.equal(x.estado, "precisa_login");
  assert.equal(x.ia_pode_usar, false);
  assert.equal((await avisos()).length, 1);
  await mockInvoke("definir_site_sem_login", { chave: "jobindex", valor: false });
  assert.equal((await avisos()).length, 1, "never duplicates an open notice");
  await mockInvoke("definir_site_sem_login", { chave: "jobindex", valor: true });
  x = await get("jobindex");
  assert.equal(x.regra_login, "nenhuma");
  assert.equal((await avisos()).length, 0, "valor=true resolves the notices");
  await assert.rejects(mockInvoke("definir_site_sem_login", { chave: "linkedin-jobs", valor: false }));
});
