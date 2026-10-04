import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Star, Upload } from "lucide-react";
import { useT } from "../../i18n";

// Provisional (attach-CV plan, part 1): the documents the agent can attach, an import button and the
// "preferred" mark. Part 2 replaces it with the Currículos tab.
interface Documento {
  id: number;
  nome: string;
  origem: string;
  formato: string;
  tamanho: number;
  criado_em: string;
}

export const ParaAnexar: React.FC = () => {
  const t = useT().profile.attach;
  const [docs, setDocs] = useState<Documento[]>([]);
  const [preferido, setPreferido] = useState<number | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  const carregar = () =>
    invoke<Documento[]>("listar_documentos", { tipo: "curriculo" })
      .then(setDocs)
      .catch((e) => setErro(String(e)));
  useEffect(() => {
    carregar();
  }, []);

  const importar = async () => {
    setErro(null);
    const escolhido = await openFileDialog({ multiple: false, filters: [{ name: "PDF / DOCX", extensions: ["pdf", "docx"] }] });
    if (typeof escolhido !== "string") return;
    try {
      await invoke("importar_documento", { tipo: "curriculo", caminhoOrigem: escolhido, nome: null });
      await carregar();
    } catch (e) {
      setErro(String(e));
    }
  };

  const marcar = async (id: number) => {
    setErro(null);
    try {
      await invoke("definir_preferido", { id, varianteId: null });
      setPreferido(id);
    } catch (e) {
      setErro(String(e));
    }
  };

  return (
    <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 10, padding: "12px 16px", marginBottom: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 8 }}>
        <strong style={{ fontSize: 14, color: "var(--text-primary)", flex: 1 }}>{t.title}</strong>
        <button
          onClick={importar}
          style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 14px", background: "var(--accent)", color: "#fff", border: "none", borderRadius: 6, fontSize: 13, fontWeight: 500, cursor: "pointer", fontFamily: "inherit" }}
        >
          <Upload size={13} /> {t.importMine}
        </button>
      </div>
      <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 8px" }}>{t.desc}</p>
      {erro && <div style={{ fontSize: 12, color: "var(--danger)", marginBottom: 8 }}>{erro}</div>}
      {docs.length === 0 ? (
        <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{t.empty}</div>
      ) : (
        docs.map((d) => (
          <div key={d.id} style={{ display: "flex", alignItems: "center", gap: 10, padding: "6px 0", borderTop: "1px solid var(--border)", fontSize: 13 }}>
            <span style={{ flex: 1, color: "var(--text-primary)" }}>{d.nome}</span>
            <span style={{ color: "var(--text-secondary)", fontSize: 12 }}>
              {d.origem === "importado" ? t.imported : t.generated} · {d.formato.toUpperCase()} · {Math.ceil(d.tamanho / 1024)} KB
            </span>
            <button
              onClick={() => marcar(d.id)}
              title={t.makePreferred}
              style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 10px", borderRadius: 6, border: "1px solid var(--border)", background: preferido === d.id ? "var(--accent-soft)" : "var(--bg-base)", color: preferido === d.id ? "var(--accent-strong)" : "var(--text-secondary)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}
            >
              <Star size={12} fill={preferido === d.id ? "currentColor" : "none"} /> {preferido === d.id ? t.preferred : t.makePreferred}
            </button>
          </div>
        ))
      )}
    </div>
  );
};
