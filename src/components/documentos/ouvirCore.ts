// Pure core of the "reload when documents change" listener: coalesces bursts of events into one
// reload and guarantees every subscription is released, even when the component unmounts before an
// async `listen` has resolved. `assinar` entries are injected so this runs under `node --test`.
export type Desassinar = () => void;
export type Assinar = (aoEvento: () => void) => Promise<Desassinar> | Desassinar;

export function criarOuvinte(
  recarregar: () => void,
  fontes: Assinar[],
  debounceMs = 150,
  agendar: { set: (f: () => void, ms: number) => unknown; clear: (h: unknown) => void } = {
    set: (f, ms) => setTimeout(f, ms),
    clear: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
  },
): Desassinar {
  let vivo = true;
  let timer: unknown = null;
  const liberar: Desassinar[] = [];

  const aoEvento = () => {
    if (!vivo) return;
    if (timer !== null) agendar.clear(timer);
    timer = agendar.set(() => { timer = null; if (vivo) recarregar(); }, debounceMs);
  };

  for (const assinar of fontes) {
    Promise.resolve(assinar(aoEvento)).then(
      (un) => { if (vivo) liberar.push(un); else un(); },
      () => {},
    );
  }

  return () => {
    vivo = false;
    if (timer !== null) { agendar.clear(timer); timer = null; }
    liberar.splice(0).forEach((un) => un());
  };
}
