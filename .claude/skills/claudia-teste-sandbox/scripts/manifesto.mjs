// PURE (no disk). Tools manifest (sandbox-lab/shared/tools.sha256) and list comparison for the lab copy.
// Line format: "<sha256 hex> <size in bytes> <path relative to the listed root, with />".

const LINHA = /^([0-9a-f]{64}) (\d+) (.+)$/;
const chave = (c) => String(c).replace(/\\/g, "/").toLowerCase();

export function escreverManifesto(entradas) {
  const linhas = [...entradas]
    .map((e) => ({ ...e, caminho: e.caminho.replace(/\\/g, "/") }))
    .sort((a, b) => (a.caminho < b.caminho ? -1 : a.caminho > b.caminho ? 1 : 0))
    .map((e) => `${e.sha256.toLowerCase()} ${e.tamanho} ${e.caminho}`);
  return linhas.join("\n") + "\n";
}

export function lerManifesto(texto) {
  const entradas = [];
  String(texto ?? "").split(/\r?\n/).forEach((l, i) => {
    if (l.trim() === "") return;
    const m = l.match(LINHA);
    if (!m) throw new Error(`manifest line ${i + 1} is malformed: ${l}`);
    entradas.push({ sha256: m[1], tamanho: Number(m[2]), caminho: m[3] });
  });
  return entradas;
}

/** Expected vs found, by path (case-insensitive, / or \), then size, then sha256 when both sides have it. */
export function compararListas(esperado, achado) {
  const mapa = new Map(achado.map((e) => [chave(e.caminho), e]));
  const vistos = new Set();
  const faltando = [];
  const diferentes = [];
  for (const e of esperado) {
    const k = chave(e.caminho);
    vistos.add(k);
    const a = mapa.get(k);
    if (!a) { faltando.push(e.caminho); continue; }
    if (a.tamanho !== e.tamanho) diferentes.push({ caminho: e.caminho, motivo: `size ${a.tamanho} != ${e.tamanho}` });
    else if (e.sha256 && a.sha256 && e.sha256.toLowerCase() !== a.sha256.toLowerCase()) diferentes.push({ caminho: e.caminho, motivo: "sha256 differs" });
  }
  const sobrando = achado.filter((a) => !vistos.has(chave(a.caminho))).map((a) => a.caminho);
  return { faltando, diferentes, sobrando };
}

/** True when `caminho` is one of `excluir` or inside one of them (folder prefix followed by "/"). */
export function excluido(caminho, excluir) {
  const k = chave(caminho);
  return excluir.some((e) => { const x = chave(e).replace(/\/+$/, ""); return k === x || k.startsWith(x + "/"); });
}
