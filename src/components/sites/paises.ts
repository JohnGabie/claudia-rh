// Country names in full, from the runtime's own ICU data (no network, no table to maintain).
import type { Site } from "./logica";

export type Idioma = "pt" | "en";

const cache = new Map<string, Intl.DisplayNames | null>();
function nomes(idioma: Idioma): Intl.DisplayNames | null {
  if (!cache.has(idioma)) {
    try {
      cache.set(idioma, new Intl.DisplayNames([idioma === "pt" ? "pt-BR" : "en"], { type: "region" }));
    } catch {
      cache.set(idioma, null);
    }
  }
  return cache.get(idioma) ?? null;
}

/** Full name of an ISO 3166 region code, or the code itself when unknown. Never throws. */
export function nomePais(codigo: string, idioma: Idioma): string {
  if (codigo.toUpperCase() === "ZZ") return codigo; // ICU's "unknown region"
  try {
    return nomes(idioma)?.of(codigo.toUpperCase()) ?? codigo;
  } catch {
    return codigo;
  }
}

/** Where a site applies: its country's full name, or its region label ("Global", "Europa"...) when it has no country. */
export function localDoSite(site: Pick<Site, "pais" | "regiao">, idioma: Idioma, regioes: Record<string, string>): string {
  if (site.pais) return nomePais(site.pais, idioma);
  return regioes[site.regiao] ?? site.regiao;
}
