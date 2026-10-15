// DEV ONLY: lets `vite dev` run in a plain browser (no Tauri) by faking `window.__TAURI_INTERNALS__`.
// main.tsx imports it behind `import.meta.env.DEV`, so it is not part of the production bundle.
// The document commands are answered by ./documentos.mock; the few Profile commands the screens need
// are answered here, with fake data only. Everything else is rejected so nothing looks silently fine.
import { mockInvoke } from "./documentos.mock";
import type { SearchVariant } from "../types";

const variante = (id: string, nome: string, peso: number, ativa: boolean, principal: boolean): SearchVariant => ({
  id, nome_exibicao: nome, peso, ativa, principal,
  foco_competencias: [], foco_experiencia: [], regioes_aceitas: ["Denmark", "Remote EU"], modelos_trabalho: ["hibrido", "remoto"],
  idiomas_aplicacao: ["en"], cv_gerado_path: "", cv_gerado_em: "",
});

let variantes: SearchVariant[] = [
  variante("backend", "Backend Engineer", 60, true, true),
  variante("devops", "DevOps / Platform", 30, true, false),
  variante("ia", "AI Engineer", 10, false, false),
];

const candidato = {
  dados_pessoais: { nome_completo: "Example Candidate", email: "example@example.com", telefone: "", localizacao_atual: "Copenhagen, Denmark", endereco: "", nacionalidade: "", data_nascimento: "", cpf: "", links: [] },
  experiencia: [{ empresa: "Example A/S", cargo: "Senior Software Engineer", inicio: "2022", fim: "", descricao: "Backend services.", conquistas: ["Cut p95 latency by 40%"], tecnologias: ["Rust", "PostgreSQL"] }],
  projetos: [], formacao: [], competencias: ["Rust", "Python", "TypeScript"], idiomas: [{ idioma: "English", nivel: "C1" }],
  gaps_conhecidos: [], respostas_modelo: { porque_esta_vaga: "", pretensao_salarial_texto: "", notice_period: "" },
  ultima_atualizacao: "2026-10-04T09:00:00Z", fontes_usadas: [],
};

const flag = (n: string) => new URLSearchParams(location.search).has(n);

const vagaAtual = () => ({
  id: 12, titulo: "Senior Software Engineer", empresa: "Novo Example A/S", url: "https://example.com/jobs/12", etapa: "formulário",
  anexos: flag("apagado")
    ? [{ tipo: "curriculo", documento_id: null, nome: null, formato: null }]
    : [
        { tipo: "curriculo", documento_id: 1, nome: "CV Backend (Classic ATS)", formato: "pdf" },
        { tipo: "carta", documento_id: 4, nome: "Cover letter - Novo Example", formato: "pdf" },
      ],
});

const cand = (id: number, titulo: string, empresa: string, documento_id: number | null, documento_nome: string | null) => ({
  id, vaga_id: id, titulo, empresa, plataforma: "LinkedIn", url: "https://example.com", enviada_em: "2026-10-04T10:00:00Z",
  pasta_arquivos: "", metodo: "easy_apply", resultado: null, documento_id, documento_nome,
});
const CANDIDATURAS = [
  cand(1, "Senior Software Engineer", "Novo Example A/S", 1, "CV Backend (Classic ATS)"),
  cand(2, "Platform Engineer", "Sample ApS", 2, "CV DevOps (Dev Compact)"),
  cand(3, "Backend Developer", "Demo Group", null, null),
];

const pend = (id: number, categoria: string, descricao: string, titulo_vaga: string, empresa_vaga: string) => ({
  id, vaga_id: id, titulo_vaga, empresa_vaga, criada_em: new Date(Date.now() - id * 600000).toISOString(),
  categoria, descricao, resolvida: false, resolvida_em: null, resolucao: null,
});
const PENDENCIAS = [
  pend(1, "sem_curriculo", "A vaga pede currículo e a biblioteca não tem nenhum. Gere ou importe um para continuar.", "Senior Software Engineer", "Novo Example A/S"),
  pend(2, "sem_carta", "A vaga pede carta de apresentação e não há nenhuma adequada na biblioteca.", "Platform Engineer", "Sample ApS"),
  pend(3, "captcha", "Captcha no envio do formulário.", "Backend Developer", "Demo Group"),
];

const ev = (id: number, over: Record<string, unknown>) => ({
  id, ts: new Date(Date.now() - id * 30000).toISOString(), kind: "action", session_id: 3, vaga_id: 12, tool: null, target: null,
  role: null, text: null, url: null, result: null, error: null, duration_ms: 120, shot_before: null, shot_after: null, extra: null, ...over,
});
const EVENTOS = [
  ev(3, { tool: "browser_upload", target: "input[type=file]", text: "documento 1 | CV Backend (Classic ATS) | pdf | 84213 bytes | sha256 a1b2c3d4e5f6", url: "https://example.com/apply", result: "ok · variante backend (a da vaga)", duration_ms: 480 }),
  ev(2, { tool: "browser_click", target: "button: Next", url: "https://example.com/apply", result: "ok" }),
  ev(1, { tool: "browser_read", url: "https://example.com/apply", result: "estado: ok", duration_ms: 900 }),
];

// Minimal Tauri event bus so screens that `listen(...)` can be driven from the console:
// `window.__mockEmit("documentos-atualizados", {novos: 1})`, or `window.__mockBackendDoc()` to add a
// cover letter without going through docsApi (like the backend queue does) and then emit the event.
let proximoCallback = 0;
let proximoOuvinte = 0;
const callbacks = new Map<number, (e: unknown) => void>();
const ouvintes = new Map<number, { evento: string; cb: (e: unknown) => void }>();
const emitir = (evento: string, payload: unknown = null) => {
  ouvintes.forEach((o, id) => { if (o.evento === evento) o.cb({ event: evento, id, payload }); });
};

async function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  switch (cmd) {
    case "ler_candidato_base": return candidato;
    case "ler_search_variants": return structuredClone(variantes);
    case "guardar_variante_unica": {
      const v = args.variante as SearchVariant;
      // Mirrors the backend contract: saving one variant as main clears the others.
      variantes = variantes.map((o) => (o.id === v.id ? v : v.principal ? { ...o, principal: false } : o));
      return undefined;
    }
    case "guardar_pesos_variantes": return undefined;
    case "listar_propostas": return [];
    case "contar_pendencias": return PENDENCIAS.length;
    case "contar_propostas": return 0;
    // ── Where the document shows up (Dashboard, History, Pendencias, Activity log, Welcome). Fake data only.
    // `?welcome` opens the Welcome flow; `?vazio` (documents mock) makes the library empty, `?apagado` deletes the attached CV.
    case "welcome_necessario": return new URLSearchParams(location.search).has("welcome");
    case "verificar_setup": return { claude_instalado: true, browser_pronto: true, perfil_preenchido: true };
    case "obter_modo_autonomo": return true;
    case "browser_status": return { stage: "ready" };
    case "marcar_welcome_concluido": return undefined;
    case "candidaturas_hoje": return 2;
    case "atividade_recente":
    case "listar_vagas": return [];
    case "vaga_atual_sessao": return vagaAtual();
    case "vagas_analisadas_hoje": return 14;
    case "vagas_analisadas_total": return 96;
    case "tempo_sessoes_hoje": return 42;
    case "obter_config_disparo": return { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, limite_vagas_sessao: 0, janelas: [] };
    case "obter_status_linkedin_rede": return { ativo: false, ultima_busca: null, vagas_encontradas: 0 };
    case "listar_candidaturas": return CANDIDATURAS;
    case "listar_pendencias": return PENDENCIAS;
    case "activity_log_list": return EVENTOS;
    case "activity_log_image": return Promise.reject("mock: no screenshot");
    case "sugerir_feedback": return { sugerir: false };
    case "plugin:dialog|open": return "C:\\Users\\example\\Downloads\\curriculo-importado.pdf";
    case "plugin:event|listen": {
      const id = ++proximoOuvinte;
      ouvintes.set(id, { evento: args.event as string, cb: callbacks.get(args.handler as number)! });
      return id;
    }
    case "plugin:event|unlisten":
      ouvintes.delete(args.eventId as number);
      return undefined;
    case "plugin:log|log":
    case "diag_heartbeat": return undefined;
    case "listar_sites":
    case "listar_avisos_sites":
    case "contar_avisos_sites":
    case "abrir_site":
    case "definir_ia_pode_usar":
    case "marcar_site_logado":
    case "adicionar_site":
    case "remover_site": {
      // Sites commands: answered by the Sites mock (dynamic import, DEV only).
      const { mockInvoke: sitesMock } = await import("./sites.mock");
      return sitesMock(cmd, args);
    }
    default:
      if (cmd.startsWith("plugin:")) return undefined;
      return mockInvoke(cmd, args);
  }
}

export function instalarMockTauri() {
  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {
    invoke,
    transformCallback: (cb: (e: unknown) => void) => { const id = ++proximoCallback; callbacks.set(id, cb); return id; },
    unregisterCallback: (id: number) => { callbacks.delete(id); },
    convertFileSrc: (p: string) => p,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
  };
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: (_evento: string, id: number) => { ouvintes.delete(id); } };
  w.__mockEmit = emitir;
  w.__mockOuvintes = () => ouvintes.size;
  w.__mockBackendDoc = async () => {
    await mockInvoke("gerar_curriculo", { templateId: "backend-saved", corPrimaria: "#D97757", idioma: "en", varianteId: "backend" });
    emitir("documentos-atualizados", { novos: 1 });
  };
}
