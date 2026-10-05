import React, { useState } from "react";
import { useT } from "../../i18n";
import type { TipoDocumento } from "../../types";
import { Grade } from "./Grade";
import { PaginaDocumento } from "./PaginaDocumento";

export const Documentos: React.FC = () => {
  const t = useT();
  const [aba, setAba] = useState<TipoDocumento>("curriculo");
  const [aberto, setAberto] = useState<number | null>(null);

  if (aberto !== null) {
    return <PaginaDocumento id={aberto} onVoltar={() => setAberto(null)} />;
  }

  const abas: { id: TipoDocumento; rotulo: string }[] = [
    { id: "curriculo", rotulo: t.documents.tabResumes },
    { id: "carta", rotulo: t.documents.tabCoverLetters },
  ];

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: "0 0 16px" }}>{t.documents.title}</h1>
      <div role="tablist" style={{ display: "flex", gap: 4, borderBottom: "1px solid var(--border)", marginBottom: 20 }}>
        {abas.map(({ id, rotulo }) => {
          const ativa = aba === id;
          return (
            <button
              key={id}
              role="tab"
              aria-selected={ativa}
              onClick={() => setAba(id)}
              style={{
                padding: "8px 16px", background: "transparent", border: "none", cursor: "pointer", fontFamily: "inherit", fontSize: 14,
                fontWeight: ativa ? 500 : 400, color: ativa ? "var(--accent-strong)" : "var(--text-secondary)",
                borderBottom: ativa ? "2px solid var(--accent)" : "2px solid transparent", marginBottom: -1,
              }}
            >
              {rotulo}
            </button>
          );
        })}
      </div>
      <Grade key={aba} tipo={aba} onAbrir={setAberto} />
    </div>
  );
};
