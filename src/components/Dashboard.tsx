import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, AlertTriangle, Lightbulb, Pause, Play, Square, Pencil, Plus, Trash2 } from "lucide-react";

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

interface StatusLinkedinRede {
  ativo: boolean;
  ultima_busca: string | null;
  vagas_encontradas: number;
}

interface SearchVariant {
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

function formatTempoCompact(m: number): string {
  if (m === 0) return "∞";
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  const rem = m % 60;
  return rem === 0 ? `${h}h` : `${h}h${rem}m`;
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

// Picker value sets
const VALS_CANDIDATURAS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30];
const VALS_VAGAS = [0, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100];
const VALS_TEMPO = [0, 15, 30, 45, 60, 90, 120, 150, 180, 210, 240, 300, 360, 420, 480];

// ── DrumPicker ────────────────────────────────────────────────────────────────
const ITEM_H = 52;

const DrumPicker: React.FC<{
  values: number[];
  selectedValue: number;
  onChange: (v: number) => void;
  formatValue: (v: number) => string;
}> = ({ values, selectedValue, onChange, formatValue }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const selectedIdx = Math.max(0, values.indexOf(selectedValue));

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = selectedIdx * ITEM_H;
    }
  }, []);

  const handleScroll = () => {
    if (scrollTimeout.current) clearTimeout(scrollTimeout.current);
    scrollTimeout.current = setTimeout(() => {
      if (!containerRef.current) return;
      const idx = Math.round(containerRef.current.scrollTop / ITEM_H);
      const clamped = Math.max(0, Math.min(values.length - 1, idx));
      onChange(values[clamped]);
    }, 80);
  };

  return (
    <div style={{ position: "relative", height: ITEM_H * 5, userSelect: "none" }}>
      {/* Selection band */}
      <div style={{
        position: "absolute", top: ITEM_H * 2, height: ITEM_H,
        left: 0, right: 0, pointerEvents: "none", zIndex: 0,
        background: "var(--accent-soft)",
        borderTop: "2px solid var(--accent)",
        borderBottom: "2px solid var(--accent)",
        borderRadius: 8,
      }} />
      {/* Top fade */}
      <div style={{
        position: "absolute", top: 0, left: 0, right: 0,
        height: ITEM_H * 2.4,
        background: "linear-gradient(to bottom, var(--bg-surface) 15%, transparent)",
        pointerEvents: "none", zIndex: 2,
      }} />
      {/* Bottom fade */}
      <div style={{
        position: "absolute", bottom: 0, left: 0, right: 0,
        height: ITEM_H * 2.4,
        background: "linear-gradient(to top, var(--bg-surface) 15%, transparent)",
        pointerEvents: "none", zIndex: 2,
      }} />
      {/* Scroll container */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        style={{
          height: "100%",
          overflowY: "scroll",
          scrollSnapType: "y mandatory",
          scrollbarWidth: "none",
        }}
      >
        <div style={{ height: ITEM_H * 2 }} />
        {values.map((v) => {
          const isSel = v === selectedValue;
          return (
            <div
              key={v}
              onClick={() => {
                onChange(v);
                const i = values.indexOf(v);
                containerRef.current?.scrollTo({ top: i * ITEM_H, behavior: "smooth" });
              }}
              style={{
                height: ITEM_H,
                scrollSnapAlign: "center",
                display: "flex", alignItems: "center", justifyContent: "center",
                cursor: "pointer", position: "relative", zIndex: 1,
              }}
            >
              <span style={{
                fontSize: isSel ? 30 : 20,
                fontWeight: isSel ? 700 : 400,
                color: isSel ? "var(--accent-strong)" : "var(--text-tertiary)",
                letterSpacing: isSel ? "-0.04em" : "-0.01em",
                fontVariantNumeric: "tabular-nums",
                transition: "font-size 0.12s ease, color 0.12s ease",
              }}>
                {formatValue(v)}
              </span>
            </div>
          );
        })}
        <div style={{ height: ITEM_H * 2 }} />
      </div>
    </div>
  );
};

// ── LimitModal ────────────────────────────────────────────────────────────────
interface ModalCfg {
  titulo: string;
  subtitulo: string;
  valores: number[];
  valorAtual: number;
  formatValue: (v: number) => string;
  onSave: (v: number) => void;
}

const LimitModal: React.FC<{ cfg: ModalCfg; onClose: () => void }> = ({ cfg, onClose }) => {
  const [valor, setValor] = useState(cfg.valorAtual);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "Enter") { cfg.onSave(valor); onClose(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [valor]);

  return (
    <div
      onClick={onClose}
      style={{
        position: "fixed", inset: 0, zIndex: 1000,
        background: "rgba(31, 29, 24, 0.35)",
        display: "flex", alignItems: "center", justifyContent: "center",
        backdropFilter: "blur(3px)",
        animation: "fadeIn 0.15s ease",
      }}
    >
      <div
        onClick={e => e.stopPropagation()}
        style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border)",
          borderRadius: 16,
          width: 272,
          boxShadow: "0 24px 64px rgba(31,29,24,0.18), 0 4px 16px rgba(31,29,24,0.08)",
          overflow: "hidden",
          animation: "slideUp 0.18s cubic-bezier(0.34,1.56,0.64,1)",
        }}
      >
        {/* Header */}
        <div style={{ padding: "18px 20px 14px", borderBottom: "1px solid var(--border)" }}>
          <div style={{ fontSize: 15, fontWeight: 600, color: "var(--text-primary)", marginBottom: 2 }}>
            {cfg.titulo}
          </div>
          <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
            {cfg.subtitulo}
          </div>
        </div>

        {/* Drum picker */}
        <div style={{ padding: "4px 20px" }}>
          <DrumPicker
            values={cfg.valores}
            selectedValue={valor}
            onChange={setValor}
            formatValue={cfg.formatValue}
          />
        </div>

        {/* Actions */}
        <div style={{ display: "flex", gap: 8, padding: "12px 16px", borderTop: "1px solid var(--border)" }}>
          <button
            onClick={onClose}
            style={{
              flex: 1, padding: "10px 0", borderRadius: 8,
              border: "1px solid var(--border)", background: "var(--bg-sunken)",
              color: "var(--text-secondary)", fontSize: 14, fontWeight: 500,
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            Cancelar
          </button>
          <button
            onClick={() => { cfg.onSave(valor); onClose(); }}
            style={{
              flex: 1, padding: "10px 0", borderRadius: 8,
              border: "none", background: "var(--accent)",
              color: "#fff", fontSize: 14, fontWeight: 600,
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            Guardar
          </button>
        </div>
      </div>
    </div>
  );
};

// ── ToggleSwitch ──────────────────────────────────────────────────────────────
const ToggleSwitch: React.FC<{ checked: boolean; onChange: () => void }> = ({ checked, onChange }) => (
  <button
    role="switch" aria-checked={checked}
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

const DIAS = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];

// ── AgendamentoModal ──────────────────────────────────────────────────────────
const AgendamentoModal: React.FC<{
  config: ConfigDisparo;
  onSave: (ativo: boolean, limiar: number, janelas: JanelaAgendamento[]) => void;
  onClose: () => void;
}> = ({ config, onSave, onClose }) => {
  const [janelas, setJanelas] = useState<JanelaAgendamento[]>(config.janelas);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const adicionar = () => {
    setJanelas(prev => [...prev, { dia_semana: 1, inicio: "09:00", fim: "23:59", ativo: true }]);
  };
  const remover = (i: number) => setJanelas(prev => prev.filter((_, idx) => idx !== i));
  const atualizar = (i: number, patch: Partial<JanelaAgendamento>) =>
    setJanelas(prev => prev.map((j, idx) => idx === i ? { ...j, ...patch } : j));

  const smallInput: React.CSSProperties = {
    padding: "5px 8px", borderRadius: 4,
    border: "1px solid var(--border)", background: "var(--bg-surface)",
    color: "var(--text-primary)", fontSize: 13, fontFamily: "inherit", outline: "none",
  };

  return (
    <div
      onClick={onClose}
      style={{
        position: "fixed", inset: 0, zIndex: 1000,
        background: "rgba(31, 29, 24, 0.35)",
        display: "flex", alignItems: "center", justifyContent: "center",
        backdropFilter: "blur(3px)",
        animation: "fadeIn 0.15s ease",
      }}
    >
      <div
        onClick={e => e.stopPropagation()}
        style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border)",
          borderRadius: 16,
          width: 420,
          maxHeight: "80vh",
          display: "flex", flexDirection: "column",
          boxShadow: "0 24px 64px rgba(31,29,24,0.18), 0 4px 16px rgba(31,29,24,0.08)",
          overflow: "hidden",
          animation: "slideUp 0.18s cubic-bezier(0.34,1.56,0.64,1)",
        }}
      >
        {/* Header */}
        <div style={{ padding: "18px 20px 14px", borderBottom: "1px solid var(--border)", flexShrink: 0 }}>
          <div style={{ fontSize: 15, fontWeight: 600, color: "var(--text-primary)", marginBottom: 2 }}>Agendamento</div>
          <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>janelas com horário ativo disparam automaticamente</div>
        </div>

        {/* Body */}
        <div style={{ padding: "16px 20px", overflowY: "auto", flex: 1 }}>
          {/* Janelas */}
          <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 10 }}>
            Janelas de atividade
          </div>

          {janelas.length === 0 ? (
            <div style={{
              padding: "12px 16px", borderRadius: 8, border: "1px dashed var(--border)",
              color: "var(--text-tertiary)", fontSize: 13, marginBottom: 12,
            }}>
              Sem janelas — o disparo pode ocorrer a qualquer hora.
            </div>
          ) : (
            <div style={{ border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden", marginBottom: 12 }}>
              <div style={{
                display: "grid", gridTemplateColumns: "1fr 90px 48px 32px",
                gap: 6, padding: "6px 10px",
                background: "var(--bg-sunken)", borderBottom: "1px solid var(--border)",
              }}>
                {["Dia", "Início", "Ativo", ""].map((h, i) => (
                  <div key={i} style={{ fontSize: 10, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                    {h}
                  </div>
                ))}
              </div>
              {janelas.map((j, i) => (
                <div key={i} style={{
                  display: "grid", gridTemplateColumns: "1fr 90px 48px 32px",
                  gap: 6, padding: "7px 10px", alignItems: "center",
                  borderBottom: i < janelas.length - 1 ? "1px solid var(--border)" : "none",
                  background: j.ativo ? "var(--bg-surface)" : "var(--bg-sunken)",
                  opacity: j.ativo ? 1 : 0.65,
                }}>
                  <select value={j.dia_semana} onChange={e => atualizar(i, { dia_semana: parseInt(e.target.value) })} style={{ ...smallInput, width: "100%" }}>
                    {DIAS.map((d, idx) => <option key={idx} value={idx}>{d}</option>)}
                  </select>
                  <input type="time" value={j.inicio} onChange={e => atualizar(i, { inicio: e.target.value })} style={{ ...smallInput, width: "100%" }} />
                  <div style={{ display: "flex", justifyContent: "center" }}>
                    <ToggleSwitch checked={j.ativo} onChange={() => atualizar(i, { ativo: !j.ativo })} />
                  </div>
                  <button
                    onClick={() => remover(i)}
                    style={{
                      display: "flex", alignItems: "center", justifyContent: "center",
                      width: 26, height: 26, borderRadius: 4, border: "none",
                      background: "transparent", cursor: "pointer", color: "var(--text-tertiary)",
                    }}
                    onMouseEnter={e => (e.currentTarget.style.color = "var(--danger)")}
                    onMouseLeave={e => (e.currentTarget.style.color = "var(--text-tertiary)")}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))}
            </div>
          )}

          <button
            onClick={adicionar}
            style={{
              display: "flex", alignItems: "center", gap: 6,
              padding: "6px 12px", borderRadius: 6,
              border: "1px solid var(--border)", background: "var(--bg-surface)",
              color: "var(--text-secondary)", fontSize: 13, fontFamily: "inherit", cursor: "pointer",
            }}
            onMouseEnter={e => (e.currentTarget.style.color = "var(--text-primary)")}
            onMouseLeave={e => (e.currentTarget.style.color = "var(--text-secondary)")}
          >
            <Plus size={13} /> Adicionar janela
          </button>
        </div>

        {/* Footer */}
        <div style={{ display: "flex", gap: 8, padding: "12px 16px", borderTop: "1px solid var(--border)", flexShrink: 0 }}>
          <button
            onClick={onClose}
            style={{
              flex: 1, padding: "10px 0", borderRadius: 8,
              border: "1px solid var(--border)", background: "var(--bg-sunken)",
              color: "var(--text-secondary)", fontSize: 14, fontWeight: 500,
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            Cancelar
          </button>
          <button
            onClick={() => { onSave(janelas.some(j => j.ativo), config.limiar_minutos, janelas); onClose(); }}
            style={{
              flex: 1, padding: "10px 0", borderRadius: 8,
              border: "none", background: "var(--accent)",
              color: "#fff", fontSize: 14, fontWeight: 600,
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            Guardar
          </button>
        </div>
      </div>
    </div>
  );
};

// ── VariantCardDash ───────────────────────────────────────────────────────────
const VariantCardDash: React.FC<{
  variant: SearchVariant;
  pct: number;
  maxPct: number;
  onDragBar: (newPct: number) => void;
  onDragEnd: () => void;
  onToggleAtiva: () => void;
}> = ({ variant, pct, maxPct, onDragBar, onDragEnd, onToggleAtiva }) => {
  const barRef = useRef<HTMLDivElement>(null);
  const onDragBarRef = useRef(onDragBar);
  const onDragEndRef = useRef(onDragEnd);
  useEffect(() => { onDragBarRef.current = onDragBar; }, [onDragBar]);
  useEffect(() => { onDragEndRef.current = onDragEnd; }, [onDragEnd]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const move = (ev: MouseEvent) => {
      if (!barRef.current) return;
      const rect = barRef.current.getBoundingClientRect();
      const newPct = Math.max(5, Math.min(maxPct, ((ev.clientX - rect.left) / rect.width) * 100));
      onDragBarRef.current(newPct);
    };
    const up = () => {
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", up);
      onDragEndRef.current();
    };
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", up);
  };

  const color = variant.ativa ? "var(--accent)" : "var(--text-tertiary)";

  return (
    <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", opacity: variant.ativa ? 1 : 0.55 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <span style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {variant.nome_exibicao}
        </span>
        <button
          onClick={onToggleAtiva}
          title={variant.ativa ? "Clica para desativar" : "Clica para ativar"}
          style={{
            fontSize: 11, padding: "2px 8px", borderRadius: 10, cursor: "pointer",
            border: `1px solid ${variant.ativa ? "var(--accent)" : "var(--border)"}`,
            background: variant.ativa ? "var(--accent-soft)" : "transparent",
            color: variant.ativa ? "var(--accent-strong)" : "var(--text-tertiary)",
            fontFamily: "inherit", fontWeight: 500, flexShrink: 0,
          }}
        >
          {variant.ativa ? "Ativa" : "Inativa"}
        </button>
        <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 4, fontVariantNumeric: "tabular-nums" }}>{pct}%</span>
      </div>
      <div
        ref={barRef}
        onMouseDown={startDrag}
        title="Arrasta para ajustar peso"
        style={{ height: 6, background: "var(--bg-sunken)", borderRadius: 3, cursor: "ew-resize", position: "relative", userSelect: "none", marginBottom: 8 }}
      >
        <div style={{ width: `${pct}%`, height: "100%", background: color, borderRadius: 3 }} />
        <div style={{
          position: "absolute", top: "50%", left: `${pct}%`,
          transform: "translate(-50%, -50%)",
          width: 13, height: 13, borderRadius: "50%",
          background: color, border: "2px solid var(--bg-surface)",
          boxShadow: "0 1px 4px rgba(0,0,0,0.18)", pointerEvents: "none",
        }} />
      </div>
      {(variant.regioes_aceitas.length > 0 || variant.modelos_trabalho.length > 0) && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
          {variant.regioes_aceitas.slice(0, 3).map((r, i) => (
            <span key={i} style={{ fontSize: 11, padding: "1px 6px", borderRadius: 4, background: "var(--bg-sunken)", color: "var(--text-secondary)" }}>{r}</span>
          ))}
          {variant.modelos_trabalho.slice(0, 2).map((m, i) => (
            <span key={i} style={{ fontSize: 11, padding: "1px 6px", borderRadius: 4, background: "var(--accent-soft)", color: "var(--accent-strong)" }}>{m}</span>
          ))}
        </div>
      )}
    </div>
  );
};

// ── ModoCard — mesmo estilo que VariantCardDash, com arrastar quando ativo ─────
const ModoCard: React.FC<{
  label: string;
  ativo: boolean;
  pct: number;
  maxPct: number;
  emBreve?: boolean;
  onToggle?: () => void;
  onDragBar?: (newPct: number) => void;
  onDragEnd?: () => void;
}> = ({ label, ativo, pct, maxPct, emBreve, onToggle, onDragBar, onDragEnd }) => {
  const barRef = useRef<HTMLDivElement>(null);
  const onDragBarRef = useRef(onDragBar);
  const onDragEndRef = useRef(onDragEnd);
  useEffect(() => { onDragBarRef.current = onDragBar; }, [onDragBar]);
  useEffect(() => { onDragEndRef.current = onDragEnd; }, [onDragEnd]);

  const canDrag = ativo && !emBreve && !!onDragBar;

  const startDrag = (e: React.MouseEvent) => {
    if (!canDrag) return;
    e.preventDefault();
    const move = (ev: MouseEvent) => {
      if (!barRef.current) return;
      const rect = barRef.current.getBoundingClientRect();
      const newPct = Math.max(5, Math.min(maxPct, ((ev.clientX - rect.left) / rect.width) * 100));
      onDragBarRef.current?.(newPct);
    };
    const up = () => {
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", up);
      onDragEndRef.current?.();
    };
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", up);
  };

  const color = ativo ? "var(--accent)" : "var(--text-tertiary)";

  return (
    <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", opacity: ativo ? 1 : 0.55 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <span style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {label}
        </span>
        {emBreve ? (
          <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 10, border: "1px solid var(--border)", color: "var(--text-tertiary)", fontWeight: 500 }}>Em breve</span>
        ) : (
          <>
            <button
              onClick={onToggle}
              title={ativo ? "Clica para desativar" : "Clica para ativar"}
              style={{
                fontSize: 11, padding: "2px 8px", borderRadius: 10, cursor: "pointer",
                border: `1px solid ${ativo ? "var(--accent)" : "var(--border)"}`,
                background: ativo ? "var(--accent-soft)" : "transparent",
                color: ativo ? "var(--accent-strong)" : "var(--text-tertiary)",
                fontFamily: "inherit", fontWeight: 500, flexShrink: 0,
              }}
            >
              {ativo ? "Ativo" : "Inativo"}
            </button>
            <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 4, fontVariantNumeric: "tabular-nums" }}>{pct}%</span>
          </>
        )}
      </div>
      <div
        ref={barRef}
        onMouseDown={startDrag}
        title={canDrag ? "Arrasta para ajustar peso" : undefined}
        style={{ height: 6, background: "var(--bg-sunken)", borderRadius: 3, position: "relative", userSelect: "none", cursor: canDrag ? "ew-resize" : "default", marginBottom: emBreve ? 0 : undefined }}
      >
        <div style={{ width: `${pct}%`, height: "100%", background: color, borderRadius: 3 }} />
        <div style={{
          position: "absolute", top: "50%", left: `${pct}%`,
          transform: "translate(-50%, -50%)",
          width: 13, height: 13, borderRadius: "50%",
          background: color, border: "2px solid var(--bg-surface)",
          boxShadow: "0 1px 4px rgba(0,0,0,0.18)", pointerEvents: "none",
        }} />
      </div>
    </div>
  );
};

// ── Dashboard ─────────────────────────────────────────────────────────────────
export const Dashboard: React.FC<{ onNavigate?: (tab: string, section?: string) => void }> = ({ onNavigate: _onNavigate }) => {
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
    titulo: "Limite de candidaturas",
    subtitulo: "máximo por dia",
    valores: VALS_CANDIDATURAS,
    valorAtual: config.limite_diario,
    formatValue: (v) => String(v),
    onSave: salvarLimite,
  });

  const abrirVagas = () => setModal({
    titulo: "Limite de vagas",
    subtitulo: "por sessão  ·  0 = sem limite",
    valores: VALS_VAGAS,
    valorAtual: config.limite_vagas_sessao ?? 0,
    formatValue: (v) => v === 0 ? "∞" : String(v),
    onSave: salvarVagas,
  });

  const abrirTempo = () => setModal({
    titulo: "Limite de tempo",
    subtitulo: "por dia  ·  0 = sem limite",
    valores: VALS_TEMPO,
    valorAtual: config.limite_tempo_minutos,
    formatValue: formatTempoCompact,
    onSave: salvarTempo,
  });

  const proximaJanela = useMemo(() => calcularProximaJanela(config.janelas), [config.janelas]);
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

      {/* ── Tab bar ── */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border)", marginBottom: 16 }}>
        {([["procura", "Procura"], ["atividade", "Atividade"]] as const).map(([id, label]) => {
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
                {paused ? <><Play size={14} /> Retomar</> : <><Pause size={14} /> Pausar</>}
              </button>
              <button onClick={interromper} style={{
                padding: "11px 20px", borderRadius: 8, fontSize: 14, fontWeight: 500,
                fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
                display: "flex", alignItems: "center", gap: 7,
                background: "#F7E2DF", color: "var(--danger)", border: "1px solid var(--danger)",
              }}>
                <Square size={13} fill="currentColor" /> Interromper
              </button>
            </div>
          ) : (
            <div style={{ marginBottom: 16 }}>
              {/* Modos de procura */}
              <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
                <ModoCard
                  label="Procurar vagas"
                  ativo={incluirBuscaNormal}
                  pct={modosAtivos.length > 0 ? Math.round(((modoPesos["busca_normal"] ?? 0) / totalModoPeso) * 100) : 0}
                  maxPct={maxModoPct}
                  onToggle={toggleBuscaNormal}
                  onDragBar={(p) => handleModoDragBar("busca_normal", p)}
                  onDragEnd={handleModoDragEnd}
                />
                <ModoCard
                  label="Procurar vagas na rede"
                  ativo={incluirLinkedinRede}
                  pct={modosAtivos.length > 0 ? Math.round(((modoPesos["linkedin_rede"] ?? 0) / totalModoPeso) * 100) : 0}
                  maxPct={maxModoPct}
                  onToggle={toggleLinkedinRede}
                  onDragBar={(p) => handleModoDragBar("linkedin_rede", p)}
                  onDragEnd={handleModoDragEnd}
                />
                <ModoCard label="Procurar freelas" ativo={false} pct={0} maxPct={95} emBreve />
                <ModoCard label="Procurar em sites de empresas" ativo={false} pct={0} maxPct={95} emBreve />
              </div>
              <button onClick={disparar} disabled={disparando} style={{
                width: "100%", padding: "12px 0",
                background: disparado ? "var(--success)" : "var(--accent)",
                color: "#fff", border: "none", borderRadius: 8, fontSize: 15, fontWeight: 500,
                cursor: disparando ? "default" : "pointer", fontFamily: "inherit",
                transition: "background 0.2s", opacity: disparando ? 0.8 : 1,
                whiteSpace: "nowrap",
              }}>
                {disparando ? "A iniciar…" : disparado ? "✓ Sessão iniciada" : incluirLinkedinRede ? "Procurar vagas na rede" : "Procurar vagas agora"}
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
                    <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>Candidaturas hoje</span>
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
                    {reached ? <span style={{ color: "var(--success)", fontWeight: 500 }}>Meta atingida ✓</span> : <>{config.limite_diario - candidaturasHoje} restantes</>}
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
                    <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>Vagas analisadas</span>
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
                    <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>Tempo de procura</span>
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
                      ? <span style={{ color: "var(--danger)", fontWeight: 500 }}>Limite atingido hoje</span>
                      : config.limite_tempo_minutos > 0
                        ? <span style={{ color: "var(--text-tertiary)" }}>{formatarTempo(config.limite_tempo_minutos - tempoMinutos)} restantes</span>
                        : <span style={{ color: "var(--text-tertiary)" }}>sem limite</span>}
                  </div>
                </div>
              );
            })()}
            {/* Card 4 — Agendamento */}
            <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 10 }}>
                <span style={{ fontSize: 11, fontWeight: 500, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.06em" }}>Agendamento</span>
                <button onClick={() => setModalAgendamento(true)} title="Configurar agendamento" className="edit-icon-btn"><Pencil size={11} /></button>
              </div>
              {proximaJanela === null ? (
                <div style={{ fontSize: 12, color: "var(--text-tertiary)", lineHeight: 1.5 }}>Nenhum horário definido — clica no lápis para agendar sessões automáticas.</div>
              ) : (
                <>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 5 }}>
                    <span style={{ width: 7, height: 7, borderRadius: "50%", flexShrink: 0, background: proximaJanela === "ATIVO_AGORA" ? "var(--success)" : "var(--text-tertiary)", display: "inline-block", animation: proximaJanela === "ATIVO_AGORA" ? "pulse 2s infinite" : "none" }} />
                    <span style={{ fontSize: 13, fontWeight: 500, color: proximaJanela === "ATIVO_AGORA" ? "var(--success)" : "var(--text-primary)" }}>
                      {proximaJanela === "ATIVO_AGORA" ? "Ativo agora" : `Próximo: ${proximaJanela}`}
                    </span>
                  </div>
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                    {config.janelas.filter(j => j.ativo).length} janela{config.janelas.filter(j => j.ativo).length !== 1 ? "s" : ""} ativa{config.janelas.filter(j => j.ativo).length !== 1 ? "s" : ""}
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
                  Vagas descobertas — últimos 7 dias
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
                      <div style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{v.titulo}</div>
                      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{v.empresa}</div>
                    </div>
                    <span style={{ fontSize: 12, fontWeight: 500, padding: "2px 8px", borderRadius: 6, whiteSpace: "nowrap", ...s }}>{STATUS_LABELS[v.status] ?? v.status}</span>
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
