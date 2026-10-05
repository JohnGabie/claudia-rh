import React, { useEffect, useRef, useState } from "react";
import { FileText, Star } from "lucide-react";
import { useT } from "../../i18n";
import { docsApi } from "../../lib/documentos-api";
import type { Documento } from "../../types";
import { FolhaA4 } from "./FolhaA4";

const ESCALA_MINIATURA = 0.2;

export const Cartao: React.FC<{ doc: Documento; onAbrir: (id: number) => void }> = ({ doc, onAbrir }) => {
  const t = useT();
  const ref = useRef<HTMLDivElement>(null);
  const [html, setHtml] = useState<string | null>(null);

  // The thumbnail HTML is fetched once, when the card first comes near the viewport.
  useEffect(() => {
    if (!doc.tem_preview || html !== null) return;
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      (entries) => {
        if (!entries.some((en) => en.isIntersecting)) return;
        obs.disconnect();
        docsApi.obter(doc.id).then((d) => setHtml(d.preview_html)).catch(() => {});
      },
      { rootMargin: "200px" },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [doc.id, doc.tem_preview, html]);

  const varPref = doc.preferido_variantes;

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => onAbrir(doc.id)}
      onKeyDown={(e) => { if (e.key === "Enter") onAbrir(doc.id); }}
      style={{
        background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8,
        padding: 12, cursor: "pointer", display: "flex", flexDirection: "column", gap: 10, minWidth: 0,
      }}
      onMouseEnter={(e) => (e.currentTarget.style.borderColor = "var(--accent)")}
      onMouseLeave={(e) => (e.currentTarget.style.borderColor = "var(--border)")}
    >
      <div
        ref={ref}
        style={{
          height: 1123 * ESCALA_MINIATURA + 2, background: "var(--bg-sunken)", borderRadius: 6,
          display: "flex", alignItems: "center", justifyContent: "center", overflow: "hidden",
        }}
      >
        {doc.tem_preview && html ? (
          <FolhaA4 html={html} escala={ESCALA_MINIATURA} titulo={doc.nome} />
        ) : (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6, color: "var(--text-tertiary)" }}>
            <FileText size={28} />
            <span style={{ fontSize: 12, fontWeight: 600, letterSpacing: "0.06em" }}>{doc.formato.toUpperCase()}</span>
          </div>
        )}
      </div>

      <div style={{ display: "flex", alignItems: "flex-start", gap: 6 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 14, fontWeight: 500, color: "var(--text-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={doc.nome}>
            {doc.nome}
          </div>
          <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{t.documents.origin[doc.origem]}</div>
        </div>
        {doc.preferido_global && (
          <span title={t.documents.preferredGlobal} style={{ color: "var(--accent)", display: "flex" }}>
            <Star size={15} fill="currentColor" />
          </span>
        )}
      </div>

      {varPref.length > 0 && (
        <div style={{ fontSize: 11, color: "var(--accent-strong)" }} title={`${t.documents.preferredForVariant}: ${varPref.join(", ")}`}>
          {t.documents.preferredForVariant}: {varPref.join(", ")}
        </div>
      )}

      {doc.tags.length > 0 && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
          {doc.tags.map((tg) => (
            <span key={`${tg.tipo}:${tg.valor}`} style={{ fontSize: 11, padding: "1px 6px", borderRadius: 6, background: "var(--bg-sunken)", color: "var(--text-secondary)" }}>
              {tg.tipo === "cor" ? (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: tg.valor }} />{tg.valor}
                </span>
              ) : tg.valor}
            </span>
          ))}
        </div>
      )}
    </div>
  );
};
