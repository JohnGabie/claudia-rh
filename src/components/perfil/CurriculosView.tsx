import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, FileText, Loader2 } from "lucide-react";
import { CurriculoInfo, DocLang, PALETTE } from "./types";

// ── Templates metadata ─────────────────────────────────────────────────────

const TEMPLATES = [
  { id: "classic-ats", nome: "Clássico ATS", badge: "95% ATS", desc: "Single column, sem cores. Máxima compatibilidade com Workday, Greenhouse e Taleo." },
  { id: "hybrid-skills", nome: "Híbrido Competências", badge: "90% ATS · Recomendado 2026", desc: "Competências em destaque no topo. Alinha com filtro skills-first dos ATS modernos." },
  { id: "dev-compact", nome: "Dev Compacto", badge: "1 página · Tech/Startups", desc: "Layout denso e técnico. GitHub em destaque, badges de tecnologia, formato PAR." },
];
export const CurriculosView: React.FC<{ onBack: () => void }> = ({ onBack }) => {
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
          Voltar
        </button>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
          Currículos
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
          <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>Idioma</span>
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
          Cor do acento
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
            title="Cor personalizada"
          />
          <span style={{ fontSize: 11, color: "var(--text-tertiary)", fontFamily: "monospace" }}>{selectedColor}</span>
        </div>
      </div>

      {/* Template cards */}
      <div style={{ display: "flex", gap: 12, marginBottom: 28, flexWrap: "wrap" }}>
        {TEMPLATES.map(t => (
          <div key={t.id} style={{
            flex: "1 1 200px", background: "var(--bg-surface)",
            border: "1px solid var(--border)", borderRadius: 10, padding: 16,
            display: "flex", flexDirection: "column", gap: 8,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>{t.nome}</span>
              <span style={{
                fontSize: 10, padding: "2px 7px", borderRadius: 10,
                background: "var(--accent-soft)", color: "var(--accent-strong)", fontWeight: 500,
              }}>{t.badge}</span>
            </div>
            <p style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.45, flex: 1 }}>{t.desc}</p>
            <button
              onClick={() => gerar(t.id)}
              disabled={gerando !== null}
              style={{
                display: "flex", alignItems: "center", justifyContent: "center", gap: 6,
                padding: "7px 14px", background: gerando === t.id ? "var(--bg-sunken)" : selectedColor,
                border: "none", borderRadius: 8, fontSize: 13,
                color: gerando === t.id ? "var(--text-secondary)" : "#fff",
                cursor: gerando !== null ? "not-allowed" : "pointer", fontFamily: "inherit",
                transition: "background 0.15s",
              }}
            >
              {gerando === t.id ? (
                <>
                  <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
                  A gerar…
                </>
              ) : (
                <>
                  <FileText size={13} />
                  Gerar
                </>
              )}
            </button>
          </div>
        ))}
      </div>

      {/* Generated CVs list */}
      <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 10 }}>
        Currículos gerados
      </div>
      {loading ? (
        <div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>A carregar…</div>
      ) : curriculos.length === 0 ? (
        <div style={{
          background: "var(--bg-surface)", border: "1px dashed var(--border)",
          borderRadius: 8, padding: "20px 16px", textAlign: "center",
          fontSize: 13, color: "var(--text-secondary)",
        }}>
          Ainda não há currículos gerados. Escolhe um template acima para começar.
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
                Abrir
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
