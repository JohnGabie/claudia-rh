import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "../../i18n";
import { X } from "lucide-react";
import { SearchVariant } from "../../types";
import { CandidateBase, EditTarget } from "./types";

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

export const SectionEditModal: React.FC<{
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
