import React from "react";
import { useT } from "../../i18n";

// The AI indicator: a strip (not a switch widget). The click is handled by the card around it.
export const Faixa: React.FC<{ ligado: boolean; bloqueado: boolean; onLogou?: () => void; className?: string }> = ({ ligado, bloqueado, onLogou, className = "" }) => {
  const t = useT();
  const estado = bloqueado ? "bloqueada" : ligado ? "aberta" : "";
  const texto = bloqueado ? t.sites.aiBlocked : ligado ? t.sites.aiOpen : t.sites.aiClosed;
  return (
    <div className={`st-faixa ${estado} ${className}`}>
      <span className="marca" aria-hidden="true" />
      <span>{texto}</span>
      {bloqueado && onLogou && (
        <button type="button" className="logou" onClick={(e) => { e.stopPropagation(); onLogou(); }}>{t.sites.enteredBtn}</button>
      )}
    </div>
  );
};
