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
  const rodando = (processos ?? []).some((p) => String(p).toLowerCase() === "claudia-rh.exe");
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

const LISTA_NEGRA = /^(apagar_|excluir_|remover_|limpar_|desinstalar|instalar_|iniciar_|disparar_|enviar_)/;

/** B5: decides whether invocar.mjs may call a command. flags = { escrever, confirmo }. */
export function classificarInvocacao(comando, flags = {}) {
  const c = String(comando ?? "");
  if (c === "obter_credencial") return { ok: false, motivo: "obter_credencial devolve a senha em texto puro: sempre recusado" };
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
