import React, { useState } from "react";
import { useT } from "../../i18n";
import type { Site } from "./logica";

interface Props {
  site: Site;
  erro?: string;
  aberta: boolean;
  onAlternar: (aberta: boolean) => void;
  onRemover: (s: Site) => void;
}

// Collapsible danger zone. Whether it is open lives in the parent so it survives data reloads.
export const ZonaRisco: React.FC<Props> = ({ site, erro, aberta, onAlternar, onRemover }) => {
  const t = useT();
  const [confirmando, setConfirmando] = useState(false);
  return (
    <details className="st-risco" open={aberta} onToggle={(e) => onAlternar((e.currentTarget as HTMLDetailsElement).open)}>
      <summary>{t.sites.riskZone}</summary>
      <div className="linha-risco">
        <div>
          <strong>{t.sites.removeFromList}</strong>
          {site.embutida && <small>{t.sites.removeHint}</small>}
        </div>
        {!confirmando ? (
          <button
            type="button" className="st-btn perigo" disabled={site.embutida}
            title={site.embutida ? t.sites.removeHint : undefined} onClick={() => setConfirmando(true)}
          >{t.sites.removeBtn}</button>
        ) : (
          <div className="st-confirma">
            <span>{t.sites.removeConfirm}</span>
            <div style={{ display: "flex", gap: 8 }}>
              <button type="button" className="st-btn perigo" onClick={() => { setConfirmando(false); onRemover(site); }}>{t.sites.removeFromList}</button>
              <button type="button" className="st-btn" onClick={() => setConfirmando(false)}>{t.sites.form.cancel}</button>
            </div>
          </div>
        )}
      </div>
      {erro && <div role="alert" className="st-erro">{erro}</div>}
    </details>
  );
};
