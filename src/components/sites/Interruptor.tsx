import React from "react";

interface Props {
  ligado: boolean;
  bloqueado: boolean;
  nome: string;
  className: string;
  onMudar: () => void;
  children: React.ReactNode;
}

// Makes a whole card or row the AI toggle: role="switch" + aria-checked, Space/Enter, visible focus.
// A blocked site is not a switch; a click still reaches onMudar so the backend's refusal is shown.
// Keys only count when the focus is on the element itself, so inner links and buttons keep their own.
export const Interruptor: React.FC<Props> = ({ ligado, bloqueado, nome, className, onMudar, children }) => {
  const classes = `${className}${ligado && !bloqueado ? " aberta" : ""}${bloqueado ? " bloqueada" : ""}`;
  if (bloqueado) {
    return <div className={classes} aria-label={nome} onClick={onMudar}>{children}</div>;
  }
  return (
    <div
      className={classes}
      role="switch"
      aria-checked={ligado}
      aria-label={nome}
      tabIndex={0}
      onClick={onMudar}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget) return;
        if (e.key === " " || e.key === "Enter") { e.preventDefault(); onMudar(); }
      }}
    >
      {children}
    </div>
  );
};
