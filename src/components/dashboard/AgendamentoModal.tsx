import React, { useState, useEffect } from "react";
import { Plus, Trash2 } from "lucide-react";
import { useT } from "../../i18n";
import { ConfigDisparo, JanelaAgendamento } from "./types";
import { ToggleSwitch } from "../ui/ToggleSwitch";

// ── AgendamentoModal ──────────────────────────────────────────────────────────
export const AgendamentoModal: React.FC<{
  config: ConfigDisparo;
  onSave: (ativo: boolean, limiar: number, janelas: JanelaAgendamento[]) => void;
  onClose: () => void;
}> = ({ config, onSave, onClose }) => {
  const t = useT();
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
          <div style={{ fontSize: 15, fontWeight: 600, color: "var(--text-primary)", marginBottom: 2 }}>{t.dashboard.schedule}</div>
          <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{t.dashboard.scheduleDesc}</div>
        </div>

        {/* Body */}
        <div style={{ padding: "16px 20px", overflowY: "auto", flex: 1 }}>
          {/* Janelas */}
          <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)", marginBottom: 10 }}>
            {t.dashboard.activityWindows}
          </div>

          {janelas.length === 0 ? (
            <div style={{
              padding: "12px 16px", borderRadius: 8, border: "1px dashed var(--border)",
              color: "var(--text-tertiary)", fontSize: 13, marginBottom: 12,
            }}>
              {t.dashboard.noWindows}
            </div>
          ) : (
            <div style={{ border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden", marginBottom: 12 }}>
              <div style={{
                display: "grid", gridTemplateColumns: "1fr 90px 48px 32px",
                gap: 6, padding: "6px 10px",
                background: "var(--bg-sunken)", borderBottom: "1px solid var(--border)",
              }}>
                {[t.dashboard.day, t.dashboard.start, t.dashboard.active, ""].map((h, i) => (
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
                    {t.dashboard.days.map((d, idx) => <option key={idx} value={idx}>{d}</option>)}
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
            <Plus size={13} /> {t.dashboard.addWindow}
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
            {t.dashboard.cancel}
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
            {t.dashboard.save}
          </button>
        </div>
      </div>
    </div>
  );
};
