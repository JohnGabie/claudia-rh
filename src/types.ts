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
  // At most one variant is the main one; the backend clears the others when one is saved with true.
  principal: boolean;
}

export type TipoDocumento = "curriculo" | "carta";
export type TipoTag = "variante" | "template" | "cor" | "idioma" | "livre";
export interface Tag { tipo: TipoTag; valor: string }

// Mirrors documentos::biblioteca::DocumentoView (spec section 6). No file names, no paths.
export interface Documento {
  id: number;
  tipo: TipoDocumento;
  nome: string;
  origem: "gerado" | "importado";
  formato: "pdf" | "docx";
  idioma: string | null;
  tags: Tag[];
  tamanho: number;
  criado_em: string;
  vaga_id: number | null;
  preferido_global: boolean;
  preferido_variantes: string[];
  tem_preview: boolean;
}
export interface DocumentoDetalhe extends Documento { preview_html: string | null }
export interface VarianteResumo { id: string; nome_exibicao: string; ativa: boolean }
