// DEV ONLY. In-memory stand-in for the Sites commands, used by `vite dev` in a plain browser. It is
// imported dynamically behind `import.meta.env.DEV`, so it never reaches `dist/`. It reproduces the
// backend rules the screen depends on (AI permission refusal, add validation, embedded sites).
export const MOCK_SITES = "mock-sites-dev-only";

import type { Site } from "../components/sites/logica";
import type { AvisoSite, SiteStats } from "./sites-api";

const diasAtras = (d: number) => new Date(Date.now() - d * 86_400_000).toISOString().slice(0, 19).replace("T", " ");

const s = (
  chave: string, nome: string, dominio: string, tipo: string, regiao: string, pais: string | null,
  extra: Partial<Site> = {},
): Site => ({
  chave, nome, url_login: `https://${dominio}/login`, dominios: [dominio], dominios_ats: [], caminho_ats: null,
  tipo, regiao, pais, papel: "candidatura", regra_login: "nenhuma", ia_pode_usar: true, embutida: true,
  padrao_atual: true, estado: "desconhecido", verificada_em: null, ...extra,
});

const inicial = (): Site[] => [
  s("linkedin", "LinkedIn", "linkedin.com", "geral", "GLOBAL", null, { regra_login: "manual", estado: "logada", verificada_em: diasAtras(3) }),
  s("linkedin-jobs", "LinkedIn Jobs", "jobs.linkedin.com", "geral", "GLOBAL", null, { regra_login: "cookie_sessao", estado: "logada", verificada_em: diasAtras(1) }),
  s("indeed", "Indeed", "indeed.com", "geral", "GLOBAL", null, { regra_login: "manual", estado: "precisa_login", ia_pode_usar: false }),
  s("glassdoor", "Glassdoor", "glassdoor.com", "geral", "GLOBAL", null, { regra_login: "manual", ia_pode_usar: false }),
  s("jobindex", "Jobindex", "jobindex.dk", "regional", "EU", "DK", { estado: "logada", verificada_em: diasAtras(0) }),
  s("jobnet", "Jobnet", "jobnet.dk", "regional", "EU", "DK", { papel: "descoberta" }),
  s("thehub", "The Hub", "thehub.io", "tecnologia", "EU", "DK", { regra_login: "manual", estado: "precisa_login", ia_pode_usar: false }),
  s("stepstone", "StepStone", "stepstone.de", "regional", "EU", "DE"),
  s("xing", "Xing", "xing.com", "geral", "EU", "DE", { regra_login: "manual", ia_pode_usar: false }),
  s("welcometothejungle", "Welcome to the Jungle", "welcometothejungle.com", "tecnologia", "EU", "FR"),
  s("infojobs", "InfoJobs", "infojobs.net", "regional", "EU", "ES"),
  s("catho", "Catho", "catho.com.br", "regional", "SA", "BR", { regra_login: "manual", estado: "logada", verificada_em: diasAtras(12) }),
  s("gupy", "Gupy", "gupy.io", "tecnologia", "SA", "BR", { papel: "descoberta" }),
  s("vagas", "Vagas.com", "vagas.com.br", "regional", "SA", "BR"),
  s("computrabajo", "Computrabajo", "computrabajo.com", "regional", "SA", null),
  s("ziprecruiter", "ZipRecruiter", "ziprecruiter.com", "geral", "NA", "US", { regra_login: "manual", estado: "precisa_login", ia_pode_usar: false }),
  s("wellfound", "Wellfound", "wellfound.com", "tecnologia", "NA", "US", { regra_login: "manual", ia_pode_usar: false }),
  s("workopolis", "Workopolis", "workopolis.com", "regional", "NA", "CA"),
  s("naukri", "Naukri", "naukri.com", "regional", "AS", "IN", { regra_login: "manual", ia_pode_usar: false }),
  s("seek", "SEEK", "seek.com.au", "regional", "OC", "AU", { estado: "logada", verificada_em: diasAtras(1) }),
  s("trademe", "Trade Me Jobs", "trademe.co.nz", "regional", "OC", "NZ"),
  s("jobberman", "Jobberman", "jobberman.com", "regional", "AF", "NG"),
  s("weworkremotely", "We Work Remotely", "weworkremotely.com", "remoto", "GLOBAL", null),
  s("remoteok", "Remote OK", "remoteok.com", "remoto", "GLOBAL", null, { ia_pode_usar: false }),
  s("upwork", "Upwork", "upwork.com", "freelance", "GLOBAL", null, { regra_login: "manual", estado: "precisa_login", ia_pode_usar: false }),
  s("toptal", "Toptal", "toptal.com", "freelance", "GLOBAL", null, { padrao_atual: false }),
  s("orsted", "Ørsted Careers", "orsted.com", "empresa", "EU", "DK", { estado: "logada", verificada_em: diasAtras(2) }),
  s("maersk", "A.P. Møller – Mærsk Careers", "maersk.com", "empresa", "EU", "DK"),
  s("climatebase", "Climatebase", "climatebase.org", "nicho", "GLOBAL", null),
  s(
    "nordic-board", "Vegan and Sustainability Jobs Board for Nordic Professionals and Researchers",
    "nordic-sustainability-professionals-and-researchers-network.example.org", "nicho", "EU", "SE", { embutida: false },
  ),
  s("minha-rede", "Vagas da minha rede", "rede.example.com", "nicho", "SA", "BR", { embutida: false, regra_login: "manual", ia_pode_usar: false }),
];

let sites = inicial();
let avisos: AvisoSite[] = [{ id: 1, chave: "indeed", nome: "Indeed", criado_em: diasAtras(0) }];
const espera = <T>(v: T) => new Promise<T>((ok) => setTimeout(() => ok(structuredClone(v)), 120));

// Fixed numbers per key (stable across reloads); every other key gets a deterministic set from its name.
const stats = (chave: string): SiteStats => {
  const n = [...chave].reduce((a, c) => a + c.charCodeAt(0), 0);
  if (chave === "linkedin") {
    return { relogins: 2, aberturas: 14, ultima_abertura: diasAtras(1), vagas: 52, candidaturas: 9, ultima_candidatura: diasAtras(2) };
  }
  if (n % 5 === 0) return { relogins: 0, aberturas: 0, ultima_abertura: null, vagas: 0, candidaturas: 0, ultima_candidatura: null };
  return { relogins: n % 3, aberturas: 3 + (n % 9), ultima_abertura: diasAtras(n % 7), vagas: 10 + (n % 40), candidaturas: n % 6, ultima_candidatura: n % 6 ? diasAtras(n % 5) : null };
};

const TIPOS = ["geral", "regional", "tecnologia", "remoto", "freelance", "empresa", "nicho"];
const hostDe = (url: string): string | null => {
  try {
    const u = new URL(url);
    return u.protocol === "https:" ? u.hostname.toLowerCase() : null;
  } catch {
    return null;
  }
};
const slug = (n: string) => n.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const r = (v: unknown) => espera(v) as Promise<T>;
  const achar = (): Site | null => sites.find((y) => y.chave === args.chave) ?? null;
  const naoAchou = () => Promise.reject<T>("Site não encontrado.");
  switch (cmd) {
    case "listar_sites": return r(sites);
    case "listar_avisos_sites": return r(avisos);
    case "contar_avisos_sites": return r(avisos.length);
    case "abrir_site": return r(undefined);
    // Dismisses only the notice (idempotent); the site keeps its state.
    case "dispensar_aviso_site": avisos = avisos.filter((a) => a.id !== args.id); return r(undefined);
    case "definir_ia_pode_usar": {
      const x = achar();
      if (!x) return naoAchou();
      if (args.valor === true && x.regra_login !== "nenhuma" && x.estado !== "logada") {
        return Promise.reject('Entre neste site (passo 1 da página do site) antes de liberar a IA.');
      }
      x.ia_pode_usar = args.valor === true;
      return r(undefined);
    }
    case "marcar_site_logado": {
      const x = achar();
      if (!x) return naoAchou();
      x.estado = "logada";
      x.verificada_em = diasAtras(0);
      avisos = avisos.filter((a) => a.chave !== x.chave);
      return r(undefined);
    }
    case "estatisticas_site": {
      const x = achar();
      if (!x) return naoAchou();
      return r(stats(x.chave));
    }
    case "definir_site_sem_login": {
      const x = achar();
      if (!x) return naoAchou();
      if (x.regra_login === "cookie_sessao") {
        return Promise.reject('Este site é verificado pelo cookie da sessão; a opção "não pede login" não vale aqui.');
      }
      if (args.valor === true) {
        x.regra_login = "nenhuma";
        avisos = avisos.filter((a) => a.chave !== x.chave);
      } else {
        // Mirrors the backend (definir_sem_login): manual login, red state, AI closed, one open notice.
        x.regra_login = "manual";
        x.estado = "precisa_login";
        x.ia_pode_usar = false;
        if (!avisos.some((a) => a.chave === x.chave)) {
          const id = avisos.reduce((m, a) => Math.max(m, a.id), 0) + 1;
          avisos = [...avisos, { id, chave: x.chave, nome: x.nome, criado_em: diasAtras(0) }];
        }
      }
      return r(undefined);
    }
    case "marcar_site_nao_logado": {
      const x = achar();
      if (!x) return naoAchou();
      x.estado = "precisa_login";
      x.ia_pode_usar = false;
      return r(undefined);
    }
    case "adicionar_site": {
      const nome = String(args.nome ?? "").trim();
      const host = hostDe(String(args.url ?? "").trim());
      if (!nome) return Promise.reject("O nome do site é obrigatório.");
      if (!host) return Promise.reject("O endereço precisa ser uma URL https válida.");
      if (!TIPOS.includes(String(args.tipo))) return Promise.reject("Tipo de site inválido.");
      const dono = sites.find((y) => y.dominios.some((d) => host === d || host.endsWith("." + d)));
      if (dono) return Promise.reject(`Este endereço já está coberto pelo site ${dono.nome}.`);
      const pais = args.pais ? String(args.pais).trim() : null;
      if (pais !== null && !/^[A-Z]{2}$/.test(pais)) return Promise.reject("O país precisa ser o código de 2 letras (ex.: DK).");
      let chave = slug(nome) || host;
      while (sites.some((y) => y.chave === chave)) chave += "-2";
      sites = [...sites, s(chave, nome, host, String(args.tipo), "GLOBAL", pais, {
        embutida: false, regra_login: "manual", ia_pode_usar: false, estado: "desconhecido", papel: "candidatura", url_login: String(args.url).trim(),
      })];
      return r(chave);
    }
    case "remover_site": {
      const x = achar();
      if (!x) return naoAchou();
      if (x.embutida) return Promise.reject("Sites que vêm com o app não podem ser removidos.");
      sites = sites.filter((y) => y.chave !== x.chave);
      return r(undefined);
    }
    default: return Promise.reject(`mock: comando desconhecido ${cmd}`);
  }
}
