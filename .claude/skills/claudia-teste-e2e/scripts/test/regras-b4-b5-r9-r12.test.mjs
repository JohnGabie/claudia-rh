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
  assert.equal(classificarInvocacao("pular_pendencia", {}).ok, false);
  assert.equal(classificarInvocacao("pular_pendencia", { escrever: true }).ok, true);
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

// I3: fail closed when the process list cannot be read
test("I3: lista de processos ilegível sem CDP: pergunta", () => {
  for (const p of [null, undefined]) {
    const r = decidirComAppInstalado({ processos: p, cdpResponde: false });
    assert.equal(r.acao, "perguntar");
    assert.match(r.motivo, /não consegui verificar/);
  }
});

// I4: blacklist completeness
test("I4: plugin:* é sempre recusado, mesmo com --escrever e --confirmo", () => {
  for (const c of ["plugin:opener|open_url", "plugin:shell|open", "Plugin:fs|remove"]) {
    assert.equal(classificarInvocacao(c, { escrever: true, confirmo: c }).ok, false, c);
  }
});

// Literal copy of generate_handler! in src-tauri/src/lib.rs (module paths dropped).
const COMANDOS_APP = [
  "iniciar_pty", "escrever_pty", "redimensionar_pty", "parar_pty", "minimize_window", "maximize_window", "close_window",
  "listar_vagas", "listar_pendencias", "resumo_memoria_recente", "candidaturas_hoje", "atividade_recente", "contar_pendencias",
  "resolver_pendencia", "pular_pendencia", "listar_candidaturas", "contar_propostas", "listar_propostas", "ignorar_proposta",
  "vaga_atual_sessao", "vagas_analisadas_hoje", "vagas_analisadas_total", "tempo_sessoes_hoje", "abrir_pasta",
  "ler_candidato_base", "guardar_candidato_base", "ler_search_variants", "guardar_search_variants", "guardar_pesos_variantes",
  "guardar_variante_unica", "ler_estrategia", "guardar_estrategia", "iniciar_sessao_perfil", "iniciar_sessao_perfil_chrome",
  "enviar_mensagem_perfil", "interromper_perfil", "remover_ultima_troca_perfil", "escrever_para_perfil_chrome",
  "guardar_credencial", "obter_credencial", "obter_utilizador_credencial", "apagar_credencial", "disparar_sessao",
  "obter_config_disparo", "configurar_disparo", "configurar_limite_diario", "obter_modo_autonomo", "configurar_modo_autonomo",
  "registar_pausa_sessao", "registar_retoma_sessao", "obter_config_notif", "configurar_notif", "agregar_dados_feedback",
  "gerar_feedback", "listar_feedbacks", "marcar_resultado_candidatura", "sugerir_feedback", "gerar_curriculo",
  "gerar_cover_letter", "abrir_ficheiro_prompt", "abrir_pasta_dados", "obter_iniciar_com_sistema", "configurar_iniciar_com_sistema",
  "iniciar_busca_linkedin_rede", "listar_vagas_linkedin_rede", "obter_status_linkedin_rede", "verificar_atualizacao",
  "instalar_atualizacao", "verificar_setup", "welcome_necessario", "marcar_welcome_concluido", "diag_heartbeat",
  "exportar_diagnostico", "activity_log_list", "listar_documentos", "importar_documento", "definir_preferido", "obter_documento",
  "renomear_documento", "atualizar_tags_documento", "remover_preferido", "abrir_documento", "apagar_documento",
  "listar_variantes", "activity_log_get", "activity_log_image", "browser_status", "browser_retry", "browser_abrir",
];
const LEITURA = /^(obter|listar|verificar|ler|consultar|contar)_/;
const PERIGOSO = /^(apagar_|excluir_|remover_|limpar_|desinstalar|instalar_|iniciar_|disparar_|enviar_|guardar_|configurar_|escrever_|resolver_|marcar_|renomear_|importar_|exportar_)/;

test("I4: todo comando do app sem prefixo de leitura exige --escrever; os perigosos exigem --confirmo", () => {
  for (const c of COMANDOS_APP) {
    if (c === "obter_credencial") {
      assert.equal(classificarInvocacao(c, { escrever: true, confirmo: c }).ok, false, c);
    } else if (LEITURA.test(c)) {
      assert.equal(classificarInvocacao(c, {}).ok, true, `${c} (leitura)`);
    } else {
      assert.equal(classificarInvocacao(c, {}).ok, false, `${c} sem --escrever`);
      if (PERIGOSO.test(c)) {
        assert.equal(classificarInvocacao(c, { escrever: true }).ok, false, `${c} sem --confirmo`);
        assert.equal(classificarInvocacao(c, { escrever: true, confirmo: c }).ok, true, `${c} confirmado`);
      } else {
        assert.equal(classificarInvocacao(c, { escrever: true }).ok, true, `${c} com --escrever`);
      }
    }
  }
});
