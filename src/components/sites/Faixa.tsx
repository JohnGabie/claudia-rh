import React from "react";
import { useT } from "../../i18n";
import type { SituacaoFaixa } from "./estado";

// The AI strip, a real button. "falta" (site not ready) only navigates (onClick opens the page);
// "fechada"/"ligada" are a switch and onClick toggles the AI.
export const Faixa: React.FC<{ situacao: SituacaoFaixa; nome: string; onClick: () => void; className?: string }> = ({ situacao, nome, onClick, className = "" }) => {
  const t = useT();
  if (situacao === "falta") {
    return (
      <button type="button" aria-label={`${nome}: ${t.sites.needStep1}`} className={`st-faixa falta ${className}`} onClick={onClick}>
        <span className="marca" aria-hidden="true" />
        <span className="txt">{t.sites.needStep1}</span>
        <span className="seta">{t.sites.enterArrow}</span>
      </button>
    );
  }
  const texto = situacao === "ligada" ? t.sites.aiOpen : t.sites.aiClosed;
  return (
    <button
      type="button"
      role="switch"
      aria-checked={situacao === "ligada"}
      aria-label={`${nome}: ${texto}`}
      className={`st-faixa ${situacao} ${className}`}
      onClick={onClick}
    >
      <span className="marca" aria-hidden="true" />
      <span className="txt">{texto}</span>
    </button>
  );
};
