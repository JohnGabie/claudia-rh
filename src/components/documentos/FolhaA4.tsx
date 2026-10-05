import React from "react";

// A4 at 96 dpi: 210 x 297 mm = 794 x 1123 px.
export const A4_LARGURA = 794;
export const A4_ALTURA = 1123;

// Read-only preview of a document's HTML on an A4 sheet. The iframe has an empty sandbox (no scripts,
// no same-origin), so a document can never run code inside the app.
export const FolhaA4: React.FC<{ html: string; escala?: number; titulo: string }> = ({ html, escala, titulo }) => {
  const e = escala ?? 1;
  return (
    <div
      style={{
        width: A4_LARGURA * e, height: A4_ALTURA * e, flexShrink: 0,
        background: "#fff", border: "1px solid var(--border)", borderRadius: 2,
        boxShadow: "0 4px 16px rgba(31,29,24,0.08)", overflow: "hidden", position: "relative",
      }}
    >
      <iframe
        sandbox=""
        srcDoc={html}
        title={titulo}
        style={{
          width: A4_LARGURA, height: A4_ALTURA, border: 0, background: "#fff",
          transform: `scale(${e})`, transformOrigin: "top left",
          pointerEvents: escala && escala < 1 ? "none" : "auto",
        }}
      />
    </div>
  );
};
