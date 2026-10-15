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

const sem = (t: string) => t.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

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
const porNome = (a: Site, b: Site) => a.nome.localeCompare(b.nome, "pt", { sensitivity: "base" }) || a.chave.localeCompare(b.chave);

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
      : String(ka).localeCompare(String(kb), "pt", { sensitivity: "base" });
    return c !== 0 ? c * dir : porNome(a, b);
  });
}

export function valoresDe(sites: Site[], campo: "tipo" | "regiao" | "pais"): string[] {
  return [...new Set(sites.map((s) => s[campo]).filter((v): v is string => !!v))].sort();
}

/** Whole days since a SQLite `datetime('now')` timestamp (UTC), or null when absent or unreadable. Strings that already carry a zone (Z or an offset) are parsed as given. */
export function diasDesde(verificadaEm: string | null, agora: Date): number | null {
  if (!verificadaEm) return null;
  const zoned = /(Z|[+-]\d{2}:?\d{2})$/i.test(verificadaEm);
  const t = Date.parse(zoned ? verificadaEm : verificadaEm.replace(" ", "T") + "Z");
  if (Number.isNaN(t)) return null;
  return Math.max(0, Math.floor((agora.getTime() - t) / 86_400_000));
}
