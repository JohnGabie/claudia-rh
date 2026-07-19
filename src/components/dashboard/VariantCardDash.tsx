import React, { useEffect, useRef } from "react";
import { useT } from "../../i18n";
import { SearchVariant } from "../../types";

// ── VariantCardDash ───────────────────────────────────────────────────────────
export const VariantCardDash: React.FC<{
  variant: SearchVariant;
  pct: number;
  maxPct: number;
  onDragBar: (newPct: number) => void;
  onDragEnd: () => void;
  onToggleAtiva: () => void;
}> = ({ variant, pct, maxPct, onDragBar, onDragEnd, onToggleAtiva }) => {
  const t = useT();
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
          title={variant.ativa ? t.dashboard.clickToDeactivate : t.dashboard.clickToActivate}
          style={{
            fontSize: 11, padding: "2px 8px", borderRadius: 10, cursor: "pointer",
            border: `1px solid ${variant.ativa ? "var(--accent)" : "var(--border)"}`,
            background: variant.ativa ? "var(--accent-soft)" : "transparent",
            color: variant.ativa ? "var(--accent-strong)" : "var(--text-tertiary)",
            fontFamily: "inherit", fontWeight: 500, flexShrink: 0,
          }}
        >
          {variant.ativa ? t.profile.variantActive : t.profile.variantInactive}
        </button>
        <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 4, fontVariantNumeric: "tabular-nums" }}>{pct}%</span>
      </div>
      <div
        ref={barRef}
        onMouseDown={startDrag}
        title={t.dashboard.dragToAdjustWeight}
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
