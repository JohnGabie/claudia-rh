import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useT } from "../../i18n";
import { ArrowLeft, MailOpen, Loader2, AlertCircle } from "lucide-react";
import { CoverLetterInfo, DocLang, PALETTE } from "./types";

// ── Cover Letters view ─────────────────────────────────────────────────────

export const CoverLettersView: React.FC<{ onBack: () => void }> = ({ onBack }) => {
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
