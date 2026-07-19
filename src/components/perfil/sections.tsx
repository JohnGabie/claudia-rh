import React, { useEffect, useRef } from "react";
import { useT } from "../../i18n";
import { Pencil } from "lucide-react";
import { SearchVariant } from "../../types";
import { CandidateBase, EditTarget } from "./types";

// ── Logo ───────────────────────────────────────────────────────────────────

export const GlassesAvatar: React.FC = () => (
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


// ── Profile section components ─────────────────────────────────────────────

export const EditBtn: React.FC<{ onClick: () => void }> = ({ onClick }) => {
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

export const SectionBlock: React.FC<{ title: string; onEdit: () => void; children: React.ReactNode }> = ({ title, onEdit, children }) => (
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

export const ProfileHeader: React.FC<{ data: CandidateBase; onEdit: () => void }> = ({ data, onEdit }) => {
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

export const ExperienciaSection: React.FC<{ items: CandidateBase["experiencia"]; onEdit: () => void }> = ({ items, onEdit }) => {
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

export const ProjetosSection: React.FC<{ items: CandidateBase["projetos"]; onEdit: () => void }> = ({ items, onEdit }) => {
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

export const FormacaoSection: React.FC<{ items: CandidateBase["formacao"]; onEdit: () => void }> = ({ items, onEdit }) => {
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

export const CompetenciasSection: React.FC<{ items: string[]; onEdit: () => void }> = ({ items, onEdit }) => {
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

export const IdiomasSection: React.FC<{ items: CandidateBase["idiomas"]; onEdit: () => void }> = ({ items, onEdit }) => {
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

export const VariantCard: React.FC<{
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
