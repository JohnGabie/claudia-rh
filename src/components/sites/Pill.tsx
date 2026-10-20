import React from "react";
import { useT } from "../../i18n";
import type { CorEstado } from "./estado";

export const Pill: React.FC<{ children: React.ReactNode }> = ({ children }) => <span className="st-pill">{children}</span>;

export const EstadoLinha: React.FC<{ cor: CorEstado; texto: string }> = ({ cor, texto }) => (
  <span className={`st-estado ${cor}`}>{texto}</span>
);

export const PapelPill: React.FC = () => {
  const t = useT();
  return <Pill>{t.sites.discovery}</Pill>;
};
