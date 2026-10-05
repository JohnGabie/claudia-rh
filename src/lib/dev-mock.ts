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
    case "contar_pendencias":
    case "contar_propostas": return 0;
    case "welcome_necessario": return false;
    case "sugerir_feedback": return { sugerir: false };
    case "plugin:dialog|open": return "C:\\Users\\example\\Downloads\\curriculo-importado.pdf";
    case "plugin:event|listen": return 1;
    case "plugin:event|unlisten":
    case "plugin:log|log":
    case "diag_heartbeat": return undefined;
    default:
      if (cmd.startsWith("plugin:")) return undefined;
      return mockInvoke(cmd, args);
  }
}

export function instalarMockTauri() {
  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {
    invoke,
    transformCallback: () => 1,
    unregisterCallback: () => {},
    convertFileSrc: (p: string) => p,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
  };
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
}
