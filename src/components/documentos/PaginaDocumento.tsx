import React, { useCallback, useEffect, useRef, useState } from "react";
import { ArrowLeft, ExternalLink, FileText, Pencil, Trash2, X } from "lucide-react";
import { useT } from "../../i18n";
import { docsApi } from "../../lib/documentos-api";
import type { DocumentoDetalhe, Tag, TipoTag, VarianteResumo } from "../../types";
import { A4_LARGURA, FolhaA4 } from "./FolhaA4";
import { avisoAoApagar, comTag, ehPreferido, semTag } from "./logica";

const TIPOS_TAG: TipoTag[] = ["variante", "template", "cor", "idioma", "livre"];

const botao: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 6, padding: "7px 12px", borderRadius: 8, fontSize: 13,
  fontFamily: "inherit", cursor: "pointer", background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)",
};
const painel: React.CSSProperties = {
  background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 10, padding: "14px 16px",
};
const titulo: React.CSSProperties = {
  fontSize: 11, fontWeight: 600, color: "var(--text-tertiary)", textTransform: "uppercase", letterSpacing: "0.07em", marginBottom: 10,
};

export const PaginaDocumento: React.FC<{ id: number; onVoltar: () => void }> = ({ id, onVoltar }) => {
  const t = useT();
  const [doc, setDoc] = useState<DocumentoDetalhe | null>(null);
  const [variantes, setVariantes] = useState<VarianteResumo[]>([]);
  const [erro, setErro] = useState<string | null>(null);
  const [renomeando, setRenomeando] = useState(false);
  const [nome, setNome] = useState("");
  const [novoTipo, setNovoTipo] = useState<TipoTag>("livre");
  const [novoValor, setNovoValor] = useState("");
  const [larguraCol, setLarguraCol] = useState(0);
  const colRef = useRef<HTMLDivElement>(null);

  const carregar = useCallback(async () => {
    try {
      const [d, v] = await Promise.all([docsApi.obter(id), docsApi.listarVariantes()]);
      setDoc(d);
      setVariantes(v);
    } catch (e) {
      setErro(String(e));
    }
  }, [id]);

  useEffect(() => { carregar(); }, [carregar]);

  // Fit the sheet to the width of its column.
  useEffect(() => {
    const el = colRef.current;
    if (!el) return;
    const obs = new ResizeObserver(() => setLarguraCol(el.clientWidth));
    obs.observe(el);
    setLarguraCol(el.clientWidth);
    return () => obs.disconnect();
  }, [doc?.id]);

  const agir = async (fn: () => Promise<unknown>) => {
    setErro(null);
    try { await fn(); await carregar(); } catch (e) { setErro(String(e)); }
  };

  const salvarTags = (tags: Tag[]) => agir(() => docsApi.atualizarTags(id, tags));

  const adicionarTag = () => {
    if (!doc) return;
    const tags = comTag(doc.tags, { tipo: novoTipo, valor: novoValor });
    setNovoValor("");
    if (tags !== doc.tags) void salvarTags(tags);
  };

  const alternarPreferido = (varianteId: string | null, marcado: boolean) =>
    agir(() => (marcado ? docsApi.definirPreferido(id, varianteId) : docsApi.removerPreferido(doc!.tipo, varianteId)));

  const salvarNome = () => {
    const n = nome.trim();
    setRenomeando(false);
    if (n && doc && n !== doc.nome) void agir(() => docsApi.renomear(id, n));
  };

  const excluir = async () => {
    if (!doc) return;
    const aviso = avisoAoApagar(doc);
    const msg = aviso ? t.documents.confirmDeletePreferred : t.documents.confirmDelete;
    if (!window.confirm(msg)) return;
    setErro(null);
    try { await docsApi.apagar(id); onVoltar(); } catch (e) { setErro(String(e)); }
  };

  const escala = larguraCol > 0 ? Math.min(1, larguraCol / A4_LARGURA) : 1;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <button onClick={onVoltar} style={botao}><ArrowLeft size={14} />{t.common.back}</button>
        {doc && !renomeando && (
          <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{doc.nome}</h1>
        )}
        {doc && renomeando && (
          <input
            autoFocus value={nome} onChange={(e) => setNome(e.target.value)} onBlur={salvarNome}
            onKeyDown={(e) => { if (e.key === "Enter") salvarNome(); if (e.key === "Escape") setRenomeando(false); }}
            style={{ fontSize: 16, padding: "6px 10px", borderRadius: 6, border: "1px solid var(--accent)", background: "var(--bg-base)", color: "var(--text-primary)", fontFamily: "inherit", minWidth: 280 }}
          />
        )}
      </div>

      {erro && (
        <div style={{ border: "1px solid var(--danger)", background: "var(--bg-surface)", borderRadius: 8, padding: "10px 14px", marginBottom: 16, fontSize: 13, color: "var(--danger)" }}>{erro}</div>
      )}

      {!doc ? (
        !erro && <div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>{t.documents.loading}</div>
      ) : (
        <div style={{ display: "flex", gap: 24, alignItems: "flex-start", flexWrap: "wrap" }}>
          {/* Sheet / imported card */}
          <div ref={colRef} style={{ flex: "1 1 420px", minWidth: 0, maxWidth: A4_LARGURA }}>
            {doc.tem_preview && doc.preview_html ? (
              <FolhaA4 html={doc.preview_html} escala={escala} titulo={doc.nome} />
            ) : (
              <div style={{ ...painel, display: "flex", flexDirection: "column", alignItems: "center", gap: 10, padding: "40px 24px", textAlign: "center" }}>
                <FileText size={40} color="var(--text-tertiary)" />
                <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)" }}>{doc.nome}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                  {doc.formato.toUpperCase()} · {Math.max(1, Math.ceil(doc.tamanho / 1024))} KB · {doc.criado_em}
                </div>
                <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{t.documents.importedCard}</div>
                <button onClick={() => agir(() => docsApi.abrir(id))} style={{ ...botao, background: "var(--accent)", color: "#fff", border: "none" }}>
                  <ExternalLink size={14} />{t.documents.open}
                </button>
              </div>
            )}
          </div>

          {/* Side column */}
          <div style={{ flex: "1 1 280px", maxWidth: 360, display: "flex", flexDirection: "column", gap: 14 }}>
            <div style={painel}>
              <div style={titulo}>{t.documents.details}</div>
              <dl style={{ margin: 0, display: "grid", gridTemplateColumns: "auto 1fr", gap: "6px 12px", fontSize: 13 }}>
                <dt style={{ color: "var(--text-secondary)" }}>{t.documents.origin.label}</dt><dd style={{ margin: 0 }}>{t.documents.origin[doc.origem]}</dd>
                <dt style={{ color: "var(--text-secondary)" }}>{t.documents.format}</dt><dd style={{ margin: 0 }}>{doc.formato.toUpperCase()}</dd>
                <dt style={{ color: "var(--text-secondary)" }}>{t.documents.size}</dt><dd style={{ margin: 0 }}>{Math.max(1, Math.ceil(doc.tamanho / 1024))} KB</dd>
                <dt style={{ color: "var(--text-secondary)" }}>{t.documents.createdAt}</dt><dd style={{ margin: 0 }}>{doc.criado_em}</dd>
              </dl>
            </div>

            <div style={painel}>
              <div style={titulo}>{t.documents.tags}</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 10 }}>
                {doc.tags.length === 0 && <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{t.documents.noTags}</span>}
                {doc.tags.map((tg) => (
                  <span key={`${tg.tipo}:${tg.valor}`} style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 12, padding: "2px 4px 2px 8px", borderRadius: 6, background: "var(--bg-sunken)", color: "var(--text-secondary)" }}>
                    <span style={{ color: "var(--text-tertiary)" }}>{t.documents.tagTypes[tg.tipo]}:</span> {tg.valor}
                    <button onClick={() => salvarTags(semTag(doc.tags, tg))} aria-label={`${t.documents.removeTag} ${tg.valor}`}
                      style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-tertiary)", display: "flex", padding: 2 }}><X size={12} /></button>
                  </span>
                ))}
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <select value={novoTipo} onChange={(e) => setNovoTipo(e.target.value as TipoTag)} aria-label={t.documents.tags}
                  style={{ padding: "6px 6px", borderRadius: 6, border: "1px solid var(--border)", background: "var(--bg-base)", fontSize: 12, fontFamily: "inherit", color: "var(--text-secondary)" }}>
                  {TIPOS_TAG.map((tp) => <option key={tp} value={tp}>{t.documents.tagTypes[tp]}</option>)}
                </select>
                <input value={novoValor} onChange={(e) => setNovoValor(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") adicionarTag(); }}
                  style={{ flex: 1, minWidth: 0, padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border)", background: "var(--bg-base)", fontSize: 12, fontFamily: "inherit", color: "var(--text-primary)" }} />
                <button onClick={adicionarTag} style={{ ...botao, padding: "5px 10px", fontSize: 12 }}>{t.documents.addTag}</button>
              </div>
            </div>

            <div style={painel}>
              <div style={titulo}>{t.documents.preferred}</div>
              <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, marginBottom: 6, cursor: "pointer" }}>
                <input type="checkbox" checked={ehPreferido(doc, null)} onChange={(e) => alternarPreferido(null, e.target.checked)} />
                {t.documents.preferredGlobal}
              </label>
              {variantes.filter((v) => v.ativa).map((v) => (
                <label key={v.id} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, marginBottom: 6, cursor: "pointer" }}>
                  <input type="checkbox" checked={ehPreferido(doc, v.id)} onChange={(e) => alternarPreferido(v.id, e.target.checked)} />
                  {t.documents.preferredForVariant}: {v.nome_exibicao}
                </label>
              ))}
            </div>

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <button onClick={() => { setNome(doc.nome); setRenomeando(true); }} style={botao}><Pencil size={14} />{t.documents.rename}</button>
              <button onClick={() => agir(() => docsApi.abrir(id))} style={botao}><ExternalLink size={14} />{t.documents.open}</button>
              <button onClick={excluir} style={{ ...botao, color: "var(--danger)", borderColor: "var(--danger)" }}><Trash2 size={14} />{t.documents.delete}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
