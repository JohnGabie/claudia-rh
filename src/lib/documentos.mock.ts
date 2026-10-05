// DEV ONLY. In-memory stand-in for the document commands (spec section 6), used by `vite dev` in a
// plain browser. It is imported dynamically behind `import.meta.env.DEV`, so it never reaches `dist/`.
export const MOCK_DOCUMENTOS = "mock-documentos-dev-only";

import type { Documento, DocumentoDetalhe, Tag, TipoDocumento, VarianteResumo } from "../types";

type Registro = Documento & { preview_html: string | null };

const folha = (nome: string, cor: string, corpo: string) => `<!doctype html><html><head><meta charset="utf-8"><style>
body{font-family:Calibri,Arial,sans-serif;margin:0;padding:56px 60px;color:#1f1d18;font-size:13px;line-height:1.5}
h1{font-size:28px;margin:0 0 2px;color:${cor}} h2{font-size:13px;letter-spacing:.08em;text-transform:uppercase;color:${cor};border-bottom:1.5px solid ${cor};padding-bottom:3px;margin:22px 0 8px}
.sub{color:#6b6759;margin-bottom:8px} .row{display:flex;justify-content:space-between;font-weight:600} ul{margin:4px 0 0 18px;padding:0}
</style></head><body><h1>${nome}</h1><div class="sub">Copenhagen, Denmark · example@example.com · +45 00 00 00 00</div>${corpo}</body></html>`;

const corpoCv = `<h2>Summary</h2><p>Backend engineer with 6 years building APIs and data pipelines. Focused on reliability and clear communication.</p>
<h2>Experience</h2><div class="row"><span>Senior Software Engineer, Example A/S</span><span>2022 – present</span></div>
<ul><li>Designed an event-driven order service handling 2M events per day.</li><li>Cut p95 latency by 40% after moving hot paths to a cache layer.</li><li>Mentored four engineers and led the on-call rotation.</li></ul>
<div class="row" style="margin-top:12px"><span>Software Engineer, Sample ApS</span><span>2019 – 2022</span></div>
<ul><li>Built internal tooling in Python and TypeScript.</li><li>Migrated the CI pipeline to containers.</li></ul>
<h2>Skills</h2><p>Rust, Python, TypeScript, PostgreSQL, Docker, AWS</p><h2>Education</h2><div class="row"><span>BSc Computer Science</span><span>2015 – 2018</span></div>`;

const corpoCarta = `<p style="margin-top:30px">Dear hiring team at Novo Example,</p>
<p>I am writing to apply for the Senior Software Engineer position. For the past six years I have built and run backend systems where reliability matters, and your focus on data quality is exactly the kind of problem I like to work on.</p>
<p>At Example A/S I designed an event-driven order service and cut p95 latency by 40%. I would bring the same care to your platform team.</p>
<p>Thank you for your time. I would be glad to talk about how I can help.</p><p>Kind regards,<br>Example Candidate</p>`;

const tag = (tipo: Tag["tipo"], valor: string): Tag => ({ tipo, valor });

const estadoInicial = (): { docs: Registro[]; proximoId: number; variantes: VarianteResumo[] } => ({
  proximoId: 5,
  variantes: [
    { id: "backend", nome_exibicao: "Backend Engineer", ativa: true },
    { id: "devops", nome_exibicao: "DevOps / Platform", ativa: true },
    { id: "ia", nome_exibicao: "AI Engineer", ativa: false },
  ],
  docs: [
    {
      id: 1, tipo: "curriculo", nome: "CV Backend (Classic ATS)", origem: "gerado", formato: "pdf", idioma: "pt",
      tags: [tag("variante", "backend"), tag("template", "classic-ats"), tag("cor", "#D97757"), tag("idioma", "pt")],
      tamanho: 84_213, criado_em: "2026-10-04 09:12", vaga_id: null, preferido_global: true, preferido_variantes: ["backend"],
      tem_preview: true, preview_html: folha("Example Candidate", "#D97757", corpoCv),
    },
    {
      id: 2, tipo: "curriculo", nome: "CV DevOps (Dev Compact)", origem: "gerado", formato: "pdf", idioma: "en",
      tags: [tag("variante", "devops"), tag("template", "dev-compact"), tag("cor", "#2563EB"), tag("idioma", "en")],
      tamanho: 79_880, criado_em: "2026-10-03 18:40", vaga_id: null, preferido_global: false, preferido_variantes: [],
      tem_preview: true, preview_html: folha("Example Candidate", "#2563EB", corpoCv),
    },
    {
      id: 3, tipo: "curriculo", nome: "meu-curriculo-2026.pdf", origem: "importado", formato: "pdf", idioma: null,
      tags: [tag("livre", "enviado por e-mail")], tamanho: 211_004, criado_em: "2026-09-28 14:03", vaga_id: null,
      preferido_global: false, preferido_variantes: [], tem_preview: false, preview_html: null,
    },
    {
      id: 4, tipo: "carta", nome: "Cover letter - Novo Example", origem: "gerado", formato: "pdf", idioma: "en",
      tags: [tag("idioma", "en"), tag("cor", "#D97757")], tamanho: 41_007, criado_em: "2026-10-04 10:30", vaga_id: 12,
      preferido_global: false, preferido_variantes: [], tem_preview: true, preview_html: folha("Example Candidate", "#D97757", corpoCarta),
    },
  ],
});

let estado = estadoInicial();

// Dev tool: ?vazio in the URL starts the mock with no documents (empty-state screenshots).
if (typeof location !== "undefined" && new URLSearchParams(location.search).has("vazio")) estado.docs = [];
if (typeof location !== "undefined" && new URLSearchParams(location.search).has("erro")) estado.docs = null as unknown as Registro[];

const visao = ({ preview_html: _p, ...d }: Registro): Documento => d;
const achar = (id: number): Registro => {
  const d = estado.docs.find((x) => x.id === id);
  if (!d) throw `o documento ${id} não existe`;
  return d;
};
const espera = <T,>(v: T, ms = 120): Promise<T> => new Promise((r) => setTimeout(() => r(v), ms));

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  if (estado.docs === null) return Promise.reject("mock: falha simulada ao ler a biblioteca");
  const r = (v: unknown) => espera(v) as Promise<T>;
  switch (cmd) {
    case "listar_documentos":
      return r(estado.docs.filter((d) => d.tipo === args.tipo).map(visao).reverse());
    case "obter_documento": {
      const d = achar(args.id as number);
      return r({ ...visao(d), preview_html: d.preview_html } satisfies DocumentoDetalhe);
    }
    case "gerar_curriculo": {
      const cor = args.corPrimaria as string;
      const variante = args.varianteId as string | null;
      const d: Registro = {
        id: estado.proximoId++, tipo: "curriculo", nome: `CV ${args.templateId}`, origem: "gerado", formato: "pdf",
        idioma: args.idioma as string,
        tags: [...(variante ? [tag("variante", variante)] : []), tag("template", args.templateId as string), tag("cor", cor), tag("idioma", args.idioma as string)],
        tamanho: 80_000, criado_em: "2026-10-04 12:00", vaga_id: null, preferido_global: false, preferido_variantes: [],
        tem_preview: true, preview_html: folha("Example Candidate", cor, corpoCv),
      };
      estado.docs.push(d);
      return r(visao(d));
    }
    case "gerar_cover_letter":
      return r(undefined);
    case "importar_documento": {
      const caminho = String(args.caminhoOrigem);
      const d: Registro = {
        id: estado.proximoId++, tipo: args.tipo as TipoDocumento, nome: (args.nome as string | null) ?? caminho.split(/[\\/]/).pop() ?? "importado.pdf",
        origem: "importado", formato: caminho.toLowerCase().endsWith(".docx") ? "docx" : "pdf", idioma: null, tags: [], tamanho: 150_000,
        criado_em: "2026-10-04 12:05", vaga_id: null, preferido_global: false, preferido_variantes: [], tem_preview: false, preview_html: null,
      };
      estado.docs.push(d);
      return r(visao(d));
    }
    case "renomear_documento":
      achar(args.id as number).nome = String(args.nome);
      return r(undefined);
    case "atualizar_tags_documento":
      achar(args.id as number).tags = args.tags as Tag[];
      return r(undefined);
    case "definir_preferido": {
      const d = achar(args.id as number);
      const v = args.varianteId as string | null;
      // One preferred per (type, variant): marking one clears the previous.
      for (const o of estado.docs.filter((x) => x.tipo === d.tipo)) {
        if (v === null) o.preferido_global = o.id === d.id;
        else o.preferido_variantes = o.id === d.id ? [...new Set([...o.preferido_variantes, v])] : o.preferido_variantes.filter((x) => x !== v);
      }
      return r(undefined);
    }
    case "remover_preferido": {
      const v = args.varianteId as string | null;
      for (const o of estado.docs.filter((x) => x.tipo === args.tipo)) {
        if (v === null) o.preferido_global = false;
        else o.preferido_variantes = o.preferido_variantes.filter((x) => x !== v);
      }
      return r(undefined);
    }
    case "abrir_documento":
      achar(args.id as number);
      return r(undefined);
    case "apagar_documento":
      achar(args.id as number);
      estado.docs = estado.docs.filter((d) => d.id !== args.id);
      return r(undefined);
    case "listar_variantes":
      return r(estado.variantes);
    default:
      return Promise.reject(`mock: comando desconhecido ${cmd}`);
  }
}
