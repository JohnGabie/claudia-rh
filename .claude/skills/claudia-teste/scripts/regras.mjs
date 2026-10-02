// Pure rules for the claudia-teste skill. No I/O here, so every rule that
// protects the user can be pinned by a test that needs no running app.

/** The app's WebView2 reports an Edge build and serves the page on :1420. */
export function validarVersaoApp(versao, paginas) {
  if (!versao || typeof versao.Browser !== "string") {
    return { ok: false, motivo: "sem campo Browser: a porta não responde como CDP" };
  }
  if (!/^Edg\//.test(versao.Browser)) {
    return { ok: false, motivo: `Browser é ${versao.Browser}, esperado Edg/ (WebView2). A porta é de outro processo` };
  }
  const temApp = Array.isArray(paginas) && paginas.some((p) => p.type === "page" && /localhost:1420/.test(p.url || ""));
  if (!temApp) {
    return { ok: false, motivo: "Edg encontrado, mas sem página em localhost:1420" };
  }
  return { ok: true, motivo: "porta do app confirmada" };
}

/** Never restart the user's app on our own: running without CDP means ask. */
export function decidirSubida({ appRodando, appTemCdp }) {
  if (!appRodando) return "subir";
  return appTemCdp ? "reutilizar" : "perguntar";
}

/** Only free :1420 when its owner is something we recognise as ours. */
export function podeMatarDonoDaPorta(cmd) {
  return /vite|tauri|claudia-rh/i.test(cmd || "");
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
