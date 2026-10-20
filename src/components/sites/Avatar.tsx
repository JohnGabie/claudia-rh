import React from "react";

// Small neutral monogram (first letter of the name). No favicon, no network.
export const Avatar: React.FC<{ nome: string }> = ({ nome }) => (
  <span className="st-mono" aria-hidden="true">{Array.from(nome.trim())[0]?.toUpperCase() ?? "?"}</span>
);
