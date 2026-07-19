import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sparkles, RefreshCw, ChevronDown, ChevronUp, Lightbulb } from "lucide-react";
import { renderMarkdown } from "../lib/markdown";

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
  if (data.length < 2) return <div style={{ fontSize: 13, color: "var(--text-tertiary)", padding: "12px 0" }}>Dados insuficientes para tendência.</div>;
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

const RESULTADO_LABELS: Record<string, string> = {
  sem_resposta: "Sem resposta", rejeitada: "Rejeitada", entrevista: "Entrevista", oferta: "Oferta",
};
const RESULTADO_COLORS: Record<string, string> = {
  sem_resposta: "var(--text-tertiary)", rejeitada: "var(--danger)", entrevista: "var(--warning)", oferta: "var(--success)",
};
const MOTIVO_LABELS: Record<string, string> = {
  sem_musthave: "Sem must-have", setor_evitar: "Setor a evitar",
  localizacao: "Localização", idioma: "Idioma",
  salario: "Salário", score_baixo: "Score baixo", outro: "Outro",
  aprovada: "Aprovada",
};
const PENDENCIA_LABELS: Record<string, string> = {
  captcha: "Captcha", dados_sensiveis: "Dados sensíveis",
  inventar_informacao: "Informação em falta", red_line: "Red line",
  salario: "Salário", pergunta_sem_resposta: "Pergunta s/ resposta",
  dialogo_bloqueante: "Diálogo bloqueante", extensao_chrome: "Extensão Chrome",
};

const BarChart: React.FC<{ data: ParContagem[]; maxItems?: number; labelMap?: Record<string, string>; colorMap?: Record<string, string>; barColor?: string }> = ({ data, maxItems = 8, labelMap, colorMap, barColor }) => {
  const rows = data.slice(0, maxItems);
  const max = Math.max(...rows.map(d => d.count), 1);
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {rows.map((d, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ width: 140, fontSize: 12, textAlign: "right", color: "var(--text-secondary)", flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {(labelMap ?? RESULTADO_LABELS)[d.chave] ?? d.chave}
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

const PendenciaChart: React.FC<{ data: PendenciaCategoria[] }> = ({ data }) => {
  const max = Math.max(...data.map(d => d.total), 1);
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {data.map((d, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ width: 140, fontSize: 12, textAlign: "right", color: "var(--text-secondary)", flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {PENDENCIA_LABELS[d.categoria] ?? d.categoria}
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
              {registo.gatilho === "marco" ? "Marco" : "Manual"}
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
    setOutputAtual("A analisar os dados e gerar feedback…");
    try {
      await invoke("gerar_feedback", { gatilho });
    } catch (e) {
      console.error(e);
      setGerando(false);
      setOutputAtual(null);
    }
  };

  if (loading) {
    return <div style={{ padding: 24, color: "var(--text-tertiary)", fontSize: 14 }}>A carregar…</div>;
  }

  return (
    <div style={{ padding: 24, paddingBottom: 40 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 24 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0, flex: 1 }}>
          Feedback
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
            ? <><RefreshCw size={13} style={{ animation: "spin 1s linear infinite" }} /> A gerar…</>
            : <><Sparkles size={13} /> Gerar feedback agora</>}
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
            Gerar
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
            <StatPill label="Candidaturas enviadas" value={dados.candidaturas_total} />
            <StatPill label="Esta semana" value={dados.candidaturas_semana} />
            <StatPill label="Vagas analisadas" value={dados.vagas_analisadas} />
            <StatPill label="Vagas puladas" value={dados.vagas_puladas} />
          </div>

          {/* Trend chart */}
          {dados.candidaturas_por_dia.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 8 }}>
                Candidaturas — últimos 30 dias
              </div>
              <TrendChart data={dados.candidaturas_por_dia} />
            </div>
          )}

          {/* Results distribution */}
          {dados.por_resultado.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 12 }}>
                Resultados conhecidos
              </div>
              <BarChart data={dados.por_resultado} labelMap={RESULTADO_LABELS} colorMap={RESULTADO_COLORS} />
            </div>
          )}

          {/* Variant distribution */}
          {dados.por_variante.length > 1 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 12 }}>
                Por variante de busca
              </div>
              <BarChart data={dados.por_variante} />
            </div>
          )}

          {/* Skip reasons */}
          {dados.motivos_puladas.length > 0 && (
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 16 }}>
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 12 }}>
                Motivos de exclusão ({dados.vagas_puladas} vagas puladas)
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
                <span style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)" }}>Pendências por tipo</span>
                <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>verde = resolvidas / vermelho = abertas</span>
              </div>
              <PendenciaChart data={dados.pendencias_por_categoria} />
            </div>
          )}
        </>
      )}

      {/* Feedback history */}
      {feedbacks.length > 0 && (
        <>
          <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 12, marginTop: 8 }}>
            Feedbacks anteriores
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
          Ainda não há candidaturas suficientes para gerar feedback.
          <br />
          <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
            O feedback será útil após as primeiras candidaturas serem enviadas.
          </span>
        </div>
      )}

      {feedbacks.length === 0 && !gerando && (dados?.candidaturas_total ?? 0) > 0 && (
        <div style={{
          background: "var(--bg-surface)", border: "1px dashed var(--border)",
          borderRadius: 8, padding: 24, textAlign: "center",
          color: "var(--text-secondary)", fontSize: 13,
        }}>
          Ainda não geraste nenhum feedback. Usa o botão acima para analisar as candidaturas.
        </div>
      )}

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes blink { 50% { opacity: 0; } }
      `}</style>
    </div>
  );
};
