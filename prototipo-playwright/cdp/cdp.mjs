// Cliente CDP minimo — WebSocket nativo do Node 24, sem dependencias.
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function alvos(port) {
  const r = await fetch(`http://127.0.0.1:${port}/json/list`);
  return r.json();
}

export async function ligar(url) {
  const ws = new WebSocket(url);
  let id = 0;
  const pend = new Map();
  const eventos = [];
  ws.addEventListener("message", (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id && pend.has(m.id)) { pend.get(m.id)(m); pend.delete(m.id); }
    else if (m.method) eventos.push(m);
  });
  await new Promise((res, rej) => {
    ws.addEventListener("open", res, { once: true });
    ws.addEventListener("error", rej, { once: true });
  });
  const cdp = (method, params = {}) =>
    new Promise((res) => {
      const i = ++id;
      pend.set(i, res);
      ws.send(JSON.stringify({ id: i, method, params }));
      setTimeout(() => { if (pend.has(i)) { pend.delete(i); res({ error: { message: "TIMEOUT" } }); } }, 20000);
    });
  return { ws, cdp, eventos, fechar: () => ws.close() };
}

export async function novaAba(port, url) {
  const r = await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`, { method: "PUT" });
  return r.json();
}

export const ok = (b) => (b ? "OK" : "FALHOU");
