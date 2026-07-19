import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sparkles, RefreshCw, ChevronDown, ChevronUp, Lightbulb } from "lucide-react";
import { renderMarkdown } from "../lib/markdown";
import { useT } from "../i18n";

// ── Types ─────────────────────────────────────────────────────────────────

interface DadosPorDia { data: string; count: number }
interface ParContagem { chave: string; count: number }
interface MotivoPulado { categoria: string; total: number }
interface PendenciaCategoria { categoria: string; total: number; resolvidas: number }

interface AgregadosFeedback {
  candidaturas_total: number;
  candidaturas_semana: number;
  candidaturas_por_dia: DadosPorDia[];
  por_variante: ParContagem[];
  por_resultado: ParContagem[];
  vagas_analisadas: number;
  vagas_puladas: number;
  vagas_pendentes: number;
  dias_desde_ultimo_feedback: number | null;
  ultimo_feedback_resumo: string | null;
  motivos_puladas: MotivoPulado[];
  pendencias_por_categoria: PendenciaCategoria[];
}

interface RegistoFeedback {
  id: number;
  gerado_em: string;
  gatilho: string;
  resumo: string;
  conteudo_completo: string;
  candidaturas_ate_aqui: number;
}

interface SugestaoFeedback { sugerir: boolean; motivo: string }

// ── Visualizações ─────────────────────────────────────────────────────────

const TrendChart: React.FC<{ data: DadosPorDia[] }> = ({ data }) => {
  const t = useT();
  if (data.length < 2) return <div style={{ fontSize: 13, color: "var(--text-tertiary)", padding: "12px 0" }}>{t.feedback.insufficientData}</div>;
  const W = 500, H = 100, PX = 8, PY = 8;
  const max = Math.max(...data.map(d => d.count), 1);
  const pts = data.map((d, i) => {
    const x = PX + (i / (data.length - 1)) * (W - PX * 2);
    const y = PY + ((max - d.count) / max) * (H - PY * 2);
    return { x, y, count: d.count, data: d.data };
  });
  const poly = pts.map(p => `${p.x},${p.y}`).join(" ");
  const areaBottom = H - 1;
  const area = `${pts[0].x},${areaBottom} ${poly} ${pts[pts.length - 1].x},${areaBottom}`;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} style={{ width: "100%", height: 80, display: "block" }}>
      <defs>
        <linearGradient id="tg" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.15" />
          <stop offset="100%" stopColor="var(--accent)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <polygon points={area} fill="url(#tg)" />
      <polyline points={poly} fill="none" stroke="var(--accent)" strokeWidth={1.5} strokeLinejoin="round" />
      {pts.map((p, i) => (
        <circle key={i} cx={p.x} cy={p.y} r={p.count > 0 ? 2.5 : 1.5}
          fill={p.count > 0 ? "var(--accent)" : "var(--border)"} />
      ))}
    </svg>
  );
};

const RESULTADO_COLORS: Record<string, string> = {
  sem_resposta: "var(--text-tertiary)", rejeitada: "var(--danger)", entrevista: "var(--warning)", oferta: "var(--success)",
};

const BarChart: React.FC<{ data: ParContagem[]; maxItems?: number; labelMap?: Record<string, string>; colorMap?: Record<string, string>; barColor?: string }> = ({ data, maxItems = 8, labelMap, colorMap, barColor }) => {
  const rows = data.slice(0, maxItems);
  const max = Math.max(...rows.map(d => d.count), 1);
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {rows.map((d, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ width: 140, fontSize: 12, textAlign: "right", color: "var(--text-secondary)", flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {(labelMap ?? {})[d.chave] ?? d.chave}
          </span>
          <div style={{ flex: 1, height: 14, background: "var(--bg-sunken)", borderRadius: 3 }}>
            <div style={{
              width: `${(d.count / max) * 100}%`, height: "100%",
              background: colorMap?.[d.chave] ?? barColor ?? "var(--accent)",
              borderRadius: 3, minWidth: d.count > 0 ? 4 : 0, transition: "width 0.4s ease",
            }} />
          </div>
          <span style={{ fontSize: 12, color: "var(--text-tertiary)", width: 24, textAlign: "right", flexShrink: 0 }}>{d.count}</span>
        </div>
      ))}
    </div>
  );
};

const PendenciaChart: React.FC<{ data: PendenciaCategoria[]; labelMap?: Record<string, string> }> = ({ data, labelMap }) => {
  const max = Math.max(...data.map(d => d.total), 1);
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {data.map((d, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ width: 140, fontSize: 12, textAlign: "right", color: "var(--text-secondary)", flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {(labelMap ?? {})[d.categoria] ?? d.categoria}
          </span>
          <div style={{ flex: 1, height: 14, background: "var(--bg-sunken)", borderRadius: 3 }}>
            <div style={{ position: "relative", width: `${(d.total / max) * 100}%`, height: "100%", borderRadius: 3, minWidth: d.total > 0 ? 4 : 0, overflow: "hidden" }}>
              <div style={{ position: "absolute", inset: 0, background: "var(--danger)", opacity: 0.35 }} />
              <div style={{ position: "absolute", inset: 0, width: `${d.total > 0 ? (d.resolvidas / d.total) * 100 : 0}%`, background: "var(--success)", opacity: 0.6 }} />
            </div>
          </div>
          <span style={{ fontSize: 12, color: "var(--text-tertiary)", width: 44, textAlign: "right", flexShrink: 0 }}>
            {d.resolvidas}/{d.total}
          </span>
        </div>
      ))}
    </div>
  );
};

// ── StatPill ──────────────────────────────────────────────────────────────

const StatPill: React.FC<{ label: string; value: string | number }> = ({ label, value }) => (
  <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "12px 16px", minWidth: 100 }}>
    <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginBottom: 4 }}>{label}</div>
    <div style={{ fontSize: 22, fontWeight: 600, color: "var(--text-primary)" }}>{value}</div>
  </div>
);

// ── FeedbackCard ──────────────────────────────────────────────────────────

const FeedbackCard: React.FC<{ registo: RegistoFeedback }> = ({ registo }) => {
  const t = useT();
  const [expandido, setExpandido] = useState(false);
  const data = new Date(registo.gerado_em).toLocaleDateString("pt-PT", { day: "numeric", month: "long", year: "numeric" });
  return (
    <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden" }}>
      <button
        onClick={() => setExpandido(e => !e)}
        style={{
          width: "100%", padding: "14px 16px", display: "flex", alignItems: "flex-start",
          gap: 12, background: "none", border: "none", cursor: "pointer", textAlign: "left",
          fontFamily: "inherit",
        }}
      >
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
            <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{data}</span>
            <span style={{
              fontSize: 11, fontWeight: 500, padding: "1px 6px", borderRadius: 4,
              background: registo.gatilho === "marco" ? "var(--accent-soft)" : "var(--bg-sunken)",
              color: registo.gatilho === "marco" ? "var(--accent-strong)" : "var(--text-secondary)",
            }}>
              {registo.gatilho === "marco" ? t.feedback.milestone : t.feedback.manual}
            </span>
            <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
              {registo.candidaturas_ate_aqui} candidaturas
            </span>
          </div>
          <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.4 }}>{registo.resumo}</div>
        </div>
        {expandido ? <ChevronUp size={15} style={{ color: "var(--text-tertiary)", flexShrink: 0 }} />
          : <ChevronDown size={15} style={{ color: "var(--text-tertiary)", flexShrink: 0 }} />}
      </button>
      {expandido && (
        <div style={{ padding: "0 16px 16px", borderTop: "1px solid var(--border)", fontSize: 13 }}>
          <div style={{ paddingTop: 12 }}>{renderMarkdown(registo.conteudo_completo, { headingSize: "md" })}</div>
        </div>
      )}
    </div>
  );
};

// ── Main ──────────────────────────────────────────────────────────────────

export const Feedback: React.FC = () => {
  const t = useT();
  const RESULTADO_LABELS: Record<string, string> = {
    sem_resposta: t.feedback.resultLabels.sem_resposta,
    rejeitada: t.feedback.resultLabels.rejeitada,
    entrevista: t.feedback.resultLabels.entrevista,
    oferta: t.feedback.resultLabels.oferta,
  };
  const MOTIVO_LABELS: Record<string, string> = {
    sem_musthave: t.feedback.skipReasonLabels.sem_musthave,
    setor_evitar: t.feedback.skipReasonLabels.setor_evitar,
    localizacao: t.feedback.skipReasonLabels.localizacao,
    idioma: t.feedback.skipReasonLabels.idioma,
    salario: t.feedback.skipReasonLabels.salario,
    score_baixo: t.feedback.skipReasonLabels.score_baixo,
    outro: t.feedback.skipReasonLabels.outro,
    aprovada: t.feedback.skipReasonLabels.aprovada,
  };
  const PENDENCIA_LABELS: Record<string, string> = {
    captcha: t.feedback.pendingLabels.captcha,
    dados_sensiveis: t.feedback.pendingLabels.dados_sensiveis,
    inventar_informacao: t.feedback.pendingLabels.inventar_informacao,
    red_line: t.feedback.pendingLabels.red_line,
    salario: t.feedback.pendingLabels.salario,
    pergunta_sem_resposta: t.feedback.pendingLabels.pergunta_sem_resposta,
    dialogo_bloqueante: t.feedback.pendingLabels.dialogo_bloqueante,
    extensao_chrome: t.feedback.pendingLabels.extensao_chrome,
  };
  const [dados, setDados] = useState<AgregadosFeedback | null>(null);
  const [feedbacks, setFeedbacks] = useState<RegistoFeedback[]>([]);
  const [sugestao, setSugestao] = useState<SugestaoFeedback | null>(null);
  const [gerando, setGerando] = useState(false);
  const [outputAtual, setOutputAtual] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const unlistenRef = useRef<(() => void)[]>([]);
  const firstChunkRef = useRef(false);

  const carregar = async () => {
    try {
      const [d, fs, s] = await Promise.all([
        invoke<AgregadosFeedback>("agregar_dados_feedback"),
        invoke<RegistoFeedback[]>("listar_feedbacks"),
        invoke<SugestaoFeedback>("sugerir_feedback"),
      ]);
      setDados(d);
      setFeedbacks(fs);
      setSugestao(s);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    carregar();

    let active = true;

    Promise.all([
      listen<string>("feedback-output", (ev) => {
        if (!firstChunkRef.current) {
          firstChunkRef.current = true;
          setOutputAtual(ev.payload);
        } else {
          setOutputAtual(prev => (prev ?? "") + ev.payload);
        }
      }),
      listen("feedback-output-done", () => {
        firstChunkRef.current = false;
        setGerando(false);
        setOutputAtual(null);
        carregar();
      }),
    ]).then((fns) => {
      if (active) {
        unlistenRef.current = fns;
      } else {
        fns.forEach((f) => f());
      }
    });

    return () => {
      active = false;
      unlistenRef.current.forEach(f => f());
      unlistenRef.current = [];
    };
  }, []);

  const gerarFeedback = async (gatilho: "manual" | "marco") => {
    firstChunkRef.current = false;
    setGerando(true);
    setOutputAtual(t.feedback.analyzingData);
    try {
      await invoke("gerar_feedback", { gatilho });
    } catch (e) {
      console.error(e);
      setGerando(false);
      setOutputAtual(null);
    }
  };

  if (loading) {
    return <div style={{ padding: 24, color: "var(--text-tertiary)", fontSize: 14 }}>{t.feedback.loading}</div>;
  }

  return (
    <div style={{ padding: 24, paddingBottom: 40 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 24 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0, flex: 1 }}>
          {t.feedback.title}
        </h1>
        <button
          onClick={() => gerarFeedback("manual")}
          disabled={gerando}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "8px 16px", background: gerando ? "var(--bg-sunken)" : "var(--accent)",
            color: gerando ? "var(--text-secondary)" : "#fff",
            border: "none", borderRadius: 8, fontSize: 13, fontWeight: 500,
            cursor: gerando ? "default" : "pointer", fontFamily: "inherit",
            transition: "background 0.2s",
          }}
        >
          {gerando
            ? <><RefreshCw size={13} style={{ animation: "spin 1s linear infinite" }} /> {t.feedback.generating}</>
            : <><Sparkles size={13} /> {t.feedback.generateNow}</>}
        </button>
      </div>

      {/* Marco suggestion */}
      {sugestao?.sugerir && !gerando && (
        <div style={{
          background: "var(--accent-soft)", border: "1px solid var(--accent)",
          borderRadius: 8, padding: "10px 16px", marginBottom: 16,
          display: "flex", alignItems: "center", gap: 10,
        }}>
          <Lightbulb size={15} style={{ color: "var(--accent-strong)", flexShrink: 0 }} />
          <span style={{ fontSize: 13, color: "var(--accent-strong)", flex: 1 }}>{sugestao.motivo}</span>
          <button
            onClick={() => gerarFeedback("marco")}
            style={{
              background: "var(--accent)", color: "#fff", border: "none",
              borderRadius: 6, fontSize: 12, fontWeight: 500, padding: "4px 12px",
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            {t.feedback.generate}
          </button>
        </div>
      )}

      {/* Generation in progress */}
      {outputAtual && (
        <div style={{
          background: "var(--bg-surface)", border: "1px solid var(--border)",
          borderRadius: 8, padding: 16, marginBottom: 16,
          fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.5,
        }}>
          {renderMarkdown(outputAtual, { headingSize: "md" })}
          {gerando && <span style={{ display: "inline-block", width: 6, height: 13, marginLeft: 4, background: "var(--accent)", borderRadius: 1, verticalAlign: "middle", animation: "blink 0.8s step-end infinite" }} />}
        </div>
      )}

      {/* Stats overview */}
      {dados && (
        <>
          <div style={{ display: "flex", gap: 10, marginBottom: 20, flexWrap: "wrap" }}>
            <StatPill label={t.feedback.applicationsSent} value={dados.candidaturas_total} />
            <StatPill label={t.feedback.thisWeek} value={dados.candidaturas_semana} />
            <StatPill label={t.feedback.jobsAnalyzed} value={dados.vagas_analisadas} />
            <StatPill label={t.feedback.jobsSkipped} value={dados.vagas_puladas} />
          </div>

          {/* Trend chart */}
          {dados.candidaturas_por_dia.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 8 }}>
                {t.feedback.last30Days}
              </div>
              <TrendChart data={dados.candidaturas_por_dia} />
            </div>
          )}

          {/* Results distribution */}
          {dados.por_resultado.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 12 }}>
                {t.feedback.knownResults}
              </div>
              <BarChart data={dados.por_resultado} labelMap={RESULTADO_LABELS} colorMap={RESULTADO_COLORS} />
            </div>
          )}

          {/* Variant distribution */}
          {dados.por_variante.length > 1 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 12 }}>
                {t.feedback.bySearchVariant}
              </div>
              <BarChart data={dados.por_variante} />
            </div>
          )}

          {/* Skip reasons */}
          {dados.motivos_puladas.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 12 }}>
                {t.feedback.skipReasonsTitle} ({dados.vagas_puladas} {t.feedback.jobsSkipped})
              </div>
              <BarChart
                data={dados.motivos_puladas.map(m => ({ chave: m.categoria, count: m.total }))}
                labelMap={MOTIVO_LABELS}
                barColor="var(--text-tertiary)"
              />
            </div>
          )}

          {/* Pending issues by category */}
          {dados.pendencias_por_categoria.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 12 }}>
                <span style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)" }}>{t.feedback.pendingsByType}</span>
                <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{t.feedback.pendingNote}</span>
              </div>
              <PendenciaChart data={dados.pendencias_por_categoria} labelMap={PENDENCIA_LABELS} />
            </div>
          )}
        </>
      )}

      {/* Feedback history */}
      {feedbacks.length > 0 && (
        <>
          <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 12, marginTop: 8 }}>
            {t.feedback.previousFeedbacks}
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {feedbacks.map(f => <FeedbackCard key={f.id} registo={f} />)}
          </div>
        </>
      )}

      {feedbacks.length === 0 && !gerando && !dados?.candidaturas_total && (
        <div style={{
          background: "var(--bg-surface)", border: "1px dashed var(--border)",
          borderRadius: 8, padding: 32, textAlign: "center",
          color: "var(--text-secondary)", fontSize: 14,
        }}>
          {t.feedback.notEnoughApplications}
          <br />
          <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
            {t.feedback.feedbackWillBeUseful}
          </span>
        </div>
      )}

      {feedbacks.length === 0 && !gerando && (dados?.candidaturas_total ?? 0) > 0 && (
        <div style={{
          background: "var(--bg-surface)", border: "1px dashed var(--border)",
          borderRadius: 8, padding: 24, textAlign: "center",
          color: "var(--text-secondary)", fontSize: 13,
        }}>
          {t.feedback.noFeedbackYet}
        </div>
      )}

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes blink { 50% { opacity: 0; } }
      `}</style>
    </div>
  );
};
