import { JanelaAgendamento, ConfigDisparo } from "./types";

export const STATUS_STYLE: Record<string, { background: string; color: string }> = {
  descoberta: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  analisada: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  candidatando: { background: "var(--accent-soft)", color: "var(--accent-strong)" },
  aplicada: { background: "#E3EFE7", color: "var(--success)" },
  pulada: { background: "var(--bg-sunken)", color: "var(--text-tertiary)" },
  pendente_revisao: { background: "#FBEFD9", color: "var(--warning)" },
  bloqueada: { background: "#F7E2DF", color: "var(--danger)" },
};

export function calcularProximaJanela(janelas: JanelaAgendamento[], days: readonly string[]): string | null {
  if (janelas.length === 0) return null;
  const agora = new Date();
  const dia = agora.getDay();
  const hhmm = `${String(agora.getHours()).padStart(2, "0")}:${String(agora.getMinutes()).padStart(2, "0")}`;

  if (janelas.some(j => j.ativo && j.dia_semana === dia && hhmm >= j.inicio && hhmm < j.fim)) {
    return "ATIVO_AGORA";
  }

  const mesmoDia = janelas
    .filter(j => j.ativo && j.dia_semana === dia && j.inicio > hhmm)
    .sort((a, b) => a.inicio.localeCompare(b.inicio));
  if (mesmoDia.length > 0) return `hoje às ${mesmoDia[0].inicio}`;

  for (let d = 1; d <= 7; d++) {
    const nd = (dia + d) % 7;
    const next = janelas.filter(j => j.ativo && j.dia_semana === nd).sort((a, b) => a.inicio.localeCompare(b.inicio));
    if (next.length > 0) return `${days[nd]} às ${next[0].inicio}`;
  }
  return "Sem janelas ativas";
}

export const CFG_DEFAULT: ConfigDisparo = { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, limite_vagas_sessao: 0, janelas: [] };

// Picker value sets
export const VALS_CANDIDATURAS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30];
export const VALS_VAGAS = [0, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100];
export const VALS_TEMPO = [0, 15, 30, 45, 60, 90, 120, 150, 180, 210, 240, 300, 360, 420, 480];
