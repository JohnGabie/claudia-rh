import React, { useState } from "react";
import { AlertTriangle, X } from "lucide-react";
import { useT } from "../i18n";
import { sitesApi, type AvisoSite } from "../lib/sites-api";
import { acoesAvisoSite } from "./avisoSiteAcoes";

const ghostBtn: React.CSSProperties = {
  background: "transparent", border: "none", cursor: "pointer", fontSize: 12,
  color: "var(--text-secondary)", fontFamily: "inherit", padding: "5px 10px", borderRadius: 6,
};
const solidBtn: React.CSSProperties = {
  background: "var(--accent)", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 12,
  fontWeight: 500, color: "#fff", fontFamily: "inherit", padding: "5px 12px",
};

// Same card shape as the other items in the notifications screen. `tempoDesde` is passed in so
// the relative-time helper stays single-sourced in Pendencias.tsx.
export const AvisoSiteItem: React.FC<{ a: AvisoSite; tempo: string; onResolved: () => void }> = ({ a, tempo, onResolved }) => {
  const t = useT();
  const [abriu, setAbriu] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const acoes = acoesAvisoSite({ abriu, ocupado });

  const abrir = async () => {
    setOcupado(true);
    setErro(null);
    try {
      await sitesApi.abrir(a.chave);
      setAbriu(true);
    } catch (e) {
      setErro(String(e));
    } finally {
      setOcupado(false);
    }
  };

  const jaEntrei = async () => {
    setOcupado(true);
    setErro(null);
    try {
      await sitesApi.marcarLogado(a.chave);
      onResolved();
    } catch (e) {
      setErro(String(e));
    } finally {
      setOcupado(false);
    }
  };

  const dispensar = async () => {
    setOcupado(true);
    setErro(null);
    try {
      await sitesApi.dispensarAviso(a.id);
      onResolved();
    } catch (e) {
      setErro(String(e));
    } finally {
      setOcupado(false);
    }
  };

  return (
    <div style={{ display: "flex", background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 10, overflow: "hidden" }}>
      <div style={{ width: 3, flexShrink: 0, background: "var(--warning)", borderRadius: "10px 0 0 10px" }} />
      <div style={{ flex: 1, padding: "12px 14px", minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
          <span style={{
            display: "inline-flex", alignItems: "center", gap: 4, fontSize: 11, fontWeight: 600,
            padding: "2px 7px", borderRadius: 5, color: "var(--warning)", background: "rgba(184,134,46,0.08)", flexShrink: 0,
          }}>
            <AlertTriangle size={10} />
            {t.notifications.categories.site_login}
          </span>
          <span style={{ fontSize: 11, color: "var(--text-tertiary)", marginLeft: "auto", flexShrink: 0 }}>{tempo}</span>
        </div>
        <div style={{ fontSize: 13, color: "var(--text-primary)", lineHeight: 1.5, marginBottom: 10, overflowWrap: "anywhere" }}>
          {t.sites.relogin(a.nome)}
        </div>
        {erro && (
          <div role="alert" style={{ fontSize: 12, color: "var(--danger)", marginBottom: 8, overflowWrap: "anywhere" }}>{erro}</div>
        )}
        <div style={{ display: "flex", alignItems: "center", gap: 4, justifyContent: "flex-end" }}>
          <button type="button" onClick={abrir} disabled={!acoes.podeAbrir}
            style={{ ...ghostBtn, color: "var(--accent-strong)", fontWeight: 500, opacity: acoes.podeAbrir ? 1 : 0.5 }}>
            {t.sites.openSite}
          </button>
          {acoes.mostrarJaEntrei && (
            <button type="button" onClick={jaEntrei} disabled={!acoes.podeJaEntrei}
              style={{ ...solidBtn, opacity: acoes.podeJaEntrei ? 1 : 0.5 }}>
              {t.sites.enteredBtn}
            </button>
          )}
          <button type="button" onClick={dispensar} disabled={ocupado} aria-label={t.sites.dismissNotice} title={t.sites.dismissNotice}
            style={{ ...ghostBtn, display: "inline-flex", alignItems: "center", justifyContent: "center", minWidth: 24, minHeight: 24, opacity: ocupado ? 0.5 : 1 }}>
            <X size={14} aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>
  );
};
