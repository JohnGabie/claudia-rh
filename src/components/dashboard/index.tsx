import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, AlertTriangle, Lightbulb, Pause, Play, Square, Pencil } from "lucide-react";
import { useT } from "../../i18n";
import { VagaResumo, SearchVariant } from "../../types";
import { tempoRelativo, formatarTempo, formatTempoCompact } from "../../lib/format";
import { VagaAtual, ConfigDisparo, StatusLinkedinRede, ModalCfg, JanelaAgendamento } from "./types";
import { STATUS_STYLE, CFG_DEFAULT, VALS_CANDIDATURAS, VALS_VAGAS, VALS_TEMPO, calcularProximaJanela } from "./constants";
import { LimitModal } from "./LimitModal";
import { AgendamentoModal } from "./AgendamentoModal";
import { VariantCardDash } from "./VariantCardDash";
import { ModoCard } from "./ModoCard";

// ── Dashboard ─────────────────────────────────────────────────────────────────
export const Dashboard: React.FC<{ onNavigate?: (tab: string, section?: string) => void }> = ({ onNavigate: _onNavigate }) => {
  const t = useT();
  const [candidaturasHoje, setCandidaturasHoje] = useState(0);
  const [vagasHoje, setVagasHoje] = useState(0);
  const [vagasTotal, setVagasTotal] = useState(0);
  const [tempoMinutos, setTempoMinutos] = useState(0);
  const [config, setConfig] = useState<ConfigDisparo>(CFG_DEFAULT);
  const [atividade, setAtividade] = useState<VagaResumo[]>([]);
  const [vagaAtual, setVagaAtual] = useState<VagaAtual | null>(null);
  const [pendenciasCount, setPendenciasCount] = useState(0);
  const [propostas, setPropostas] = useState(0);
  const [sessionActive, setSessionActive] = useState(false);
  const [paused, setPaused] = useState(false);
  const [loading, setLoading] = useState(true);
  const [disparando, setDisparando] = useState(false);
  const [disparado, setDisparado] = useState(false);
  const [modal, setModal] = useState<ModalCfg | null>(null);
  const [modalAgendamento, setModalAgendamento] = useState(false);

  const [incluirBuscaNormal, setIncluirBuscaNormal] = useState(true);
  const [incluirLinkedinRede, setIncluirLinkedinRede] = useState(false);
  const [modoPesos, setModoPesos] = useState<Record<string, number>>({ busca_normal: 100, linkedin_rede: 0 });
  const committedModoPesosRef = useRef<Record<string, number>>({ busca_normal: 100, linkedin_rede: 0 });
  const [variants, setVariants] = useState<SearchVariant[]>([]);
  const [localPesos, setLocalPesos] = useState<Record<string, number>>({});
  const committedPesosRef = useRef<Record<string, number>>({});
  const [abaAtiva, setAbaAtiva] = useState<"procura" | "atividade">("procura");

  const checkpointTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const disparadoTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [_linkedinStatus, setLinkedinStatus] = useState<StatusLinkedinRede>({ ativo: false, ultima_busca: null, vagas_encontradas: 0 });
  const [_linkedinScanning, setLinkedinScanning] = useState(false);

  const carregarLinkedin = () =>
    invoke<StatusLinkedinRede>("obter_status_linkedin_rede")
      .then((status) => { setLinkedinStatus(status); setLinkedinScanning(status.ativo); })
      .catch(console.error);

  const carregarVariantes = () =>
    invoke<SearchVariant[]>("ler_search_variants")
      .then((vs) => {
        setVariants(vs);
        const map: Record<string, number> = {};
        vs.forEach(v => { map[v.id] = v.peso; });
        setLocalPesos(map);
        committedPesosRef.current = map;
      })
      .catch(() => {});

  const activeVariants = variants.filter(v => v.ativa);
  const totalPeso = activeVariants.reduce((s, v) => s + (localPesos[v.id] ?? v.peso), 0) || 1;
  const maxPct = activeVariants.length > 1 ? 100 - (activeVariants.length - 1) * 5 : 95;

  const handleDragBar = (variantId: string, newPct: number) => {
    setLocalPesos(prev => {
      const curTotal = activeVariants.reduce((s, v) => s + (prev[v.id] ?? v.peso), 0);
      const newPesoForId = (newPct / 100) * curTotal;
      const others = activeVariants.filter(v => v.id !== variantId);
      const sumOthers = others.reduce((s, v) => s + (prev[v.id] ?? v.peso), 0);
      const remaining = curTotal - newPesoForId;
      const minPeso = (5 / 100) * curTotal;
      const next: Record<string, number> = { ...prev, [variantId]: newPesoForId };
      others.forEach(v => {
        const oldPeso = prev[v.id] ?? v.peso;
        next[v.id] = sumOthers > 0 ? Math.max(minPeso, (oldPeso / sumOthers) * remaining) : remaining / others.length;
      });
      committedPesosRef.current = next;
      return next;
    });
  };

  const handleDragEnd = async () => {
    const pesos: Record<string, number> = {};
    variants.forEach(v => { pesos[v.id] = committedPesosRef.current[v.id] ?? v.peso; });
    await invoke("guardar_pesos_variantes", { pesos }).catch(console.error);
    carregarVariantes();
  };

  const handleToggleAtiva = async (variantId: string) => {
    const v = variants.find(x => x.id === variantId);
    if (!v) return;
    await invoke("guardar_variante_unica", { variante: { ...v, ativa: !v.ativa } }).catch(console.error);
    carregarVariantes();
  };

  const carregar = () =>
    Promise.all([
      invoke<number>("candidaturas_hoje"),
      invoke<VagaResumo[]>("atividade_recente"),
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
    }).catch(console.error).finally(() => setLoading(false));

  useEffect(() => {
    carregar();
    carregarLinkedin();
    carregarVariantes();

    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen("session-checkpoint-requested", () => {
        if (checkpointTimerRef.current) clearTimeout(checkpointTimerRef.current);
        checkpointTimerRef.current = setTimeout(() => invoke("disparar_sessao", { motivo: "checkpoint" }).catch(console.error), 500);
      }),
      listen("session-started", () => { setSessionActive(true); carregar(); }),
      listen<string>("session-ended", () => {
        setSessionActive(false);
        setPaused(false);
        setLinkedinScanning(false);
        carregar();
        carregarLinkedin();
      }),
      listen("linkedin-session-started", () => { setLinkedinScanning(true); setSessionActive(true); }),
      listen("db-atualizada", () => { carregar(); carregarLinkedin(); carregarVariantes(); }),
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

  const MODOS = [
    { id: "busca_normal", ativo: incluirBuscaNormal },
    { id: "linkedin_rede", ativo: incluirLinkedinRede },
  ];
  const modosAtivos = MODOS.filter(m => m.ativo);
  const totalModoPeso = modosAtivos.reduce((s, m) => s + (modoPesos[m.id] ?? 0), 0) || 1;
  const maxModoPct = modosAtivos.length > 1 ? 100 - (modosAtivos.length - 1) * 5 : 95;

  const redistribuir = (ativos: string[]) => {
    const peso = ativos.length > 0 ? 100 / ativos.length : 0;
    const next: Record<string, number> = { busca_normal: 0, linkedin_rede: 0 };
    ativos.forEach(id => { next[id] = peso; });
    setModoPesos(next);
    committedModoPesosRef.current = next;
  };

  const toggleBuscaNormal = () => {
    const novoAtivo = !incluirBuscaNormal;
    setIncluirBuscaNormal(novoAtivo);
    const ativos = [
      ...(novoAtivo ? ["busca_normal"] : []),
      ...(incluirLinkedinRede ? ["linkedin_rede"] : []),
    ];
    redistribuir(ativos);
  };

  const toggleLinkedinRede = () => {
    const novoAtivo = !incluirLinkedinRede;
    setIncluirLinkedinRede(novoAtivo);
    const ativos = [
      ...(incluirBuscaNormal ? ["busca_normal"] : []),
      ...(novoAtivo ? ["linkedin_rede"] : []),
    ];
    redistribuir(ativos);
  };

  const handleModoDragBar = (modoId: string, newPct: number) => {
    setModoPesos(prev => {
      const curTotal = modosAtivos.reduce((s, m) => s + (prev[m.id] ?? 0), 0);
      const newPesoForId = (newPct / 100) * curTotal;
      const others = modosAtivos.filter(m => m.id !== modoId);
      const sumOthers = others.reduce((s, m) => s + (prev[m.id] ?? 0), 0);
      const remaining = curTotal - newPesoForId;
      const minPeso = (5 / 100) * curTotal;
      const next: Record<string, number> = { ...prev, [modoId]: newPesoForId };
      others.forEach(m => {
        const old = prev[m.id] ?? 0;
        next[m.id] = sumOthers > 0 ? Math.max(minPeso, (old / sumOthers) * remaining) : remaining / others.length;
      });
      committedModoPesosRef.current = next;
      return next;
    });
  };

  const handleModoDragEnd = () => { /* pesos já em committedModoPesosRef, sem backend por agora */ };

  const disparar = async () => {
    if (!incluirBuscaNormal && !incluirLinkedinRede) return;
    setDisparando(true);
    try {
      if (incluirLinkedinRede && !incluirBuscaNormal) {
        await invoke("iniciar_busca_linkedin_rede");
        setLinkedinScanning(true);
      } else {
        await invoke("disparar_sessao", { motivo: "manual" });
        if (incluirLinkedinRede) setLinkedinScanning(true);
      }
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

  const salvarLimite = async (val: number) => {
    if (val < 1 || val === config.limite_diario) return;
    try {
      await invoke("configurar_limite_diario", { limite: val });
      setConfig(prev => ({ ...prev, limite_diario: val }));
    } catch (e) { console.error(e); }
  };

  const salvarVagas = async (val: number) => {
    if (val === (config.limite_vagas_sessao ?? 0)) return;
    try {
      await invoke("configurar_limite_vagas_sessao", { limite: val });
      setConfig(prev => ({ ...prev, limite_vagas_sessao: val }));
    } catch (e) { console.error(e); }
  };

  const salvarTempo = async (val: number) => {
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

  const salvarAgendamento = async (ativo: boolean, limiar: number, janelas: JanelaAgendamento[]) => {
    try {
      await invoke("configurar_disparo", {
        ativo,
        limiarMinutos: limiar,
        limiteDiario: config.limite_diario,
        limiteTempoMinutos: config.limite_tempo_minutos,
        janelas,
      });
      setConfig(prev => ({ ...prev, ativo, limiar_minutos: limiar, janelas }));
    } catch (e) { console.error(e); }
  };


  // Modal openers
  const abrirCandidaturas = () => setModal({
    titulo: t.dashboard.searchLimit,
    subtitulo: t.dashboard.maxPerDay,
    valores: VALS_CANDIDATURAS,
    valorAtual: config.limite_diario,
    formatValue: (v) => String(v),
    onSave: salvarLimite,
  });

  const abrirVagas = () => setModal({
    titulo: t.dashboard.jobLimit,
    subtitulo: t.dashboard.perSession,
    valores: VALS_VAGAS,
    valorAtual: config.limite_vagas_sessao ?? 0,
    formatValue: (v) => v === 0 ? "∞" : String(v),
    onSave: salvarVagas,
  });

  const abrirTempo = () => setModal({
    titulo: t.dashboard.timeLimit,
    subtitulo: t.dashboard.perDay,
    valores: VALS_TEMPO,
    valorAtual: config.limite_tempo_minutos,
    formatValue: formatTempoCompact,
    onSave: salvarTempo,
  });

  const proximaJanela = useMemo(() => calcularProximaJanela(config.janelas, t.dashboard.days), [config.janelas, t.dashboard.days]);
  const limiteEsgotado = config.limite_tempo_minutos > 0 && tempoMinutos >= config.limite_tempo_minutos;
  const pctCandidaturas = Math.min((candidaturasHoje / Math.max(config.limite_diario, 1)) * 100, 100);
  const pctTempo = config.limite_tempo_minutos > 0 ? Math.min((tempoMinutos / config.limite_tempo_minutos) * 100, 100) : 0;
  const limVagas = config.limite_vagas_sessao ?? 0;
  const pctVagas = limVagas > 0 ? Math.min((vagasHoje / limVagas) * 100, 100) : 0;

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
              {paused ? t.dashboard.paused : t.dashboard.working}
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
            <div style={{ fontSize: 12, color: "var(--accent-strong)", fontWeight: 500, marginBottom: 2 }}>{t.dashboard.nowApplying}</div>
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
          {pendenciasCount === 1 ? t.dashboard.pending_one : `${pendenciasCount}${t.dashboard.pending_many}`}
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
          <span>{propostas === 1 ? t.dashboard.suggestions_one : `${propostas}${t.dashboard.suggestions_many}`}</span>
        </div>
      )}

      {/* ── Tab bar ── */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border)", marginBottom: 16 }}>
        {([["procura", t.dashboard.tabSearch], ["atividade", t.dashboard.tabActivity]] as const).map(([id, label]) => {
          const ativo = abaAtiva === id;
          return (
            <button key={id} onClick={() => setAbaAtiva(id)} style={{
              padding: "8px 18px", fontSize: 13, fontWeight: ativo ? 600 : 400,
              color: ativo ? "var(--accent-strong)" : "var(--text-secondary)",
              background: "transparent", border: "none",
              borderBottom: ativo ? "2px solid var(--accent)" : "2px solid transparent",
              cursor: "pointer", fontFamily: "inherit", marginBottom: -1,
              transition: "color 0.15s, border-color 0.15s",
            }}>
              {label}
            </button>
          );
        })}
      </div>

      {/* ── Tab: Procura ── */}
      {abaAtiva === "procura" && (
        <>
          {/* Controlo de sessão */}
          {sessionActive ? (
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <button onClick={paused ? retomar : pausar} style={{
                flex: 1, padding: "11px 0", borderRadius: 8, fontSize: 14, fontWeight: 500,
                fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
                display: "flex", alignItems: "center", justifyContent: "center", gap: 7,
                background: paused ? "var(--accent)" : "var(--bg-sunken)",
                color: paused ? "#fff" : "var(--text-secondary)",
                border: paused ? "none" : "1px solid var(--border)",
              }}>
                {paused ? <><Play size={14} /> {t.dashboard.resume}</> : <><Pause size={14} /> {t.dashboard.pause}</>}
              </button>
              <button onClick={interromper} style={{
                padding: "11px 20px", borderRadius: 8, fontSize: 14, fontWeight: 500,
                fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
                display: "flex", alignItems: "center", gap: 7,
                background: "#F7E2DF", color: "var(--danger)", border: "1px solid var(--danger)",
              }}>
                <Square size={13} fill="currentColor" /> {t.dashboard.interrupt}
              </button>
            </div>
          ) : (
            <div style={{ marginBottom: 16 }}>
              {/* Modos de procura */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
                <ModoCard
                  label={t.dashboard.searchJobs}
                  ativo={incluirBuscaNormal}
                  pct={modosAtivos.length > 0 ? Math.round(((modoPesos["busca_normal"] ?? 0) / totalModoPeso) * 100) : 0}
                  maxPct={maxModoPct}
                  onToggle={toggleBuscaNormal}
                  onDragBar={(p) => handleModoDragBar("busca_normal", p)}
                  onDragEnd={handleModoDragEnd}
                />
                <ModoCard
                  label={t.dashboard.searchLinkedIn}
                  ativo={incluirLinkedinRede}
                  pct={modosAtivos.length > 0 ? Math.round(((modoPesos["linkedin_rede"] ?? 0) / totalModoPeso) * 100) : 0}
                  maxPct={maxModoPct}
                  onToggle={toggleLinkedinRede}
                  onDragBar={(p) => handleModoDragBar("linkedin_rede", p)}
                  onDragEnd={handleModoDragEnd}
                />
                <ModoCard label={t.dashboard.searchFreelas} ativo={false} pct={0} maxPct={95} emBreve />
                <ModoCard label={t.dashboard.searchCompanySites} ativo={false} pct={0} maxPct={95} emBreve />
              </div>
              <button onClick={disparar} disabled={disparando} style={{
                width: "100%", padding: "12px 0",
                background: disparado ? "var(--success)" : "var(--accent)",
                color: "#fff", border: "none", borderRadius: 8, fontSize: 15, fontWeight: 500,
                cursor: disparando ? "default" : "pointer", fontFamily: "inherit",
                transition: "background 0.2s", opacity: disparando ? 0.8 : 1,
                whiteSpace: "nowrap",
              }}>
                {disparando ? t.dashboard.starting : disparado ? t.dashboard.sessionStarted : incluirLinkedinRede && !incluirBuscaNormal ? t.dashboard.searchLinkedInNow : t.dashboard.searchJobsNow}
              </button>
            </div>
          )}

          {/* Grelha de indicadores 2×2 */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 16 }}>
            {/* Card 1 — Candidaturas hoje */}
            {(() => {
              const reached = candidaturasHoje >= config.limite_diario;
              const barColor = reached ? "var(--success)" : pctCandidaturas >= 80 ? "var(--warning)" : "var(--accent)";
              return (
                <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                    <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>{t.dashboard.applicationsToday}</span>
                    <button onClick={abrirCandidaturas} title="Editar limite" className="edit-icon-btn"><Pencil size={11} /></button>
                  </div>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 3, marginBottom: 10 }}>
                    <span style={{ fontSize: 30, fontWeight: 700, color: "var(--text-primary)", lineHeight: 1, fontVariantNumeric: "tabular-nums" }}>{loading ? "—" : candidaturasHoje}</span>
                    <span style={{ fontSize: 15, color: "var(--text-tertiary)", margin: "0 2px" }}>/</span>
                    <span style={{ fontSize: 20, fontWeight: 600, color: "var(--text-secondary)", fontVariantNumeric: "tabular-nums" }}>{config.limite_diario}</span>
                  </div>
                  <div style={{ height: 5, background: "var(--bg-sunken)", borderRadius: 3, overflow: "hidden", marginBottom: 5 }}>
                    <div style={{ width: `${pctCandidaturas}%`, height: "100%", background: barColor, borderRadius: 3, transition: "width 0.5s ease, background 0.3s ease" }} />
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                    {reached ? <span style={{ color: "var(--success)", fontWeight: 500 }}>{t.dashboard.goalReached}</span> : <>{config.limite_diario - candidaturasHoje}{t.dashboard.remaining}</>}
                  </div>
                </div>
              );
            })()}
            {/* Card 2 — Vagas analisadas */}
            {(() => {
              const reached = limVagas > 0 && vagasHoje >= limVagas;
              const barColor = reached ? "var(--success)" : pctVagas >= 80 ? "var(--warning)" : "var(--accent)";
              return (
                <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                    <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>{t.dashboard.jobsAnalyzed}</span>
                    <button onClick={abrirVagas} title="Editar limite" className="edit-icon-btn"><Pencil size={11} /></button>
                  </div>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 3, marginBottom: 10 }}>
                    <span style={{ fontSize: 30, fontWeight: 700, color: "var(--text-primary)", lineHeight: 1, fontVariantNumeric: "tabular-nums" }}>{loading ? "—" : vagasHoje}</span>
                    <span style={{ fontSize: 15, color: "var(--text-tertiary)", margin: "0 2px" }}>/</span>
                    <span style={{ fontSize: 20, fontWeight: 600, color: limVagas > 0 ? "var(--text-secondary)" : "var(--text-tertiary)", fontVariantNumeric: "tabular-nums" }}>{limVagas > 0 ? limVagas : "∞"}</span>
                  </div>
                  <div style={{ height: 5, background: "var(--bg-sunken)", borderRadius: 3, overflow: "hidden", marginBottom: 5 }}>
                    <div style={{ width: limVagas > 0 ? `${pctVagas}%` : "0%", height: "100%", background: barColor, borderRadius: 3, transition: "width 0.5s ease, background 0.3s ease" }} />
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                    hoje · <span style={{ color: "var(--text-secondary)", fontWeight: 500 }}>{vagasTotal}</span> total
                  </div>
                </div>
              );
            })()}
            {/* Card 3 — Tempo de procura */}
            {(() => {
              const barColor = limiteEsgotado ? "var(--danger)" : pctTempo >= 85 ? "var(--warning)" : "var(--accent)";
              return (
                <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                    <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>{t.dashboard.searchTime}</span>
                    <button onClick={abrirTempo} title="Editar limite" className="edit-icon-btn"><Pencil size={11} /></button>
                  </div>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 3, marginBottom: 10 }}>
                    <span style={{ fontSize: 24, fontWeight: 700, color: limiteEsgotado ? "var(--danger)" : "var(--text-primary)", lineHeight: 1, fontVariantNumeric: "tabular-nums" }}>{formatarTempo(tempoMinutos)}</span>
                    <span style={{ fontSize: 14, color: "var(--text-tertiary)", margin: "0 2px" }}>/</span>
                    <span style={{ fontSize: 16, fontWeight: 600, color: config.limite_tempo_minutos > 0 ? "var(--text-secondary)" : "var(--text-tertiary)" }}>{config.limite_tempo_minutos > 0 ? formatTempoCompact(config.limite_tempo_minutos) : "∞"}</span>
                  </div>
                  <div style={{ height: 5, background: "var(--bg-sunken)", borderRadius: 3, overflow: "hidden", marginBottom: 5 }}>
                    <div style={{ width: config.limite_tempo_minutos > 0 ? `${pctTempo}%` : "0%", height: "100%", background: barColor, borderRadius: 3, transition: "width 0.5s ease, background 0.3s ease" }} />
                  </div>
                  <div style={{ fontSize: 11 }}>
                    {limiteEsgotado
                      ? <span style={{ color: "var(--danger)", fontWeight: 500 }}>{t.dashboard.limitReached}</span>
                      : config.limite_tempo_minutos > 0
                        ? <span style={{ color: "var(--text-tertiary)" }}>{formatarTempo(config.limite_tempo_minutos - tempoMinutos)}{t.dashboard.remaining}</span>
                        : <span style={{ color: "var(--text-tertiary)" }}>{t.dashboard.noLimit}</span>}
                  </div>
                </div>
              );
            })()}
            {/* Card 4 — Agendamento */}
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 10 }}>
                <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>{t.dashboard.scheduleStat}</span>
                <button onClick={() => setModalAgendamento(true)} title="Configurar agendamento" className="edit-icon-btn"><Pencil size={11} /></button>
              </div>
              {proximaJanela === null ? (
                <div style={{ fontSize: 12, color: "var(--text-tertiary)", lineHeight: 1.5 }}>{t.dashboard.noSchedule}</div>
              ) : (
                <>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 5 }}>
                    <span style={{ width: 7, height: 7, borderRadius: "50%", flexShrink: 0, background: proximaJanela === "ATIVO_AGORA" ? "var(--success)" : "var(--text-tertiary)", display: "inline-block", animation: proximaJanela === "ATIVO_AGORA" ? "pulse 2s infinite" : "none" }} />
                    <span style={{ fontSize: 13, fontWeight: 500, color: proximaJanela === "ATIVO_AGORA" ? "var(--success)" : "var(--text-primary)" }}>
                      {proximaJanela === "ATIVO_AGORA" ? t.dashboard.activeNow : `${t.dashboard.next}${proximaJanela}`}
                    </span>
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                    {(() => {
                      const count = config.janelas.filter(j => j.ativo).length;
                      return `${count} ${count === 1 ? t.dashboard.windowsActive_one : t.dashboard.windowsActive_many}`;
                    })()}
                  </div>
                </>
              )}
            </div>
          </div>

          {/* Variantes de procura */}
          {variants.length > 0 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {variants.map((v) => {
                const peso = localPesos[v.id] ?? v.peso;
                const pct = Math.round((peso / totalPeso) * 100);
                return (
                  <VariantCardDash
                    key={v.id}
                    variant={v}
                    pct={pct}
                    maxPct={maxPct}
                    onDragBar={(newPct) => handleDragBar(v.id, newPct)}
                    onDragEnd={handleDragEnd}
                    onToggleAtiva={() => handleToggleAtiva(v.id)}
                  />
                );
              })}
            </div>
          )}
        </>
      )}

      {/* ── Tab: Atividade ── */}
      {abaAtiva === "atividade" && (
        <>
          {/* Mini gráfico de barras — vagas por dia (últimos 7 dias) */}
          {(() => {
            const hoje = new Date();
            const dias: { label: string; count: number }[] = Array.from({ length: 7 }, (_, i) => {
              const d = new Date(hoje);
              d.setDate(hoje.getDate() - (6 - i));
              const iso = d.toISOString().slice(0, 10);
              const label = d.toLocaleDateString("pt-PT", { weekday: "short" }).slice(0, 3);
              const count = atividade.filter(v => v.descoberta_em.slice(0, 10) === iso).length;
              return { label, count };
            });
            const maxCount = Math.max(...dias.map(d => d.count), 1);
            return (
              <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
                <div style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em", marginBottom: 12 }}>
                  {t.dashboard.discoveredJobs}
                </div>
                <div style={{ display: "flex", alignItems: "flex-end", gap: 6, height: 56 }}>
                  {dias.map((d, i) => {
                    const isHoje = i === 6;
                    const h = Math.max(4, Math.round((d.count / maxCount) * 48));
                    return (
                      <div key={i} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 4 }}>
                        <div style={{ width: "100%", height: h, background: isHoje ? "var(--accent)" : "var(--accent-soft)", borderRadius: "3px 3px 0 0" }} />
                        <span style={{ fontSize: 10, color: isHoje ? "var(--accent-strong)" : "var(--text-tertiary)", fontWeight: isHoje ? 600 : 400 }}>{d.label}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })()}

          {/* Lista de atividade */}
          {loading ? (
            <div style={{ color: "var(--text-tertiary)", fontSize: 14 }}>{t.dashboard.loading}</div>
          ) : atividade.length === 0 ? (
            <div style={{ color: "var(--text-secondary)", fontSize: 14 }}>{t.dashboard.noActivity}</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
              {atividade.map((v) => {
                const s = STATUS_STYLE[v.status] ?? STATUS_STYLE.descoberta;
                return (
                  <div key={v.id} style={{ display: "flex", alignItems: "center", gap: 12, padding: "10px 0", borderBottom: "1px solid var(--border)" }}>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{v.titulo}</div>
                      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{v.empresa}</div>
                    </div>
                    <span style={{ fontSize: 12, fontWeight: 500, padding: "2px 8px", borderRadius: 6, whiteSpace: "nowrap", ...s }}>{t.dashboard.statusLabels[v.status as keyof typeof t.dashboard.statusLabels] ?? v.status}</span>
                    <span style={{ fontSize: 12, color: "var(--text-tertiary)", whiteSpace: "nowrap" }}>{tempoRelativo(v.descoberta_em)}</span>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}

      {/* Drum picker modal */}
      {modal && <LimitModal cfg={modal} onClose={() => setModal(null)} />}

      {/* Agendamento modal */}
      {modalAgendamento && (
        <AgendamentoModal
          config={config}
          onSave={salvarAgendamento}
          onClose={() => setModalAgendamento(false)}
        />
      )}

      <style>{`
        @keyframes pulse {
          0% { box-shadow: 0 0 0 0 rgba(34,197,94,0.4); }
          70% { box-shadow: 0 0 0 6px rgba(34,197,94,0); }
          100% { box-shadow: 0 0 0 0 rgba(34,197,94,0); }
        }
        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes slideUp {
          from { opacity: 0; transform: translateY(12px) scale(0.97); }
          to { opacity: 1; transform: translateY(0) scale(1); }
        }
        .edit-icon-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 22px;
          height: 22px;
          border-radius: 5px;
          border: 1px solid transparent;
          background: transparent;
          color: var(--text-tertiary);
          cursor: pointer;
          padding: 0;
          transition: background 0.15s, color 0.15s, border-color 0.15s;
        }
        .edit-icon-btn:hover {
          background: var(--bg-sunken);
          border-color: var(--border);
          color: var(--accent);
        }
        div[style*="overflowY: scroll"]::-webkit-scrollbar { display: none; }
      `}</style>
    </div>
  );
};
