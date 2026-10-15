import React, { useState } from "react";
import { useT } from "../../i18n";
import { rotulo } from "./rotulos";

const TIPOS = ["geral", "regional", "tecnologia", "remoto", "freelance", "empresa", "nicho"];

const campo: React.CSSProperties = {
  padding: "7px 10px", borderRadius: 6, border: "1px solid var(--border)", background: "var(--bg-surface)",
  fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit", minWidth: 0, boxSizing: "border-box",
};

interface Props {
  onSalvar: (nome: string, url: string, tipo: string, pais: string | null) => Promise<void>;
  onCancelar: () => void;
}

// The backend validates; this panel only forwards the fields and shows the message it returns.
export const FormAdicionar: React.FC<Props> = ({ onSalvar, onCancelar }) => {
  const t = useT();
  const [nome, setNome] = useState("");
  const [url, setUrl] = useState("");
  const [tipo, setTipo] = useState("geral");
  const [pais, setPais] = useState("");
  const [erro, setErro] = useState<string | null>(null);
  const [salvando, setSalvando] = useState(false);

  const enviar = async (e: React.FormEvent) => {
    e.preventDefault();
    setErro(null);
    setSalvando(true);
    try {
      await onSalvar(nome, url, tipo, pais.trim() ? pais.trim() : null);
    } catch (err) {
      setErro(String(err));
      setSalvando(false);
    }
  };

  return (
    <form
      onSubmit={enviar}
      aria-label={t.sites.form.title}
      style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: 16, marginBottom: 16 }}
    >
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 10 }}>
        <input aria-label={t.sites.form.name} placeholder={t.sites.form.name} value={nome} onChange={(e) => setNome(e.target.value)} style={campo} autoFocus />
        <input aria-label={t.sites.form.url} placeholder={t.sites.form.url} value={url} onChange={(e) => setUrl(e.target.value)} style={campo} />
        <select aria-label={t.sites.form.type} value={tipo} onChange={(e) => setTipo(e.target.value)} style={campo}>
          {TIPOS.map((v) => <option key={v} value={v}>{rotulo(t.sites.types, v)}</option>)}
        </select>
        <input aria-label={t.sites.form.country} placeholder={t.sites.form.country} value={pais} onChange={(e) => setPais(e.target.value)} style={campo} />
      </div>
      {erro && <div role="alert" style={{ fontSize: 13, color: "var(--danger)", marginTop: 10, overflowWrap: "anywhere" }}>{erro}</div>}
      <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
        <button
          type="submit" disabled={salvando}
          style={{ padding: "7px 14px", borderRadius: 8, fontSize: 13, fontFamily: "inherit", cursor: salvando ? "default" : "pointer", background: "var(--accent)", border: "none", color: "#fff", opacity: salvando ? 0.7 : 1 }}
        >
          {salvando ? t.sites.form.saving : t.sites.form.save}
        </button>
        <button
          type="button" onClick={onCancelar}
          style={{ padding: "7px 14px", borderRadius: 8, fontSize: 13, fontFamily: "inherit", cursor: "pointer", background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)" }}
        >
          {t.sites.form.cancel}
        </button>
      </div>
    </form>
  );
};
