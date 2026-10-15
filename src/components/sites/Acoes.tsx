import React, { useState } from "react";
import { useT } from "../../i18n";
import type { Site } from "./logica";
import type { AcoesSite } from "./tipos";

const btn: React.CSSProperties = {
  padding: "4px 10px", borderRadius: 6, fontSize: 12, fontFamily: "inherit", cursor: "pointer",
  background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)", whiteSpace: "nowrap",
};

// Open / "I am logged in" / Remove, with the rules of which one shows. Remove asks once, inline.
export const Acoes: React.FC<{ site: Site } & AcoesSite> = ({ site, onAbrir, onLogado, onRemover }) => {
  const t = useT();
  const [confirmando, setConfirmando] = useState(false);

  if (confirmando) {
    return (
      <>
        <span style={{ fontSize: 12, color: "var(--text-secondary)", flexBasis: "100%" }}>{t.sites.removeConfirm}</span>
        <button type="button" onClick={() => { setConfirmando(false); onRemover(site); }} style={{ ...btn, color: "var(--danger)", borderColor: "var(--danger)" }}>
          {t.sites.remove}
        </button>
        <button type="button" onClick={() => setConfirmando(false)} style={btn}>{t.sites.form.cancel}</button>
      </>
    );
  }

  return (
    <>
      <button type="button" onClick={() => onAbrir(site)} style={{ ...btn, color: "var(--accent-strong)", borderColor: "var(--accent)" }}>
        {t.sites.open}
      </button>
      {site.regra_login === "manual" && site.estado !== "logada" && (
        <button type="button" onClick={() => onLogado(site)} style={btn}>{t.sites.loggedIn}</button>
      )}
      {!site.embutida && (
        <button type="button" onClick={() => setConfirmando(true)} style={btn}>{t.sites.remove}</button>
      )}
    </>
  );
};
