export interface VagaAtual {
  id: number;
  titulo: string;
  empresa: string;
  url: string;
  etapa: string | null;
}

export interface JanelaAgendamento {
  dia_semana: number;
  inicio: string;
  fim: string;
  ativo: boolean;
}

export interface ConfigDisparo {
  ativo: boolean;
  limiar_minutos: number;
  limite_diario: number;
  limite_tempo_minutos: number;
  limite_vagas_sessao?: number;
  janelas: JanelaAgendamento[];
}

export interface StatusLinkedinRede {
  ativo: boolean;
  ultima_busca: string | null;
  vagas_encontradas: number;
}

export interface ModalCfg {
  titulo: string;
  subtitulo: string;
  valores: number[];
  valorAtual: number;
  formatValue: (v: number) => string;
  onSave: (v: number) => void;
}
