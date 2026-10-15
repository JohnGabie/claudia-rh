import React from "react";

// A real switch: a button with role="switch" and an accessible name. Controlled by the server state.
export const Interruptor: React.FC<{ ligado: boolean; nome: string; onMudar: (v: boolean) => void }> = ({ ligado, nome, onMudar }) => (
  <button
    type="button"
    role="switch"
    aria-checked={ligado}
    aria-label={nome}
    onClick={() => onMudar(!ligado)}
    style={{
      width: 34, height: 20, minWidth: 34, borderRadius: 999, border: "none", padding: 2, cursor: "pointer",
      background: ligado ? "var(--accent)" : "var(--border)", display: "flex", justifyContent: ligado ? "flex-end" : "flex-start",
    }}
  >
    <span style={{ width: 16, height: 16, borderRadius: "50%", background: "#fff", boxShadow: "0 1px 2px rgba(0,0,0,.25)" }} />
  </button>
);
