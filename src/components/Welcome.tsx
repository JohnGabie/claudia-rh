import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  LayoutDashboard,
  UserRound,
  History,
  SquareTerminal,
  Bell,
  MessageSquareText,
  Settings,
  CheckCircle2,
  AlertCircle,
  Info,
  RefreshCw,
  Check,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  Copy,
  ShieldCheck,
  Zap,
} from "lucide-react";
import { useT, useLocale } from "../i18n";
import type { Locale } from "../i18n";
import type { View } from "./Sidebar";

interface SetupStatus {
  claude_instalado: boolean;
  nativehost_chrome: boolean;
  perfil_preenchido: boolean;
}

interface WelcomeProps {
  onComplete: (navigateTo: View) => void;
}

// ── Logo ──────────────────────────────────────────────────────────────────────

const LogoOculos: React.FC = () => (
  <svg width={28} height={28 * (12 / 22)} viewBox="0 0 660 360" fill="none"
    strokeLinecap="round" stroke="var(--accent)" strokeWidth={28}>
    <circle cx="160" cy="195" r="135" />
    <circle cx="500" cy="195" r="135" />
    <path d="M295 180 Q330 130 365 180" />
    <path d="M10 195 L35 192" />
    <path d="M650 195 L625 192" />
  </svg>
);

// ── Stepper ───────────────────────────────────────────────────────────────────

const Stepper: React.FC<{ current: number; labels: string[] }> = ({ current, labels }) => (
  <div style={{ display: "flex", alignItems: "flex-start", marginBottom: 36, userSelect: "none" }}>
    {labels.map((label, i) => (
      <React.Fragment key={i}>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6, minWidth: 72 }}>
          <div style={{
            width: 32, height: 32, borderRadius: "50%",
            display: "flex", alignItems: "center", justifyContent: "center",
            fontSize: 13, fontWeight: 600,
            transition: "background 0.2s",
            ...(i < current
              ? { background: "var(--success)", color: "#fff", border: "none" }
              : i === current
                ? { background: "var(--accent)", color: "#fff", border: "none" }
                : { background: "var(--bg-surface)", color: "var(--text-tertiary)", border: "1px solid var(--border)" }
            ),
          }}>
            {i < current ? <Check size={15} /> : i + 1}
          </div>
          <div style={{
            fontSize: 11, textAlign: "center", whiteSpace: "nowrap",
            fontWeight: i === current ? 600 : 400,
            color: i === current ? "var(--accent-strong)" : i < current ? "var(--text-secondary)" : "var(--text-tertiary)",
          }}>
            {label}
          </div>
        </div>
        {i < labels.length - 1 && (
          <div style={{
            flex: 1, height: 1, marginTop: 16,
            background: i < current ? "var(--success)" : "var(--border)",
            transition: "background 0.3s",
          }} />
        )}
      </React.Fragment>
    ))}
  </div>
);

// ── Accordion card ────────────────────────────────────────────────────────────

type CardStatus = "ok" | "warn" | "info";

const AccordionCard: React.FC<{
  status: CardStatus;
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}> = ({ status, title, defaultOpen = false, children }) => {
  const [open, setOpen] = useState(defaultOpen);

  const iconEl =
    status === "ok" ? <CheckCircle2 size={18} color="var(--success)" /> :
    status === "warn" ? <AlertCircle size={18} color="var(--warning)" /> :
    <Info size={18} color="var(--accent)" />;

  const borderColor =
    status === "ok" ? "color-mix(in srgb, var(--success) 30%, var(--border))" :
    status === "warn" ? "color-mix(in srgb, var(--warning) 30%, var(--border))" :
    "color-mix(in srgb, var(--accent) 25%, var(--border))";

  const bgColor =
    status === "ok" ? "color-mix(in srgb, var(--success) 4%, var(--bg-surface))" :
    status === "info" ? "color-mix(in srgb, var(--accent) 4%, var(--bg-surface))" :
    "var(--bg-surface)";

  return (
    <div style={{ borderRadius: 10, border: `1px solid ${borderColor}`, background: bgColor, overflow: "hidden" }}>
      <button
        onClick={() => setOpen(!open)}
        style={{
          width: "100%", display: "flex", alignItems: "center", gap: 12,
          padding: "14px 16px", background: "transparent", border: "none",
          cursor: "pointer", fontFamily: "inherit", textAlign: "left",
        }}
      >
        {iconEl}
        <span style={{ flex: 1, fontSize: 14, fontWeight: 500, color: "var(--text-primary)" }}>
          {title}
        </span>
        {open ? <ChevronUp size={16} color="var(--text-tertiary)" /> : <ChevronDown size={16} color="var(--text-tertiary)" />}
      </button>
      {open && (
        <div style={{
          padding: "0 16px 16px 46px",
          fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.65,
          borderTop: `1px solid ${borderColor}`,
          paddingTop: 14,
        }}>
          {children}
        </div>
      )}
    </div>
  );
};

// ── Small primitives ──────────────────────────────────────────────────────────

const CodeBlock: React.FC<{ children: string }> = ({ children }) => {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard.writeText(children).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 8,
      background: "var(--bg-sunken)", border: "1px solid var(--border)",
      borderRadius: 6, padding: "7px 12px", marginTop: 6,
    }}>
      <code style={{ flex: 1, fontSize: 12, fontFamily: "monospace", color: "var(--text-primary)", userSelect: "all" }}>
        {children}
      </code>
      <button
        onClick={copy}
        title="Copy"
        style={{
          flexShrink: 0, background: "transparent", border: "none", cursor: "pointer",
          color: copied ? "var(--success)" : "var(--text-tertiary)", padding: 2,
          display: "flex", alignItems: "center",
        }}
      >
        {copied ? <Check size={13} /> : <Copy size={13} />}
      </button>
    </div>
  );
};

const LinkBtn: React.FC<{ onClick: () => void; children: React.ReactNode }> = ({ onClick, children }) => (
  <button
    onClick={onClick}
    style={{
      display: "inline-flex", alignItems: "center", gap: 5,
      padding: "5px 12px", borderRadius: 6, fontFamily: "inherit", fontSize: 12,
      border: "1px solid var(--border)", background: "var(--bg-base)",
      color: "var(--text-secondary)", cursor: "pointer",
    }}
    onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-primary)")}
    onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")}
  >
    <ExternalLink size={11} />
    {children}
  </button>
);

const PrimaryBtn: React.FC<{ onClick: () => void; children: React.ReactNode }> = ({ onClick, children }) => (
  <button
    onClick={onClick}
    style={{
      display: "inline-flex", alignItems: "center", gap: 5,
      padding: "6px 14px", borderRadius: 6, fontFamily: "inherit", fontSize: 12,
      border: "none", background: "var(--accent)", color: "#fff",
      fontWeight: 500, cursor: "pointer",
    }}
  >
    {children}
  </button>
);

const Badge: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <span style={{
    display: "inline-block", padding: "1px 8px", borderRadius: 20,
    fontSize: 11, fontWeight: 600,
    background: "color-mix(in srgb, var(--accent) 12%, var(--bg-sunken))",
    color: "var(--accent-strong)", border: "1px solid color-mix(in srgb, var(--accent) 20%, var(--border))",
  }}>
    {children}
  </span>
);

// Numbered step row — title + optional description + optional children
const S: React.FC<{ n: number; title: string; desc?: string; children?: React.ReactNode; last?: boolean }> = ({
  n, title, desc, children, last,
}) => (
  <div style={{ display: "flex", gap: 12, paddingBottom: last ? 0 : 18 }}>
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", flexShrink: 0 }}>
      <div style={{
        width: 24, height: 24, borderRadius: "50%", flexShrink: 0,
        background: "var(--accent-soft)", color: "var(--accent-strong)",
        display: "flex", alignItems: "center", justifyContent: "center",
        fontSize: 11, fontWeight: 700,
      }}>
        {n}
      </div>
      {!last && <div style={{ flex: 1, width: 1, background: "var(--border)", marginTop: 4 }} />}
    </div>
    <div style={{ flex: 1, minWidth: 0, paddingTop: 3, paddingBottom: last ? 0 : 4 }}>
      <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: desc || children ? 4 : 0 }}>
        {title}
      </div>
      {desc && <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.6 }}>{desc}</div>}
      {children && <div style={{ marginTop: 6 }}>{children}</div>}
    </div>
  </div>
);

const SuccessNote: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{
    display: "flex", gap: 8, alignItems: "flex-start",
    padding: "8px 12px", borderRadius: 6, marginTop: 4,
    background: "color-mix(in srgb, var(--success) 8%, var(--bg-sunken))",
    border: "1px solid color-mix(in srgb, var(--success) 25%, var(--border))",
    fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5,
  }}>
    <CheckCircle2 size={13} color="var(--success)" style={{ flexShrink: 0, marginTop: 1 }} />
    {children}
  </div>
);

// ── Step 0: Language ──────────────────────────────────────────────────────────

const StepLanguage: React.FC = () => {
  const { locale, setLocale } = useLocale();

  const options: { id: Locale; label: string; desc: string }[] = [
    { id: "en", label: "English", desc: "Interface in English" },
    { id: "pt", label: "Português (Brasil)", desc: "Interface em Português" },
  ];

  return (
    <div>
      <p style={{ fontSize: 13, color: "var(--text-secondary)", textAlign: "center", marginBottom: 24, lineHeight: 1.5 }}>
        What language do you prefer? / Qual idioma você prefere?
      </p>
      <div style={{ display: "flex", gap: 14 }}>
        {options.map((opt) => {
          const active = locale === opt.id;
          return (
            <button
              key={opt.id}
              onClick={() => setLocale(opt.id)}
              style={{
                flex: 1, padding: "20px 16px", borderRadius: 10, fontFamily: "inherit",
                border: `2px solid ${active ? "var(--accent)" : "var(--border)"}`,
                background: active ? "color-mix(in srgb, var(--accent) 8%, var(--bg-surface))" : "var(--bg-surface)",
                cursor: "pointer", textAlign: "left",
                transition: "border-color 0.15s, background 0.15s",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 6 }}>
                <span style={{ fontSize: 15, fontWeight: 600, color: active ? "var(--accent-strong)" : "var(--text-primary)" }}>
                  {opt.label}
                </span>
                {active && <CheckCircle2 size={18} color="var(--accent)" />}
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{opt.desc}</div>
            </button>
          );
        })}
      </div>
    </div>
  );
};

// ── Step 1: Install ───────────────────────────────────────────────────────────

const StepInstall: React.FC<{
  status: SetupStatus | null;
  checking: boolean;
  onRecheck: () => void;
  onOpenTerminal: () => void;
}> = ({ status, checking, onRecheck, onOpenTerminal }) => {
  const t = useT();
  const cliOk = !!status?.claude_instalado;
  const chromeOk = !!status?.nativehost_chrome;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>

      {/* 1 · Subscription */}
      <AccordionCard status="info" title={t.welcome.subscriptionTitle} defaultOpen={!chromeOk}>
        {chromeOk ? (
          <SuccessNote>{t.welcome.subscriptionOk}</SuccessNote>
        ) : (
          <>
            <S n={1} title={t.welcome.subscriptionS1} desc={t.welcome.subscriptionS1Desc}>
              <LinkBtn onClick={() => openUrl("https://claude.ai").catch(console.error)}>
                {t.welcome.subscriptionS1Btn}
              </LinkBtn>
            </S>
            <S n={2} title={t.welcome.subscriptionS2} desc={t.welcome.subscriptionS2Desc}>
              <Badge>{t.welcome.subscriptionS2Badge}</Badge>
            </S>
            <S n={3} title={t.welcome.subscriptionS3} desc={t.welcome.subscriptionS3Desc} last />
          </>
        )}
      </AccordionCard>

      {/* 2 · Claude Code CLI */}
      <AccordionCard status={cliOk ? "ok" : "warn"} title={t.welcome.claudeCodeTitle} defaultOpen={!cliOk}>
        {cliOk ? (
          <SuccessNote>{t.welcome.claudeCodeOk}</SuccessNote>
        ) : (
          <>
            <S n={1} title={t.welcome.claudeCodeS1} desc={t.welcome.claudeCodeS1Desc}>
              <LinkBtn onClick={() => openUrl("https://nodejs.org").catch(console.error)}>
                {t.welcome.claudeCodeS1Btn}
              </LinkBtn>
            </S>
            <S n={2} title={t.welcome.claudeCodeS2} desc={t.welcome.claudeCodeS2Desc}>
              <CodeBlock>npm install -g @anthropic-ai/claude-code</CodeBlock>
            </S>
            <S n={3} title={t.welcome.claudeCodeS3} desc={t.welcome.claudeCodeS3Desc}>
              <CodeBlock>claude --version</CodeBlock>
            </S>
            <S n={4} title={t.welcome.claudeCodeS4} desc={t.welcome.claudeCodeS4Desc} last>
              <CodeBlock>{t.welcome.claudeCodeS4Cmd}</CodeBlock>
              <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 8, lineHeight: 1.5 }}>
                {t.welcome.claudeCodeRestartNote}
              </div>
            </S>
          </>
        )}
      </AccordionCard>

      {/* 3 · Chrome extension */}
      <AccordionCard status={chromeOk ? "ok" : "warn"} title={t.welcome.chromeExtTitle} defaultOpen={!chromeOk}>
        {chromeOk ? (
          <SuccessNote>{t.welcome.chromeExtOk}</SuccessNote>
        ) : (
          <>
            <S n={1} title={t.welcome.chromeExtS1} desc={t.welcome.chromeExtS1Desc} />
            <S n={2} title={t.welcome.chromeExtS2} desc={t.welcome.chromeExtS2Desc}>
              <LinkBtn onClick={() => openUrl("https://chromewebstore.google.com/detail/claude/fcoeoabgfenejglbffodgkkbkcdhcgfn").catch(console.error)}>
                {t.welcome.chromeExtS2Btn}
              </LinkBtn>
            </S>
            <S n={3} title={t.welcome.chromeExtS3} desc={t.welcome.chromeExtS3Desc} />
            <S n={4} title={t.welcome.chromeExtS4} desc={t.welcome.chromeExtS4Desc} last />
          </>
        )}
      </AccordionCard>

      {/* 4 · Enable Chrome in CLI */}
      <AccordionCard status={chromeOk ? "ok" : "warn"} title={t.welcome.chromeHostTitle} defaultOpen={!chromeOk}>
        {chromeOk ? (
          <SuccessNote>{t.welcome.chromeHostOk}</SuccessNote>
        ) : (
          <>
            <S n={1} title={t.welcome.chromeHostS1} desc={t.welcome.chromeHostS1Desc}>
              <PrimaryBtn onClick={onOpenTerminal}>{t.welcome.chromeHostS1Btn}</PrimaryBtn>
            </S>
            <S n={2} title={t.welcome.chromeHostS2} desc={t.welcome.chromeHostS2Desc}>
              <CodeBlock>{t.welcome.chromeHostS2Cmd}</CodeBlock>
            </S>
            <S n={3} title={t.welcome.chromeHostS3} desc={t.welcome.chromeHostS3Desc}>
              <CodeBlock>{t.welcome.chromeHostS3Cmd}</CodeBlock>
            </S>
            <S n={4} title={t.welcome.chromeHostS4} desc={t.welcome.chromeHostS4Desc} last>
              <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 6, lineHeight: 1.5 }}>
                {t.welcome.chromeHostPermanentNote}
              </div>
            </S>
          </>
        )}
      </AccordionCard>

      {/* Permissions note */}
      <div style={{
        padding: "10px 14px", borderRadius: 8, background: "var(--bg-sunken)",
        fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.6,
      }}>
        {t.welcome.permissionsNote}
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end" }}>
        <button
          onClick={onRecheck}
          disabled={checking}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "6px 14px", borderRadius: 6,
            border: "1px solid var(--border)", background: "transparent",
            color: "var(--text-secondary)", fontSize: 12, fontFamily: "inherit",
            cursor: checking ? "default" : "pointer",
          }}
        >
          <RefreshCw size={12} style={{ animation: checking ? "spin 1s linear infinite" : "none" }} />
          {checking ? t.welcome.checking : t.welcome.recheckBtn}
        </button>
      </div>
    </div>
  );
};

// ── Step 2: Tour ──────────────────────────────────────────────────────────────

const StepTour: React.FC = () => {
  const t = useT();
  const [selected, setSelected] = useState(0);

  const tabs = [
    { icon: <LayoutDashboard size={17} />, iconLg: <LayoutDashboard size={28} />, label: t.nav.dashboard, desc: t.welcome.tourDashboard },
    { icon: <UserRound size={17} />, iconLg: <UserRound size={28} />, label: t.nav.profile, desc: t.welcome.tourProfile },
    { icon: <History size={17} />, iconLg: <History size={28} />, label: t.nav.history, desc: t.welcome.tourHistory },
    { icon: <SquareTerminal size={17} />, iconLg: <SquareTerminal size={28} />, label: t.nav.terminal, desc: t.welcome.tourTerminal },
    { icon: <Bell size={17} />, iconLg: <Bell size={28} />, label: t.nav.notifications, desc: t.welcome.tourNotifications },
    { icon: <MessageSquareText size={17} />, iconLg: <MessageSquareText size={28} />, label: t.nav.feedback, desc: t.welcome.tourFeedback },
    { icon: <Settings size={17} />, iconLg: <Settings size={28} />, label: t.nav.settings, desc: t.welcome.tourSettings },
  ];

  const active = tabs[selected];

  return (
    <div style={{
      display: "flex", borderRadius: 12,
      border: "1px solid var(--border)", overflow: "hidden",
      background: "var(--bg-surface)", minHeight: 320,
    }}>
      {/* Mini sidebar */}
      <div style={{
        width: 168, flexShrink: 0,
        borderRight: "1px solid var(--border)",
        background: "var(--bg-surface)",
        display: "flex", flexDirection: "column",
        padding: "8px 0",
      }}>
        {/* Logo bar */}
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "6px 14px 14px",
          borderBottom: "1px solid var(--border)", marginBottom: 6,
        }}>
          <svg width={16} height={16 * (12 / 22)} viewBox="0 0 660 360" fill="none"
            strokeLinecap="round" stroke="var(--accent)" strokeWidth={32}>
            <circle cx="160" cy="195" r="135" />
            <circle cx="500" cy="195" r="135" />
            <path d="M295 180 Q330 130 365 180" />
            <path d="M10 195 L35 192" />
            <path d="M650 195 L625 192" />
          </svg>
          <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>Claudia RH</span>
        </div>

        {tabs.map((tab, i) => {
          const isActive = i === selected;
          return (
            <button
              key={i}
              onClick={() => setSelected(i)}
              style={{
                display: "flex", alignItems: "center", gap: 8,
                width: "100%", height: 36, padding: "0 14px",
                border: "none", cursor: "pointer", fontFamily: "inherit",
                background: isActive ? "var(--accent-soft)" : "transparent",
                color: isActive ? "var(--accent-strong)" : "var(--text-secondary)",
                fontSize: 13, textAlign: "left",
                transition: "background 0.1s, color 0.1s",
              }}
              onMouseEnter={(e) => { if (!isActive) (e.currentTarget as HTMLElement).style.background = "var(--bg-sunken)"; }}
              onMouseLeave={(e) => { if (!isActive) (e.currentTarget as HTMLElement).style.background = "transparent"; }}
            >
              {tab.icon}
              <span style={{ fontWeight: isActive ? 500 : 400 }}>{tab.label}</span>
            </button>
          );
        })}
      </div>

      {/* Detail panel */}
      <div style={{
        flex: 1, padding: "32px 28px",
        display: "flex", flexDirection: "column", justifyContent: "center",
        background: "var(--bg-base)",
      }}>
        <div style={{
          width: 52, height: 52, borderRadius: 14, marginBottom: 16,
          background: "var(--accent-soft)", color: "var(--accent-strong)",
          display: "flex", alignItems: "center", justifyContent: "center",
        }}>
          {active.iconLg}
        </div>
        <div style={{ fontSize: 18, fontWeight: 700, color: "var(--text-primary)", marginBottom: 10 }}>
          {active.label}
        </div>
        <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.75, maxWidth: 360 }}>
          {active.desc}
        </div>
        <div style={{ display: "flex", gap: 6, marginTop: 24 }}>
          {tabs.map((_, i) => (
            <div key={i} onClick={() => setSelected(i)} style={{
              width: i === selected ? 18 : 6, height: 6, borderRadius: 3,
              background: i === selected ? "var(--accent)" : "var(--border)",
              cursor: "pointer", transition: "width 0.2s, background 0.2s",
            }} />
          ))}
        </div>
      </div>
    </div>
  );
};

// ── Step 3: Permissions / Mode ────────────────────────────────────────────────

const StepPermissions: React.FC = () => {
  const t = useT();
  const [autonomous, setAutonomous] = useState<boolean | null>(null);

  useEffect(() => {
    invoke<boolean>("obter_modo_autonomo")
      .then(v => setAutonomous(!!v))
      .catch(() => setAutonomous(false));
  }, []);

  const choose = (value: boolean) => {
    setAutonomous(value);
    invoke("configurar_modo_autonomo", { ativo: value }).catch(console.error);
  };

  const modes: {
    value: boolean;
    icon: React.ReactNode;
    title: string;
    subtitle: string;
    pros: string[];
    con: string;
    borderColor: string;
    bgColor: string;
    iconColor: string;
  }[] = [
    {
      value: false,
      icon: <ShieldCheck size={26} />,
      title: t.welcome.modeSupTitle,
      subtitle: t.welcome.modeSupSubtitle,
      pros: [t.welcome.modeSupPro1, t.welcome.modeSupPro2, t.welcome.modeSupPro3],
      con: t.welcome.modeSupCon,
      borderColor: "color-mix(in srgb, var(--success) 35%, var(--border))",
      bgColor: "color-mix(in srgb, var(--success) 5%, var(--bg-surface))",
      iconColor: "var(--success)",
    },
    {
      value: true,
      icon: <Zap size={26} />,
      title: t.welcome.modeAutoTitle,
      subtitle: t.welcome.modeAutoSubtitle,
      pros: [t.welcome.modeAutoPro1, t.welcome.modeAutoPro2, t.welcome.modeAutoPro3],
      con: t.welcome.modeAutoCon,
      borderColor: "color-mix(in srgb, var(--warning) 35%, var(--border))",
      bgColor: "color-mix(in srgb, var(--warning) 5%, var(--bg-surface))",
      iconColor: "var(--warning)",
    },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <p style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.65, margin: 0 }}>
        {t.welcome.modeDesc}
      </p>

      <div style={{ display: "flex", gap: 12 }}>
        {modes.map((mode) => {
          const active = autonomous === mode.value;
          return (
            <button
              key={String(mode.value)}
              onClick={() => choose(mode.value)}
              style={{
                flex: 1, padding: "20px 18px", borderRadius: 12, fontFamily: "inherit",
                border: `2px solid ${active ? mode.borderColor : "var(--border)"}`,
                background: active ? mode.bgColor : "var(--bg-surface)",
                cursor: "pointer", textAlign: "left",
                transition: "border-color 0.15s, background 0.15s",
                position: "relative",
              }}
            >
              {/* Active badge */}
              {active && (
                <div style={{
                  position: "absolute", top: 12, right: 12,
                  display: "flex", alignItems: "center", gap: 4,
                  padding: "2px 8px", borderRadius: 20,
                  fontSize: 11, fontWeight: 600,
                  background: mode.value
                    ? "color-mix(in srgb, var(--warning) 15%, var(--bg-sunken))"
                    : "color-mix(in srgb, var(--success) 15%, var(--bg-sunken))",
                  color: mode.iconColor,
                  border: `1px solid ${mode.borderColor}`,
                }}>
                  <Check size={10} /> {t.welcome.modeActive}
                </div>
              )}

              {/* Icon */}
              <div style={{
                width: 44, height: 44, borderRadius: 10, marginBottom: 14,
                display: "flex", alignItems: "center", justifyContent: "center",
                background: active
                  ? `color-mix(in srgb, ${mode.iconColor} 12%, var(--bg-sunken))`
                  : "var(--bg-sunken)",
                color: active ? mode.iconColor : "var(--text-tertiary)",
                transition: "background 0.15s, color 0.15s",
              }}>
                {mode.icon}
              </div>

              <div style={{ fontSize: 15, fontWeight: 600, color: "var(--text-primary)", marginBottom: 3 }}>
                {mode.title}
              </div>
              <div style={{ fontSize: 11, fontWeight: 500, color: mode.iconColor, marginBottom: 14 }}>
                {mode.subtitle}
              </div>

              {/* Pros */}
              <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 }}>
                {mode.pros.map((pro, i) => (
                  <div key={i} style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
                    <CheckCircle2 size={13} color="var(--success)" style={{ flexShrink: 0, marginTop: 1 }} />
                    <span style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>{pro}</span>
                  </div>
                ))}
              </div>

              {/* Con */}
              <div style={{ display: "flex", gap: 8, alignItems: "flex-start", paddingTop: 10, borderTop: "1px solid var(--border)" }}>
                <AlertCircle size={13} color="var(--text-tertiary)" style={{ flexShrink: 0, marginTop: 1 }} />
                <span style={{ fontSize: 12, color: "var(--text-tertiary)", lineHeight: 1.5 }}>{mode.con}</span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Risk note — shown only in autonomous */}
      {autonomous === true && (
        <div style={{
          display: "flex", gap: 10, alignItems: "flex-start",
          padding: "10px 14px", borderRadius: 8,
          background: "color-mix(in srgb, var(--warning) 8%, var(--bg-sunken))",
          border: "1px solid color-mix(in srgb, var(--warning) 30%, var(--border))",
          fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.6,
        }}>
          <AlertCircle size={14} color="var(--warning)" style={{ flexShrink: 0, marginTop: 1 }} />
          {t.welcome.modeRiskNote}
        </div>
      )}

      {/* Flag info */}
      <div style={{
        display: "flex", gap: 10, alignItems: "flex-start",
        padding: "10px 14px", borderRadius: 8,
        background: "var(--bg-sunken)",
        border: "1px solid var(--border)",
        fontSize: 12, color: "var(--text-tertiary)", lineHeight: 1.6,
      }}>
        <Info size={14} color="var(--text-tertiary)" style={{ flexShrink: 0, marginTop: 1 }} />
        {t.welcome.modeFlagNote}
      </div>
    </div>
  );
};

// ── Step 4: Profile ───────────────────────────────────────────────────────────

const StepProfile: React.FC<{
  hasProfile: boolean;
  onSetup: () => void;
  onDashboard: () => void;
}> = ({ hasProfile, onSetup, onDashboard }) => {
  const t = useT();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      <div style={{
        padding: "20px 22px", borderRadius: 10,
        border: "1px solid var(--border)", background: "var(--bg-surface)",
      }}>
        <p style={{ fontSize: 14, color: "var(--text-secondary)", lineHeight: 1.7, margin: "0 0 16px 0" }}>
          {t.welcome.profileDesc}
        </p>
        <div style={{
          display: "flex", alignItems: "center", gap: 10,
          padding: "10px 14px", borderRadius: 8,
          background: hasProfile ? "color-mix(in srgb, var(--success) 8%, var(--bg-sunken))" : "var(--bg-sunken)",
          border: `1px solid ${hasProfile ? "color-mix(in srgb, var(--success) 25%, var(--border))" : "var(--border)"}`,
        }}>
          {hasProfile ? <CheckCircle2 size={16} color="var(--success)" /> : <AlertCircle size={16} color="var(--warning)" />}
          <span style={{ fontSize: 13, color: "var(--text-primary)" }}>
            {hasProfile ? t.welcome.profileAlreadyDone : t.welcome.profileEmpty}
          </span>
        </div>
      </div>
      <div style={{ display: "flex", gap: 10 }}>
        <button
          onClick={onSetup}
          style={{
            flex: 1, padding: "12px 24px", borderRadius: 8, border: "none",
            background: "var(--accent)", color: "#fff",
            fontSize: 14, fontWeight: 500, fontFamily: "inherit", cursor: "pointer",
          }}
        >
          {t.welcome.setupProfile}
        </button>
        <button
          onClick={onDashboard}
          style={{
            padding: "12px 20px", borderRadius: 8,
            border: "1px solid var(--border)", background: "transparent",
            color: "var(--text-secondary)", fontSize: 14, fontFamily: "inherit", cursor: "pointer",
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-primary)")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")}
        >
          {t.welcome.goToDashboard}
        </button>
      </div>
    </div>
  );
};

// ── Main ──────────────────────────────────────────────────────────────────────

export const Welcome: React.FC<WelcomeProps> = ({ onComplete }) => {
  const t = useT();
  const [step, setStep] = useState(0);
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [checking, setChecking] = useState(false);

  const TOTAL = 5;

  const recheck = () => {
    setChecking(true);
    invoke<SetupStatus>("verificar_setup")
      .then(setStatus)
      .catch(console.error)
      .finally(() => setChecking(false));
  };

  useEffect(() => { recheck(); }, []);

  const complete = async (view: View) => {
    await invoke("marcar_welcome_concluido").catch(console.error);
    onComplete(view);
  };

  const stepLabels = [t.welcome.stepLang, t.welcome.stepInstall, t.welcome.stepTour, t.welcome.stepMode, t.welcome.stepProfile];
  const stepTitles = [t.welcome.step0Title, t.welcome.step1Title, t.welcome.step2Title, t.welcome.step3Title, t.welcome.step4Title];

  return (
    <div style={{
      flex: 1, display: "flex", alignItems: "flex-start", justifyContent: "center",
      background: "var(--bg-base)", overflow: "auto", padding: "36px 24px",
    }}>
      <div style={{ width: "100%", maxWidth: 700 }}>
        {/* Header */}
        <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 32 }}>
          <LogoOculos />
          <span style={{ fontSize: 20, fontWeight: 700, color: "var(--text-primary)" }}>Claudia RH</span>
        </div>

        {/* Stepper */}
        <Stepper current={step} labels={stepLabels} />

        {/* Step title */}
        <h2 style={{ fontSize: 17, fontWeight: 600, color: "var(--text-primary)", margin: "0 0 20px 0" }}>
          {stepTitles[step]}
        </h2>

        {/* Content */}
        <div style={{ marginBottom: 32 }}>
          {step === 0 && <StepLanguage />}
          {step === 1 && (
            <StepInstall
              status={status}
              checking={checking}
              onRecheck={recheck}
              onOpenTerminal={() => complete("terminal")}
            />
          )}
          {step === 2 && <StepTour />}
          {step === 3 && <StepPermissions />}
          {step === 4 && (
            <StepProfile
              hasProfile={status?.perfil_preenchido ?? false}
              onSetup={() => complete("perfil")}
              onDashboard={() => complete("dashboard")}
            />
          )}
        </div>

        {/* Footer */}
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          {step > 0 && (
            <button
              onClick={() => setStep(step - 1)}
              style={{
                padding: "8px 18px", borderRadius: 6,
                border: "1px solid var(--border)", background: "transparent",
                color: "var(--text-secondary)", fontSize: 13, fontFamily: "inherit", cursor: "pointer",
              }}
            >
              {t.welcome.back}
            </button>
          )}
          <div style={{ flex: 1 }} />
          <button
            onClick={() => complete("dashboard")}
            style={{
              padding: "8px 14px", borderRadius: 6, border: "none",
              background: "transparent", color: "var(--text-tertiary)",
              fontSize: 13, fontFamily: "inherit", cursor: "pointer",
            }}
            onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")}
            onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-tertiary)")}
          >
            {t.welcome.skip}
          </button>
          {step < TOTAL - 1 && (
            <button
              onClick={() => setStep(step + 1)}
              style={{
                padding: "8px 22px", borderRadius: 6, border: "none",
                background: "var(--accent)", color: "#fff",
                fontSize: 13, fontWeight: 500, fontFamily: "inherit", cursor: "pointer",
              }}
            >
              {t.welcome.next}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
