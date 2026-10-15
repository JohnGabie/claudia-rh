import React from "react";
import { useT } from "../../i18n";
import type { Site } from "./logica";

export const Pill: React.FC<{ children: React.ReactNode; titulo?: string }> = ({ children, titulo }) => (
  <span
    title={titulo}
    style={{
      fontSize: 11, lineHeight: "16px", padding: "1px 8px", borderRadius: 999, background: "var(--bg-sunken)",
      color: "var(--text-secondary)", whiteSpace: "nowrap",
    }}
  >
    {children}
  </span>
);

// logada = green, desconhecido = amber, precisa_login = grey (not red: it is a normal state, not an error).
const COR: Record<Site["estado"], string> = {
  logada: "var(--success)", desconhecido: "var(--warning)", precisa_login: "var(--text-tertiary)",
};

export const EstadoBadge: React.FC<{ estado: Site["estado"] }> = ({ estado }) => {
  const t = useT();
  return (
    <span
      style={{
        display: "inline-flex", alignItems: "center", gap: 5, fontSize: 11, lineHeight: "16px", padding: "1px 8px",
        borderRadius: 999, whiteSpace: "nowrap", color: COR[estado],
        background: `color-mix(in srgb, ${COR[estado]} 14%, transparent)`,
      }}
    >
      <span style={{ width: 6, height: 6, borderRadius: "50%", background: COR[estado] }} />
      {t.sites.state[estado]}
    </span>
  );
};
