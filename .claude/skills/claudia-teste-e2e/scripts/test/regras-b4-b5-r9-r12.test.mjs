import test from "node:test";
import assert from "node:assert/strict";
import { decidirComAppInstalado, classificarInvocacao, destinoPrintValido, ambienteSubida, decidirReutilizacao, pastaPrintsSegura } from "../regras.mjs";

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

// Hand-written expectations (NOT derived from the rule's own regex), one command per line.
const ESPERADO_LEITURA = [
  "listar_vagas", "listar_pendencias", "contar_pendencias", "listar_candidaturas", "contar_propostas", "listar_propostas",
  "ler_candidato_base", "ler_search_variants", "ler_estrategia", "obter_utilizador_credencial", "obter_config_disparo",
  "obter_modo_autonomo", "obter_config_notif", "listar_feedbacks", "obter_iniciar_com_sistema", "listar_vagas_linkedin_rede",
  "obter_status_linkedin_rede", "verificar_atualizacao", "verificar_setup", "listar_documentos", "obter_documento", "listar_variantes",
];
const ESPERADO_ESCRITA_COMUM = [
  "redimensionar_pty", "parar_pty", "minimize_window", "maximize_window", "close_window", "resumo_memoria_recente",
  "candidaturas_hoje", "atividade_recente", "pular_pendencia", "ignorar_proposta", "vaga_atual_sessao", "vagas_analisadas_hoje",
  "vagas_analisadas_total", "tempo_sessoes_hoje", "interromper_perfil", "registar_pausa_sessao", "registar_retoma_sessao",
  "agregar_dados_feedback", "welcome_necessario", "diag_heartbeat", "activity_log_list", "definir_preferido",
  "atualizar_tags_documento", "activity_log_get", "activity_log_image", "browser_status",
];
const ESPERADO_PERIGOSO = [
  "iniciar_pty", "escrever_pty", "resolver_pendencia", "abrir_pasta", "guardar_candidato_base", "guardar_search_variants",
  "guardar_pesos_variantes", "guardar_variante_unica", "guardar_estrategia", "iniciar_sessao_perfil", "iniciar_sessao_perfil_chrome",
  "enviar_mensagem_perfil", "remover_ultima_troca_perfil", "escrever_para_perfil_chrome", "guardar_credencial", "apagar_credencial",
  "disparar_sessao", "configurar_disparo", "configurar_limite_diario", "configurar_modo_autonomo", "configurar_notif",
  "marcar_resultado_candidatura", "sugerir_feedback", "gerar_feedback", "gerar_curriculo", "gerar_cover_letter",
  "abrir_ficheiro_prompt", "abrir_pasta_dados", "configurar_iniciar_com_sistema", "iniciar_busca_linkedin_rede",
  "instalar_atualizacao", "marcar_welcome_concluido", "exportar_diagnostico", "importar_documento", "renomear_documento",
  "remover_preferido", "abrir_documento", "apagar_documento", "browser_abrir", "browser_retry",
];

test("I4: as listas esperadas cobrem exatamente os comandos do app (comando novo sem classificação falha aqui)", () => {
  const todos = [...ESPERADO_LEITURA, ...ESPERADO_ESCRITA_COMUM, ...ESPERADO_PERIGOSO, "obter_credencial"];
  assert.equal(new Set(todos).size, todos.length, "comando em duas listas");
  assert.deepEqual([...todos].sort(), [...COMANDOS_APP].sort());
});
test("I4: leitura conhecida passa sem flags", () => {
  for (const c of ESPERADO_LEITURA) assert.equal(classificarInvocacao(c, {}).ok, true, c);
});
test("I4: escrita comum exige --escrever e não exige --confirmo", () => {
  for (const c of ESPERADO_ESCRITA_COMUM) {
    assert.equal(classificarInvocacao(c, {}).ok, false, `${c} sem --escrever`);
    assert.equal(classificarInvocacao(c, { escrever: true }).ok, true, `${c} com --escrever`);
  }
});
test("I4: perigosos exigem --escrever e --confirmo=<mesmo comando>", () => {
  for (const c of ESPERADO_PERIGOSO) {
    assert.equal(classificarInvocacao(c, {}).ok, false, `${c} sem flags`);
    assert.equal(classificarInvocacao(c, { escrever: true }).ok, false, `${c} sem --confirmo`);
    assert.equal(classificarInvocacao(c, { escrever: true, confirmo: "outro" }).ok, false, `${c} confirmo errado`);
    assert.equal(classificarInvocacao(c, { escrever: true, confirmo: c }, {}).ok, true, `${c} confirmado`);
  }
});
test("I4: obter_credencial recusado sempre", () => {
  assert.equal(classificarInvocacao("obter_credencial", { escrever: true, confirmo: "obter_credencial" }).ok, false);
});

// Important 1: abrir_* with something that is not a local folder path is always refused.
test("abrir_*: argumento que não é caminho local é sempre recusado, mesmo confirmado", () => {
  const f = { escrever: true, confirmo: "abrir_pasta" };
  for (const v of ["https://example.com", "http://x", "HTTP://x", "mailto:a@b.c", "\\\\servidor\\share", "//servidor/share", "file:///x", "ms-settings:", "ftp://x"]) {
    assert.equal(classificarInvocacao("abrir_pasta", f, { caminho: v }).ok, false, v);
    assert.equal(classificarInvocacao("abrir_pasta", f, { a: { b: [v] } }).ok, false, `aninhado ${v}`);
  }
});
test("abrir_*: caminho local relativo ou com letra de unidade passa quando confirmado", () => {
  const f = { escrever: true, confirmo: "abrir_pasta" };
  assert.equal(classificarInvocacao("abrir_pasta", f, { caminho: "debug-logs" }).ok, true);
  assert.equal(classificarInvocacao("abrir_pasta", f, { caminho: "D:\\pasta" }).ok, true);
});

// Important 2: reuse an existing dev app only if this skill started it (state file) and it is still alive.
test("reutilização: só com estado gravado por subir.mjs, mesma porta e PID vivo", () => {
  assert.equal(decidirReutilizacao({ estado: { pid: 10, porta: 9333 }, portaApp: 9333, pidVivo: true }).acao, "reutilizar");
});
test("reutilização: sem estado, PID morto ou porta diferente: pergunta", () => {
  for (const a of [
    { estado: null, portaApp: 9333, pidVivo: false },
    { estado: { pid: 10, porta: 9333 }, portaApp: 9333, pidVivo: false },
    { estado: { pid: 10, porta: 9334 }, portaApp: 9333, pidVivo: true },
    { estado: { porta: 9333 }, portaApp: 9333, pidVivo: true },
    { estado: "lixo", portaApp: 9333, pidVivo: true },
  ]) {
    const r = decidirReutilizacao(a);
    assert.equal(r.acao, "perguntar");
    assert.match(r.motivo, /sem garantia de automação desligada/);
  }
});

// M4: prints folder must be a real directory (no link) inside the real repo root.
test("M4: pasta de prints dentro da raiz real e sem link: ok", () => {
  assert.equal(pastaPrintsSegura({ raizReal: "/repo", pastaReal: "/repo/debug-logs/prints", ehLink: false }).ok, true);
});
test("M4: pasta que é link é recusada", () => {
  assert.equal(pastaPrintsSegura({ raizReal: "/repo", pastaReal: "/repo/debug-logs/prints", ehLink: true }).ok, false);
});
test("M4: caminho real fora da raiz é recusado (inclusive prefixo parecido)", () => {
  for (const p of ["/outro/prints", "/repo-fora/debug-logs/prints", "/repo", "/"]) {
    assert.equal(pastaPrintsSegura({ raizReal: "/repo", pastaReal: p, ehLink: false }).ok, false, p);
  }
});
test("M4: entradas ausentes são recusadas", () => {
  assert.equal(pastaPrintsSegura({}).ok, false);
});
