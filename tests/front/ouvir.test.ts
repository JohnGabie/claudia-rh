import { test } from "node:test";
import assert from "node:assert/strict";
import { criarOuvinte } from "../../src/components/documentos/ouvirCore.ts";

// Fake clock: timers run only when the test calls `avancar`.
function relogio() {
  let pend: { f: () => void; id: number }[] = []; let n = 0;
  return {
    set: (f: () => void) => { pend.push({ f, id: ++n }); return n; },
    clear: (h: unknown) => { pend = pend.filter((p) => p.id !== h); },
    avancar: () => { const ps = pend; pend = []; ps.forEach((p) => p.f()); },
    temPendente: () => pend.length > 0,
  };
}

// A fake event source that records subscribers and how many were released.
function fonte(atraso = false) {
  const cbs: (() => void)[] = []; let liberados = 0; let resolver: (() => void) | null = null;
  const assinar = (cb: () => void) => {
    cbs.push(cb);
    const un = () => { liberados++; };
    if (!atraso) return un;
    return new Promise<() => void>((ok) => { resolver = () => ok(un); });
  };
  return { assinar, emitir: () => cbs.forEach((c) => c()), get liberados() { return liberados; }, resolver: () => resolver!() };
}

const tick = () => new Promise((r) => setImmediate(r));

test("a burst of events reloads once", async () => {
  const rel = relogio(); const f = fonte(); let n = 0;
  criarOuvinte(() => n++, [f.assinar], 150, rel);
  await tick();
  f.emitir(); f.emitir(); f.emitir();
  assert.equal(n, 0);
  rel.avancar();
  assert.equal(n, 1);
});

test("each source triggers the reload", async () => {
  const rel = relogio(); const a = fonte(); const b = fonte(); let n = 0;
  criarOuvinte(() => n++, [a.assinar, b.assinar], 150, rel);
  await tick();
  a.emitir(); rel.avancar(); b.emitir(); rel.avancar();
  assert.equal(n, 2);
});

test("teardown releases every subscription and cancels a pending reload", async () => {
  const rel = relogio(); const a = fonte(); const b = fonte(); let n = 0;
  const parar = criarOuvinte(() => n++, [a.assinar, b.assinar], 150, rel);
  await tick();
  a.emitir();
  parar();
  assert.equal(a.liberados + b.liberados, 2);
  assert.equal(rel.temPendente(), false);
  rel.avancar();
  assert.equal(n, 0);
  a.emitir();
  assert.equal(rel.temPendente(), false);
});

test("unmounting before listen resolves still releases the late subscription", async () => {
  const rel = relogio(); const f = fonte(true);
  const parar = criarOuvinte(() => {}, [f.assinar], 150, rel);
  parar();
  assert.equal(f.liberados, 0);
  f.resolver();
  await tick();
  assert.equal(f.liberados, 1);
});

// The Sites screen listens to every Tauri event that can change what it shows.
test("the Sites screen subscribes to db-atualizada, novo-aviso-site and aviso-site-resolvido", async () => {
  const { fontesTauri } = await import("../../src/components/sites/ouvirEventos.ts");
  const handlers = new Map<string, () => void>();
  const escutar = async (ev: string, cb: () => void) => { handlers.set(ev, cb); return () => {}; };
  const rel = relogio(); let n = 0;
  criarOuvinte(() => n++, fontesTauri(escutar), 150, rel);
  await tick();
  assert.deepEqual([...handlers.keys()].sort(), ["aviso-site-resolvido", "db-atualizada", "novo-aviso-site"]);
  for (const ev of ["novo-aviso-site", "aviso-site-resolvido"]) {
    const antes = n;
    handlers.get(ev)!();
    rel.avancar();
    assert.equal(n, antes + 1, `${ev} must reload the screen`);
  }
});
