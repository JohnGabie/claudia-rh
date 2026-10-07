import test from "node:test";
import assert from "node:assert/strict";
import { decidirComAppInstalado, classificarInvocacao, destinoPrintValido, ambienteSubida } from "../regras.mjs";

// R12
test("R12: claudia-rh.exe rodando sem CDP: pergunta", () => {
  const r = decidirComAppInstalado({ processos: ["explorer.exe", "claudia-rh.exe"], cdpResponde: false });
  assert.equal(r.acao, "perguntar");
  assert.match(r.motivo, /claudia-rh\.exe/);
});
test("R12: claudia-rh.exe com CDP respondendo (dev): segue", () => {
  assert.equal(decidirComAppInstalado({ processos: ["claudia-rh.exe"], cdpResponde: true }).acao, "seguir");
});
test("R12: nenhum claudia-rh.exe: segue", () => {
  assert.equal(decidirComAppInstalado({ processos: ["chrome.exe"], cdpResponde: false }).acao, "seguir");
  assert.equal(decidirComAppInstalado({ processos: [], cdpResponde: false }).acao, "seguir");
});
test("R12: nome do processo ignora maiúsculas", () => {
  assert.equal(decidirComAppInstalado({ processos: ["Claudia-RH.EXE"], cdpResponde: false }).acao, "perguntar");
});

// B4
test("B4: ambiente de subida desliga a automação e mantém o resto", () => {
  const e = ambienteSubida({ PATH: "x" }, 9333);
  assert.equal(e.CLAUDIA_SEM_AUTOMACAO, "1");
  assert.equal(e.PATH, "x");
  assert.match(e.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, /--remote-debugging-port=9333/);
});

// B5
test("B5: obter_credencial é sempre recusado, mesmo com tudo", () => {
  assert.equal(classificarInvocacao("obter_credencial", { escrever: true, confirmo: "obter_credencial" }).ok, false);
  assert.equal(classificarInvocacao("obter_credencial", {}).ok, false);
});
test("B5: leitura passa sem flags", () => {
  assert.equal(classificarInvocacao("listar_pendencias", {}).ok, true);
});
test("B5: escrita comum exige --escrever", () => {
  assert.equal(classificarInvocacao("configurar_modo_autonomo", {}).ok, false);
  assert.equal(classificarInvocacao("configurar_modo_autonomo", { escrever: true }).ok, true);
});
test("B5: lista negra recusa mesmo com --escrever", () => {
  for (const c of ["apagar_curriculo", "excluir_x", "remover_x", "limpar_x", "desinstalar_app", "instalar_atualizacao", "iniciar_sessao", "disparar_agora", "enviar_candidatura"]) {
    assert.equal(classificarInvocacao(c, { escrever: true }).ok, false, c);
  }
});
test("B5: lista negra só passa com --confirmo igual ao comando", () => {
  assert.equal(classificarInvocacao("apagar_curriculo", { escrever: true, confirmo: "apagar_curriculo" }).ok, true);
  assert.equal(classificarInvocacao("apagar_curriculo", { escrever: true, confirmo: "outro" }).ok, false);
});
test("B5: confirmo sem --escrever não basta", () => {
  assert.equal(classificarInvocacao("apagar_curriculo", { confirmo: "apagar_curriculo" }).ok, false);
});

// R9
const RAIZ = "/repo";
const nao = () => false;
test("R9: aceita arquivo novo dentro de debug-logs/prints", () => {
  assert.equal(destinoPrintValido("debug-logs/prints/a.png", RAIZ, nao).ok, true);
});
test("R9: recusa fora da pasta de prints", () => {
  for (const a of ["tela.png", "debug-logs/a.png", "src/a.png", "debug-logs/prints", "debug-logs/prints/"]) {
    assert.equal(destinoPrintValido(a, RAIZ, nao).ok, false, a);
  }
});
test("R9: recusa .. em qualquer forma", () => {
  for (const a of ["debug-logs/prints/../../x.png", "debug-logs\\prints\\..\\x.png", "../debug-logs/prints/x.png"]) {
    assert.equal(destinoPrintValido(a, RAIZ, nao).ok, false, a);
  }
});
test("R9: recusa caminho absoluto", () => {
  assert.equal(destinoPrintValido("/etc/x.png", RAIZ, nao).ok, false);
  assert.equal(destinoPrintValido("C:\\x\\a.png", RAIZ, nao).ok, false);
});
test("R9: recusa arquivo que já existe (sem sobrescrever)", () => {
  assert.equal(destinoPrintValido("debug-logs/prints/a.png", RAIZ, () => true).ok, false);
});
test("R9: recusa vazio e exige .png", () => {
  assert.equal(destinoPrintValido("", RAIZ, nao).ok, false);
  assert.equal(destinoPrintValido(undefined, RAIZ, nao).ok, false);
  assert.equal(destinoPrintValido("debug-logs/prints/a.txt", RAIZ, nao).ok, false);
});
