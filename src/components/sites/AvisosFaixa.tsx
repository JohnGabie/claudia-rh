import React from "react";
import { AlertTriangle } from "lucide-react";
import { useT } from "../../i18n";
import type { AvisoSite } from "../../lib/sites-api";

// Strip at the top of the screen: one line per site whose login was lost.
export const AvisosFaixa: React.FC<{ avisos: AvisoSite[]; onAbrir: (chave: string) => void }> = ({ avisos, onAbrir }) => {
  const t = useT();
  if (avisos.length === 0) return null;
  return (
    <div role="status" style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
      {avisos.map((a) => (
        <div
          key={a.id}
          style={{
            display: "flex", alignItems: "center", gap: 10, padding: "8px 12px", borderRadius: 8, fontSize: 13,
            border: "1px solid var(--danger)", background: "color-mix(in srgb, var(--danger) 10%, var(--bg-surface))", color: "var(--text-primary)",
          }}
        >
          <AlertTriangle size={15} color="var(--danger)" style={{ flexShrink: 0 }} aria-hidden="true" />
          <span style={{ flex: 1, minWidth: 0, overflowWrap: "anywhere" }}>{t.sites.relogin(a.nome)}</span>
          <button
            type="button" onClick={() => onAbrir(a.chave)}
            style={{ padding: "4px 10px", borderRadius: 6, fontSize: 12, fontFamily: "inherit", cursor: "pointer", background: "transparent", border: "1px solid var(--danger)", color: "var(--text-primary)" }}
          >
            {t.sites.open}
          </button>
        </div>
      ))}
    </div>
  );
};
