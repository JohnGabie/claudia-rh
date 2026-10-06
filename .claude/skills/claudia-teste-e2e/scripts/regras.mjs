// Pure rules for the claudia-teste-e2e skill. No I/O here, so every rule that
// protects the user can be pinned by a test that needs no running app.

const PAGINA_APP = /localhost:1420|tauri\.localhost/;

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
