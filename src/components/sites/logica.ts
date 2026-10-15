// Pure search, filter and sort of the site list. No React, no Tauri: tested with node --test.

export interface Site {
  chave: string;
  nome: string;
  url_login: string;
  dominios: string[];
  dominios_ats: string[];
  caminho_ats: string | null;
  tipo: string;
  regiao: string;
  pais: string | null;
  papel: "candidatura" | "descoberta";
  regra_login: string;
  ia_pode_usar: boolean;
  embutida: boolean;
  padrao_atual: boolean;
  estado: "logada" | "precisa_login" | "desconhecido";
  verificada_em: string | null;
}

export interface Filtros { busca: string; tipo: string; regiao: string; pais: string }
export type Campo = "nome" | "tipo" | "regiao" | "pais" | "estado";
export interface Ordem { campo: Campo; desc: boolean }

// NFD does not decompose these, so fold them by hand ("orsted" must find "Ørsted").
const DOBRAS: Record<string, string> = { "ø": "o", "æ": "ae", "œ": "oe", "ß": "ss", "đ": "d", "ł": "l", "þ": "th" };
const sem = (t: string) => t.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[øæœßđłþ]/g, (ch) => DOBRAS[ch]);

export function filtrar(sites: Site[], f: Filtros): Site[] {
  const q = sem(f.busca.trim());
  return sites.filter((s) => {
    if (f.tipo && s.tipo !== f.tipo) return false;
    if (f.regiao && s.regiao !== f.regiao) return false;
    if (f.pais && s.pais !== f.pais) return false;
    if (!q) return true;
    return sem(s.nome).includes(q) || s.dominios.some((d) => sem(d).includes(q));
  });
}

const RANK_ESTADO: Record<Site["estado"], number> = { logada: 0, desconhecido: 1, precisa_login: 2 };
const porNome = (a: Site, b: Site) => a.nome.localeCompare(b.nome, "pt", { sensitivity: "base", numeric: true }) || a.chave.localeCompare(b.chave);

function chave(s: Site, campo: Campo): string | number {
  if (campo === "estado") return RANK_ESTADO[s.estado];
  if (campo === "nome") return s.nome;
  return s[campo] ?? ""; // null countries are placed by ordenar, never by this key
}

export function ordenar(sites: Site[], o: Ordem): Site[] {
  const dir = o.desc ? -1 : 1;
  return [...sites].sort((a, b) => {
    // sites without a country go last in BOTH directions, so the direction must not flip them
    if (o.campo === "pais" && (a.pais === null) !== (b.pais === null)) return a.pais === null ? 1 : -1;
    const ka = chave(a, o.campo);
    const kb = chave(b, o.campo);
    const c = typeof ka === "number" && typeof kb === "number"
      ? ka - kb
      : String(ka).localeCompare(String(kb), "pt", { sensitivity: "base", numeric: true });
    return c !== 0 ? c * dir : porNome(a, b);
  });
}

export function valoresDe(sites: Site[], campo: "tipo" | "regiao" | "pais"): string[] {
  return [...new Set(sites.map((s) => s[campo]).filter((v): v is string => !!v))].sort();
}

function parseUtc(ts: string | null): number | null {
  if (!ts) return null;
  const zoned = /(Z|[+-]\d{2}:?\d{2})$/i.test(ts);
  const t = Date.parse(zoned ? ts : ts.replace(" ", "T") + "Z");
  return Number.isNaN(t) ? null : t;
}

/** Whole days since a SQLite `datetime('now')` timestamp (UTC), or null when absent or unreadable. Strings that already carry a zone (Z or an offset) are parsed as given. */
export function diasDesde(verificadaEm: string | null, agora: Date): number | null {
  const t = parseUtc(verificadaEm);
  if (t === null) return null;
  return Math.max(0, Math.floor((agora.getTime() - t) / 86_400_000));
}

/** A SQLite UTC timestamp as a local date in the app locale, or null when absent or unreadable (never "Invalid Date"). */
export function dataLocal(ts: string | null, locale: string): string | null {
  const t = parseUtc(ts);
  if (t === null) return null;
  return new Date(t).toLocaleDateString(locale === "pt" ? "pt-BR" : "en-GB");
}
