import test from "node:test";
import assert from "node:assert/strict";
import { validarVersaoApp, decidirSubida, podeMatarDonoDaPorta } from "../regras.mjs";

const versaoApp = { Browser: "Edg/154.0.4258.48", "Protocol-Version": "1.3" };
const paginaApp = [{ type: "page", url: "http://localhost:1420/" }];

test("aceita a porta quando é Edge com a página do app", () => {
  assert.equal(validarVersaoApp(versaoApp, paginaApp).ok, true);
});

test("rejeita Chrome comum na porta (caso real: 9222 era o Chrome pessoal)", () => {
  const r = validarVersaoApp({ Browser: "Chrome/153.0.0.0" }, paginaApp);
  assert.equal(r.ok, false);
  assert.match(r.motivo, /Edg/);
});

test("rejeita Edge sem a página do app", () => {
  const r = validarVersaoApp(versaoApp, [{ type: "page", url: "https://example.com/" }]);
  assert.equal(r.ok, false);
  assert.match(r.motivo, /1420/);
});

test("rejeita resposta vazia ou sem Browser", () => {
  assert.equal(validarVersaoApp({}, []).ok, false);
  assert.equal(validarVersaoApp(null, null).ok, false);
});

test("app rodando sem CDP: pergunta, nunca reinicia sozinha", () => {
  assert.equal(decidirSubida({ appRodando: true, appTemCdp: false }), "perguntar");
});

test("app rodando com CDP: reutiliza", () => {
  assert.equal(decidirSubida({ appRodando: true, appTemCdp: true }), "reutilizar");
});

test("app parado: sobe", () => {
  assert.equal(decidirSubida({ appRodando: false, appTemCdp: false }), "subir");
});

test("só mata dono da porta que seja vite ou tauri", () => {
  assert.equal(podeMatarDonoDaPorta("node.exe ... vite --port 1420"), true);
  assert.equal(podeMatarDonoDaPorta("claudia-rh.exe"), true);
  assert.equal(podeMatarDonoDaPorta('"C:\Program Files\Google\Chrome\Application\chrome.exe"'), false);
  assert.equal(podeMatarDonoDaPorta(""), false);
});

import { rotuloPerigoso } from "../regras.mjs";

test("rótulos destrutivos ou de envio são perigosos", () => {
  for (const r of ["Apagar perfil", "Excluir", "Remover vaga", "Enviar candidatura", "Submit application"]) {
    assert.equal(rotuloPerigoso(r), true, r);
  }
});

test("rótulos de navegação não são perigosos", () => {
  for (const r of ["Dashboard", "Profile", "History", "Notifications 9+", "Terminal"]) {
    assert.equal(rotuloPerigoso(r), false, r);
  }
});

test("rótulo vazio é tratado como perigoso: sem nome não há como julgar", () => {
  assert.equal(rotuloPerigoso(""), true);
  assert.equal(rotuloPerigoso(undefined), true);
});

import { classificarComando, sqlSomenteLeitura } from "../regras.mjs";

test("comandos com prefixo de leitura são leitura", () => {
  for (const c of ["obter_modo_autonomo", "obter_config_disparo", "listar_pendencias", "ler_candidato_base", "verificar_setup"]) {
    assert.equal(classificarComando(c), "leitura", c);
  }
});

test("comandos que gravam são escrita", () => {
  for (const c of ["configurar_modo_autonomo", "guardar_pesos_variantes", "apagar_curriculo", "iniciar_pty", "escrever_pty"]) {
    assert.equal(classificarComando(c), "escrita", c);
  }
});

test("prefixo desconhecido é escrita: na dúvida, exige --escrever", () => {
  assert.equal(classificarComando("fazer_algo_novo"), "escrita");
  assert.equal(classificarComando(""), "escrita");
});

test("aceita SELECT simples", () => {
  assert.equal(sqlSomenteLeitura("SELECT COUNT(*) FROM vagas").ok, true);
  assert.equal(sqlSomenteLeitura("  select id from sessoes order by id desc limit 3;").ok, true);
});

test("recusa escrita e DDL", () => {
  for (const s of ["INSERT INTO vagas VALUES (1)", "UPDATE vagas SET status='x'", "DELETE FROM vagas",
                   "DROP TABLE vagas", "ATTACH DATABASE 'x' AS y", "PRAGMA writable_schema=1", "CREATE TABLE t(a)"]) {
    assert.equal(sqlSomenteLeitura(s).ok, false, s);
  }
});

test("recusa múltiplos statements, mesmo que o primeiro seja SELECT", () => {
  assert.equal(sqlSomenteLeitura("SELECT 1; DELETE FROM vagas").ok, false);
});

test("recusa SQL vazio", () => {
  assert.equal(sqlSomenteLeitura("").ok, false);
});
