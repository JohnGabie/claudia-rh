import React, { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlertCircle, FileText, Loader2, MailOpen, X } from "lucide-react";
import { useT } from "../../i18n";
import { docsApi } from "../../lib/documentos-api";
import type { Documento, TipoDocumento, VarianteResumo } from "../../types";
import { DocLang, PALETTE } from "../perfil/types";

const card: React.CSSProperties = {
  background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 10, padding: "12px 16px",
};
const input: React.CSSProperties = {
  width: "100%", padding: "8px 11px", borderRadius: 7, border: "1px solid var(--border)",
  background: "var(--bg-base)", fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit", outline: "none",
};
const rotulo: React.CSSProperties = { fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 5, fontWeight: 500 };

export const Gerar: React.FC<{
  tipo: TipoDocumento;
  variantes: VarianteResumo[];
  onFeito: () => void;
  onCancelar: () => void;
}> = ({ tipo, variantes, onFeito, onCancelar }) => {
  const t = useT();
  const [cor, setCor] = useState("#D97757");
  const [idioma, setIdioma] = useState<DocLang>("pt");
  const [varianteId, setVarianteId] = useState<string | null>(null);
  const [gerando, setGerando] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  // Cover letter fields
  const [empresa, setEmpresa] = useState("");
  const [cargo, setCargo] = useState("");
  const [descricaoVaga, setDescricaoVaga] = useState("");
  const [notaExtra, setNotaExtra] = useState("");
  const [streamText, setStreamText] = useState("");
  const streamEndRef = useRef<HTMLDivElement>(null);
  const onFeitoRef = useRef(onFeito);
  useEffect(() => { onFeitoRef.current = onFeito; }, [onFeito]);

  const TEMPLATES = [
    { id: "classic-ats", ...t.profile.templates.classicAts },
    { id: "hybrid-skills", ...t.profile.templates.hybridSkills },
    { id: "dev-compact", ...t.profile.templates.devCompact },
  ];

  useEffect(() => {
    if (tipo !== "carta") return;
    let active = true;
    const unlisteners: (() => void)[] = [];
    Promise.all([
      listen<string>("cover-letter-stream", (e) => {
        setStreamText((prev) => prev + e.payload);
        streamEndRef.current?.scrollIntoView({ behavior: "smooth" });
      }),
      // P1: the payload is the library Document; the grid reloads from the backend.
      listen<Documento>("cover-letter-done", () => {
        setGerando(null);
        onFeitoRef.current();
      }),
      listen<string>("cover-letter-error", (e) => {
        setErro(e.payload);
        setGerando(null);
      }),
    ]).then((fns) => {
      if (active) unlisteners.push(...fns);
      else fns.forEach((f) => f());
    });
    return () => { active = false; unlisteners.forEach((f) => f()); };
  }, [tipo]);

  const gerarCurriculo = async (templateId: string) => {
    setErro(null);
    setGerando(templateId);
    try {
      await docsApi.gerarCurriculo({ templateId, corPrimaria: cor, idioma, varianteId });
      onFeito();
    } catch (e) {
      setErro(String(e));
    } finally {
      setGerando(null);
    }
  };

  const gerarCarta = async () => {
    const emp = empresa.trim();
    const car = cargo.trim();
    if (!emp || !car) return;
    setErro(null);
    setStreamText("");
    setGerando("carta");
    try {
      await docsApi.gerarCarta({
        empresa: emp, cargo: car, descricaoVaga: descricaoVaga.trim() || null, notaExtra: notaExtra.trim() || null,
        idioma, corPrimaria: cor, varianteId,
      });
    } catch (e) {
      setErro(String(e));
      setGerando(null);
    }
  };

  const ocupado = gerando !== null;
  const canGerarCarta = empresa.trim().length > 0 && cargo.trim().length > 0;

  return (
    <div style={{ ...card, padding: 20, marginBottom: 20, display: "flex", flexDirection: "column", gap: 14 }}>
      <div style={{ display: "flex", alignItems: "center" }}>
        <span style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", flex: 1 }}>
          {tipo === "curriculo" ? t.documents.generateResume : t.profile.newCoverLetter}
        </span>
        <button onClick={onCancelar} disabled={ocupado} title={t.common.cancel} aria-label={t.common.cancel}
          style={{ background: "transparent", border: "none", cursor: ocupado ? "default" : "pointer", color: "var(--text-tertiary)", display: "flex" }}>
          <X size={16} />
        </button>
      </div>

      {erro && (
        <div style={{ border: "1px solid var(--danger)", borderRadius: 8, padding: "10px 14px", fontSize: 13, color: "var(--danger)", display: "flex", alignItems: "center", gap: 8 }}>
          <AlertCircle size={14} /> {erro}
        </div>
      )}

      {tipo === "carta" && (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <label style={rotulo}>{t.profile.company} <span style={{ color: "var(--danger)" }}>*</span></label>
              <input value={empresa} onChange={(e) => setEmpresa(e.target.value)} placeholder="ex: Novo Nordisk" disabled={ocupado} style={input}
                onFocus={(e) => (e.target.style.borderColor = "var(--accent)")} onBlur={(e) => (e.target.style.borderColor = "var(--border)")} />
            </div>
            <div>
              <label style={rotulo}>{t.profile.position} <span style={{ color: "var(--danger)" }}>*</span></label>
              <input value={cargo} onChange={(e) => setCargo(e.target.value)} placeholder="ex: Senior Software Engineer" disabled={ocupado} style={input}
                onFocus={(e) => (e.target.style.borderColor = "var(--accent)")} onBlur={(e) => (e.target.style.borderColor = "var(--border)")} />
            </div>
          </div>
          <div>
            <label style={rotulo}>{t.profile.jobDescriptionLabel} <span style={{ color: "var(--text-tertiary)", fontWeight: 400 }}>{t.profile.jobDescriptionOpt}</span></label>
            <textarea value={descricaoVaga} onChange={(e) => setDescricaoVaga(e.target.value)} placeholder={t.profile.pasteJobText} disabled={ocupado} rows={4}
              style={{ ...input, resize: "vertical" }}
              onFocus={(e) => (e.target.style.borderColor = "var(--accent)")} onBlur={(e) => (e.target.style.borderColor = "var(--border)")} />
          </div>
          <div>
            <label style={rotulo}>{t.profile.extraNoteLabel} <span style={{ color: "var(--text-tertiary)", fontWeight: 400 }}>{t.profile.extraNoteOpt}</span></label>
            <textarea value={notaExtra} onChange={(e) => setNotaExtra(e.target.value)} disabled={ocupado} rows={2}
              style={{ ...input, resize: "vertical" }}
              onFocus={(e) => (e.target.style.borderColor = "var(--accent)")} onBlur={(e) => (e.target.style.borderColor = "var(--border)")} />
          </div>
        </>
      )}

      {/* Language, color and variant */}
      <div style={{ display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.profile.resumeLanguage}</span>
          {(["pt", "en"] as DocLang[]).map((l) => (
            <button key={l} onClick={() => setIdioma(l)} style={{
              padding: "3px 10px", borderRadius: 6, fontSize: 12, fontWeight: 600,
              border: idioma === l ? `1.5px solid ${cor}` : "1.5px solid var(--border)",
              background: idioma === l ? cor : "transparent", color: idioma === l ? "#fff" : "var(--text-secondary)",
              cursor: "pointer", fontFamily: "inherit",
            }}>{l.toUpperCase()}</button>
          ))}
        </div>
        <div style={{ width: 1, height: 24, background: "var(--border)" }} />
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.profile.accentColor}</span>
          {PALETTE.map((p) => (
            <button key={p.hex} title={p.label} onClick={() => setCor(p.hex)} style={{
              width: 22, height: 22, borderRadius: "50%", background: p.hex, padding: 0, cursor: "pointer", boxSizing: "border-box",
              border: cor === p.hex ? "3px solid var(--text-primary)" : "2px solid transparent",
              outline: cor === p.hex ? "2px solid var(--bg-surface)" : "none", outlineOffset: -5,
            }} />
          ))}
          <input type="color" value={cor} onChange={(e) => setCor(e.target.value)} title={t.profile.customColor}
            style={{ width: 22, height: 22, borderRadius: "50%", border: "none", padding: 0, cursor: "pointer", background: "none" }} />
        </div>
        <div style={{ width: 1, height: 24, background: "var(--border)" }} />
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <label htmlFor="gerar-variante" style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>{t.documents.variant}</label>
          <select id="gerar-variante" value={varianteId ?? ""} onChange={(e) => setVarianteId(e.target.value || null)} disabled={ocupado}
            style={{ ...input, width: "auto", padding: "5px 8px", fontSize: 12 }}>
            <option value="">{t.documents.noVariant}</option>
            {variantes.filter((v) => v.ativa).map((v) => <option key={v.id} value={v.id}>{v.nome_exibicao}</option>)}
          </select>
        </div>
      </div>

      {tipo === "curriculo" ? (
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          {TEMPLATES.map((tmpl) => (
            <div key={tmpl.id} style={{ flex: "1 1 200px", background: "var(--bg-base)", border: "1px solid var(--border)", borderRadius: 10, padding: 14, display: "flex", flexDirection: "column", gap: 8 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                <span style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>{tmpl.name}</span>
                <span style={{ fontSize: 10, padding: "2px 7px", borderRadius: 10, background: "var(--accent-soft)", color: "var(--accent-strong)", fontWeight: 500 }}>{tmpl.badge}</span>
              </div>
              <p style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.45, flex: 1, margin: 0 }}>{tmpl.desc}</p>
              <button onClick={() => gerarCurriculo(tmpl.id)} disabled={ocupado} style={{
                display: "flex", alignItems: "center", justifyContent: "center", gap: 6, padding: "7px 14px",
                background: gerando === tmpl.id ? "var(--bg-sunken)" : cor, border: "none", borderRadius: 8, fontSize: 13,
                color: gerando === tmpl.id ? "var(--text-secondary)" : "#fff", cursor: ocupado ? "not-allowed" : "pointer", fontFamily: "inherit",
              }}>
                {gerando === tmpl.id ? <><Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />{t.common.generating}</> : <><FileText size={13} />{t.common.generate}</>}
              </button>
            </div>
          ))}
        </div>
      ) : (
        <>
          <button onClick={gerarCarta} disabled={!canGerarCarta || ocupado} style={{
            width: "100%", padding: "11px 20px", background: canGerarCarta && !ocupado ? cor : "var(--bg-sunken)", border: "none", borderRadius: 8,
            fontSize: 14, fontWeight: 500, color: canGerarCarta && !ocupado ? "#fff" : "var(--text-tertiary)",
            cursor: canGerarCarta && !ocupado ? "pointer" : "not-allowed", fontFamily: "inherit",
            display: "flex", alignItems: "center", justifyContent: "center", gap: 8,
          }}>
            {ocupado ? <><Loader2 size={15} style={{ animation: "spin 1s linear infinite" }} />{t.common.generating}</> : <><MailOpen size={15} />{t.profile.generateCoverLetter}</>}
          </button>
          {(ocupado || streamText) && (
            <div style={{ background: "var(--bg-base)", border: "1px solid var(--border)", borderRadius: 10, padding: "14px 18px" }}>
              <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.07em", marginBottom: 10 }}>
                {ocupado ? t.profile.claudeWriting : t.profile.draftGenerated}
              </div>
              <div style={{ fontSize: 13, color: "var(--text-primary)", lineHeight: 1.75, whiteSpace: "pre-wrap", maxHeight: 260, overflowY: "auto" }}>
                {streamText}
                <div ref={streamEndRef} />
              </div>
            </div>
          )}
        </>
      )}
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
};
