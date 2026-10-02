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
