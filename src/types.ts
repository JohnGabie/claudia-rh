// Types shared across components — mirror the Rust backend structs.
// A type used by 2+ components belongs here, not duplicated locally.

export interface Vaga {
  id: number;
  titulo: string;
  empresa: string;
  plataforma: string;
  url: string;
  localizacao: string | null;
  modelo_trabalho: string | null;
  descoberta_em: string;
  status: string;
  motivo_status: string | null;
  match_score: string | null;
}

// Subset returned by the dashboard queries
export type VagaResumo = Pick<Vaga, "id" | "titulo" | "empresa" | "status" | "descoberta_em">;

export interface SearchVariant {
  id: string;
  nome_exibicao: string;
  peso: number;
  ativa: boolean;
  foco_competencias: string[];
  foco_experiencia: string[];
  regioes_aceitas: string[];
  modelos_trabalho: string[];
  idiomas_aplicacao: string[];
  cv_gerado_path: string;
  cv_gerado_em: string;
}
