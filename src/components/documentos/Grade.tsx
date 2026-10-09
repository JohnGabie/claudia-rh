import React, { useCallback, useEffect, useState } from "react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Loader2, Plus, Upload } from "lucide-react";
import { useT } from "../../i18n";
import { docsApi } from "../../lib/documentos-api";
import type { Documento, TipoDocumento, VarianteResumo } from "../../types";
import { Cartao } from "./Cartao";
import { Gerar } from "./Gerar";
import { ouvirDocumentos } from "./ouvir";
import { Filtro, filtrar, valoresDeTag } from "./logica";

const btn: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 6, padding: "7px 14px", borderRadius: 8, fontSize: 13,
  fontFamily: "inherit", cursor: "pointer",
};

export const Grade: React.FC<{ tipo: TipoDocumento; onAbrir: (id: number) => void }> = ({ tipo, onAbrir }) => {
  const t = useT();
  const [docs, setDocs] = useState<Documento[] | null>(null);
  const [variantes, setVariantes] = useState<VarianteResumo[]>([]);
  const [erro, setErro] = useState<string | null>(null);
  const [erroAcao, setErroAcao] = useState<string | null>(null);
  const [filtro, setFiltro] = useState<Filtro>({});
  const [gerando, setGerando] = useState(false);
  const [importando, setImportando] = useState(false);

  const carregar = useCallback(async () => {
    setErro(null);
    try {
      const [lista, vars] = await Promise.all([docsApi.listar(tipo), docsApi.listarVariantes()]);
      setDocs(lista);
      setVariantes(vars);
    } catch (e) {
      setErro(String(e));
    }
  }, [tipo]);

  useEffect(() => { setDocs(null); setFiltro({}); setGerando(false); carregar(); }, [carregar]);

  // Writes made elsewhere (Profile selects, background migration, saved cover letters) reload the grid.
  useEffect(() => ouvirDocumentos(carregar), [carregar]);

  const importar = async () => {
    setErroAcao(null);
    const escolhido = await openFileDialog({ multiple: false, filters: [{ name: "PDF / DOCX", extensions: ["pdf", "docx"] }] });
    if (typeof escolhido !== "string") return;
    setImportando(true);
    try {
      await docsApi.importar(tipo, escolhido, null);
      await carregar();
    } catch (e) {
      setErroAcao(String(e));
    } finally {
      setImportando(false);
    }
  };

  const opcoes = (docs ?? []);
  const filtros: { chave: keyof Filtro; rotulo: string }[] = [
    { chave: "variante", rotulo: t.documents.filterVariant },
    { chave: "template", rotulo: t.documents.filterTemplate },
    { chave: "cor", rotulo: t.documents.filterColor },
    { chave: "idioma", rotulo: t.documents.filterLanguage },
  ];

  const botoes = (
    <>
      <button onClick={() => setGerando(true)} style={{ ...btn, background: "var(--accent)", border: "none", color: "#fff" }}>
        <Plus size={14} /> {t.documents.generate}
      </button>
      <button onClick={importar} disabled={importando} style={{ ...btn, background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)" }}>
        {importando ? <Loader2 size={14} style={{ animation: "spin 1s linear infinite" }} /> : <Upload size={14} />} {t.documents.importMine}
      </button>
    </>
  );

  if (erro) {
    return (
      <div style={{ border: "1px solid var(--danger)", background: "var(--bg-surface)", borderRadius: 8, padding: "14px 16px", display: "flex", alignItems: "center", gap: 12 }}>
        <span style={{ flex: 1, fontSize: 13, color: "var(--danger)" }}>{t.documents.loadError}: {erro}</span>
        <button onClick={carregar} style={{ ...btn, background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)" }}>{t.documents.retry}</button>
      </div>
    );
  }

  if (docs === null) {
    return <div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>{t.documents.loading}</div>;
  }

  const visiveis = filtrar(docs, filtro);

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap", marginBottom: 16 }}>
        {docs.length > 0 && filtros.map(({ chave, rotulo }) => (
          <select
            key={chave}
            aria-label={rotulo}
            value={filtro[chave as keyof Filtro] ?? ""}
            onChange={(e) => setFiltro((f) => ({ ...f, [chave]: e.target.value || undefined }))}
            style={{ padding: "6px 8px", borderRadius: 6, border: "1px solid var(--border)", background: "var(--bg-surface)", fontSize: 12, color: "var(--text-secondary)", fontFamily: "inherit" }}
          >
            <option value="">{rotulo}: {t.documents.filterAll}</option>
            {valoresDeTag(opcoes, chave).map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        ))}
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>{botoes}</div>
      </div>

      {erroAcao && <div style={{ fontSize: 13, color: "var(--danger)", marginBottom: 12 }}>{erroAcao}</div>}

      {gerando && (
        <Gerar tipo={tipo} variantes={variantes} onCancelar={() => setGerando(false)} onFeito={() => { setGerando(false); carregar(); }} />
      )}

      {docs.length === 0 ? (
        <div style={{ background: "var(--bg-surface)", border: "1px dashed var(--border)", borderRadius: 8, padding: "32px 16px", textAlign: "center" }}>
          <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 6 }}>{t.documents.empty[tipo]}</div>
          <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{t.documents.emptyHint}</div>
        </div>
      ) : visiveis.length === 0 ? (
        <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{t.documents.noMatch}</div>
      ) : (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(190px, 1fr))", gap: 16 }}>
          {visiveis.map((d) => <Cartao key={d.id} doc={d} onAbrir={onAbrir} />)}
        </div>
      )}
      <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
    </div>
  );
};
