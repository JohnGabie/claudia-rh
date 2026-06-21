import React from "react";
import { invoke } from "@tauri-apps/api/core";

const LogoOculos: React.FC = () => (
  <svg width={22} height={12} viewBox="0 0 660 360" fill="none" strokeLinecap="round" stroke="#fff" strokeWidth={28}>
    <circle cx="160" cy="195" r="135" />
    <circle cx="500" cy="195" r="135" />
    <path d="M295 180 Q330 130 365 180" />
    <path d="M10 195 L35 192" />
    <path d="M650 195 L625 192" />
  </svg>
);

export const TitleBar: React.FC = () => {
  return (
    <div
      data-tauri-drag-region
      style={{
        height: 36,
        background: "var(--accent)",
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        userSelect: "none",
        flexShrink: 0,
        WebkitAppRegion: "drag",
      } as React.CSSProperties}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1 }}>
        <LogoOculos />
        <span style={{ color: "#fff", fontSize: 13, fontWeight: 600, letterSpacing: "0.01em" }}>
          Claudia RH
        </span>
      </div>

      <div
        style={{ display: "flex", gap: 4, alignItems: "center" }}
        onClick={(e) => e.stopPropagation()}
      >
        <WinBtn
          title="Minimizar"
          onClick={() => invoke("minimize_window")}
          icon={
            <svg width={10} height={2} viewBox="0 0 10 2">
              <rect width={10} height={2} fill="#fff" />
            </svg>
          }
        />
        <WinBtn
          title="Fechar"
          onClick={() => invoke("close_window")}
          hoverColor="rgba(196,43,28,0.85)"
          icon={
            <svg width={10} height={10} viewBox="0 0 10 10">
              <line x1={0} y1={0} x2={10} y2={10} stroke="#fff" strokeWidth={1.5} />
              <line x1={10} y1={0} x2={0} y2={10} stroke="#fff" strokeWidth={1.5} />
            </svg>
          }
        />
      </div>
    </div>
  );
};

const WinBtn: React.FC<{
  onClick: () => void;
  icon: React.ReactNode;
  title: string;
  hoverColor?: string;
}> = ({ onClick, icon, title, hoverColor = "rgba(255,255,255,0.2)" }) => {
  const [hovered, setHovered] = React.useState(false);
  return (
    <button
      title={title}
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        width: 32,
        height: 28,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: hovered ? hoverColor : "transparent",
        border: "none",
        cursor: "pointer",
        borderRadius: 4,
        transition: "background 0.1s",
        WebkitAppRegion: "no-drag",
      } as React.CSSProperties}
    >
      {icon}
    </button>
  );
};
