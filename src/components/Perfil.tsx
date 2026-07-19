import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useT } from "../i18n";
import {
  ArrowLeft,
  Paperclip,
  Plus,
  Send,
  FileText,
  Link2,
  ClipboardList,
  Pencil,
  Square,
  AlertCircle,
  Loader2,
  Lightbulb,
  MailOpen,
  X,
} from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────────────

interface DadosPessoais {
  nome_completo: string;
  email: string;
  telefone: string;
  localizacao_atual: string;
  endereco: string;
  nacionalidade: string;
  data_nascimento: string;
  cpf: string;
  links: { tipo: string; url: string }[];
}

interface Experiencia {
  empresa: string;
  cargo: string;
  inicio: string;
  fim: string;
  descricao: string;
  conquistas: string[];
  tecnologias: string[];
}

interface CandidateBase {
  dados_pessoais: DadosPessoais;
  experiencia: Experiencia[];
  projetos: { nome: string; descricao: string; tecnologias: string[]; url: string; origem: string }[];
  formacao: { instituicao: string; curso: string; inicio: string; fim: string }[];
  competencias: string[];
  idiomas: { idioma: string; nivel: string }[];
  gaps_conhecidos: { competencia: string; contexto: string; como_abordar: string }[];
  respostas_modelo: { porque_esta_vaga: string; pretensao_salarial_texto: string; notice_period: string };
  ultima_atualizacao: string;
  fontes_usadas: { tipo: string; referencia: string; consultado_em: string }[];
}

interface SearchVariant {
  id: string;
  nome_exibicao: string;
  peso: number;
  ativa: boolean;
  foco_competencias: string[];
  foco_experiencia: string[];
  regioes_aceitas: string[];
  modelos_trabalho: string[];
  idiomas_aplicacao: string[];
  cv_gerado_path: string;
  cv_gerado_em: string;
}

interface CurriculoInfo {
  path: string;
  file_name: string;
  template_id: string;
  template_nome: string;
  gerado_em: string;
}

interface CoverLetterInfo {
  path: string;
  file_name: string;
  empresa: string;
  cargo: string;
  idioma: string;
  gerado_em: string;
}

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  streaming?: boolean;
}

type Mode = "resumo" | "chat" | "curriculos" | "cover_letters";
interface ChatFocus {
  section: string;
  label: string;
  preMessage?: string;
  chromeSessao?: boolean;
}

type EditTarget =
  | { kind: "dados_pessoais" }
  | { kind: "experiencia" }
  | { kind: "projetos" }
  | { kind: "formacao" }
  | { kind: "competencias" }
  | { kind: "idiomas" }
  | { kind: "variante"; id: string }
  | { kind: "nova_variante" };

// ── Logo ───────────────────────────────────────────────────────────────────

const GlassesAvatar: React.FC = () => (
  <div style={{
    width: 30, height: 30, borderRadius: "50%",
    background: "var(--accent-soft)",
    display: "flex", alignItems: "center", justifyContent: "center",
    flexShrink: 0,
  }}>
    <svg width={18} height={10} viewBox="0 0 660 360" fill="none" strokeLinecap="round"
      stroke="var(--accent)" strokeWidth={30}>
      <circle cx="160" cy="195" r="135" />
      <circle cx="500" cy="195" r="135" />
      <path d="M295 180 Q330 130 365 180" />
      <path d="M10 195 L35 192" />
      <path d="M650 195 L625 192" />
    </svg>
  </div>
);

// ── Markdown renderer ──────────────────────────────────────────────────────

function applyInline(text: string): React.ReactNode[] {
  const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g);
  return parts.map((part, i) => {
    if (part.startsWith("**") && part.endsWith("**"))
      return <strong key={i}>{part.slice(2, -2)}</strong>;
    if (part.startsWith("*") && part.endsWith("*"))
      return <em key={i}>{part.slice(1, -1)}</em>;
    if (part.startsWith("`") && part.endsWith("`"))
      return (
        <code key={i} style={{
          fontFamily: "JetBrains Mono, Consolas, monospace",
          fontSize: 12, background: "var(--bg-sunken)",
          padding: "1px 4px", borderRadius: 3,
        }}>
          {part.slice(1, -1)}
        </code>
      );
    return part;
  });
}

function renderMarkdown(text: string): React.ReactNode {
  const lines = text.split("\n");
  const nodes: React.ReactNode[] = [];
  let codeLines: string[] = [];
  let inCode = false;

  lines.forEach((line, i) => {
    if (line.startsWith("```")) {
      if (!inCode) { inCode = true; codeLines = []; return; }
      inCode = false;
      nodes.push(
        <pre key={i} style={{
          background: "var(--bg-sunken)", borderRadius: 6, padding: "8px 10px",
          margin: "6px 0", fontFamily: "JetBrains Mono, Consolas, monospace",
          fontSize: 12, overflow: "auto", whiteSpace: "pre-wrap",
        }}>
          <code>{codeLines.join("\n")}</code>
        </pre>
      );
      return;
    }
    if (inCode) { codeLines.push(line); return; }

    if (line === "") { nodes.push(<div key={i} style={{ height: 6 }} />); return; }

    if (line.match(/^#{1,3} /)) {
      const level = line.match(/^#+/)![0].length;
      const content = line.replace(/^#+\s/, "");
      nodes.push(
        <div key={i} style={{
          fontSize: level === 1 ? 15 : 13,
          fontWeight: 600,
          color: "var(--text-primary)",
          marginTop: 8, marginBottom: 2,
        }}>
          {content}
        </div>
      );
      return;
    }

    if (line.startsWith("- ") || line.startsWith("• ")) {
      nodes.push(
        <div key={i} style={{ display: "flex", gap: 6, marginLeft: 2, lineHeight: "1.5" }}>
          <span style={{ color: "var(--accent)", flexShrink: 0, marginTop: 1 }}>•</span>
          <span>{applyInline(line.slice(2))}</span>
        </div>
      );
      return;
    }

    nodes.push(
      <div key={i} style={{ lineHeight: "1.55" }}>{applyInline(line)}</div>
    );
  });

  return <>{nodes}</>;
}

// ── Profile section components ─────────────────────────────────────────────

const EditBtn: React.FC<{ onClick: () => void }> = ({ onClick }) => {
  const t = useT();
  return (
    <button onClick={onClick} style={{
      display: "flex", alignItems: "center", gap: 4, background: "none", border: "none",
      cursor: "pointer", fontSize: 12, color: "var(--text-tertiary)", fontFamily: "inherit",
      padding: "2px 6px", borderRadius: 4, flexShrink: 0,
    }}
    onMouseEnter={e => (e.currentTarget.style.color = "var(--accent-strong)")}
    onMouseLeave={e => (e.currentTarget.style.color = "var(--text-tertiary)")}
    >
      <Pencil size={11} /> {t.common.edit}
    </button>
  );
};

const SectionBlock: React.FC<{ title: string; onEdit: () => void; children: React.ReactNode }> = ({ title, onEdit, children }) => (
  <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 10, padding: "18px 20px", marginBottom: 12 }}>
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 14 }}>
      <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.07em" }}>
        {title}
      </span>
      <EditBtn onClick={onEdit} />
    </div>
    {children}
  </div>
);

const ProfileHeader: React.FC<{ data: CandidateBase; onEdit: () => void }> = ({ data, onEdit }) => {
  const t = useT();
  const dp = data.dados_pessoais;
  const initials = (dp.nome_completo || "?").split(" ").filter(Boolean).slice(0, 2).map(w => w[0]).join("").toUpperCase();
  const headline = data.experiencia?.[0]
    ? [data.experiencia[0].cargo, data.experiencia[0].empresa].filter(Boolean).join(" · ")
    : null;
  return (
    <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 10, padding: "20px 24px", marginBottom: 12 }}>
      <div style={{ display: "flex", alignItems: "flex-start", gap: 16 }}>
        <div style={{
          width: 52, height: 52, borderRadius: "50%", flexShrink: 0,
          background: "var(--accent-soft)", border: "2px solid var(--accent)",
          display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 18, fontWeight: 700, color: "var(--accent-strong)",
        }}>{initials}</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 18, fontWeight: 600, color: "var(--text-primary)", lineHeight: 1.2 }}>
            {dp.nome_completo || <span style={{ color: "var(--text-tertiary)" }}>{t.profile.nameNotFilled}</span>}
          </div>
          {headline && <div style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 3 }}>{headline}</div>}
          <div style={{ display: "flex", flexWrap: "wrap", gap: "2px 14px", marginTop: 6 }}>
            {dp.localizacao_atual && <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>📍 {dp.localizacao_atual}</span>}
            {dp.email && <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{dp.email}</span>}
            {dp.telefone && <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{dp.telefone}</span>}
          </div>
          {dp.links.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 8 }}>
              {dp.links.map((l, i) => (
                <span key={i} style={{ fontSize: 11, padding: "2px 9px", borderRadius: 4, background: "var(--accent-soft)", color: "var(--accent-strong)", fontWeight: 500 }}>
                  {l.tipo}
                </span>
              ))}
            </div>
          )}
        </div>
        <EditBtn onClick={onEdit} />
      </div>
    </div>
  );
};

const ExperienciaSection: React.FC<{ items: CandidateBase["experiencia"]; onEdit: () => void }> = ({ items, onEdit }) => {
  const t = useT();
  return (
  <SectionBlock title={t.profile.sectionTitles.professionalExperience} onEdit={onEdit}>
    <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
      {items.map((exp, i) => (
        <div key={i} style={{ paddingLeft: 14, borderLeft: "2px solid var(--border)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "flex-start" }}>
            <div>
              <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>{exp.cargo}</div>
              {exp.empresa && <div style={{ fontSize: 13, color: "var(--accent-strong)", marginTop: 1 }}>{exp.empresa}</div>}
            </div>
            {(exp.inicio || exp.fim) && (
              <div style={{ fontSize: 11, color: "var(--text-tertiary)", flexShrink: 0, whiteSpace: "nowrap", marginTop: 2 }}>
                {exp.inicio}{exp.fim ? ` – ${exp.fim}` : ` – ${t.profile.present}`}
              </div>
            )}
          </div>
          {exp.descricao && <div style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 6, lineHeight: 1.6 }}>{exp.descricao}</div>}
          {exp.conquistas?.length > 0 && (
            <div style={{ marginTop: 6, display: "flex", flexDirection: "column", gap: 3 }}>
              {exp.conquistas.map((c, j) => (
                <div key={j} style={{ fontSize: 12, color: "var(--text-secondary)", display: "flex", gap: 7 }}>
                  <span style={{ color: "var(--accent)", flexShrink: 0 }}>•</span>{c}
                </div>
              ))}
            </div>
          )}
          {exp.tecnologias?.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 8 }}>
              {exp.tecnologias.map((t, j) => (
                <span key={j} style={{ fontSize: 11, padding: "2px 7px", borderRadius: 4, background: "var(--bg-sunken)", color: "var(--text-secondary)", border: "1px solid var(--border)" }}>{t}</span>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  </SectionBlock>
  );
};

const ProjetosSection: React.FC<{ items: CandidateBase["projetos"]; onEdit: () => void }> = ({ items, onEdit }) => {
  const t = useT();
  return (
  <SectionBlock title={t.profile.sectionTitles.projects} onEdit={onEdit}>
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
      {items.map((proj, i) => (
        <div key={i} style={{ background: "var(--bg-sunken)", borderRadius: 8, padding: "12px 14px", border: "1px solid var(--border)" }}>
          <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)", marginBottom: 4 }}>{proj.nome}</div>
          {proj.descricao && <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5, marginBottom: 8 }}>{proj.descricao}</div>}
          {proj.tecnologias?.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
              {proj.tecnologias.map((t, j) => (
                <span key={j} style={{ fontSize: 11, padding: "1px 6px", borderRadius: 4, background: "var(--bg-surface)", color: "var(--text-secondary)", border: "1px solid var(--border)" }}>{t}</span>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  </SectionBlock>
  );
};

const FormacaoSection: React.FC<{ items: CandidateBase["formacao"]; onEdit: () => void }> = ({ items, onEdit }) => {
  const t = useT();
  return (
  <SectionBlock title={t.profile.sectionTitles.education} onEdit={onEdit}>
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {items.map((f, i) => (
        <div key={i} style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 8 }}>
          <div>
            <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>{f.curso}</div>
            {f.instituicao && <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{f.instituicao}</div>}
          </div>
          {(f.inicio || f.fim) && (
            <div style={{ fontSize: 11, color: "var(--text-tertiary)", flexShrink: 0, whiteSpace: "nowrap" }}>
              {f.inicio}{f.fim ? ` – ${f.fim}` : ""}
            </div>
          )}
        </div>
      ))}
    </div>
  </SectionBlock>
  );
};

const CompetenciasSection: React.FC<{ items: string[]; onEdit: () => void }> = ({ items, onEdit }) => {
  const t = useT();
  return (
  <SectionBlock title={t.profile.sectionTitles.skills} onEdit={onEdit}>
    <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
      {items.map((skill, i) => (
        <span key={i} style={{
          fontSize: 12, padding: "4px 11px", borderRadius: 16,
          background: "var(--accent-soft)", color: "var(--accent-strong)",
          fontWeight: 500, border: "1px solid var(--accent)",
        }}>{skill}</span>
      ))}
    </div>
  </SectionBlock>
  );
};

const NIVEL_COLOR: Record<string, string> = {
  Nativo: "var(--success)", C2: "var(--success)", C1: "var(--accent)",
  B2: "var(--warning)", B1: "var(--warning)", A2: "var(--text-tertiary)", A1: "var(--text-tertiary)",
};

const IdiomasSection: React.FC<{ items: CandidateBase["idiomas"]; onEdit: () => void }> = ({ items, onEdit }) => {
  const t = useT();
  return (
  <SectionBlock title={t.profile.sectionTitles.languages} onEdit={onEdit}>
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {items.map((lang, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>{lang.idioma}</span>
          <span style={{ fontSize: 11, padding: "2px 9px", borderRadius: 10, background: "var(--bg-sunken)", color: NIVEL_COLOR[lang.nivel] ?? "var(--text-secondary)", fontWeight: 500 }}>
            {lang.nivel}
          </span>
        </div>
      ))}
    </div>
  </SectionBlock>
  );
};

const VariantCard: React.FC<{
  variant: SearchVariant;
  pct: number;
  maxPct: number;
  onEdit: (target: EditTarget) => void;
  onDragBar: (newPct: number) => void;
  onDragEnd: () => void;
  onToggleAtiva: () => void;
}> = ({ variant, pct, maxPct, onEdit, onDragBar, onDragEnd, onToggleAtiva }) => {
  const t = useT();
  const barRef = useRef<HTMLDivElement>(null);
  const onDragBarRef = useRef(onDragBar);
  const onDragEndRef = useRef(onDragEnd);
  useEffect(() => { onDragBarRef.current = onDragBar; }, [onDragBar]);
  useEffect(() => { onDragEndRef.current = onDragEnd; }, [onDragEnd]);

  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const move = (ev: MouseEvent) => {
      if (!barRef.current) return;
      const rect = barRef.current.getBoundingClientRect();
      const newPct = Math.max(5, Math.min(maxPct, ((ev.clientX - rect.left) / rect.width) * 100));
      onDragBarRef.current(newPct);
    };
    const up = () => {
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", up);
      onDragEndRef.current();
    };
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", up);
  };

  const displayPct = Math.round(pct);
  const color = variant.ativa ? "var(--accent)" : "var(--text-tertiary)";

  return (
    <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", display: "flex", alignItems: "center", gap: 14, opacity: variant.ativa ? 1 : 0.55 }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
          <span style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)" }}>{variant.nome_exibicao}</span>
          <button
            onClick={onToggleAtiva}
            title={variant.ativa ? t.profile.clickToDeactivate : t.profile.clickToActivate}
            style={{
              fontSize: 11, padding: "2px 8px", borderRadius: 10, cursor: "pointer",
              border: `1px solid ${variant.ativa ? "var(--accent)" : "var(--border)"}`,
              background: variant.ativa ? "var(--accent-soft)" : "transparent",
              color: variant.ativa ? "var(--accent-strong)" : "var(--text-tertiary)",
              fontFamily: "inherit", fontWeight: 500, flexShrink: 0,
            }}
          >
            {variant.ativa ? t.profile.variantActive : t.profile.variantInactive}
          </button>
          <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: "auto", fontVariantNumeric: "tabular-nums" }}>{displayPct}%</span>
        </div>

        {/* Draggable bar */}
        <div
          ref={barRef}
          onMouseDown={startDrag}
          title={t.profile.dragToAdjustWeight}
          style={{ height: 6, background: "var(--bg-sunken)", borderRadius: 3, cursor: "ew-resize", position: "relative", userSelect: "none" }}
        >
          <div style={{ width: `${pct}%`, height: "100%", background: color, borderRadius: 3 }} />
          <div style={{
            position: "absolute", top: "50%", left: `${pct}%`,
            transform: "translate(-50%, -50%)",
            width: 13, height: 13, borderRadius: "50%",
            background: color, border: "2px solid var(--bg-surface)",
            boxShadow: "0 1px 4px rgba(0,0,0,0.18)", pointerEvents: "none",
          }} />
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 8 }}>
          {variant.regioes_aceitas.slice(0, 3).map((r, i) => (
            <span key={i} style={{ fontSize: 11, padding: "1px 6px", borderRadius: 4, background: "var(--bg-sunken)", color: "var(--text-secondary)" }}>{r}</span>
          ))}
          {variant.modelos_trabalho.slice(0, 2).map((m, i) => (
            <span key={i} style={{ fontSize: 11, padding: "1px 6px", borderRadius: 4, background: "var(--accent-soft)", color: "var(--accent-strong)" }}>{m}</span>
          ))}
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6, flexShrink: 0 }}>
        <button onClick={() => onEdit({ kind: "variante", id: variant.id })} style={{ padding: "5px 12px", borderRadius: 6, cursor: "pointer", background: "none", border: "1px solid var(--border)", fontSize: 12, color: "var(--text-secondary)", fontFamily: "inherit" }}>
          {t.profile.editVariant}
        </button>
        <button disabled title={t.profile.exportCVSoon} style={{ padding: "5px 12px", borderRadius: 6, cursor: "default", background: "var(--bg-sunken)", border: "1px solid var(--border)", fontSize: 12, color: "var(--text-tertiary)", fontFamily: "inherit" }}>
          {t.profile.exportCV}
        </button>
      </div>
    </div>
  );
};

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

const EmptyState: React.FC<{ onStart: (focus: ChatFocus) => void }> = ({ onStart }) => {
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

// ── Proposta type ──────────────────────────────────────────────────────────

interface Proposta {
  id: number;
  vaga_id: number | null;
  titulo_vaga: string | null;
  empresa_vaga: string | null;
  criada_em: string;
  pergunta: string;
  contexto: string | null;
}

// ── Resumo view ────────────────────────────────────────────────────────────

const ResumoView: React.FC<{
  data: CandidateBase | null;
  variants: SearchVariant[];
  onOpenChat: (focus?: ChatFocus) => void;
  onOpenCurriculos: () => void;
  onOpenCoverLetters: () => void;
  onDirectEdit: (target: EditTarget) => void;
  onReloadData: () => void;
}> = ({ data, variants, onOpenChat, onOpenCurriculos, onOpenCoverLetters, onDirectEdit, onReloadData }) => {
  const t = useT();
  const [localPesos, setLocalPesos] = useState<Record<string, number>>({});
  const committedPesosRef = useRef<Record<string, number>>({});

  const [propostas, setPropostas] = useState<Proposta[]>([]);

  const carregarPropostas = useCallback(() => {
    invoke<Proposta[]>("listar_propostas").then(setPropostas).catch(console.error);
  }, []);

  useEffect(() => {
    carregarPropostas();
    const unsubs = [
      listen("nova-proposta", carregarPropostas),
      listen("proposta-resolvida", carregarPropostas),
    ];
    return () => { unsubs.forEach((p) => p.then((f) => f())); };
  }, [carregarPropostas]);

  const handleAplicarProposta = (p: Proposta) => {
    onOpenChat({
      section: "sugestao_perfil",
      label: t.profile.applySuggestion,
      preMessage: p.pergunta + (p.contexto ? `\n\nContexto: ${p.contexto}` : ""),
    });
  };

  const handleIgnorarProposta = async (id: number) => {
    await invoke("ignorar_proposta", { id }).catch(console.error);
    carregarPropostas();
  };

  useEffect(() => {
    const map: Record<string, number> = {};
    variants.forEach(v => { map[v.id] = v.peso; });
    setLocalPesos(map);
    committedPesosRef.current = map;
  }, [variants]);

  const activeVariants = variants.filter(v => v.ativa);
  const totalPeso = activeVariants.reduce((sum, v) => sum + (localPesos[v.id] ?? v.peso), 0);
  const maxPct = activeVariants.length > 1 ? 100 - (activeVariants.length - 1) * 5 : 95;

  const handleDragBar = (variantId: string, newPct: number) => {
    setLocalPesos(prev => {
      const curTotal = activeVariants.reduce((s, v) => s + (prev[v.id] ?? v.peso), 0);
      const newPesoForId = (newPct / 100) * curTotal;
      const others = activeVariants.filter(v => v.id !== variantId);
      const sumOthers = others.reduce((s, v) => s + (prev[v.id] ?? v.peso), 0);
      const remaining = curTotal - newPesoForId;
      const minPeso = (5 / 100) * curTotal;
      const next: Record<string, number> = { ...prev, [variantId]: newPesoForId };
      others.forEach(v => {
        const oldPeso = prev[v.id] ?? v.peso;
        next[v.id] = sumOthers > 0 ? Math.max(minPeso, (oldPeso / sumOthers) * remaining) : remaining / others.length;
      });
      committedPesosRef.current = next;
      return next;
    });
  };

  const handleDragEnd = async () => {
    const pesos: Record<string, number> = {};
    variants.forEach(v => { pesos[v.id] = committedPesosRef.current[v.id] ?? v.peso; });
    await invoke("guardar_pesos_variantes", { pesos }).catch(console.error);
    onReloadData();
  };

  const handleToggleAtiva = async (variantId: string) => {
    const v = variants.find(x => x.id === variantId);
    if (!v) return;
    await invoke("guardar_variante_unica", { variante: { ...v, ativa: !v.ativa } }).catch(console.error);
    onReloadData();
  };

  const atualizadoEm = data?.ultima_atualizacao
    ? new Date(data.ultima_atualizacao).toLocaleDateString("pt-PT", { day: "numeric", month: "long", year: "numeric" })
    : null;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      {/* Header row */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 4 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0, flex: 1 }}>{t.profile.title}</h1>
        <button
          onClick={() => onDirectEdit({ kind: "nova_variante" })}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <Plus size={14} />
          {t.profile.newVariant}
        </button>
        <button
          onClick={onOpenCurriculos}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <FileText size={14} />
          {t.profile.resumes}
        </button>
        <button
          onClick={onOpenCoverLetters}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <MailOpen size={14} />
          {t.profile.coverLetters}
        </button>
        <button
          onClick={() => onOpenChat()}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "var(--accent)",
            border: "none", borderRadius: 8,
            fontSize: 13, color: "#fff", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          {t.profile.updateProfile}
        </button>
      </div>
      {atualizadoEm ? (
        <div style={{ fontSize: 12, color: "var(--text-tertiary)", marginBottom: 20 }}>
          {t.profile.lastUpdated}{atualizadoEm}
        </div>
      ) : (
        <div style={{ marginBottom: 20 }} />
      )}

      {/* Propostas de perfil */}
      {propostas.length > 0 && (
        <div style={{
          background: "var(--accent-soft)",
          border: "1px solid var(--accent)",
          borderRadius: 10,
          padding: "14px 16px",
          marginBottom: 20,
        }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <Lightbulb size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
            <span style={{ fontSize: 13, fontWeight: 600, color: "var(--accent-strong)" }}>
              {propostas.length === 1 ? t.profile.suggestions_one : `${propostas.length}${t.profile.suggestions_many}`}
            </span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {propostas.map((p) => (
              <div key={p.id} style={{
                background: "var(--bg-surface)",
                border: "1px solid var(--border)",
                borderRadius: 8,
                padding: "10px 12px",
              }}>
                {(p.titulo_vaga || p.empresa_vaga) && (
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginBottom: 4 }}>
                    {[p.titulo_vaga, p.empresa_vaga].filter(Boolean).join(" · ")}
                  </div>
                )}
                <div style={{ fontSize: 13, color: "var(--text-primary)", lineHeight: 1.5, marginBottom: 8 }}>
                  {p.pergunta}
                </div>
                <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                  <button
                    onClick={() => handleIgnorarProposta(p.id)}
                    style={{
                      padding: "4px 12px", borderRadius: 6, fontSize: 12,
                      background: "transparent", border: "1px solid var(--border)",
                      color: "var(--text-secondary)", cursor: "pointer", fontFamily: "inherit",
                    }}
                  >
                    {t.profile.ignore}
                  </button>
                  <button
                    onClick={() => handleAplicarProposta(p)}
                    style={{
                      padding: "4px 12px", borderRadius: 6, fontSize: 12, fontWeight: 500,
                      background: "var(--accent)", border: "none",
                      color: "#fff", cursor: "pointer", fontFamily: "inherit",
                    }}
                  >
                    {t.profile.applySuggestion}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* CV preview */}
      {data && (
        <>
          <ProfileHeader
            data={data}
            onEdit={() => onDirectEdit({ kind: "dados_pessoais" })}
          />
          {(data.experiencia?.length ?? 0) > 0 && (
            <ExperienciaSection
              items={data.experiencia}
              onEdit={() => onDirectEdit({ kind: "experiencia" })}
            />
          )}
          {(data.projetos?.length ?? 0) > 0 && (
            <ProjetosSection
              items={data.projetos}
              onEdit={() => onDirectEdit({ kind: "projetos" })}
            />
          )}
          {(data.formacao?.length ?? 0) > 0 && (
            <FormacaoSection
              items={data.formacao}
              onEdit={() => onDirectEdit({ kind: "formacao" })}
            />
          )}
          {(data.idiomas?.length ?? 0) > 0 && (
            <IdiomasSection
              items={data.idiomas}
              onEdit={() => onDirectEdit({ kind: "idiomas" })}
            />
          )}
          {(data.competencias?.length ?? 0) > 0 && (
            <CompetenciasSection
              items={data.competencias}
              onEdit={() => onDirectEdit({ kind: "competencias" })}
            />
          )}
        </>
      )}

      {/* Variants section */}
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
          <span style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)" }}>{t.profile.searchVariants}</span>
          <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
            {variants.filter(v => v.ativa).length} {variants.filter(v => v.ativa).length === 1 ? t.profile.activeCount_one : t.profile.activeCount_many}
          </span>
          <button
            onClick={() => onDirectEdit({ kind: "nova_variante" })}
            style={{
              marginLeft: "auto",
              display: "flex", alignItems: "center", gap: 5,
              padding: "5px 12px", background: "var(--accent)",
              border: "none", borderRadius: 7,
              fontSize: 12, fontWeight: 500, color: "#fff",
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            <Plus size={12} />
            {t.profile.addNewSearch}
          </button>
        </div>

        {variants.length === 0 ? (
          <div style={{
            background: "var(--bg-surface)", border: "1px dashed var(--border)",
            borderRadius: 8, padding: "20px 16px", textAlign: "center",
          }}>
            <div style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 10 }}>
              {t.profile.noVariantsDesc}
            </div>
            <button
              onClick={() => onDirectEdit({ kind: "nova_variante" })}
              style={{
                display: "inline-flex", alignItems: "center", gap: 6,
                padding: "7px 14px", background: "var(--accent-soft)",
                border: "none", borderRadius: 8, fontSize: 13,
                color: "var(--accent-strong)", cursor: "pointer", fontFamily: "inherit",
              }}
            >
              <Plus size={14} />
              {t.profile.createVariant}
            </button>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {variants.map(v => (
              <VariantCard
                key={v.id}
                variant={v}
                pct={totalPeso > 0 ? ((localPesos[v.id] ?? v.peso) / totalPeso) * 100 : 0}
                maxPct={maxPct}
                onEdit={onDirectEdit}
                onDragBar={(newPct) => handleDragBar(v.id, newPct)}
                onDragEnd={handleDragEnd}
                onToggleAtiva={() => handleToggleAtiva(v.id)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

// ── Chat view ──────────────────────────────────────────────────────────────

const ChatView: React.FC<{
  focus: ChatFocus | null;
  onBack: () => void;
  data: CandidateBase | null;
}> = ({ focus, onBack, data }) => {
  const t = useT();
  const isChrome = focus?.chromeSessao === true;
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<string[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Chrome selection state — shown before the session starts
  const [selectionDone, setSelectionDone] = useState(false);
  const [importLinkedin, setImportLinkedin] = useState(false);
  const [importGithub, setImportGithub] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const perfilSavedRef = useRef(false);
  const onBackRef = useRef(onBack);
  useEffect(() => { onBackRef.current = onBack; }, [onBack]);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  useEffect(() => {
    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen<string>("perfil-output", (event) => {
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last && last.role === "assistant" && last.streaming) {
            return [...prev.slice(0, -1), { ...last, content: last.content + event.payload }];
          }
          return [...prev, { id: crypto.randomUUID(), role: "assistant", content: event.payload, streaming: true }];
        });
      }),
      listen("perfil-atualizado", () => {
        perfilSavedRef.current = true;
      }),
      listen("perfil-output-done", () => {
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last?.streaming) return [...prev.slice(0, -1), { ...last, streaming: false }];
          return prev;
        });
        setSending(false);

        if (isChrome && perfilSavedRef.current) {
          setTimeout(() => onBackRef.current(), 2000);
        }
      }),
    ]).then((fns) => {
      if (active) {
        unlisteners.push(...fns);
      } else {
        fns.forEach((f) => f());
      }
    });

    return () => {
      active = false;
      unlisteners.forEach((f) => f());
    };
  }, [isChrome]);

  useEffect(() => {
    if (isChrome) {
      setMessages([{
        id: crypto.randomUUID(),
        role: "assistant",
        content: t.profile.importQuestion,
        streaming: false,
      }]);
      // Don't start the session yet — wait for the user's selection
    } else if (focus?.preMessage) {
      setMessages([{ id: crypto.randomUUID(), role: "user", content: focus.preMessage, streaming: false }]);
      startSession(focus.preMessage);
    } else {
      const greeting = data
        ? t.profile.greetingWithProfile
        : t.profile.greetingNoProfile;
      setMessages([{ id: crypto.randomUUID(), role: "assistant", content: greeting, streaming: false }]);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleImport = async () => {
    const sources: string[] = [];
    if (importLinkedin) sources.push("LinkedIn");
    if (importGithub) sources.push("GitHub");
    if (sources.length === 0) return;

    const primeiraMsg = `Quero importar o meu perfil do ${sources.join(" e ")}. Acede com a minha sessão autenticada e extrai toda a informação profissional.`;
    const userMsg: Message = { id: crypto.randomUUID(), role: "user", content: primeiraMsg, streaming: false };

    setMessages(prev => [...prev, userMsg]);
    setSelectionDone(true);
    setSending(true);
    setError(null);

    try {
      await invoke("iniciar_sessao_perfil_chrome", { primeiraMensagem: primeiraMsg });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setSending(false);
    }
  };

  const startSession = async (firstMessage: string) => {
    setSending(true);
    setError(null);
    try {
      await invoke("iniciar_sessao_perfil", {
        contexto: focus?.section ?? "geral",
        primeiraMessage: firstMessage,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setSending(false);
    }
  };

  const attachFiles = async () => {
    const selected = await openFileDialog({ multiple: true, title: t.profile.attachFiles });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    setAttachments(prev => [...prev, ...paths.filter(p => !prev.includes(p))]);
  };

  const fileName = (path: string) => path.split(/[/\\]/).pop() ?? path;

  const sendMessage = async () => {
    const text = input.trim();
    if ((!text && attachments.length === 0) || sending) return;

    // Bubble shows only the text + file names; the invoke gets the full paths
    // with an instruction so Claude reads them with the Read tool.
    const displayText = attachments.length > 0
      ? `${text}${text ? "\n\n" : ""}📎 ${attachments.map(fileName).join(", ")}`
      : text;
    const promptText = attachments.length > 0
      ? `${text}${text ? "\n\n" : ""}${t.profile.attachedNote}\n${attachments.map(p => `- ${p}`).join("\n")}`
      : text;

    const userMsg: Message = { id: crypto.randomUUID(), role: "user", content: displayText, streaming: false };
    setMessages(prev => [...prev, userMsg]);
    setInput("");
    setAttachments([]);
    if (inputRef.current) inputRef.current.style.height = "auto";
    setSending(true);
    setError(null);

    try {
      if (isChrome) {
        await invoke("escrever_para_perfil_chrome", { input: promptText });
      } else {
        await invoke("enviar_mensagem_perfil", { mensagem: promptText });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  const stopGeneration = async () => {
    try {
      await invoke("interromper_perfil");
    } catch {
      // the done event resets the UI either way
    }
  };

  const lastUserIdx = messages.map(m => m.role).lastIndexOf("user");

  const editLastMessage = async () => {
    if (sending || lastUserIdx === -1) return;
    const msg = messages[lastUserIdx];
    try {
      await invoke("remover_ultima_troca_perfil");
    } catch {
      // history stays as-is; resending will still work, just with extra context
    }
    setMessages(prev => prev.slice(0, lastUserIdx));
    setInput(msg.content);
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (el) {
        el.style.height = "auto";
        el.style.height = Math.min(el.scrollHeight, 160) + "px";
        el.focus();
      }
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Chat header */}
      <div style={{
        height: 48, flexShrink: 0,
        background: "var(--bg-surface)", borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center", padding: "0 16px", gap: 10,
      }}>
        <button
          onClick={onBack}
          style={{
            background: "none", border: "none", cursor: "pointer",
            display: "flex", alignItems: "center", gap: 6,
            color: "var(--text-secondary)", fontSize: 13, fontFamily: "inherit",
            padding: "4px 0",
          }}
        >
          <ArrowLeft size={15} />
          {t.profile.title}
        </button>

        {focus && focus.section !== "geral" && (
          <>
            <span style={{ color: "var(--border)" }}>|</span>
            <span style={{
              fontSize: 12, padding: "2px 8px", borderRadius: 6,
              background: "var(--accent-soft)", color: "var(--accent-strong)", fontWeight: 500,
            }}>
              {focus.label}
            </span>
          </>
        )}

      </div>

      {/* Messages area */}
      <div style={{ flex: 1, overflowY: "auto", padding: "16px" }}>
        {error && (
          <div style={{
            display: "flex", alignItems: "center", gap: 8,
            padding: "10px 14px", background: "#F7E2DF",
            border: "1px solid var(--danger)", borderRadius: 8,
            fontSize: 13, color: "var(--danger)", marginBottom: 12,
          }}>
            <AlertCircle size={14} />
            <span>
              {t.profile.sessionError.replace("{error}", error)}
            </span>
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: 12, maxWidth: 680, margin: "0 auto" }}>
          {messages.map((msg, i) => (
            <div
              key={msg.id}
              style={{
                display: "flex",
                flexDirection: msg.role === "user" ? "row-reverse" : "row",
                alignItems: "flex-start",
                gap: 10,
              }}
            >
              {msg.role === "assistant" && <GlassesAvatar />}

              <div style={{
                maxWidth: "82%",
                padding: "10px 14px",
                borderRadius: msg.role === "user" ? "12px 12px 4px 12px" : "12px 12px 12px 4px",
                background: msg.role === "user" ? "var(--accent-soft)" : "var(--bg-surface)",
                border: msg.role === "user" ? "none" : "1px solid var(--border)",
                fontSize: 13,
                color: msg.role === "user" ? "var(--accent-strong)" : "var(--text-primary)",
                lineHeight: "1.55",
              }}>
                {msg.role === "user"
                  ? <span style={{ whiteSpace: "pre-wrap" }}>{msg.content}</span>
                  : renderMarkdown(msg.content)
                }
                {msg.streaming && (
                  <span style={{
                    display: "inline-block", width: 6, height: 13, marginLeft: 2,
                    background: "var(--accent)", borderRadius: 1, verticalAlign: "middle",
                    animation: "blink 0.8s step-end infinite",
                  }} />
                )}
              </div>

              {msg.role === "user" && i === lastUserIdx && !sending && (
                <button
                  onClick={editLastMessage}
                  title={t.profile.editMessage}
                  style={{
                    background: "none", border: "none", cursor: "pointer",
                    padding: 4, marginTop: 4, flexShrink: 0,
                    display: "flex", alignItems: "center",
                    color: "var(--text-tertiary)", transition: "color 0.15s",
                  }}
                  onMouseEnter={e => e.currentTarget.style.color = "var(--accent)"}
                  onMouseLeave={e => e.currentTarget.style.color = "var(--text-tertiary)"}
                >
                  <Pencil size={13} />
                </button>
              )}
            </div>
          ))}

          {sending && !messages[messages.length - 1]?.streaming && (
            <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
              <GlassesAvatar />
              <div style={{
                padding: "12px 14px",
                borderRadius: "12px 12px 12px 4px",
                background: "var(--bg-surface)",
                border: "1px solid var(--border)",
                display: "flex", alignItems: "center", gap: 4,
              }}>
                {[0, 1, 2].map(n => (
                  <span key={n} style={{
                    width: 6, height: 6, borderRadius: "50%",
                    background: "var(--text-tertiary)",
                    animation: `typingDot 1.2s ease-in-out ${n * 0.18}s infinite`,
                  }} />
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Chrome source selection — shown before session starts */}
        {isChrome && !selectionDone && (
          <div style={{ maxWidth: 680, margin: "0 auto", paddingLeft: 40, paddingBottom: 8 }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
              {(["LinkedIn", "GitHub"] as const).map(src => {
                const active = src === "LinkedIn" ? importLinkedin : importGithub;
                const toggle = src === "LinkedIn"
                  ? () => setImportLinkedin(v => !v)
                  : () => setImportGithub(v => !v);
                return (
                  <button key={src} onClick={toggle} style={{
                    padding: "7px 18px", borderRadius: 20, cursor: "pointer",
                    border: `1px solid ${active ? "var(--accent)" : "var(--border)"}`,
                    background: active ? "var(--accent-soft)" : "var(--bg-surface)",
                    color: active ? "var(--accent-strong)" : "var(--text-secondary)",
                    fontSize: 13, fontWeight: active ? 500 : 400,
                    fontFamily: "inherit", transition: "all 0.15s",
                  }}>
                    {active ? "✓ " : ""}{src}
                  </button>
                );
              })}
            </div>
            <button
              onClick={handleImport}
              disabled={!importLinkedin && !importGithub}
              style={{
                padding: "8px 20px", borderRadius: 8,
                background: (importLinkedin || importGithub) ? "var(--accent)" : "var(--bg-sunken)",
                color: (importLinkedin || importGithub) ? "#fff" : "var(--text-tertiary)",
                border: "none",
                cursor: (importLinkedin || importGithub) ? "pointer" : "default",
                fontSize: 13, fontWeight: 500, fontFamily: "inherit",
                transition: "background 0.15s, color 0.15s",
              }}
            >
              {t.profile.importBtn}
            </button>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input area — hidden while chrome selection is pending */}
      <div style={{
        flexShrink: 0,
        borderTop: "1px solid var(--border)",
        background: "var(--bg-surface)",
        padding: "12px 16px",
        display: isChrome && !selectionDone ? "none" : undefined,
      }}>
        {attachments.length > 0 && (
          <div style={{
            display: "flex", flexWrap: "wrap", gap: 6,
            maxWidth: 680, margin: "0 auto 8px",
          }}>
            {attachments.map(path => (
              <span
                key={path}
                title={path}
                style={{
                  display: "inline-flex", alignItems: "center", gap: 5,
                  padding: "3px 8px", borderRadius: 6,
                  background: "var(--bg-sunken)", border: "1px solid var(--border)",
                  fontSize: 12, color: "var(--text-secondary)", maxWidth: 220,
                }}
              >
                <Paperclip size={11} style={{ flexShrink: 0 }} />
                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {fileName(path)}
                </span>
                <button
                  onClick={() => setAttachments(prev => prev.filter(p => p !== path))}
                  title={t.profile.removeAttachment}
                  style={{
                    background: "none", border: "none", cursor: "pointer", padding: 0,
                    display: "flex", alignItems: "center", color: "var(--text-tertiary)",
                  }}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
          </div>
        )}
        <div style={{
          display: "flex", alignItems: "flex-end", gap: 10,
          maxWidth: 680, margin: "0 auto",
        }}>
          <button
            onClick={attachFiles}
            disabled={sending}
            title={t.profile.attachFiles}
            style={{
              width: 36, height: 36, flexShrink: 0,
              background: "none", border: "none", borderRadius: 8,
              cursor: sending ? "default" : "pointer",
              display: "flex", alignItems: "center", justifyContent: "center",
              color: "var(--text-secondary)", transition: "color 0.15s",
            }}
            onMouseEnter={e => { if (!sending) e.currentTarget.style.color = "var(--accent)"; }}
            onMouseLeave={e => e.currentTarget.style.color = "var(--text-secondary)"}
          >
            <Paperclip size={16} />
          </button>
          <textarea
            ref={inputRef}
            value={input}
            onChange={e => {
              setInput(e.target.value);
              const el = e.target;
              el.style.height = "auto";
              el.style.height = Math.min(el.scrollHeight, 160) + "px";
            }}
            onKeyDown={handleKeyDown}
            placeholder={t.profile.chatPlaceholder}
            disabled={sending}
            rows={1}
            style={{
              flex: 1,
              resize: "none",
              border: "1px solid var(--border)",
              borderRadius: 8,
              padding: "9px 12px",
              fontFamily: "inherit",
              fontSize: 13,
              color: "var(--text-primary)",
              background: sending ? "var(--bg-sunken)" : "var(--bg-base)",
              outline: "none",
              lineHeight: "1.5",
              maxHeight: 160,
              overflow: "auto",
              transition: "border-color 0.15s",
            }}
            onFocus={e => e.target.style.borderColor = "var(--accent)"}
            onBlur={e => e.target.style.borderColor = "var(--border)"}
          />
          {sending ? (
            <button
              onClick={stopGeneration}
              title={t.profile.stopGeneration}
              style={{
                width: 36, height: 36, flexShrink: 0,
                background: "var(--accent)",
                border: "none", borderRadius: 8, cursor: "pointer",
                display: "flex", alignItems: "center", justifyContent: "center",
                transition: "background 0.15s",
              }}
            >
              <Square size={12} fill="#fff" color="#fff" />
            </button>
          ) : (
            <button
              onClick={sendMessage}
              disabled={!input.trim() && attachments.length === 0}
              title="Enviar (Enter)"
              style={{
                width: 36, height: 36, flexShrink: 0,
                background: (!input.trim() && attachments.length === 0) ? "var(--bg-sunken)" : "var(--accent)",
                border: "none", borderRadius: 8, cursor: (!input.trim() && attachments.length === 0) ? "default" : "pointer",
                display: "flex", alignItems: "center", justifyContent: "center",
                transition: "background 0.15s",
              }}
            >
              <Send size={15} color={(!input.trim() && attachments.length === 0) ? "var(--text-tertiary)" : "#fff"} />
            </button>
          )}
        </div>
        <div style={{ fontSize: 11, color: "var(--text-tertiary)", textAlign: "center", marginTop: 6, maxWidth: 680, margin: "6px auto 0" }}>
          {t.profile.enterHint}
        </div>
      </div>

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes blink { 50% { opacity: 0; } }
        @keyframes typingDot {
          0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
          30% { opacity: 1; transform: translateY(-3px); }
        }
      `}</style>
    </div>
  );
};

// ── Curriculos view ────────────────────────────────────────────────────────

const PALETTE = [
  { hex: "#1a1a1a", label: "Preto" },
  { hex: "#D97757", label: "Laranja" },
  { hex: "#2563EB", label: "Azul" },
  { hex: "#16A34A", label: "Verde" },
  { hex: "#7C3AED", label: "Roxo" },
  { hex: "#DC2626", label: "Vermelho" },
  { hex: "#0891B2", label: "Teal" },
  { hex: "#475569", label: "Slate" },
];

type DocLang = "pt" | "en";

const CurriculosView: React.FC<{ onBack: () => void }> = ({ onBack }) => {
  const t = useT();
  const TEMPLATES = [
    { id: "classic-ats", nome: t.profile.templates.classicAts.name, badge: t.profile.templates.classicAts.badge, desc: t.profile.templates.classicAts.desc },
    { id: "hybrid-skills", nome: t.profile.templates.hybridSkills.name, badge: t.profile.templates.hybridSkills.badge, desc: t.profile.templates.hybridSkills.desc },
    { id: "dev-compact", nome: t.profile.templates.devCompact.name, badge: t.profile.templates.devCompact.badge, desc: t.profile.templates.devCompact.desc },
  ];
  const [gerando, setGerando] = useState<string | null>(null);
  const [curriculos, setCurriculos] = useState<CurriculoInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [selectedColor, setSelectedColor] = useState("#D97757");
  const [selectedLang, setSelectedLang] = useState<DocLang>("pt");

  const carregarLista = async () => {
    try {
      const list = await invoke<CurriculoInfo[]>("listar_curriculos");
      setCurriculos(list ?? []);
    } catch (e) {
      console.error("[CurriculosView] listar_curriculos error:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { carregarLista(); }, []);

  const gerar = async (templateId: string) => {
    setErro(null);
    setGerando(templateId);
    try {
      await invoke<CurriculoInfo>("gerar_curriculo", { templateId, corPrimaria: selectedColor, idioma: selectedLang });
      await carregarLista();
    } catch (e) {
      setErro(String(e));
    } finally {
      setGerando(null);
    }
  };

  const abrir = async (path: string) => {
    try {
      await invoke("abrir_curriculo", { path });
    } catch (e) {
      setErro(String(e));
    }
  };

  const apagar = async (path: string) => {
    try {
      await invoke("apagar_curriculo", { path });
      await carregarLista();
    } catch (e) {
      setErro(String(e));
    }
  };

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <button
          onClick={onBack}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 12px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <ArrowLeft size={14} />
          {t.common.back}
        </button>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
          {t.profile.resumesTitle}
        </h1>
      </div>

      {erro && (
        <div style={{
          background: "var(--bg-surface)", border: "1px solid var(--danger)",
          borderRadius: 8, padding: "10px 14px", marginBottom: 16,
          fontSize: 13, color: "var(--danger)",
        }}>
          {erro}
        </div>
      )}

      {/* Language + Color row */}
      <div style={{
        background: "var(--bg-surface)", border: "1px solid var(--border)",
        borderRadius: 10, padding: "12px 16px", marginBottom: 16,
        display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap",
      }}>
        {/* Language toggle */}
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.profile.resumeLanguage}</span>
          {(["pt", "en"] as DocLang[]).map(l => (
            <button
              key={l}
              onClick={() => setSelectedLang(l)}
              style={{
                padding: "3px 10px", borderRadius: 6, fontSize: 12, fontWeight: 600,
                border: selectedLang === l ? `1.5px solid ${selectedColor}` : "1.5px solid var(--border)",
                background: selectedLang === l ? selectedColor : "transparent",
                color: selectedLang === l ? "#fff" : "var(--text-secondary)",
                cursor: "pointer", fontFamily: "inherit", transition: "all 0.12s",
              }}
            >
              {l.toUpperCase()}
            </button>
          ))}
        </div>
        <div style={{ width: 1, height: 24, background: "var(--border)", flexShrink: 0 }} />
        {/* Color palette (existing) */}
      </div>
      <div style={{
        background: "var(--bg-surface)", border: "1px solid var(--border)",
        borderRadius: 10, padding: "12px 16px", marginBottom: 16,
        display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap",
      }}>
        <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500, flexShrink: 0 }}>
          {t.profile.accentColor}
        </span>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {PALETTE.map(p => (
            <button
              key={p.hex}
              title={p.label}
              onClick={() => setSelectedColor(p.hex)}
              style={{
                width: 26, height: 26, borderRadius: "50%",
                background: p.hex,
                border: selectedColor === p.hex ? "3px solid var(--text-primary)" : "2px solid transparent",
                outline: selectedColor === p.hex ? "2px solid var(--bg-surface)" : "none",
                outlineOffset: -5,
                cursor: "pointer", flexShrink: 0, padding: 0,
                boxSizing: "border-box",
                transition: "border 0.12s",
              }}
            />
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <input
            type="color"
            value={selectedColor}
            onChange={e => setSelectedColor(e.target.value)}
            style={{ width: 26, height: 26, borderRadius: "50%", border: "none", padding: 0, cursor: "pointer", background: "none" }}
            title={t.profile.customColor}
          />
          <span style={{ fontSize: 11, color: "var(--text-tertiary)", fontFamily: "monospace" }}>{selectedColor}</span>
        </div>
      </div>

      {/* Template cards */}
      <div style={{ display: "flex", gap: 12, marginBottom: 28, flexWrap: "wrap" }}>
        {TEMPLATES.map(tmpl => (
          <div key={tmpl.id} style={{
            flex: "1 1 200px", background: "var(--bg-surface)",
            border: "1px solid var(--border)", borderRadius: 10, padding: 16,
            display: "flex", flexDirection: "column", gap: 8,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>{tmpl.nome}</span>
              <span style={{
                fontSize: 10, padding: "2px 7px", borderRadius: 10,
                background: "var(--accent-soft)", color: "var(--accent-strong)", fontWeight: 500,
              }}>{tmpl.badge}</span>
            </div>
            <p style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.45, flex: 1 }}>{tmpl.desc}</p>
            <button
              onClick={() => gerar(tmpl.id)}
              disabled={gerando !== null}
              style={{
                display: "flex", alignItems: "center", justifyContent: "center", gap: 6,
                padding: "7px 14px", background: gerando === tmpl.id ? "var(--bg-sunken)" : selectedColor,
                border: "none", borderRadius: 8, fontSize: 13,
                color: gerando === tmpl.id ? "var(--text-secondary)" : "#fff",
                cursor: gerando !== null ? "not-allowed" : "pointer", fontFamily: "inherit",
                transition: "background 0.15s",
              }}
            >
              {gerando === tmpl.id ? (
                <>
                  <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
                  {t.common.generating}
                </>
              ) : (
                <>
                  <FileText size={13} />
                  {t.common.generate}
                </>
              )}
            </button>
          </div>
        ))}
      </div>

      {/* Generated CVs list */}
      <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 10 }}>
        {t.profile.generatedResumes}
      </div>
      {loading ? (
        <div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>{t.common.loading}</div>
      ) : curriculos.length === 0 ? (
        <div style={{
          background: "var(--bg-surface)", border: "1px dashed var(--border)",
          borderRadius: 8, padding: "20px 16px", textAlign: "center",
          fontSize: 13, color: "var(--text-secondary)",
        }}>
          {t.profile.noResumesYet}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {curriculos.map(cv => (
            <div key={cv.path} style={{
              background: "var(--bg-surface)", border: "1px solid var(--border)",
              borderRadius: 8, padding: "10px 14px",
              display: "flex", alignItems: "center", gap: 12,
            }}>
              <FileText size={16} color="var(--text-tertiary)" style={{ flexShrink: 0 }} />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: 2 }}>
                  {cv.template_nome}
                </div>
                <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                  {cv.gerado_em.replace("_", " ")} &middot; {cv.file_name}
                </div>
              </div>
              <button
                onClick={() => abrir(cv.path)}
                style={{
                  padding: "5px 12px", background: "transparent",
                  border: "1px solid var(--border)", borderRadius: 6,
                  fontSize: 12, color: "var(--text-secondary)", cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                {t.common.open}
              </button>
              <button
                onClick={() => apagar(cv.path)}
                style={{
                  padding: "5px 10px", background: "transparent",
                  border: "1px solid var(--danger)", borderRadius: 6,
                  fontSize: 12, color: "var(--danger)", cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}

      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
};

// ── Cover Letters view ─────────────────────────────────────────────────────

const CoverLettersView: React.FC<{ onBack: () => void }> = ({ onBack }) => {
  const t = useT();
  const [empresa, setEmpresa] = useState("");
  const [cargo, setCargo] = useState("");
  const [descricaoVaga, setDescricaoVaga] = useState("");
  const [notaExtra, setNotaExtra] = useState("");
  const [selectedColor, setSelectedColor] = useState("#D97757");
  const [selectedLang, setSelectedLang] = useState<DocLang>("pt");
  const [gerando, setGerando] = useState(false);
  const [streamText, setStreamText] = useState("");
  const [erro, setErro] = useState<string | null>(null);
  const [coverLetters, setCoverLetters] = useState<CoverLetterInfo[]>([]);
  const [loadingList, setLoadingList] = useState(true);
  const [lastGenerated, setLastGenerated] = useState<CoverLetterInfo | null>(null);
  const streamEndRef = useRef<HTMLDivElement>(null);

  const carregarLista = useCallback(async () => {
    try {
      const list = await invoke<CoverLetterInfo[]>("listar_cover_letters");
      setCoverLetters(list ?? []);
    } catch (e) {
      console.error("[CoverLettersView] listar_cover_letters error:", e);
    } finally {
      setLoadingList(false);
    }
  }, []);

  useEffect(() => { carregarLista(); }, [carregarLista]);

  useEffect(() => {
    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen<string>("cover-letter-stream", (e) => {
        setStreamText(prev => prev + e.payload);
        streamEndRef.current?.scrollIntoView({ behavior: "smooth" });
      }),
      listen<CoverLetterInfo>("cover-letter-done", (e) => {
        setGerando(false);
        setLastGenerated(e.payload);
        carregarLista();
      }),
      listen<string>("cover-letter-error", (e) => {
        setErro(e.payload);
        setGerando(false);
      }),
    ]).then((fns) => {
      if (active) {
        unlisteners.push(...fns);
      } else {
        fns.forEach((f) => f());
      }
    });

    return () => {
      active = false;
      unlisteners.forEach((f) => f());
    };
  }, [carregarLista]);

  const gerar = async () => {
    const emp = empresa.trim();
    const car = cargo.trim();
    if (!emp || !car) return;
    setErro(null);
    setStreamText("");
    setLastGenerated(null);
    setGerando(true);
    try {
      await invoke("gerar_cover_letter", {
        empresa: emp,
        cargo: car,
        descricaoVaga: descricaoVaga.trim() || null,
        notaExtra: notaExtra.trim() || null,
        idioma: selectedLang,
        corPrimaria: selectedColor,
      });
    } catch (e) {
      setErro(String(e));
      setGerando(false);
    }
  };

  const abrir = async (path: string) => {
    try { await invoke("abrir_cover_letter", { path }); } catch (e) { setErro(String(e)); }
  };

  const apagar = async (path: string) => {
    try { await invoke("apagar_cover_letter", { path }); await carregarLista(); } catch (e) { setErro(String(e)); }
  };

  const canGerar = empresa.trim().length > 0 && cargo.trim().length > 0;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <button
          onClick={onBack}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 12px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <ArrowLeft size={14} />
          {t.common.back}
        </button>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
          {t.profile.coverLettersTitle}
        </h1>
      </div>

      {erro && (
        <div style={{
          background: "var(--bg-surface)", border: "1px solid var(--danger)",
          borderRadius: 8, padding: "10px 14px", marginBottom: 16,
          fontSize: 13, color: "var(--danger)", display: "flex", alignItems: "center", gap: 8,
        }}>
          <AlertCircle size={14} />
          {erro}
        </div>
      )}

      {/* Form */}
      <div style={{
        background: "var(--bg-surface)", border: "1px solid var(--border)",
        borderRadius: 10, padding: "18px 20px", marginBottom: 12,
      }}>
        <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.07em", marginBottom: 14 }}>
          {t.profile.newCoverLetter}
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
          <div>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 5, fontWeight: 500 }}>
              {t.profile.company} <span style={{ color: "var(--danger)" }}>*</span>
            </label>
            <input
              value={empresa}
              onChange={e => setEmpresa(e.target.value)}
              placeholder="ex: Novo Nordisk"
              disabled={gerando}
              style={{
                width: "100%", padding: "8px 11px", borderRadius: 7,
                border: "1px solid var(--border)", background: "var(--bg-base)",
                fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit",
                outline: "none",
              }}
              onFocus={e => e.target.style.borderColor = "var(--accent)"}
              onBlur={e => e.target.style.borderColor = "var(--border)"}
            />
          </div>
          <div>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 5, fontWeight: 500 }}>
              {t.profile.position} <span style={{ color: "var(--danger)" }}>*</span>
            </label>
            <input
              value={cargo}
              onChange={e => setCargo(e.target.value)}
              placeholder="ex: Senior Software Engineer"
              disabled={gerando}
              style={{
                width: "100%", padding: "8px 11px", borderRadius: 7,
                border: "1px solid var(--border)", background: "var(--bg-base)",
                fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit",
                outline: "none",
              }}
              onFocus={e => e.target.style.borderColor = "var(--accent)"}
              onBlur={e => e.target.style.borderColor = "var(--border)"}
            />
          </div>
        </div>
        <div style={{ marginBottom: 12 }}>
          <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 5, fontWeight: 500 }}>
            {t.profile.jobDescriptionLabel}{" "}
            <span style={{ color: "var(--text-tertiary)", fontWeight: 400 }}>{t.profile.jobDescriptionOpt}</span>
          </label>
          <textarea
            value={descricaoVaga}
            onChange={e => setDescricaoVaga(e.target.value)}
            placeholder={t.profile.pasteJobText}
            disabled={gerando}
            rows={5}
            style={{
              width: "100%", padding: "8px 11px", borderRadius: 7,
              border: "1px solid var(--border)", background: "var(--bg-base)",
              fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit",
              outline: "none", resize: "vertical",
            }}
            onFocus={e => e.target.style.borderColor = "var(--accent)"}
            onBlur={e => e.target.style.borderColor = "var(--border)"}
          />
        </div>
        <div>
          <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 5, fontWeight: 500 }}>
            {t.profile.extraNoteLabel}{" "}
            <span style={{ color: "var(--text-tertiary)", fontWeight: 400 }}>{t.profile.extraNoteOpt}</span>
          </label>
          <textarea
            value={notaExtra}
            onChange={e => setNotaExtra(e.target.value)}
            placeholder="e.g.: I met the CEO at conference X, I want to mention project Y..."
            disabled={gerando}
            rows={2}
            style={{
              width: "100%", padding: "8px 11px", borderRadius: 7,
              border: "1px solid var(--border)", background: "var(--bg-base)",
              fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit",
              outline: "none", resize: "vertical",
            }}
            onFocus={e => e.target.style.borderColor = "var(--accent)"}
            onBlur={e => e.target.style.borderColor = "var(--border)"}
          />
        </div>
      </div>

      {/* Language + Color row */}
      <div style={{
        background: "var(--bg-surface)", border: "1px solid var(--border)",
        borderRadius: 10, padding: "12px 16px", marginBottom: 14,
        display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.profile.resumeLanguage}</span>
          {(["pt", "en"] as DocLang[]).map(l => (
            <button
              key={l}
              onClick={() => setSelectedLang(l)}
              style={{
                padding: "3px 10px", borderRadius: 6, fontSize: 12, fontWeight: 600,
                border: selectedLang === l ? `1.5px solid ${selectedColor}` : "1.5px solid var(--border)",
                background: selectedLang === l ? selectedColor : "transparent",
                color: selectedLang === l ? "#fff" : "var(--text-secondary)",
                cursor: "pointer", fontFamily: "inherit", transition: "all 0.12s",
              }}
            >
              {l.toUpperCase()}
            </button>
          ))}
        </div>
        <div style={{ width: 1, height: 24, background: "var(--border)", flexShrink: 0 }} />
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.profile.accentColor}</span>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {PALETTE.map(p => (
              <button
                key={p.hex}
                title={p.label}
                onClick={() => setSelectedColor(p.hex)}
                style={{
                  width: 22, height: 22, borderRadius: "50%",
                  background: p.hex,
                  border: selectedColor === p.hex ? "3px solid var(--text-primary)" : "2px solid transparent",
                  outline: selectedColor === p.hex ? "2px solid var(--bg-surface)" : "none",
                  outlineOffset: -5,
                  cursor: "pointer", flexShrink: 0, padding: 0,
                  boxSizing: "border-box",
                  transition: "border 0.12s",
                }}
              />
            ))}
          </div>
          <input
            type="color"
            value={selectedColor}
            onChange={e => setSelectedColor(e.target.value)}
            style={{ width: 22, height: 22, borderRadius: "50%", border: "none", padding: 0, cursor: "pointer", background: "none" }}
            title={t.profile.customColor}
          />
        </div>
      </div>

      {/* Generate button */}
      <button
        onClick={gerar}
        disabled={!canGerar || gerando}
        style={{
          width: "100%", padding: "11px 20px", marginBottom: 20,
          background: canGerar && !gerando ? selectedColor : "var(--bg-sunken)",
          border: "none", borderRadius: 8, fontSize: 14, fontWeight: 500,
          color: canGerar && !gerando ? "#fff" : "var(--text-tertiary)",
          cursor: canGerar && !gerando ? "pointer" : "not-allowed",
          fontFamily: "inherit", display: "flex", alignItems: "center", justifyContent: "center", gap: 8,
          transition: "background 0.15s, color 0.15s",
        }}
      >
        {gerando ? (
          <>
            <Loader2 size={15} style={{ animation: "spin 1s linear infinite" }} />
            {t.common.generating}
          </>
        ) : (
          <>
            <MailOpen size={15} />
            {t.profile.generateCoverLetter}
          </>
        )}
      </button>

      {/* Streaming preview */}
      {(gerando || streamText) && (
        <div style={{
          background: "var(--bg-surface)", border: "1px solid var(--border)",
          borderRadius: 10, padding: "16px 20px", marginBottom: 20,
        }}>
          <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.07em", marginBottom: 12, display: "flex", alignItems: "center", gap: 8 }}>
            {gerando ? (
              <>
                <Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} />
                {t.profile.claudeWriting}
              </>
            ) : t.profile.draftGenerated}
          </div>
          <div style={{
            fontSize: 13, color: "var(--text-primary)", lineHeight: 1.75,
            whiteSpace: "pre-wrap", maxHeight: 300, overflowY: "auto",
            fontFamily: "Calibri, Arial, sans-serif",
          }}>
            {streamText}
            {gerando && (
              <span style={{
                display: "inline-block", width: 6, height: 13, marginLeft: 2,
                background: "var(--accent)", borderRadius: 1, verticalAlign: "middle",
                animation: "blink 0.8s step-end infinite",
              }} />
            )}
            <div ref={streamEndRef} />
          </div>
          {lastGenerated && (
            <div style={{ marginTop: 14, display: "flex", gap: 10, alignItems: "center", paddingTop: 12, borderTop: "1px solid var(--border)" }}>
              <div style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--success)", flexShrink: 0 }} />
              <span style={{ fontSize: 13, color: "var(--text-secondary)", flex: 1 }}>
                Guardada — {lastGenerated.file_name}
              </span>
              <button
                onClick={() => abrir(lastGenerated.path)}
                style={{
                  padding: "6px 14px", background: selectedColor, border: "none",
                  borderRadius: 7, fontSize: 13, color: "#fff", cursor: "pointer",
                  fontFamily: "inherit", fontWeight: 500,
                }}
              >
                Abrir
              </button>
            </div>
          )}
        </div>
      )}

      {/* List */}
      <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 10 }}>
        Cover letters geradas
      </div>
      {loadingList ? (
        <div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>A carregar…</div>
      ) : coverLetters.length === 0 ? (
        <div style={{
          background: "var(--bg-surface)", border: "1px dashed var(--border)",
          borderRadius: 8, padding: "20px 16px", textAlign: "center",
          fontSize: 13, color: "var(--text-secondary)",
        }}>
          Ainda não há cover letters geradas.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {coverLetters.map(cl => (
            <div key={cl.path} style={{
              background: "var(--bg-surface)", border: "1px solid var(--border)",
              borderRadius: 8, padding: "10px 14px",
              display: "flex", alignItems: "center", gap: 12,
            }}>
              <MailOpen size={15} color="var(--text-tertiary)" style={{ flexShrink: 0 }} />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: 2 }}>
                  {cl.empresa || cl.file_name}
                  {cl.cargo && <span style={{ color: "var(--text-secondary)", fontWeight: 400 }}> — {cl.cargo}</span>}
                </div>
                <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>
                  {cl.gerado_em}{cl.idioma ? ` · ${cl.idioma.toUpperCase()}` : ""} · {cl.file_name}
                </div>
              </div>
              <button
                onClick={() => abrir(cl.path)}
                style={{
                  padding: "5px 12px", background: "transparent",
                  border: "1px solid var(--border)", borderRadius: 6,
                  fontSize: 12, color: "var(--text-secondary)", cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                Abrir
              </button>
              <button
                onClick={() => apagar(cl.path)}
                style={{
                  padding: "5px 10px", background: "transparent",
                  border: "1px solid var(--danger)", borderRadius: 6,
                  fontSize: 12, color: "var(--danger)", cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes blink { 50% { opacity: 0; } }
      `}</style>
    </div>
  );
};

// ── Section Edit Modal ─────────────────────────────────────────────────────

const edInput: React.CSSProperties = {
  width: "100%", padding: "7px 10px", borderRadius: 6,
  border: "1px solid var(--border)", background: "var(--bg-base)",
  fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit",
  outline: "none", boxSizing: "border-box",
};
const edTextarea: React.CSSProperties = { ...edInput, resize: "vertical" as const, lineHeight: 1.5 };

const tagsToStr = (arr: string[]) => arr.join(", ");
const strToTags = (s: string) => s.split(",").map(t => t.trim()).filter(Boolean);

const EdField: React.FC<{ label: string; hint?: string; children: React.ReactNode }> = ({ label, hint, children }) => (
  <div style={{ marginBottom: 14 }}>
    <label style={{ display: "block", fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.06em", marginBottom: 5 }}>
      {label}
    </label>
    {children}
    {hint && <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 3 }}>{hint}</div>}
  </div>
);

const ModalActions: React.FC<{ saving: boolean; onSave: () => void; onClose: () => void }> = ({ saving, onSave, onClose }) => {
  const t = useT();
  return (
    <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, paddingTop: 8, borderTop: "1px solid var(--border)", marginTop: 8 }}>
      <button onClick={onClose} style={{ padding: "7px 14px", background: "transparent", border: "none", borderRadius: 7, fontSize: 13, color: "var(--text-secondary)", cursor: "pointer", fontFamily: "inherit" }}>
        {t.profile.editor.cancel}
      </button>
      <button onClick={onSave} disabled={saving} style={{ padding: "7px 18px", background: saving ? "var(--bg-sunken)" : "var(--accent)", border: "none", borderRadius: 7, fontSize: 13, fontWeight: 500, color: saving ? "var(--text-tertiary)" : "#fff", cursor: saving ? "default" : "pointer", fontFamily: "inherit" }}>
        {saving ? t.profile.editor.saving : t.profile.editor.save}
      </button>
    </div>
  );
};

const DadosPessoaisEditor: React.FC<{ pd: CandidateBase; onSaved: (u: CandidateBase) => void; onClose: () => void }> = ({ pd, onSaved, onClose }) => {
  const t = useT();
  const [draft, setDraft] = useState({
    ...pd.dados_pessoais,
    endereco: pd.dados_pessoais.endereco ?? "",
    nacionalidade: pd.dados_pessoais.nacionalidade ?? "",
    data_nascimento: pd.dados_pessoais.data_nascimento ?? "",
    cpf: pd.dados_pessoais.cpf ?? "",
    links: pd.dados_pessoais.links.map(l => ({ ...l })),
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    setSaving(true); setError(null);
    try {
      const updated = { ...pd, dados_pessoais: draft };
      await invoke("guardar_candidato_base", { dados: updated });
      onSaved(updated);
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  const updLink = (i: number, field: "tipo" | "url", v: string) =>
    setDraft(d => ({ ...d, links: d.links.map((l, j) => j === i ? { ...l, [field]: v } : l) }));

  const sectionLabel: React.CSSProperties = {
    fontSize: 10, fontWeight: 700, color: "var(--text-tertiary)",
    textTransform: "uppercase", letterSpacing: "0.08em",
    marginBottom: 10, marginTop: 18, paddingBottom: 6,
    borderBottom: "1px solid var(--border)",
  };

  return (
    <>
      <div style={sectionLabel}>{t.profile.editor.identification}</div>
      <EdField label={t.profile.editor.fullName}>
        <input style={edInput} value={draft.nome_completo} onChange={e => setDraft(d => ({ ...d, nome_completo: e.target.value }))} />
      </EdField>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
        <EdField label={t.profile.editor.nationality}>
          <input style={edInput} value={draft.nacionalidade} onChange={e => setDraft(d => ({ ...d, nacionalidade: e.target.value }))} />
        </EdField>
        <EdField label={t.profile.editor.dateOfBirth} hint={t.profile.editor.dateOfBirthHint}>
          <input style={edInput} placeholder="YYYY-MM-DD" value={draft.data_nascimento} onChange={e => setDraft(d => ({ ...d, data_nascimento: e.target.value }))} />
        </EdField>
      </div>
      <EdField label={t.profile.editor.cpf} hint={t.profile.editor.cpfHint}>
        <input style={edInput} placeholder="000.000.000-00" value={draft.cpf} onChange={e => setDraft(d => ({ ...d, cpf: e.target.value }))} />
      </EdField>

      <div style={sectionLabel}>{t.profile.editor.contact}</div>
      <EdField label={t.profile.editor.email}>
        <input style={edInput} type="email" value={draft.email} onChange={e => setDraft(d => ({ ...d, email: e.target.value }))} />
      </EdField>
      <EdField label={t.profile.editor.phone} hint={t.profile.editor.phoneHint}>
        <input style={edInput} placeholder="+55 11 91234-5678" value={draft.telefone} onChange={e => setDraft(d => ({ ...d, telefone: e.target.value }))} />
      </EdField>

      <div style={sectionLabel}>{t.profile.editor.location}</div>
      <EdField label={t.profile.editor.cityCountry} hint={t.profile.editor.cityCountryHint}>
        <input style={edInput} placeholder="Copenhagen, Denmark" value={draft.localizacao_atual} onChange={e => setDraft(d => ({ ...d, localizacao_atual: e.target.value }))} />
      </EdField>
      <EdField label={t.profile.editor.fullAddress} hint={t.profile.editor.fullAddressHint}>
        <textarea style={{ ...edTextarea }} rows={2} value={draft.endereco} onChange={e => setDraft(d => ({ ...d, endereco: e.target.value }))} />
      </EdField>

      <div style={sectionLabel}>{t.profile.editor.professionalLinks}</div>
      {draft.links.map((l, i) => (
        <div key={i} style={{ display: "grid", gridTemplateColumns: "130px 1fr 28px", gap: 6, marginBottom: 6 }}>
          <input style={edInput} placeholder="LinkedIn" value={l.tipo} onChange={e => updLink(i, "tipo", e.target.value)} />
          <input style={edInput} placeholder="https://..." value={l.url} onChange={e => updLink(i, "url", e.target.value)} />
          <button onClick={() => setDraft(d => ({ ...d, links: d.links.filter((_, j) => j !== i) }))} style={{ background: "none", border: "1px solid var(--border)", borderRadius: 5, cursor: "pointer", color: "var(--text-tertiary)" }}>×</button>
        </div>
      ))}
      <button onClick={() => setDraft(d => ({ ...d, links: [...d.links, { tipo: "", url: "" }] }))} style={{ fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit", marginBottom: 4 }}>
        {t.profile.editor.addLink}
      </button>

      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginTop: 10, marginBottom: 4 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

const ExperienciaEditor: React.FC<{ pd: CandidateBase; onSaved: (u: CandidateBase) => void; onClose: () => void }> = ({ pd, onSaved, onClose }) => {
  const t = useT();
  const [items, setItems] = useState(pd.experiencia.map(e => ({ ...e, conquistas: [...e.conquistas], tecnologias: [...e.tecnologias] })));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const upd = (i: number, field: string, v: string | string[]) =>
    setItems(arr => arr.map((x, j) => j === i ? { ...x, [field]: v } : x));
  const save = async () => {
    setSaving(true); setError(null);
    try {
      const updated = { ...pd, experiencia: items };
      await invoke("guardar_candidato_base", { dados: updated });
      onSaved(updated);
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  return (
    <>
      {items.map((exp, i) => (
        <div key={i} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 12, position: "relative" }}>
          <button onClick={() => setItems(arr => arr.filter((_, j) => j !== i))} style={{ position: "absolute", top: 8, right: 10, background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", fontSize: 18, lineHeight: 1 }}>×</button>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
            <EdField label={t.profile.editor.position}><input style={edInput} value={exp.cargo} onChange={e => upd(i, "cargo", e.target.value)} /></EdField>
            <EdField label={t.profile.editor.company}><input style={edInput} value={exp.empresa} onChange={e => upd(i, "empresa", e.target.value)} /></EdField>
            <EdField label={t.profile.editor.start}><input style={edInput} placeholder="2020-01" value={exp.inicio} onChange={e => upd(i, "inicio", e.target.value)} /></EdField>
            <EdField label={t.profile.editor.end}><input style={edInput} placeholder={t.profile.present} value={exp.fim} onChange={e => upd(i, "fim", e.target.value)} /></EdField>
          </div>
          <EdField label={t.profile.editor.description}><textarea style={edTextarea} rows={3} value={exp.descricao} onChange={e => upd(i, "descricao", e.target.value)} /></EdField>
          <EdField label={t.profile.editor.achievements} hint={t.profile.editor.achievementsHint}><textarea style={edTextarea} rows={3} value={exp.conquistas.join("\n")} onChange={e => upd(i, "conquistas", e.target.value.split("\n"))} /></EdField>
          <EdField label={t.profile.editor.technologies} hint={t.profile.editor.technologiesHint}><input style={edInput} value={tagsToStr(exp.tecnologias)} onChange={e => upd(i, "tecnologias", strToTags(e.target.value))} /></EdField>
        </div>
      ))}
      <button onClick={() => setItems(arr => [...arr, { empresa: "", cargo: "", inicio: "", fim: "", descricao: "", conquistas: [], tecnologias: [] }])} style={{ fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit", marginBottom: 16 }}>{t.profile.editor.addExperience}</button>
      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 10 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

const ProjetosEditor: React.FC<{ pd: CandidateBase; onSaved: (u: CandidateBase) => void; onClose: () => void }> = ({ pd, onSaved, onClose }) => {
  const t = useT();
  const [items, setItems] = useState(pd.projetos.map(p => ({ ...p, tecnologias: [...p.tecnologias] })));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const upd = (i: number, field: string, v: string | string[]) =>
    setItems(arr => arr.map((x, j) => j === i ? { ...x, [field]: v } : x));
  const save = async () => {
    setSaving(true); setError(null);
    try {
      const updated = { ...pd, projetos: items };
      await invoke("guardar_candidato_base", { dados: updated });
      onSaved(updated);
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  return (
    <>
      {items.map((proj, i) => (
        <div key={i} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 12, position: "relative" }}>
          <button onClick={() => setItems(arr => arr.filter((_, j) => j !== i))} style={{ position: "absolute", top: 8, right: 10, background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", fontSize: 18, lineHeight: 1 }}>×</button>
          <EdField label={t.profile.editor.projectName}><input style={edInput} value={proj.nome} onChange={e => upd(i, "nome", e.target.value)} /></EdField>
          <EdField label={t.profile.editor.description}><textarea style={edTextarea} rows={3} value={proj.descricao} onChange={e => upd(i, "descricao", e.target.value)} /></EdField>
          <EdField label={t.profile.editor.projectUrl}><input style={edInput} placeholder="https://github.com/..." value={proj.url} onChange={e => upd(i, "url", e.target.value)} /></EdField>
          <EdField label={t.profile.editor.technologies} hint={t.profile.editor.technologiesHint}><input style={edInput} value={tagsToStr(proj.tecnologias)} onChange={e => upd(i, "tecnologias", strToTags(e.target.value))} /></EdField>
        </div>
      ))}
      <button onClick={() => setItems(arr => [...arr, { nome: "", descricao: "", tecnologias: [], url: "", origem: "" }])} style={{ fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit", marginBottom: 16 }}>{t.profile.editor.addProject}</button>
      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 10 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

const FormacaoEditor: React.FC<{ pd: CandidateBase; onSaved: (u: CandidateBase) => void; onClose: () => void }> = ({ pd, onSaved, onClose }) => {
  const t = useT();
  const [items, setItems] = useState(pd.formacao.map(f => ({ ...f })));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const upd = (i: number, field: string, v: string) =>
    setItems(arr => arr.map((x, j) => j === i ? { ...x, [field]: v } : x));
  const save = async () => {
    setSaving(true); setError(null);
    try {
      const updated = { ...pd, formacao: items };
      await invoke("guardar_candidato_base", { dados: updated });
      onSaved(updated);
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  return (
    <>
      {items.map((f, i) => (
        <div key={i} style={{ border: "1px solid var(--border)", borderRadius: 8, padding: "14px 16px", marginBottom: 12, position: "relative" }}>
          <button onClick={() => setItems(arr => arr.filter((_, j) => j !== i))} style={{ position: "absolute", top: 8, right: 10, background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", fontSize: 18, lineHeight: 1 }}>×</button>
          <EdField label={t.profile.editor.course}><input style={edInput} value={f.curso} onChange={e => upd(i, "curso", e.target.value)} /></EdField>
          <EdField label={t.profile.editor.institution}><input style={edInput} value={f.instituicao} onChange={e => upd(i, "instituicao", e.target.value)} /></EdField>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
            <EdField label={t.profile.editor.start}><input style={edInput} placeholder="2018-09" value={f.inicio} onChange={e => upd(i, "inicio", e.target.value)} /></EdField>
            <EdField label={t.profile.editor.end}><input style={edInput} placeholder="2021-06" value={f.fim} onChange={e => upd(i, "fim", e.target.value)} /></EdField>
          </div>
        </div>
      ))}
      <button onClick={() => setItems(arr => [...arr, { curso: "", instituicao: "", inicio: "", fim: "" }])} style={{ fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit", marginBottom: 16 }}>{t.profile.editor.addEducation}</button>
      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 10 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

const CompetenciasEditor: React.FC<{ pd: CandidateBase; onSaved: (u: CandidateBase) => void; onClose: () => void }> = ({ pd, onSaved, onClose }) => {
  const t = useT();
  const [text, setText] = useState(pd.competencias.join("\n"));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    setSaving(true); setError(null);
    try {
      const competencias = text.split("\n").map(s => s.trim()).filter(Boolean);
      const updated = { ...pd, competencias };
      await invoke("guardar_candidato_base", { dados: updated });
      onSaved(updated);
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  return (
    <>
      <EdField label={t.profile.editor.skills} hint={t.profile.editor.skillsHint}>
        <textarea style={edTextarea} rows={10} value={text} onChange={e => setText(e.target.value)} />
      </EdField>
      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 10 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

const NIVEIS_IDIOMA = ["Native", "C2", "C1", "B2", "B1", "A2", "A1"];

const IdiomasEditor: React.FC<{ pd: CandidateBase; onSaved: (u: CandidateBase) => void; onClose: () => void }> = ({ pd, onSaved, onClose }) => {
  const t = useT();
  const [items, setItems] = useState(pd.idiomas.map(l => ({ ...l })));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const upd = (i: number, field: string, v: string) =>
    setItems(arr => arr.map((x, j) => j === i ? { ...x, [field]: v } : x));
  const save = async () => {
    setSaving(true); setError(null);
    try {
      const updated = { ...pd, idiomas: items };
      await invoke("guardar_candidato_base", { dados: updated });
      onSaved(updated);
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  return (
    <>
      {items.map((l, i) => (
        <div key={i} style={{ display: "grid", gridTemplateColumns: "1fr 140px 28px", gap: 8, alignItems: "end", marginBottom: 10 }}>
          <EdField label={i === 0 ? t.profile.editor.language : ""}>
            <input style={edInput} value={l.idioma} onChange={e => upd(i, "idioma", e.target.value)} placeholder="Portuguese" />
          </EdField>
          <EdField label={i === 0 ? t.profile.editor.level : ""}>
            <select value={l.nivel} onChange={e => upd(i, "nivel", e.target.value)} style={{ ...edInput, cursor: "pointer" }}>
              {NIVEIS_IDIOMA.map(n => <option key={n} value={n}>{n}</option>)}
            </select>
          </EdField>
          <button onClick={() => setItems(arr => arr.filter((_, j) => j !== i))} style={{ height: 33, background: "none", border: "1px solid var(--border)", borderRadius: 6, cursor: "pointer", color: "var(--text-tertiary)", marginBottom: 14 }}>×</button>
        </div>
      ))}
      <button onClick={() => setItems(arr => [...arr, { idioma: "", nivel: "B2" }])} style={{ fontSize: 12, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0, fontFamily: "inherit", marginBottom: 16 }}>{t.profile.editor.addLanguage}</button>
      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 10 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

// ── Region picker ─────────────────────────────────────────────────────────────

const REGION_OPTIONS: { value: string; label: string; group: string }[] = [
  // Mode
  { value: "remoto-global", label: "Global Remote",  group: "Mode" },
  { value: "remoto-europa",  label: "Europe Remote",  group: "Mode" },
  // Europe
  { value: "Europa",          label: "Europe (any country)", group: "Europe" },
  { value: "Portugal",        label: "Portugal",        group: "Europe" },
  { value: "Dinamarca",       label: "Denmark",         group: "Europe" },
  { value: "Alemanha",        label: "Germany",         group: "Europe" },
  { value: "Holanda",         label: "Netherlands",     group: "Europe" },
  { value: "Espanha",         label: "Spain",           group: "Europe" },
  { value: "França",          label: "France",          group: "Europe" },
  { value: "Reino Unido",     label: "United Kingdom",  group: "Europe" },
  { value: "Irlanda",         label: "Ireland",         group: "Europe" },
  { value: "Suécia",          label: "Sweden",          group: "Europe" },
  { value: "Noruega",         label: "Norway",          group: "Europe" },
  { value: "Finlândia",       label: "Finland",         group: "Europe" },
  { value: "Suíça",           label: "Switzerland",     group: "Europe" },
  { value: "Áustria",         label: "Austria",         group: "Europe" },
  { value: "Bélgica",         label: "Belgium",         group: "Europe" },
  { value: "Itália",          label: "Italy",           group: "Europe" },
  { value: "Polónia",         label: "Poland",          group: "Europe" },
  { value: "República Checa", label: "Czech Republic",  group: "Europe" },
  { value: "Luxemburgo",      label: "Luxembourg",      group: "Europe" },
  // North America
  { value: "Estados Unidos",  label: "United States",   group: "North America" },
  { value: "Canadá",          label: "Canada",          group: "North America" },
  // Latin America
  { value: "América Latina",  label: "Latin America (any country)", group: "Latin America" },
  { value: "Brasil",          label: "Brazil",          group: "Latin America" },
  { value: "Argentina",       label: "Argentina",       group: "Latin America" },
  { value: "México",          label: "Mexico",          group: "Latin America" },
  { value: "Colômbia",        label: "Colombia",        group: "Latin America" },
  { value: "Chile",           label: "Chile",           group: "Latin America" },
  // Asia / Oceania
  { value: "Ásia",            label: "Asia (any country)", group: "Asia & Oceania" },
  { value: "Japão",           label: "Japan",           group: "Asia & Oceania" },
  { value: "Singapura",       label: "Singapore",       group: "Asia & Oceania" },
  { value: "Austrália",       label: "Australia",       group: "Asia & Oceania" },
  { value: "Nova Zelândia",   label: "New Zealand",     group: "Asia & Oceania" },
];

const GROUP_ORDER = ["Mode", "Europe", "North America", "Latin America", "Asia & Oceania"];

const RegionPicker: React.FC<{ value: string[]; onChange: (v: string[]) => void }> = ({ value, onChange }) => {
  const t = useT();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setQuery("");
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const q = query.toLowerCase();
  const filtered = REGION_OPTIONS.filter(
    opt => !value.includes(opt.value) &&
      (opt.label.toLowerCase().includes(q) || opt.group.toLowerCase().includes(q))
  );

  const grouped = GROUP_ORDER.reduce<Record<string, typeof REGION_OPTIONS>>((acc, g) => {
    const opts = filtered.filter(o => o.group === g);
    if (opts.length) acc[g] = opts;
    return acc;
  }, {});

  const add = (val: string) => { onChange([...value, val]); setQuery(""); };
  const remove = (val: string) => onChange(value.filter(v => v !== val));

  return (
    <div ref={containerRef} style={{ position: "relative" }}>
      {value.length > 0 && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }}>
          {value.map(v => (
            <span key={v} style={{
              display: "inline-flex", alignItems: "center", gap: 4,
              fontSize: 12, padding: "2px 8px",
              background: "var(--accent-soft)", color: "var(--accent-strong)",
              borderRadius: 5, border: "1px solid var(--accent)",
            }}>
              {v}
              <button
                onClick={() => remove(v)}
                style={{ background: "none", border: "none", cursor: "pointer", padding: 0, color: "var(--accent-strong)", display: "flex", alignItems: "center", lineHeight: 1 }}
              >
                <X size={11} />
              </button>
            </span>
          ))}
        </div>
      )}
      <input
        style={edInput}
        placeholder={t.profile.editor.searchRegions}
        value={query}
        onChange={e => { setQuery(e.target.value); setOpen(true); }}
        onFocus={() => setOpen(true)}
      />
      {open && Object.keys(grouped).length > 0 && (
        <div style={{
          position: "absolute", top: "calc(100% + 3px)", left: 0, right: 0, zIndex: 300,
          background: "var(--bg-surface)", border: "1px solid var(--border)",
          borderRadius: 7, maxHeight: 220, overflowY: "auto",
          boxShadow: "0 4px 16px rgba(0,0,0,0.14)",
        }}>
          {Object.entries(grouped).map(([group, opts]) => (
            <div key={group}>
              <div style={{
                fontSize: 10, fontWeight: 700, color: "var(--text-tertiary)",
                textTransform: "uppercase", letterSpacing: "0.07em",
                padding: "7px 10px 3px",
              }}>
                {group}
              </div>
              {opts.map(opt => (
                <button
                  key={opt.value}
                  onMouseDown={e => { e.preventDefault(); add(opt.value); }}
                  style={{
                    width: "100%", textAlign: "left", padding: "6px 12px",
                    background: "none", border: "none", cursor: "pointer",
                    fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit",
                    display: "block",
                  }}
                  onMouseEnter={e => (e.currentTarget.style.background = "var(--bg-sunken)")}
                  onMouseLeave={e => (e.currentTarget.style.background = "none")}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const VarianteEditor: React.FC<{ varianteId: string; variants: SearchVariant[]; onSaved: () => void; onClose: () => void }> = ({ varianteId, variants, onSaved, onClose }) => {
  const t = useT();
  const [draft, setDraft] = useState<SearchVariant>(() => {
    const initial = variants.find(v => v.id === varianteId);
    if (initial) {
      return { ...initial, foco_competencias: [...initial.foco_competencias], foco_experiencia: [...initial.foco_experiencia], regioes_aceitas: [...initial.regioes_aceitas], modelos_trabalho: [...initial.modelos_trabalho], idiomas_aplicacao: [...initial.idiomas_aplicacao] };
    }
    return { id: varianteId || `variante_${Date.now()}`, nome_exibicao: "", peso: 50, ativa: true, foco_competencias: [], foco_experiencia: [], regioes_aceitas: [], modelos_trabalho: [], idiomas_aplicacao: [], cv_gerado_path: "", cv_gerado_em: "" };
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    setSaving(true); setError(null);
    try {
      await invoke("guardar_variante_unica", { variante: draft });
      onSaved();
    } catch (e) { setError(String(e)); setSaving(false); }
  };
  return (
    <>
      <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 16, alignItems: "start" }}>
        <EdField label={t.profile.editor.variantName}><input style={edInput} value={draft.nome_exibicao} onChange={e => setDraft(d => ({ ...d, nome_exibicao: e.target.value }))} /></EdField>
        <EdField label={t.profile.editor.variantActive}>
          <div style={{ height: 33, display: "flex", alignItems: "center" }}>
            <button onClick={() => setDraft(d => ({ ...d, ativa: !d.ativa }))} style={{ padding: "5px 14px", borderRadius: 6, border: `1px solid ${draft.ativa ? "var(--accent)" : "var(--border)"}`, background: draft.ativa ? "var(--accent-soft)" : "transparent", color: draft.ativa ? "var(--accent-strong)" : "var(--text-secondary)", fontSize: 13, cursor: "pointer", fontFamily: "inherit", fontWeight: 500 }}>
              {draft.ativa ? t.profile.variantActive : t.profile.variantInactive}
            </button>
          </div>
        </EdField>
      </div>
      <EdField label={t.profile.editor.acceptedRegions}>
        <RegionPicker
          value={draft.regioes_aceitas}
          onChange={v => setDraft(d => ({ ...d, regioes_aceitas: v }))}
        />
      </EdField>
      <EdField label={t.profile.editor.workModels}>
        <div style={{ display: "flex", gap: 6 }}>
          {["remoto", "híbrido", "presencial"].map(m => {
            const active = draft.modelos_trabalho.includes(m);
            return (
              <button
                key={m}
                type="button"
                onClick={() => setDraft(d => ({
                  ...d,
                  modelos_trabalho: active
                    ? d.modelos_trabalho.filter(x => x !== m)
                    : [...d.modelos_trabalho, m],
                }))}
                style={{
                  padding: "5px 14px", borderRadius: 6, cursor: "pointer",
                  border: `1px solid ${active ? "var(--accent)" : "var(--border)"}`,
                  background: active ? "var(--accent-soft)" : "transparent",
                  color: active ? "var(--accent-strong)" : "var(--text-secondary)",
                  fontSize: 13, fontFamily: "inherit", fontWeight: active ? 500 : 400,
                  transition: "all 0.12s",
                }}
              >
                {m.charAt(0).toUpperCase() + m.slice(1)}
              </button>
            );
          })}
        </div>
      </EdField>
      <EdField label={t.profile.editor.applicationLanguages} hint={t.profile.editor.applicationLanguagesHint}><input style={edInput} value={tagsToStr(draft.idiomas_aplicacao)} onChange={e => setDraft(d => ({ ...d, idiomas_aplicacao: strToTags(e.target.value) }))} placeholder="en, da, pt" /></EdField>
      <EdField label={t.profile.editor.skillFocus} hint={t.profile.editor.skillFocusHint}><input style={edInput} value={tagsToStr(draft.foco_competencias)} onChange={e => setDraft(d => ({ ...d, foco_competencias: strToTags(e.target.value) }))} /></EdField>
      {error && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 10 }}>{error}</div>}
      <ModalActions saving={saving} onSave={save} onClose={onClose} />
    </>
  );
};

const SectionEditModal: React.FC<{
  target: EditTarget;
  profileData: CandidateBase;
  variants: SearchVariant[];
  onSaved: (updatedData?: CandidateBase) => void;
  onClose: () => void;
}> = ({ target, profileData, variants, onSaved, onClose }) => {
  const t = useT();
  const sectionLabels = t.profile.sectionLabels as Record<string, string>;
  const title = target.kind === "variante"
    ? (variants.find(v => v.id === target.id)?.nome_exibicao ?? t.profile.variantActive)
    : sectionLabels[target.kind] ?? target.kind;

  return (
    <div
      onClick={onClose}
      style={{ position: "fixed", inset: 0, zIndex: 200, background: "rgba(0,0,0,0.35)", backdropFilter: "blur(2px)", display: "flex", alignItems: "center", justifyContent: "center", padding: 24 }}
    >
      <div
        onClick={e => e.stopPropagation()}
        style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 12, padding: "22px 24px", width: "100%", maxWidth: 580, maxHeight: "calc(100vh - 80px)", overflow: "auto", position: "relative" }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 20 }}>
          <span style={{ fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>{title}</span>
          <button onClick={onClose} style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", display: "flex", alignItems: "center" }}>
            <X size={18} />
          </button>
        </div>

        {target.kind === "dados_pessoais" && <DadosPessoaisEditor pd={profileData} onSaved={onSaved} onClose={onClose} />}
        {target.kind === "experiencia" && <ExperienciaEditor pd={profileData} onSaved={onSaved} onClose={onClose} />}
        {target.kind === "projetos" && <ProjetosEditor pd={profileData} onSaved={onSaved} onClose={onClose} />}
        {target.kind === "formacao" && <FormacaoEditor pd={profileData} onSaved={onSaved} onClose={onClose} />}
        {target.kind === "competencias" && <CompetenciasEditor pd={profileData} onSaved={onSaved} onClose={onClose} />}
        {target.kind === "idiomas" && <IdiomasEditor pd={profileData} onSaved={onSaved} onClose={onClose} />}
        {target.kind === "variante" && <VarianteEditor varianteId={target.id} variants={variants} onSaved={() => onSaved()} onClose={onClose} />}
        {target.kind === "nova_variante" && <VarianteEditor varianteId="" variants={variants} onSaved={() => onSaved()} onClose={onClose} />}
      </div>
    </div>
  );
};

// ── Main component ─────────────────────────────────────────────────────────

export const Perfil: React.FC<{ initialSection?: string | null; onSectionHandled?: () => void }> = ({ initialSection, onSectionHandled }) => {
  const [mode, setMode] = useState<Mode>("resumo");
  const [chatFocus, setChatFocus] = useState<ChatFocus | null>(null);
  const [profileData, setProfileData] = useState<CandidateBase | null>(null);
  const [variants, setVariants] = useState<SearchVariant[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasProfile, setHasProfile] = useState(false);
  const [editTarget, setEditTarget] = useState<EditTarget | null>(null);
  const hasLoadedOnce = useRef(false);

  const loadData = useCallback(async () => {
    try {
      const [base, vars] = await Promise.all([
        invoke<CandidateBase>("ler_candidato_base"),
        invoke<SearchVariant[]>("ler_search_variants"),
      ]);
      setProfileData(base);
      setVariants(vars ?? []);
      hasLoadedOnce.current = true;
      setHasProfile(
        Boolean(base?.dados_pessoais?.nome_completo) ||
        Boolean(base?.dados_pessoais?.email) ||
        (base?.experiencia?.length ?? 0) > 0 ||
        (base?.competencias?.length ?? 0) > 0 ||
        (base?.projetos?.length ?? 0) > 0 ||
        (vars?.length ?? 0) > 0
      );
    } catch (e) {
      console.error("[Perfil] loadData error:", e);
      // Se já tínhamos dados carregados, manter o estado — não apagar o perfil por erro de parse
      if (!hasLoadedOnce.current) setHasProfile(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();

    let active = true;
    let unlisten: (() => void) | undefined;

    listen("perfil-atualizado", loadData).then((fn) => {
      if (active) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [loadData]);

  useEffect(() => {
    if (!initialSection) return;
    const sectionTargets: Record<string, EditTarget> = {
      dados_pessoais: { kind: "dados_pessoais" },
      experiencia:    { kind: "experiencia" },
      projetos:       { kind: "projetos" },
      formacao:       { kind: "formacao" },
      competencias:   { kind: "competencias" },
      idiomas:        { kind: "idiomas" },
    };
    const target = sectionTargets[initialSection];
    if (!target) return;
    setEditTarget(target);
    onSectionHandled?.();
  }, [initialSection, onSectionHandled]);

  const openChat = (focus?: ChatFocus) => {
    setChatFocus(focus ?? null);
    setMode("chat");
  };

  const openCurriculos = () => setMode("curriculos");
  const openCoverLetters = () => setMode("cover_letters");

  const backToResumo = () => {
    setMode("resumo");
    setChatFocus(null);
    loadData();
  };

  if (loading) {
    return (
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <span style={{ fontSize: 13, color: "var(--text-tertiary)" }}>A carregar perfil…</span>
      </div>
    );
  }

  if (mode === "chat") {
    return <ChatView focus={chatFocus} onBack={backToResumo} data={profileData} />;
  }

  if (mode === "curriculos") {
    return <CurriculosView onBack={backToResumo} />;
  }

  if (mode === "cover_letters") {
    return <CoverLettersView onBack={backToResumo} />;
  }

  if (!hasProfile) {
    return <EmptyState onStart={openChat} />;
  }

  return (
    <>
      <ResumoView
        data={profileData}
        variants={variants}
        onOpenChat={openChat}
        onOpenCurriculos={openCurriculos}
        onOpenCoverLetters={openCoverLetters}
        onDirectEdit={setEditTarget}
        onReloadData={loadData}
      />
      {editTarget && profileData && (
        <SectionEditModal
          target={editTarget}
          profileData={profileData}
          variants={variants}
          onSaved={(updatedData) => {
            if (updatedData) setProfileData(updatedData);
            setEditTarget(null);
            loadData();
          }}
          onClose={() => setEditTarget(null)}
        />
      )}
    </>
  );
};
