import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, AlertTriangle, Lightbulb, Pause, Play, Square, Calendar } from "lucide-react";

interface Vaga {
  id: number;
  titulo: string;
  empresa: string;
  status: string;
  descoberta_em: string;
}

interface VagaAtual {
  id: number;
  titulo: string;
  empresa: string;
  url: string;
  etapa: string | null;
}

interface JanelaAgendamento {
  dia_semana: number;
  inicio: string;
  fim: string;
  ativo: boolean;
}

interface ConfigDisparo {
  ativo: boolean;
  limiar_minutos: number;
  limite_diario: number;
  limite_tempo_minutos: number;
  limite_vagas_sessao?: number;
  janelas: JanelaAgendamento[];
}

const STATUS_LABELS: Record<string, string> = {
  descoberta: "Descoberta",
  analisada: "Analisada",
  candidatando: "A candidatar",
  aplicada: "Aplicada",
  pulada: "Pulada",
  pendente_revisao: "Pendente revisão",
  bloqueada: "Bloqueada",
};

const STATUS_STYLE: Record<string, { background: string; color: string }> = {
  descoberta: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  analisada: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  candidatando: { background: "var(--accent-soft)", color: "var(--accent-strong)" },
  aplicada: { background: "#E3EFE7", color: "var(--success)" },
  pulada: { background: "var(--bg-sunken)", color: "var(--text-tertiary)" },
  pendente_revisao: { background: "#FBEFD9", color: "var(--warning)" },
  bloqueada: { background: "#F7E2DF", color: "var(--danger)" },
};

const DIAS_LABEL = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];

function tempoRelativo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "agora mesmo";
  if (mins < 60) return `há ${mins} min`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `há ${hours}h`;
  return `há ${Math.floor(hours / 24)}d`;
}

function formatarTempo(minutos: number): string {
  const h = Math.floor(minutos / 60);
  const m = Math.round(minutos % 60);
  if (h === 0) return `${m}min`;
  return m === 0 ? `${h}h` : `${h}h ${m}min`;
}

function calcularProximaJanela(janelas: JanelaAgendamento[]): string | null {
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
    if (next.length > 0) return `${DIAS_LABEL[nd]} às ${next[0].inicio}`;
  }
  return "Sem janelas ativas";
}

const CFG_DEFAULT: ConfigDisparo = { ativo: false, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, limite_vagas_sessao: 0, janelas: [] };

export const Dashboard: React.FC<{ onNavigate?: (tab: string, section?: string) => void }> = ({ onNavigate }) => {
  const [candidaturasHoje, setCandidaturasHoje] = useState(0);
  const [vagasHoje, setVagasHoje] = useState(0);
  const [vagasTotal, setVagasTotal] = useState(0);
  const [tempoMinutos, setTempoMinutos] = useState(0);
  const [config, setConfig] = useState<ConfigDisparo>(CFG_DEFAULT);
  const [atividade, setAtividade] = useState<Vaga[]>([]);
  const [vagaAtual, setVagaAtual] = useState<VagaAtual | null>(null);
  const [pendenciasCount, setPendenciasCount] = useState(0);
  const [propostas, setPropostas] = useState(0);
  const [sessionActive, setSessionActive] = useState(false);
  const [paused, setPaused] = useState(false);
  const [loading, setLoading] = useState(true);
  const [disparando, setDisparando] = useState(false);
  const [disparado, setDisparado] = useState(false);

  const [editandoLimite, setEditandoLimite] = useState(false);
  const [limiteInput, setLimiteInput] = useState(10);
  const limiteInputRef = useRef<HTMLInputElement>(null);
  const [editandoTempo, setEditandoTempo] = useState(false);
  const [tempoInput, setTempoInput] = useState(0);
  const tempoInputRef = useRef<HTMLInputElement>(null);
  const [editandoVagas, setEditandoVagas] = useState(false);
  const [vagasInput, setVagasInput] = useState(0);
  const vagasInputRef = useRef<HTMLInputElement>(null);
  const checkpointTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const disparadoTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const carregar = () =>
    Promise.all([
      invoke<number>("candidaturas_hoje"),
      invoke<Vaga[]>("atividade_recente"),
      invoke<VagaAtual | null>("vaga_atual_sessao"),
      invoke<number>("contar_pendencias"),
      invoke<number>("contar_propostas"),
      invoke<number>("vagas_analisadas_hoje"),
      invoke<number>("vagas_analisadas_total"),
      invoke<number>("tempo_sessoes_hoje"),
      invoke<ConfigDisparo>("obter_config_disparo"),
    ]).then(([cands, vagas, atual, pend, props, vagasH, vagasT, tempo, cfg]) => {
      setCandidaturasHoje(cands);
      setAtividade(vagas);
      setVagaAtual(atual);
      setPendenciasCount(pend);
      setPropostas(props);
      setVagasHoje(vagasH);
      setVagasTotal(vagasT);
      setTempoMinutos(Math.round(tempo));
      setConfig(cfg);
      setLimiteInput(cfg.limite_diario);
      setTempoInput(cfg.limite_tempo_minutos);
      setVagasInput(cfg.limite_vagas_sessao ?? 0);
    }).catch(console.error).finally(() => setLoading(false));

  useEffect(() => {
    carregar();

    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen("session-checkpoint-requested", () => {
        if (checkpointTimerRef.current) clearTimeout(checkpointTimerRef.current);
        checkpointTimerRef.current = setTimeout(() => invoke("disparar_sessao", { motivo: "checkpoint" }).catch(console.error), 500);
      }),
      listen("session-started", () => { setSessionActive(true); carregar(); }),
      listen<string>("session-ended", () => { setSessionActive(false); setPaused(false); carregar(); }),
      listen("db-atualizada", () => carregar()),
      listen("chrome-reconnect-failed", () => console.warn("[Claudia RH] Chrome extension reconnection failed")),
    ]).then((fns) => {
      if (active) {
        unlisteners.push(...fns);
      } else {
        fns.forEach((f) => f());
      }
    });

    return () => {
      active = false;
      unlisteners.forEach((f) => f());
      if (checkpointTimerRef.current) clearTimeout(checkpointTimerRef.current);
    };
  }, []);

  // Tick time counter while session is active and not paused
  useEffect(() => {
    if (!sessionActive || paused) return;
    const id = setInterval(() => {
      invoke<number>("tempo_sessoes_hoje").then(t => setTempoMinutos(Math.round(t))).catch(() => {});
    }, 60000);
    return () => clearInterval(id);
  }, [sessionActive, paused]);

  const disparar = async () => {
    setDisparando(true);
    try {
      await invoke("disparar_sessao", { motivo: "manual" });
      setDisparado(true);
      if (disparadoTimerRef.current) clearTimeout(disparadoTimerRef.current);
      disparadoTimerRef.current = setTimeout(() => setDisparado(false), 3000);
      await carregar();
    } catch (e) { console.error(e); } finally { setDisparando(false); }
  };

  const pausar = async () => {
    try {
      await invoke("escrever_pty", { input: "\x03" });
      invoke("registar_pausa_sessao").catch(console.error);
      setPaused(true);
    } catch (e) { console.error(e); }
  };
  const retomar = async () => {
    try {
      await invoke("escrever_pty", { input: "continue\r" });
      invoke("registar_retoma_sessao").catch(console.error);
      setPaused(false);
      invoke<number>("tempo_sessoes_hoje").then(t => setTempoMinutos(Math.round(t))).catch(() => {});
    } catch (e) { console.error(e); }
  };
  const interromper = async () => { try { await invoke("parar_pty"); setSessionActive(false); setPaused(false); } catch (e) { console.error(e); } };

  const salvarLimite = async () => {
    setEditandoLimite(false);
    if (limiteInput === config.limite_diario || limiteInput < 1) return;
    try {
      await invoke("configurar_limite_diario", { limite: limiteInput });
      setConfig(prev => ({ ...prev, limite_diario: limiteInput }));
    } catch (e) { console.error(e); }
  };

  const salvarTempo = async () => {
    setEditandoTempo(false);
    const val = Math.max(0, tempoInput);
    if (val === config.limite_tempo_minutos) return;
    try {
      await invoke("configurar_disparo", {
        ativo: config.ativo,
        limiarMinutos: config.limiar_minutos,
        limiteDiario: config.limite_diario,
        limiteTempoMinutos: val,
        janelas: config.janelas,
      });
      setConfig(prev => ({ ...prev, limite_tempo_minutos: val }));
    } catch (e) { console.error(e); }
  };

  const salvarVagas = async () => {
    setEditandoVagas(false);
    const val = Math.max(0, vagasInput);
    if (val === (config.limite_vagas_sessao ?? 0)) return;
    try {
      await invoke("configurar_limite_vagas_sessao", { limite: val });
      setConfig(prev => ({ ...prev, limite_vagas_sessao: val }));
    } catch (e) { console.error(e); }
  };

  const toggleAtivo = async () => {
    const novoAtivo = !config.ativo;
    try {
      await invoke("configurar_disparo", {
        ativo: novoAtivo,
        limiarMinutos: config.limiar_minutos,
        limiteDiario: config.limite_diario,
        limiteTempoMinutos: config.limite_tempo_minutos,
        janelas: config.janelas,
      });
      setConfig(prev => ({ ...prev, ativo: novoAtivo }));
    } catch (e) { console.error(e); }
  };

  const proximaJanela = useMemo(() => calcularProximaJanela(config.janelas), [config.janelas]);
  const limiteEsgotado = config.limite_tempo_minutos > 0 && tempoMinutos >= config.limite_tempo_minutos;
  const pctCandidaturas = Math.min((candidaturasHoje / Math.max(config.limite_diario, 1)) * 100, 100);
  const pctTempo = config.limite_tempo_minutos > 0 ? Math.min((tempoMinutos / config.limite_tempo_minutos) * 100, 100) : 0;

  return (
    <div style={{ padding: 24 }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 16 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
          Dashboard
        </h1>
        {sessionActive && (
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{
              width: 8, height: 8, borderRadius: "50%",
              background: paused ? "var(--warning)" : "var(--success)",
              animation: paused ? "none" : "pulse 2s infinite",
              display: "inline-block",
            }} />
            <span style={{ fontSize: 13, color: paused ? "var(--warning)" : "var(--success)", fontWeight: 500 }}>
              {paused ? "Pausada" : "A trabalhar…"}
            </span>
          </div>
        )}
      </div>

      {/* Vaga em curso */}
      {vagaAtual && (
        <div style={{
          background: "var(--accent-soft)", border: "1px solid var(--accent)",
          borderRadius: 8, padding: "12px 16px", marginBottom: 12,
          display: "flex", alignItems: "center", gap: 12,
        }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 12, color: "var(--accent-strong)", fontWeight: 500, marginBottom: 2 }}>Agora a candidatar</div>
            <div style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {vagaAtual.titulo}
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {vagaAtual.empresa}{vagaAtual.etapa && ` — ${vagaAtual.etapa}`}
            </div>
          </div>
          <button onClick={() => openUrl(vagaAtual.url).catch(console.error)}
            style={{ background: "transparent", border: "none", cursor: "pointer", padding: 4, color: "var(--accent-strong)", flexShrink: 0, display: "flex", alignItems: "center" }}>
            <ExternalLink size={14} />
          </button>
        </div>
      )}

      {/* Pendências */}
      {pendenciasCount > 0 && (
        <div style={{
          background: "#FBEFD9", border: "1px solid var(--warning)", borderRadius: 8,
          padding: "10px 16px", marginBottom: 12,
          display: "flex", alignItems: "center", gap: 10, fontSize: 13, color: "var(--warning)", fontWeight: 500,
        }}>
          <AlertTriangle size={16} />
          {pendenciasCount === 1 ? "1 pendência aguarda resolução" : `${pendenciasCount} pendências aguardam resolução`}
        </div>
      )}

      {/* Propostas */}
      {propostas > 0 && (
        <div style={{
          background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8,
          padding: "10px 16px", marginBottom: 12,
          display: "flex", alignItems: "center", gap: 10, fontSize: 13, color: "var(--text-secondary)",
        }}>
          <Lightbulb size={16} style={{ color: "var(--accent)", flexShrink: 0 }} />
          <span>{propostas === 1 ? "1 proposta de evolução do perfil disponível" : `${propostas} propostas de evolução do perfil disponíveis`}</span>
        </div>
      )}

      {/* ── Grelha de indicadores 2×2 ── */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 16 }}>

        {/* Card 1 — Candidaturas hoje (limite editável) */}
        <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
          <div style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: 6 }}>
            Candidaturas hoje
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 5, marginBottom: 10 }}>
            <span style={{ fontSize: 28, fontWeight: 600, color: "var(--text-primary)", lineHeight: 1 }}>
              {loading ? "—" : candidaturasHoje}
            </span>
            <span style={{ fontSize: 13, color: "var(--text-tertiary)" }}>/</span>
            {editandoLimite ? (
              <input
                ref={limiteInputRef}
                type="number" min={1} max={99} value={limiteInput} autoFocus
                onChange={e => setLimiteInput(Math.max(1, Math.min(99, parseInt(e.target.value) || 1)))}
                onBlur={salvarLimite}
                onKeyDown={e => { if (e.key === "Enter") salvarLimite(); if (e.key === "Escape") setEditandoLimite(false); }}
                style={{
                  width: 44, fontSize: 18, fontWeight: 600, color: "var(--accent)",
                  background: "var(--bg-sunken)", border: "1px solid var(--accent)",
                  borderRadius: 4, padding: "1px 4px", fontFamily: "inherit",
                  textAlign: "center", outline: "none",
                }}
              />
            ) : (
              <button
                onClick={() => { setLimiteInput(config.limite_diario); setEditandoLimite(true); }}
                title="Editar limite diário"
                style={{
                  fontSize: 18, fontWeight: 600, color: "var(--text-secondary)",
                  background: "transparent", border: "none", cursor: "pointer",
                  fontFamily: "inherit", padding: 0,
                  borderBottom: "1px dashed var(--border)",
                  lineHeight: 1,
                }}
              >
                {config.limite_diario}
              </button>
            )}
          </div>
          <div style={{ height: 3, background: "var(--bg-sunken)", borderRadius: 2 }}>
            <div style={{
              width: `${pctCandidaturas}%`, height: "100%",
              background: candidaturasHoje >= config.limite_diario ? "var(--success)" : "var(--accent)",
              borderRadius: 2, transition: "width 0.4s ease",
            }} />
          </div>
        </div>

        {/* Card 2 — Vagas analisadas */}
        <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
          <div style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: 6 }}>
            Vagas analisadas
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 5, marginBottom: 6 }}>
            <span style={{ fontSize: 28, fontWeight: 600, color: "var(--text-primary)", lineHeight: 1 }}>
              {loading ? "—" : vagasHoje}
            </span>
            {(config.limite_vagas_sessao ?? 0) > 0 && (
              <>
                <span style={{ fontSize: 13, color: "var(--text-tertiary)" }}>/</span>
                {editandoVagas ? (
                  <input
                    ref={vagasInputRef}
                    type="number" min={0} max={999} value={vagasInput} autoFocus
                    onChange={e => setVagasInput(Math.max(0, parseInt(e.target.value) || 0))}
                    onBlur={salvarVagas}
                    onKeyDown={e => { if (e.key === "Enter") salvarVagas(); if (e.key === "Escape") { setEditandoVagas(false); setVagasInput(config.limite_vagas_sessao ?? 0); } }}
                    style={{
                      width: 52, fontSize: 18, fontWeight: 600, color: "var(--accent)",
                      background: "var(--bg-sunken)", border: "1px solid var(--accent)",
                      borderRadius: 4, padding: "1px 4px", fontFamily: "inherit",
                      textAlign: "center", outline: "none",
                    }}
                  />
                ) : (
                  <button
                    onClick={() => { setVagasInput(config.limite_vagas_sessao ?? 0); setEditandoVagas(true); }}
                    title="Editar limite por sessão"
                    style={{
                      fontSize: 18, fontWeight: 600, color: "var(--text-secondary)",
                      background: "transparent", border: "none", cursor: "pointer",
                      fontFamily: "inherit", padding: 0,
                      borderBottom: "1px dashed var(--border)", lineHeight: 1,
                    }}
                  >
                    {config.limite_vagas_sessao}
                  </button>
                )}
              </>
            )}
          </div>
          {(config.limite_vagas_sessao ?? 0) === 0 && (
            <div style={{ fontSize: 12, color: "var(--text-tertiary)", display: "flex", alignItems: "center", gap: 6 }}>
              hoje · <span style={{ color: "var(--text-secondary)", fontWeight: 500 }}>{vagasTotal}</span> total
              {editandoVagas ? (
                <div style={{ display: "flex", alignItems: "baseline", gap: 3 }}>
                  <input
                    ref={vagasInputRef}
                    type="number" min={0} max={999} value={vagasInput} autoFocus
                    onChange={e => setVagasInput(Math.max(0, parseInt(e.target.value) || 0))}
                    onBlur={salvarVagas}
                    onKeyDown={e => { if (e.key === "Enter") salvarVagas(); if (e.key === "Escape") { setEditandoVagas(false); setVagasInput(0); } }}
                    style={{
                      width: 52, fontSize: 13, fontWeight: 600, color: "var(--accent)",
                      background: "var(--bg-sunken)", border: "1px solid var(--accent)",
                      borderRadius: 4, padding: "1px 4px", fontFamily: "inherit",
                      textAlign: "center", outline: "none",
                    }}
                  />
                  <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>por sessão</span>
                </div>
              ) : (
                <button
                  onClick={() => { setVagasInput(20); setEditandoVagas(true); }}
                  title="Definir limite por sessão"
                  style={{ fontSize: 11, color: "var(--accent)", background: "transparent", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit" }}
                >
                  + limite
                </button>
              )}
            </div>
          )}
          {(config.limite_vagas_sessao ?? 0) > 0 && (
            <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
              hoje · <span style={{ color: "var(--text-secondary)", fontWeight: 500 }}>{vagasTotal}</span> total
            </div>
          )}
        </div>

        {/* Card 3 — Tempo de procura */}
        <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
          <div style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: 6 }}>
            Tempo de procura
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 5, marginBottom: config.limite_tempo_minutos > 0 ? 10 : 4 }}>
            <span style={{ fontSize: 22, fontWeight: 600, color: limiteEsgotado ? "var(--danger)" : "var(--text-primary)", lineHeight: 1 }}>
              {formatarTempo(tempoMinutos)}
            </span>
            {config.limite_tempo_minutos > 0 && (
              <>
                <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>/</span>
                {editandoTempo ? (
                  <div style={{ display: "flex", alignItems: "baseline", gap: 3 }}>
                    <input
                      ref={tempoInputRef}
                      type="number" min={0} max={480} value={tempoInput} autoFocus
                      onChange={e => setTempoInput(Math.max(0, parseInt(e.target.value) || 0))}
                      onBlur={salvarTempo}
                      onKeyDown={e => { if (e.key === "Enter") salvarTempo(); if (e.key === "Escape") { setEditandoTempo(false); setTempoInput(config.limite_tempo_minutos); } }}
                      style={{
                        width: 52, fontSize: 14, fontWeight: 600, color: "var(--accent)",
                        background: "var(--bg-sunken)", border: "1px solid var(--accent)",
                        borderRadius: 4, padding: "1px 4px", fontFamily: "inherit",
                        textAlign: "center", outline: "none",
                      }}
                    />
                    <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>min</span>
                  </div>
                ) : (
                  <button
                    onClick={() => { setTempoInput(config.limite_tempo_minutos); setEditandoTempo(true); }}
                    title="Editar limite de tempo"
                    style={{
                      fontSize: 14, fontWeight: 600, color: "var(--text-secondary)",
                      background: "transparent", border: "none", cursor: "pointer",
                      fontFamily: "inherit", padding: 0,
                      borderBottom: "1px dashed var(--border)", lineHeight: 1,
                    }}
                  >
                    {formatarTempo(config.limite_tempo_minutos)}
                  </button>
                )}
              </>
            )}
          </div>
          {config.limite_tempo_minutos > 0 && (
            <div style={{ height: 3, background: "var(--bg-sunken)", borderRadius: 2 }}>
              <div style={{
                width: `${pctTempo}%`, height: "100%",
                background: limiteEsgotado ? "var(--danger)" : "var(--accent)",
                borderRadius: 2, transition: "width 0.3s ease",
              }} />
            </div>
          )}
          {limiteEsgotado && (
            <div style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>Limite atingido hoje</div>
          )}
          {config.limite_tempo_minutos === 0 && (
            <div style={{ fontSize: 11, color: "var(--text-tertiary)", display: "flex", alignItems: "center", gap: 6 }}>
              {editandoTempo ? (
                <div style={{ display: "flex", alignItems: "baseline", gap: 3 }}>
                  <input
                    ref={tempoInputRef}
                    type="number" min={0} max={480} value={tempoInput} autoFocus
                    onChange={e => setTempoInput(Math.max(0, parseInt(e.target.value) || 0))}
                    onBlur={salvarTempo}
                    onKeyDown={e => { if (e.key === "Enter") salvarTempo(); if (e.key === "Escape") { setEditandoTempo(false); setTempoInput(0); } }}
                    style={{
                      width: 52, fontSize: 14, fontWeight: 600, color: "var(--accent)",
                      background: "var(--bg-sunken)", border: "1px solid var(--accent)",
                      borderRadius: 4, padding: "1px 4px", fontFamily: "inherit",
                      textAlign: "center", outline: "none",
                    }}
                  />
                  <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>min (0 = sem limite)</span>
                </div>
              ) : (
                <>
                  sem limite
                  <button
                    onClick={() => { setTempoInput(60); setEditandoTempo(true); }}
                    title="Definir limite de tempo"
                    style={{ fontSize: 11, color: "var(--accent)", background: "transparent", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit" }}
                  >
                    + definir
                  </button>
                </>
              )}
            </div>
          )}
        </div>

        {/* Card 4 — Agendamento */}
        <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 5, marginBottom: 10 }}>
            <Calendar size={11} style={{ color: "var(--text-tertiary)" }} />
            <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.05em", flex: 1 }}>
              Agendamento
            </span>
            <button
              onClick={toggleAtivo}
              title={config.ativo ? "Desativar modo automático" : "Ativar modo automático"}
              style={{
                fontSize: 11, fontWeight: 600, padding: "2px 8px", borderRadius: 10,
                border: "none", cursor: "pointer", fontFamily: "inherit",
                background: config.ativo ? "var(--success)" : "var(--bg-sunken)",
                color: config.ativo ? "#fff" : "var(--text-tertiary)",
                transition: "background 0.2s, color 0.2s",
              }}
            >
              {config.ativo ? "Auto" : "Manual"}
            </button>
          </div>
          {proximaJanela === null ? (
            <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
              Sem janelas configuradas
              <button
                onClick={() => onNavigate?.("configuracoes")}
                style={{ marginLeft: 6, fontSize: 11, color: "var(--accent)", background: "transparent", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit" }}
              >
                Configurar →
              </button>
            </div>
          ) : (
            <>
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
                <span style={{
                  width: 7, height: 7, borderRadius: "50%", flexShrink: 0,
                  background: proximaJanela === "ATIVO_AGORA" ? "var(--success)" : "var(--text-tertiary)",
                  display: "inline-block",
                  animation: proximaJanela === "ATIVO_AGORA" ? "pulse 2s infinite" : "none",
                }} />
                <span style={{ fontSize: 13, fontWeight: 500, color: proximaJanela === "ATIVO_AGORA" ? "var(--success)" : "var(--text-primary)" }}>
                  {proximaJanela === "ATIVO_AGORA" ? "Ativo agora" : `Próximo: ${proximaJanela}`}
                </span>
              </div>
              <div style={{ fontSize: 11, color: "var(--text-tertiary)", display: "flex", alignItems: "center", gap: 6 }}>
                {config.janelas.filter(j => j.ativo).length} janela{config.janelas.filter(j => j.ativo).length !== 1 ? "s" : ""} ativa{config.janelas.filter(j => j.ativo).length !== 1 ? "s" : ""}
                <button
                  onClick={() => onNavigate?.("configuracoes")}
                  style={{ fontSize: 11, color: "var(--accent)", background: "transparent", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit" }}
                >
                  Editar →
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Controlo de sessão */}
      {sessionActive ? (
        <div style={{ display: "flex", gap: 8, marginBottom: 24 }}>
          <button
            onClick={paused ? retomar : pausar}
            style={{
              flex: 1, padding: "11px 0", borderRadius: 8, fontSize: 14, fontWeight: 500,
              fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
              display: "flex", alignItems: "center", justifyContent: "center", gap: 7,
              background: paused ? "var(--accent)" : "var(--bg-sunken)",
              color: paused ? "#fff" : "var(--text-secondary)",
              border: paused ? "none" : "1px solid var(--border)",
            }}
          >
            {paused ? <><Play size={14} /> Retomar</> : <><Pause size={14} /> Pausar</>}
          </button>
          <button
            onClick={interromper}
            style={{
              padding: "11px 20px", borderRadius: 8, fontSize: 14, fontWeight: 500,
              fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
              display: "flex", alignItems: "center", gap: 7,
              background: "#F7E2DF", color: "var(--danger)",
              border: "1px solid var(--danger)",
            }}
          >
            <Square size={13} fill="currentColor" /> Interromper
          </button>
        </div>
      ) : (
        <button
          onClick={disparar}
          disabled={disparando}
          style={{
            width: "100%", padding: "12px 0",
            background: disparado ? "var(--success)" : "var(--accent)",
            color: "#fff", border: "none", borderRadius: 8, fontSize: 15, fontWeight: 500,
            cursor: disparando ? "default" : "pointer", fontFamily: "inherit",
            marginBottom: 24, transition: "background 0.2s", opacity: disparando ? 0.8 : 1,
          }}
        >
          {disparando ? "A iniciar sessão…" : disparado ? "✓ Sessão iniciada — ver Terminal" : "Procurar vagas agora"}
        </button>
      )}

      {/* Atividade recente */}
      <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 12 }}>
        Atividade recente
      </div>

      {loading ? (
        <div style={{ color: "var(--text-tertiary)", fontSize: 14 }}>A carregar…</div>
      ) : atividade.length === 0 ? (
        <div style={{ color: "var(--text-secondary)", fontSize: 14 }}>Nenhuma atividade ainda.</div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {atividade.map((v) => {
            const s = STATUS_STYLE[v.status] ?? STATUS_STYLE.descoberta;
            return (
              <div key={v.id} style={{ display: "flex", alignItems: "center", gap: 12, padding: "10px 0", borderBottom: "1px solid var(--border)" }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                    {v.titulo}
                  </div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{v.empresa}</div>
                </div>
                <span style={{ fontSize: 12, fontWeight: 500, padding: "2px 8px", borderRadius: 6, whiteSpace: "nowrap", ...s }}>
                  {STATUS_LABELS[v.status] ?? v.status}
                </span>
                <span style={{ fontSize: 12, color: "var(--text-tertiary)", whiteSpace: "nowrap" }}>
                  {tempoRelativo(v.descoberta_em)}
                </span>
              </div>
            );
          })}
        </div>
      )}

      <style>{`
        @keyframes pulse {
          0% { box-shadow: 0 0 0 0 rgba(34,197,94,0.4); }
          70% { box-shadow: 0 0 0 6px rgba(34,197,94,0); }
          100% { box-shadow: 0 0 0 0 rgba(34,197,94,0); }
        }
      `}</style>
    </div>
  );
};
