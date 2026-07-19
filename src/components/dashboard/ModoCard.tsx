import React, { useEffect, useRef } from "react";

// ── ModoCard — mesmo estilo que VariantCardDash, com arrastar quando ativo ─────
export const ModoCard: React.FC<{
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
