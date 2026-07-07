import React from "react";
import {
  LayoutDashboard,
  UserRound,
  History,
  SquareTerminal,
  Settings,
  MessageSquareText,
  Bell,
} from "lucide-react";
import { useT } from "../i18n";

export type View = "dashboard" | "perfil" | "historico" | "feedback" | "pendencias" | "terminal" | "configuracoes";

interface SidebarProps {
  active: View;
  onChange: (v: View) => void;
  sugerirFeedback?: boolean;
  pendenciasCount?: number;
}

const LogoOculos: React.FC = () => (
  <svg width={22} height={12} viewBox="0 0 660 360" fill="none" strokeLinecap="round"
    stroke="var(--accent)" strokeWidth={28}>
    <circle cx="160" cy="195" r="135" />
    <circle cx="500" cy="195" r="135" />
    <path d="M295 180 Q330 130 365 180" />
    <path d="M10 195 L35 192" />
    <path d="M650 195 L625 192" />
  </svg>
);

export const Sidebar: React.FC<SidebarProps> = ({ active, onChange, sugerirFeedback = false, pendenciasCount = 0 }) => {
  const t = useT();

  const items: { id: View; label: string; Icon: React.FC<{ size: number }> }[] = [
    { id: "dashboard", label: t.nav.dashboard, Icon: LayoutDashboard },
    { id: "perfil", label: t.nav.profile, Icon: UserRound },
    { id: "historico", label: t.nav.history, Icon: History },
    { id: "feedback", label: t.nav.feedback, Icon: MessageSquareText },
    { id: "pendencias", label: t.nav.notifications, Icon: Bell },
    { id: "terminal", label: t.nav.terminal, Icon: SquareTerminal },
  ];

  return (
    <aside style={{
      width: 220,
      minWidth: 220,
      height: "100%",
      background: "var(--bg-surface)",
      borderRight: "1px solid var(--border)",
      display: "flex",
      flexDirection: "column",
      overflow: "hidden",
    }}>
      {/* Logo + name */}
      <div style={{
        height: 56, flexShrink: 0,
        display: "flex", alignItems: "center",
        padding: "0 16px", gap: 10,
        borderBottom: "1px solid var(--border)",
      }}>
        <LogoOculos />
        <span style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)", letterSpacing: "0.01em" }}>
          Claudia RH
        </span>
      </div>

      {/* Nav */}
      <nav style={{ flex: 1, padding: "8px 0", overflowY: "auto" }}>
        {items.map(({ id, label, Icon }) => {
          const isActive = active === id;
          return (
            <NavItem
              key={id}
              label={label}
              isActive={isActive}
              onClick={() => onChange(id)}
              dot={id === "feedback" && sugerirFeedback}
              badge={id === "pendencias" && pendenciasCount > 0 ? pendenciasCount : undefined}
            >
              <Icon size={18} />
            </NavItem>
          );
        })}
      </nav>

      {/* Bottom gear → settings */}
      <div style={{ padding: "12px 10px", borderTop: "1px solid var(--border)", flexShrink: 0 }}>
        <button
          onClick={() => onChange("configuracoes")}
          title={t.nav.settings}
          style={{
            width: "100%",
            display: "flex",
            alignItems: "center",
            gap: 10,
            height: 36,
            padding: "0 6px",
            background: active === "configuracoes" ? "var(--accent-soft)" : "transparent",
            border: "none",
            borderRadius: 6,
            cursor: "pointer",
            color: active === "configuracoes" ? "var(--accent-strong)" : "var(--text-tertiary)",
            fontFamily: "inherit",
            fontSize: 13,
            transition: "color 0.1s, background 0.1s",
          }}
          onMouseEnter={(e) => {
            if (active !== "configuracoes")
              (e.currentTarget as HTMLElement).style.color = "var(--text-secondary)";
          }}
          onMouseLeave={(e) => {
            if (active !== "configuracoes")
              (e.currentTarget as HTMLElement).style.color = "var(--text-tertiary)";
          }}
        >
          <Settings size={18} />
          <span>{t.nav.settings}</span>
        </button>
      </div>
    </aside>
  );
};

const NavItem: React.FC<{
  label: string;
  isActive: boolean;
  onClick: () => void;
  badge?: number;
  dot?: boolean;
  children: React.ReactNode;
}> = ({ label, isActive, onClick, badge, dot, children }) => {
  const [hovered, setHovered] = React.useState(false);
  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        width: "100%",
        height: 40,
        padding: "0 16px",
        border: "none",
        cursor: "pointer",
        background: isActive ? "var(--accent-soft)" : hovered ? "var(--bg-sunken)" : "transparent",
        color: isActive ? "var(--accent-strong)" : hovered ? "var(--text-primary)" : "var(--text-secondary)",
        fontSize: 14,
        fontFamily: "inherit",
        textAlign: "left",
        transition: "background 0.1s, color 0.1s",
      }}
    >
      {children}
      <span style={{ fontWeight: isActive ? 500 : 400, flex: 1 }}>{label}</span>
      {badge !== undefined && (
        <span style={{
          background: "var(--danger)",
          color: "#fff",
          fontSize: 11,
          fontWeight: 600,
          padding: "1px 6px",
          borderRadius: 6,
          minWidth: 20,
          textAlign: "center",
        }}>
          {badge > 9 ? "9+" : badge}
        </span>
      )}
      {dot && badge === undefined && (
        <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--accent)", flexShrink: 0 }} />
      )}
    </button>
  );
};
