import React from "react";
import { useT } from "../../i18n";
import { Link2, ClipboardList, FileText } from "lucide-react";
import { ChatFocus } from "./types";

// ── Empty state ────────────────────────────────────────────────────────────

interface OptionDef {
  Icon: React.FC<{ size: number; color?: string }>;
  title: string;
  desc: string;
  preMessage: string;
  focus: string;
}

const OptionCard: React.FC<{ option: OptionDef; onStart: (focus: ChatFocus) => void }> = ({ option, onStart }) => {
  const [hov, setHov] = React.useState(false);
  const { Icon, title, desc, preMessage, focus } = option;
  return (
    <button
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      onClick={() => onStart({ section: focus, label: title, preMessage })}
      style={{
        flex: 1, background: "var(--bg-surface)",
        border: `1px solid ${hov ? "var(--accent)" : "var(--border)"}`,
        borderRadius: 8, padding: "16px 14px", cursor: "pointer",
        textAlign: "left", fontFamily: "inherit",
        transition: "border-color 0.15s, box-shadow 0.15s",
        boxShadow: hov ? "0 2px 12px rgba(217,119,87,0.12)" : "none",
      }}
    >
      <Icon size={20} color={hov ? "var(--accent)" : "var(--text-secondary)"} />
      <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginTop: 10, marginBottom: 4 }}>
        {title}
      </div>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
        {desc}
      </div>
    </button>
  );
};

const GithubLinkedinCard: React.FC<{ onStart: (focus: ChatFocus) => void }> = ({ onStart }) => {
  const t = useT();
  const [hov, setHov] = React.useState(false);
  return (
    <button
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      onClick={() => onStart({ section: "chrome", label: t.profile.githubLinkedin, chromeSessao: true })}
      style={{
        flex: 1, background: "var(--bg-surface)",
        border: `1px solid ${hov ? "var(--accent)" : "var(--border)"}`,
        borderRadius: 8, padding: "16px 14px", cursor: "pointer", textAlign: "left",
        fontFamily: "inherit",
        transition: "border-color 0.15s, box-shadow 0.15s",
        boxShadow: hov ? "0 2px 12px rgba(217,119,87,0.12)" : "none",
      }}
    >
      <Link2 size={20} color={hov ? "var(--accent)" : "var(--text-secondary)"} />
      <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginTop: 10, marginBottom: 4 }}>
        {t.profile.githubLinkedin}
      </div>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
        {t.profile.githubLinkedinDesc}
      </div>
    </button>
  );
};

export const EmptyState: React.FC<{ onStart: (focus: ChatFocus) => void }> = ({ onStart }) => {
  const t = useT();
  const EMPTY_OPTIONS: OptionDef[] = [
    {
      Icon: ClipboardList,
      title: t.profile.pasteResume,
      desc: t.profile.pasteResumeDesc,
      preMessage: "Quero colar o texto do meu currículo para estruturar o perfil.",
      focus: "colar_curriculo",
    },
    {
      Icon: FileText,
      title: t.profile.importFile,
      desc: t.profile.importFileDesc,
      preMessage: "Quero importar um arquivo de currículo (PDF ou DOCX).",
      focus: "importar_ficheiro",
    },
  ];
  return (
  <div style={{
    flex: 1, display: "flex", flexDirection: "column",
    alignItems: "center", justifyContent: "center",
    padding: 40, textAlign: "center",
  }}>
    {/* Logo */}
    <div style={{ marginBottom: 20 }}>
      <svg width={52} height={29} viewBox="0 0 660 360" fill="none" strokeLinecap="round"
        stroke="var(--accent)" strokeWidth={28}>
        <circle cx="160" cy="195" r="135" />
        <circle cx="500" cy="195" r="135" />
        <path d="M295 180 Q330 130 365 180" />
        <path d="M10 195 L35 192" />
        <path d="M650 195 L625 192" />
      </svg>
    </div>

    <h2 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0, marginBottom: 8 }}>
      {t.profile.emptyStateTitle}
    </h2>
    <p style={{ fontSize: 14, color: "var(--text-secondary)", maxWidth: 420, lineHeight: 1.6, margin: "0 0 32px" }}>
      {t.profile.emptyStateDesc}
    </p>

    <div style={{ display: "flex", gap: 12, width: "100%", maxWidth: 560 }}>
      {EMPTY_OPTIONS.map((opt) => (
        <OptionCard key={opt.focus} option={opt} onStart={onStart} />
      ))}
      <GithubLinkedinCard onStart={onStart} />
    </div>

    <button
      onClick={() => onStart({ section: "geral", label: t.profile.title, preMessage: "Olá! Vamos começar a construir o meu perfil." })}
      style={{
        marginTop: 20, padding: "9px 20px",
        background: "transparent", border: "1px solid var(--border)",
        borderRadius: 8, fontSize: 13, color: "var(--text-secondary)",
        cursor: "pointer", fontFamily: "inherit",
      }}
    >
      {t.profile.startDirectly}
    </button>
  </div>
  );
};
