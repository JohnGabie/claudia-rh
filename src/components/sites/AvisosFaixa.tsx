import React, { useState } from "react";
import { AlertTriangle, X } from "lucide-react";
import { useT } from "../../i18n";
import { sitesApi, type AvisoSite } from "../../lib/sites-api";

// One line of the strip. The X only dismisses the notice (the site stays red); a refusal shows in the row.
const Linha: React.FC<{ a: AvisoSite; onAbrir: (chave: string) => void }> = ({ a, onAbrir }) => {
  const t = useT();
  const [erro, setErro] = useState<string | null>(null);
  const dispensar = async () => {
    setErro(null);
    try {
      await sitesApi.dispensarAviso(a.id);
    } catch (e) {
      setErro(String(e));
    }
  };
  return (
    <div
      style={{
        display: "flex", flexDirection: "column", gap: 4, padding: "8px 12px", borderRadius: 8, fontSize: 13,
        border: "1px solid var(--danger)", background: "color-mix(in srgb, var(--danger) 10%, var(--bg-surface))", color: "var(--text-primary)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <AlertTriangle size={15} color="var(--danger)" style={{ flexShrink: 0 }} aria-hidden="true" />
        <span style={{ flex: 1, minWidth: 0, overflowWrap: "anywhere" }}>{t.sites.relogin(a.nome)}</span>
        <button
          type="button" onClick={() => onAbrir(a.chave)}
          style={{ padding: "4px 10px", borderRadius: 6, fontSize: 12, fontFamily: "inherit", cursor: "pointer", background: "transparent", border: "1px solid var(--danger)", color: "var(--text-primary)" }}
        >
          {t.sites.open}
        </button>
        <button
          type="button" onClick={dispensar} aria-label={t.sites.dismissNotice} title={t.sites.dismissNotice}
          style={{ display: "inline-flex", alignItems: "center", justifyContent: "center", width: 24, height: 24, flexShrink: 0, padding: 0, borderRadius: 6, cursor: "pointer", background: "transparent", border: "none", color: "var(--danger)" }}
        >
          <X size={14} aria-hidden="true" />
        </button>
      </div>
      {erro && <div role="alert" style={{ fontSize: 12, color: "var(--danger)", overflowWrap: "anywhere" }}>{erro}</div>}
    </div>
  );
};

// Strip at the top of the screen: one line per site whose login was lost.
export const AvisosFaixa: React.FC<{ avisos: AvisoSite[]; onAbrir: (chave: string) => void }> = ({ avisos, onAbrir }) => {
  if (avisos.length === 0) return null;
  return (
    <div role="status" style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
      {avisos.map((a) => <Linha key={a.id} a={a} onAbrir={onAbrir} />)}
    </div>
  );
};
