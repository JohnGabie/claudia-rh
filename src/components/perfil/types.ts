// ── Types ──────────────────────────────────────────────────────────────────

export interface DadosPessoais {
  nome_completo: string;
  email: string;
  telefone: string;
  localizacao_atual: string;
  endereco: string;
  nacionalidade: string;
  data_nascimento: string;
  cpf: string;
  links: { tipo: string; url: string }[];
}

export interface Experiencia {
  empresa: string;
  cargo: string;
  inicio: string;
  fim: string;
  descricao: string;
  conquistas: string[];
  tecnologias: string[];
}

export interface CandidateBase {
  dados_pessoais: DadosPessoais;
  experiencia: Experiencia[];
  projetos: { nome: string; descricao: string; tecnologias: string[]; url: string; origem: string }[];
  formacao: { instituicao: string; curso: string; inicio: string; fim: string }[];
  competencias: string[];
  idiomas: { idioma: string; nivel: string }[];
  gaps_conhecidos: { competencia: string; contexto: string; como_abordar: string }[];
  respostas_modelo: { porque_esta_vaga: string; pretensao_salarial_texto: string; notice_period: string };
  ultima_atualizacao: string;
  fontes_usadas: { tipo: string; referencia: string; consultado_em: string }[];
}

export interface CurriculoInfo {
  path: string;
  file_name: string;
  template_id: string;
  template_nome: string;
  gerado_em: string;
}

export interface CoverLetterInfo {
  path: string;
  file_name: string;
  empresa: string;
  cargo: string;
  idioma: string;
  gerado_em: string;
}

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  streaming?: boolean;
}

export type Mode = "resumo" | "chat" | "curriculos" | "cover_letters";
export interface ChatFocus {
  section: string;
  label: string;
  preMessage?: string;
  chromeSessao?: boolean;
}

export type EditTarget =
  | { kind: "dados_pessoais" }
  | { kind: "experiencia" }
  | { kind: "projetos" }
  | { kind: "formacao" }
  | { kind: "competencias" }
  | { kind: "idiomas" }
  | { kind: "variante"; id: string }
  | { kind: "nova_variante" };

// ── Proposta type ──────────────────────────────────────────────────────────

export interface Proposta {
  id: number;
  vaga_id: number | null;
  titulo_vaga: string | null;
  empresa_vaga: string | null;
  criada_em: string;
  pergunta: string;
  contexto: string | null;
}

// Shared metadata for generated documents (CVs and cover letters)
export const PALETTE = [
  { hex: "#1a1a1a", label: "Preto" },
  { hex: "#D97757", label: "Laranja" },
  { hex: "#2563EB", label: "Azul" },
  { hex: "#16A34A", label: "Verde" },
  { hex: "#7C3AED", label: "Roxo" },
  { hex: "#DC2626", label: "Vermelho" },
  { hex: "#0891B2", label: "Teal" },
  { hex: "#475569", label: "Slate" },
];

export type DocLang = "pt" | "en";
