import React from "react";

// Toggle único da app — substitui as cópias que existiam em Dashboard,
// Configuracoes e Terminal.
export const ToggleSwitch: React.FC<{ checked: boolean; onChange: () => void }> = ({ checked, onChange }) => (
  <button
    role="switch"
    aria-checked={checked}
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
