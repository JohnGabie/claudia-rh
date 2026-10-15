import React, { useState } from "react";
import { useT } from "../../i18n";
import { bloqueado as estaBloqueado } from "./estado";
import { Faixa } from "./Faixa";
import type { Site } from "./logica";
import type { AcoesSite } from "./tipos";

// Panel "What the AI can do here" of the detail page: the AI strip (clickable) and the actions.
export const Acoes: React.FC<{ site: Site; erro?: string } & AcoesSite> = ({ site, erro, onAbrir, onLogado, onIa, onRemover }) => {
  const t = useT();
  const [confirmando, setConfirmando] = useState(false);
  const bloq = estaBloqueado(site);
  const alternar = () => onIa(site, !site.ia_pode_usar);

  return (
    <div className="st-painel">
      <h3>{t.sites.aiTitle}</h3>
      <div className="st-acoes-p">
        {bloq ? (
          <div onClick={() => onIa(site, true)}>
            <Faixa ligado={false} bloqueado className="solta" />
          </div>
        ) : (
          <div
            role="switch"
            aria-checked={site.ia_pode_usar}
            aria-label={`${site.nome}: ${site.ia_pode_usar ? t.sites.aiOpen : t.sites.aiClosed}`}
            tabIndex={0}
            className="st-switch-faixa"
            onClick={alternar}
            onKeyDown={(e) => { if (e.key === " " || e.key === "Enter") { e.preventDefault(); alternar(); } }}
          >
            <Faixa ligado={site.ia_pode_usar} bloqueado={false} className="solta" />
          </div>
        )}
        <button type="button" className="st-btn primario" onClick={() => onAbrir(site)}>{t.sites.openSite}</button>
        {bloq && <button type="button" className="st-btn" onClick={() => onLogado(site)}>{t.sites.loggedIn}</button>}
        <button type="button" className="st-btn" disabled title={t.sites.configureHint}>{t.sites.configure}</button>
        {!site.embutida && !confirmando && (
          <button type="button" className="st-btn perigo" onClick={() => setConfirmando(true)}>{t.sites.removeFromList}</button>
        )}
        {confirmando && (
          <div className="st-confirma">
            <span>{t.sites.removeConfirm}</span>
            <button type="button" className="st-btn perigo" onClick={() => { setConfirmando(false); onRemover(site); }}>{t.sites.removeFromList}</button>
            <button type="button" className="st-btn" onClick={() => setConfirmando(false)}>{t.sites.form.cancel}</button>
          </div>
        )}
        {erro && <div role="alert" className="st-erro">{erro}</div>}
      </div>
    </div>
  );
};
