import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, ExternalLink, FolderOpen, Plus, Trash2 } from "lucide-react";

// ── sub-components ────────────────────────────────────────────────────────────

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 16 }}>
    {children}
  </div>
);

const Section: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ marginBottom: 32 }}>{children}</div>
);

const ToggleSwitch: React.FC<{ checked: boolean; onChange: () => void }> = ({ checked, onChange }) => (
  <button
    role="switch"
    aria-checked={checked}
    onMouseDown={(e) => { e.preventDefault(); onChange(); }}
    style={{
      width: 40, height: 22, borderRadius: 11, flexShrink: 0, padding: 0,
      background: checked ? "var(--accent)" : "var(--bg-sunken)",
      border: `1px solid ${checked ? "var(--accent)" : "var(--border)"}`,
      cursor: "pointer", position: "relative", transition: "background 0.2s, border-color 0.2s",
    }}
  >
    <span style={{
      position: "absolute", top: 2, left: checked ? 20 : 2, width: 16, height: 16,
      borderRadius: "50%", background: checked ? "#fff" : "var(--text-tertiary)",
      transition: "left 0.2s, background 0.2s", display: "block",
    }} />
  </button>
);

// ── types ─────────────────────────────────────────────────────────────────────

interface JanelaAgendamento {
  dia_semana: number;
  inicio: string;
  fim: string;
  ativo: boolean;
}

// ── constants ─────────────────────────────────────────────────────────────────

const DIAS = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];

const PROMPTS = [
  {
    id: "runtime",
    label: "Sessão de execução",
    desc: "Prompt injetado em cada sessão automática de candidaturas. Contém o perfil, estratégia e regras de comportamento do agente.",
  },
  {
    id: "perfil",
    label: "Assistente de Perfil",
    desc: "Instruções para o chat de construção de perfil. Define como a Claudia interpreta e edita candidate_base.yaml e search_variants.yaml.",
  },
  {
    id: "feedback",
    label: "Análise de Feedback",
    desc: "Instruções para geração de relatórios de feedback. Define estrutura, tom e regras de inferência a partir dos dados de candidaturas.",
  },
  {
    id: "cover_letter_pt",
    label: "Cover Letter — Português",
    desc: "Template para cartas de apresentação em PT. Inclui regras de estilo, estrutura obrigatória e palavras proibidas.",
  },
  {
    id: "cover_letter_en",
    label: "Cover Letter — English",
    desc: "Template for cover letters in EN. Same rules as PT variant but in English, with substitution-test enforcement.",
  },
];

// ── main component ────────────────────────────────────────────────────────────

export const Configuracoes: React.FC = () => {
  const [estrategia, setEstrategia] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const savedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Disparo automático
  const [dispaAuto, setDispaAuto] = useState(true);
  const [limiarMinutos, setLimiarMinutos] = useState(15);
  const [modoAutonomo, setModoAutonomo] = useState(false);

  // Agendamento
  const [limiteDiario, setLimiteDiario] = useState(10);
  const [limiteTempoMinutos, setLimiteTempoMinutos] = useState(0);
  const [janelas, setJanelas] = useState<JanelaAgendamento[]>([]);

  useEffect(() => {
    invoke<string>("ler_estrategia")
      .then((e) => setEstrategia(e ?? ""))
      .catch(console.error)
      .finally(() => setLoading(false));

    invoke<{
      ativo: boolean;
      limiar_minutos: number;
      limite_diario: number;
      limite_tempo_minutos: number;
      janelas: JanelaAgendamento[];
    }>("obter_config_disparo")
      .then((cfg) => {
        setDispaAuto(cfg.ativo);
        setLimiarMinutos(cfg.limiar_minutos);
        setLimiteDiario(cfg.limite_diario ?? 10);
        setLimiteTempoMinutos(cfg.limite_tempo_minutos ?? 0);
        setJanelas(cfg.janelas ?? []);
      })
      .catch(() => {});

    invoke<boolean>("obter_modo_autonomo").then((v) => setModoAutonomo(!!v)).catch(() => {});
  }, []);

  const salvarModoAutonomo = (ativo: boolean) => {
    invoke("configurar_modo_autonomo", { ativo }).catch(console.error);
  };

  const salvarDisparo = (ativo: boolean, minutos: number) => {
    invoke("configurar_disparo", { ativo, limiarMinutos: minutos }).catch(console.error);
  };

  const salvarLimiteDiario = (v: number) => {
    invoke("configurar_limite_diario", { limite: v }).catch(console.error);
  };

  const salvarJanelas = (novasJanelas: JanelaAgendamento[], tempoMins?: number) => {
    invoke("configurar_disparo", {
      ativo: dispaAuto,
      limiarMinutos: limiarMinutos,
      limiteDiario: limiteDiario,
      limiteTempoMinutos: tempoMins ?? limiteTempoMinutos,
      janelas: novasJanelas,
    }).catch(console.error);
  };

  const adicionarJanela = () => {
    const nova: JanelaAgendamento = { dia_semana: 1, inicio: "09:00", fim: "17:00", ativo: true };
    const novas = [...janelas, nova];
    setJanelas(novas);
    salvarJanelas(novas);
  };

  const removerJanela = (i: number) => {
    const novas = janelas.filter((_, idx) => idx !== i);
    setJanelas(novas);
    salvarJanelas(novas);
  };

  const atualizarJanela = (i: number, patch: Partial<JanelaAgendamento>) => {
    const novas = janelas.map((j, idx) => idx === i ? { ...j, ...patch } : j);
    setJanelas(novas);
    salvarJanelas(novas);
  };

  const guardar = async () => {
    setSaving(true);
    try {
      await invoke("guardar_estrategia", { conteudo: estrategia });
      setSaved(true);
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
      savedTimerRef.current = setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <div style={{ padding: 24, color: "var(--text-tertiary)", fontSize: 14 }}>A carregar…</div>;
  }

  const inputStyle: React.CSSProperties = {
    width: 100, padding: "7px 10px", borderRadius: 4,
    border: "1px solid var(--border)", background: "var(--bg-surface)",
    color: "var(--text-primary)", fontSize: 14, fontFamily: "inherit", outline: "none",
  };

  const smallInputStyle: React.CSSProperties = {
    padding: "5px 8px", borderRadius: 4,
    border: "1px solid var(--border)", background: "var(--bg-surface)",
    color: "var(--text-primary)", fontSize: 13, fontFamily: "inherit", outline: "none",
  };

  return (
    <div style={{ padding: 24, paddingBottom: 80 }}>
      <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", marginBottom: 24 }}>
        Configurações
      </h1>

      {/* 1. Disparo automático */}
      <Section>
        <SectionTitle>Disparo automático</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          Quando ativo, inicia uma sessão automaticamente após o sistema estar inativo pelo limiar configurado.
        </p>
        <label style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer", marginBottom: dispaAuto ? 16 : 0 }}>
          <ToggleSwitch
            checked={dispaAuto}
            onChange={() => { const v = !dispaAuto; setDispaAuto(v); salvarDisparo(v, limiarMinutos); }}
          />
          <span style={{ fontSize: 14, color: "var(--text-primary)" }}>
            Disparar ao detetar inatividade do sistema
          </span>
        </label>
        {dispaAuto && (
          <div>
            <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
              Limiar de inatividade (minutos)
            </label>
            <input
              type="number" min={1} max={120} value={limiarMinutos}
              onChange={(e) => {
                const v = Math.max(1, Math.min(120, parseInt(e.target.value) || 15));
                setLimiarMinutos(v); salvarDisparo(dispaAuto, v);
              }}
              style={inputStyle}
            />
          </div>
        )}
      </Section>

      {/* 2. Agendamento de sessões */}
      <Section>
        <SectionTitle>Agendamento de sessões</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 20, lineHeight: 1.5 }}>
          Limites e janelas de tempo para as sessões automáticas. O disparo por inatividade só ocorre dentro das janelas ativas.
        </p>

        {/* Limites */}
        <div style={{ display: "flex", gap: 32, marginBottom: 24, flexWrap: "wrap" }}>
          <div>
            <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
              Limite diário de candidaturas
            </label>
            <input
              type="number" min={1} max={99} value={limiteDiario}
              onChange={(e) => setLimiteDiario(Math.max(1, Math.min(99, parseInt(e.target.value) || 10)))}
              onBlur={() => salvarLimiteDiario(limiteDiario)}
              style={{ ...inputStyle, width: 80 }}
            />
          </div>
          <div>
            <label style={{ display: "block", fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
              Limite de tempo por dia (minutos, 0 = sem limite)
            </label>
            <input
              type="number" min={0} max={480} value={limiteTempoMinutos}
              onChange={(e) => setLimiteTempoMinutos(Math.max(0, Math.min(480, parseInt(e.target.value) || 0)))}
              onBlur={() => salvarJanelas(janelas, limiteTempoMinutos)}
              style={{ ...inputStyle, width: 110 }}
            />
          </div>
        </div>

        {/* Janelas */}
        <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 10 }}>
          Janelas de atividade
        </div>

        {janelas.length === 0 ? (
          <div style={{
            padding: "16px 20px", borderRadius: 8, border: "1px dashed var(--border)",
            color: "var(--text-tertiary)", fontSize: 13, marginBottom: 12,
          }}>
            Sem janelas configuradas — o disparo pode ocorrer em qualquer hora do dia.
          </div>
        ) : (
          <div style={{
            border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden", marginBottom: 12,
          }}>
            {/* Header */}
            <div style={{
              display: "grid", gridTemplateColumns: "110px 90px 90px 60px 36px",
              gap: 8, padding: "8px 12px",
              background: "var(--bg-sunken)",
              borderBottom: "1px solid var(--border)",
            }}>
              {["Dia", "Início", "Fim", "Ativo", ""].map((h, i) => (
                <div key={i} style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                  {h}
                </div>
              ))}
            </div>

            {/* Rows */}
            {janelas.map((j, i) => (
              <div
                key={i}
                style={{
                  display: "grid", gridTemplateColumns: "110px 90px 90px 60px 36px",
                  gap: 8, padding: "8px 12px", alignItems: "center",
                  borderBottom: i < janelas.length - 1 ? "1px solid var(--border)" : "none",
                  background: j.ativo ? "var(--bg-surface)" : "var(--bg-sunken)",
                  opacity: j.ativo ? 1 : 0.7,
                }}
              >
                <select
                  value={j.dia_semana}
                  onChange={(e) => atualizarJanela(i, { dia_semana: parseInt(e.target.value) })}
                  style={{ ...smallInputStyle, width: "100%" }}
                >
                  {DIAS.map((d, idx) => (
                    <option key={idx} value={idx}>{d}</option>
                  ))}
                </select>

                <input
                  type="time" value={j.inicio}
                  onChange={(e) => atualizarJanela(i, { inicio: e.target.value })}
                  style={{ ...smallInputStyle, width: "100%" }}
                />

                <input
                  type="time" value={j.fim}
                  onChange={(e) => atualizarJanela(i, { fim: e.target.value })}
                  style={{ ...smallInputStyle, width: "100%" }}
                />

                <div style={{ display: "flex", justifyContent: "center" }}>
                  <ToggleSwitch
                    checked={j.ativo}
                    onChange={() => atualizarJanela(i, { ativo: !j.ativo })}
                  />
                </div>

                <button
                  onClick={() => removerJanela(i)}
                  title="Remover"
                  style={{
                    display: "flex", alignItems: "center", justifyContent: "center",
                    width: 28, height: 28, borderRadius: 4, border: "none",
                    background: "transparent", cursor: "pointer",
                    color: "var(--text-tertiary)",
                  }}
                  onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--danger)")}
                  onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-tertiary)")}
                >
                  <Trash2 size={13} />
                </button>
              </div>
            ))}
          </div>
        )}

        <button
          onClick={adicionarJanela}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", borderRadius: 6,
            border: "1px solid var(--border)", background: "var(--bg-surface)",
            color: "var(--text-secondary)", fontSize: 13, fontFamily: "inherit", cursor: "pointer",
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-primary)")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")}
        >
          <Plus size={13} />
          Adicionar janela
        </button>
      </Section>

      {/* 3. Modo autónomo */}
      <Section>
        <SectionTitle>Modo autónomo do agente</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          Quando <strong>inativo</strong> (recomendado), o Claude pede confirmação no terminal antes de cada ação. Quando <strong>ativo</strong>, age sem interrupções — necessário para sessões completamente não supervisionadas.
        </p>
        <label style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer" }}>
          <ToggleSwitch
            checked={modoAutonomo}
            onChange={() => { const v = !modoAutonomo; setModoAutonomo(v); salvarModoAutonomo(v); }}
          />
          <span style={{ fontSize: 14, color: "var(--text-primary)" }}>
            Pular confirmações de permissão (--dangerously-skip-permissions)
          </span>
        </label>
        {modoAutonomo && (
          <p style={{ fontSize: 12, color: "var(--warning)", marginTop: 10, lineHeight: 1.5 }}>
            ⚠️ Com isto ativo, o agente age sem travas técnicas. As únicas restrições passam a ser as regras do prompt — certifica-te de que a estratégia e o perfil estão bem configurados.
          </p>
        )}
      </Section>

      {/* 4. Estratégia de busca */}
      <Section>
        <SectionTitle>Estratégia de busca</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 12, lineHeight: 1.5 }}>
          Texto livre injetado no prompt de execução em cada sessão. Usa para definir foco, restrições temporárias, ou notas para o agente.
        </p>
        <textarea
          value={estrategia}
          onChange={(e) => setEstrategia(e.target.value)}
          placeholder={"# Estratégia ativa\n\nFoco atual, restrições, notas para o agente…"}
          rows={7}
          style={{
            width: "100%", padding: "7px 10px", borderRadius: 4,
            border: "1px solid var(--border)", background: "var(--bg-surface)",
            color: "var(--text-primary)", fontSize: 14, fontFamily: "inherit",
            boxSizing: "border-box", resize: "vertical", outline: "none", lineHeight: 1.5,
          }}
        />
        <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 8 }}>
          <button
            onClick={guardar}
            disabled={saving}
            style={{
              padding: "7px 20px",
              background: saved ? "var(--success)" : "var(--accent)",
              color: "#fff", border: "none", borderRadius: 6, fontSize: 13, fontWeight: 500,
              cursor: saving ? "default" : "pointer", fontFamily: "inherit",
              display: "flex", alignItems: "center", gap: 6, transition: "background 0.2s",
            }}
          >
            {saved ? <><Check size={13} /> Guardado</> : saving ? "A guardar…" : "Guardar"}
          </button>
        </div>
      </Section>

      {/* 4. Prompts do sistema */}
      <Section>
        <SectionTitle>Prompts do sistema</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          Todos os prompts usados pelo Claude estão em ficheiros de texto na pasta de dados. Edita com qualquer editor — as alterações entram em vigor na próxima invocação, sem reiniciar a aplicação.
        </p>

        <div style={{
          border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden",
        }}>
          {PROMPTS.map((p, i) => (
            <div
              key={p.id}
              style={{
                display: "flex", alignItems: "center", gap: 16,
                padding: "12px 16px",
                borderBottom: i < PROMPTS.length - 1 ? "1px solid var(--border)" : "none",
                background: "var(--bg-surface)",
              }}
            >
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: 2 }}>
                  {p.label}
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.4 }}>
                  {p.desc}
                </div>
              </div>
              <button
                onClick={() => invoke("abrir_ficheiro_prompt", { id: p.id }).catch(console.error)}
                style={{
                  flexShrink: 0,
                  display: "flex", alignItems: "center", gap: 6,
                  padding: "6px 14px", borderRadius: 6,
                  border: "1px solid var(--border)",
                  background: "var(--bg-base)",
                  color: "var(--text-secondary)",
                  fontSize: 12, fontFamily: "inherit", cursor: "pointer",
                  transition: "color 0.1s, border-color 0.1s",
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.color = "var(--text-primary)";
                  (e.currentTarget as HTMLElement).style.borderColor = "var(--text-tertiary)";
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.color = "var(--text-secondary)";
                  (e.currentTarget as HTMLElement).style.borderColor = "var(--border)";
                }}
              >
                <ExternalLink size={12} />
                Abrir
              </button>
            </div>
          ))}
        </div>

        <button
          onClick={() => invoke("abrir_pasta_dados").catch(console.error)}
          style={{
            marginTop: 12,
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 16px", borderRadius: 6,
            border: "1px solid var(--border)",
            background: "transparent",
            color: "var(--text-secondary)",
            fontSize: 13, fontFamily: "inherit", cursor: "pointer",
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-primary)")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")}
        >
          <FolderOpen size={14} />
          Abrir pasta de dados
        </button>
      </Section>
    </div>
  );
};
