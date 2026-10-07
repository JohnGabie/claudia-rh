// Pure rules for the claudia-teste-e2e skill. No I/O here, so every rule that
// protects the user can be pinned by a test that needs no running app.

import path from "node:path";

const PAGINA_APP =/localhost:1420|tauri\.localhost/;

/** The app's WebView2 reports an Edge build and serves the page on :1420 (tauri dev) or tauri.localhost (release). */
export function validarVersaoApp(versao, paginas) {
  if (!versao || typeof versao.Browser !== "string") {
    return { ok: false, motivo: "sem campo Browser: a porta não responde como CDP" };
  }
  if (!/^Edg\//.test(versao.Browser)) {
    return { ok: false, motivo: `Browser é ${versao.Browser}, esperado Edg/ (WebView2). A porta é de outro processo` };
  }
  const temApp = Array.isArray(paginas) && paginas.some((p) => p.type === "page" && PAGINA_APP.test(p.url || ""));
  if (!temApp) {
    return { ok: false, motivo: "Edg encontrado, mas sem página do app (localhost:1420 ou tauri.localhost)" };
  }
  return { ok: true, motivo: "porta do app confirmada" };
}

/** A CDP host other than this machine means the app runs elsewhere (e.g. a sandbox guest). */
export function hostRemoto(host) {
  return !!host && !/^(127\.0\.0\.1|localhost|::1)$/i.test(host);
}

/** Never restart the user's app on our own: running without CDP means ask. Remote targets are never started from here. */
export function decidirSubida({ appRodando, appTemCdp, remoto = false }) {
  if (remoto) return appTemCdp ? "reutilizar" : "abortar";
  if (!appRodando) return "subir";
  return appTemCdp ? "reutilizar" : "perguntar";
}

/** A button we cannot name, or that deletes/sends, is never clicked blind. */
export function rotuloPerigoso(rotulo) {
  if (!rotulo || !String(rotulo).trim()) return true;
  return /apagar|excluir|remover|delete|enviar|submit|disparar/i.test(rotulo);
}

const PREFIXOS_LEITURA = /^(obter|listar|ler|verificar|consultar|contar|abrir_leitura)_/;

/** Unknown prefixes count as writes: calling a mutating command by accident is the costly mistake. */
export function classificarComando(nome) {
  return PREFIXOS_LEITURA.test(nome || "") ? "leitura" : "escrita";
}

/** The DB is opened read-only as well; this rule just fails early with a clear reason. */
export function sqlSomenteLeitura(sql) {
  const s = String(sql ?? "").trim().replace(/;+\s*$/, "");
  if (!s) return { ok: false, motivo: "SQL vazio" };
  if (s.includes(";")) return { ok: false, motivo: "um único statement por chamada" };
  if (!/^select\b/i.test(s)) return { ok: false, motivo: "só SELECT; escrita passa por invocar.mjs --escrever" };
  if (/\b(insert|update|delete|drop|alter|create|attach|pragma|replace)\b/i.test(s)) {
    return { ok: false, motivo: "palavra de escrita dentro do SELECT" };
  }
  return { ok: true, motivo: "ok" };
}

/** R12: an installed/normal claudia-rh.exe without CDP is the user's session; never touch it, ask. */
export function decidirComAppInstalado({ processos, cdpResponde }) {
  // processos null/undefined = the process list could not be read: fail closed, ask.
  if (!Array.isArray(processos)) {
    return cdpResponde
      ? { acao: "seguir", motivo: "ok (CDP de dev responde)" }
      : { acao: "perguntar", motivo: "não consegui verificar se o app instalado está aberto" };
  }
  const rodando = processos.some((p) => String(p).toLowerCase() === "claudia-rh.exe");
  if (rodando && !cdpResponde) {
    return { acao: "perguntar", motivo: "claudia-rh.exe já está rodando sem CDP de dev (provavelmente o app instalado do usuário); não subir por cima" };
  }
  return { acao: "seguir", motivo: "ok" };
}

/** B4: environment for the `tauri dev` we launch: CDP port plus automation off for this run only. */
export function ambienteSubida(base, porta) {
  return {
    ...base,
    CLAUDIA_SEM_AUTOMACAO: "1",
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${porta}`,
  };
}

const LISTA_NEGRA = /^(apagar_|excluir_|remover_|limpar_|desinstalar|instalar_|iniciar_|disparar_|enviar_|guardar_|configurar_|escrever_|resolver_|marcar_|renomear_|importar_|exportar_|abrir_|gerar_|sugerir_)|^(browser_abrir|browser_retry)$/;

/** All string values inside a JSON-ish argument, at any depth. */
function textos(v, acc = []) {
  if (typeof v === "string") acc.push(v);
  else if (Array.isArray(v)) v.forEach((x) => textos(x, acc));
  else if (v && typeof v === "object") Object.values(v).forEach((x) => textos(x, acc));
  return acc;
}

/** A URL/UNC/scheme handed to abrir_* would open the user's default (personal) browser or a network share. */
function naoEhCaminhoLocal(s) {
  const t = s.trim();
  return /^(\\\\|\/\/)/.test(t) || /:\/\//.test(t) || /^[a-z][a-z0-9+.-]+:/i.test(t);
}

/** B5: decides whether invocar.mjs may call a command. flags = { escrever, confirmo }; args = parsed JSON argument. */
export function classificarInvocacao(comando, flags = {}, args = undefined) {
  const c = String(comando ?? "");
  if (c.startsWith("abrir_") &&textos(args).some(naoEhCaminhoLocal)) {
    return { ok: false, motivo: `"${c}" só aceita caminho de pasta local; URL, UNC ou esquema é sempre recusado` };
  }
  if (c === "obter_credencial") return { ok: false, motivo: "obter_credencial devolve a senha em texto puro: sempre recusado" };
  // plugin:* (e.g. plugin:opener|open_url opens the user's default, personal browser): never, even confirmed.
  if (/^plugin:/i.test(c)) return { ok: false, motivo: "comandos plugin:* são sempre recusados (ex.: plugin:opener|open_url abriria o navegador pessoal do usuário)" };
  if (LISTA_NEGRA.test(c)) {
    if (!flags.escrever) return { ok: false, motivo: `"${c}" é destrutivo ou dispara ação real: exige --escrever e --confirmo=${c}` };
    if (flags.confirmo !== c) return { ok: false, motivo: `"${c}" é destrutivo ou dispara ação real: repita com --confirmo=${c}` };
    return { ok: true, motivo: "confirmado" };
  }
  if (classificarComando(c) === "escrita" && !flags.escrever) {
    return { ok: false, motivo: `"${c}" não é de leitura reconhecida. Se grava de propósito, repita com --escrever` };
  }
  return { ok: true, motivo: "ok" };
}

/** R9: screenshots only go to <repo>/debug-logs/prints/, never overwrite. Pure: `existe` is injected. */
export function destinoPrintValido(arq, raizRepo, existe) {
  if (!arq || !String(arq).trim()) return { ok: false, motivo: "arquivo vazio" };
  const a = String(arq);
  if (a.split(/[\\/]/).includes("..")) return { ok: false, motivo: "'..' não é permitido" };
  if (path.isAbsolute(a) || path.win32.isAbsolute(a) || path.posix.isAbsolute(a)) return { ok: false, motivo: "caminho absoluto não é permitido; use debug-logs/prints/<nome>.png" };
  if (!/\.png$/i.test(a)) return { ok: false, motivo: "só .png" };
  const pasta = path.resolve(raizRepo, "debug-logs", "prints");
  const destino = path.resolve(raizRepo, a);
  if (!destino.startsWith(pasta + path.sep)) return { ok: false, motivo: "só dentro de debug-logs/prints/" };
  if (existe(destino)) return { ok: false, motivo: "o arquivo já existe; não sobrescrevo" };
  return { ok: true, motivo: "ok", destino };
}

/** Reuse an already-running dev app only if this skill started it (state file) and that process is still alive. */
export function decidirReutilizacao({ estado, portaApp, pidVivo }) {
  const gravado = estado && typeof estado === "object" && Number.isInteger(estado.pid) && estado.porta === portaApp;
  if (gravado && pidVivo) return { acao: "reutilizar", motivo: "app de dev iniciado por subir.mjs, ainda vivo" };
  return { acao: "perguntar", motivo: "app de dev já aberto sem garantia de automação desligada" };
}

/** M4: the prints folder must be a real directory (not a link) whose real path is inside the real repo root. */
export function pastaPrintsSegura({ raizReal, pastaReal, ehLink }) {
  if (!raizReal || !pastaReal) return { ok: false, motivo: "caminho real indisponível" };
  if (ehLink) return { ok: false, motivo: "debug-logs/prints é link/junction; recusado" };
  const base = raizReal.replace(/[\\/]+$/, "");
  if (!pastaReal.startsWith(base + "/") && !pastaReal.startsWith(base + "\\")) {
    return { ok: false, motivo: "o caminho real de debug-logs/prints está fora da raiz do repositório" };
  }
  return { ok: true, motivo: "ok" };
}
