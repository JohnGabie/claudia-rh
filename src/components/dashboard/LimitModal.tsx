import React, { useState, useEffect } from "react";
import { ModalCfg } from "./types";
import { DrumPicker } from "./DrumPicker";

export const LimitModal: React.FC<{ cfg: ModalCfg; onClose: () => void }> = ({ cfg, onClose }) => {
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
