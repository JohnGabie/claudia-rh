import React from "react";

// Neutral circle with the first letter of the name. No favicon, no network.
export const Avatar: React.FC<{ nome: string; tamanho?: number }> = ({ nome, tamanho = 32 }) => {
  const letra = Array.from(nome.trim())[0]?.toUpperCase() ?? "?";
  return (
    <div
      aria-hidden="true"
      style={{
        width: tamanho, height: tamanho, minWidth: tamanho, borderRadius: "50%", background: "var(--bg-sunken)",
        color: "var(--text-secondary)", display: "flex", alignItems: "center", justifyContent: "center",
        fontSize: Math.round(tamanho * 0.42), fontWeight: 600,
      }}
    >
      {letra}
    </div>
  );
};
