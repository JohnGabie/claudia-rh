import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ExternalLink, Folder } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";

interface Vaga {
  id: number;
  titulo: string;
  empresa: string;
  plataforma: string;
  url: string;
  localizacao: string | null;
  modelo_trabalho: string | null;
  descoberta_em: string;
  status: string;
  motivo_status: string | null;
  match_score: string | null;
}

interface Candidatura {
  id: number;
  vaga_id: number;
  titulo: string;
  empresa: string;
  plataforma: string;
  url: string;
  enviada_em: string;
  pasta_arquivos: string;
  metodo: string;
  resultado: string | null;
}

const STATUS_LABELS: Record<string, string> = {
  descoberta: "Descoberta",
  analisada: "Analisada",
  candidatando: "A candidatar",
  aplicada: "Aplicada",
  pulada: "Pulada",
  pendente_revisao: "Pendente revisão",
  bloqueada: "Bloqueada",
};

const STATUS_STYLE: Record<string, { background: string; color: string }> = {
  descoberta: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  analisada: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  candidatando: { background: "var(--accent-soft)", color: "var(--accent-strong)" },
  aplicada: { background: "#E3EFE7", color: "var(--success)" },
  pulada: { background: "var(--bg-sunken)", color: "var(--text-tertiary)" },
  pendente_revisao: { background: "#FBEFD9", color: "var(--warning)" },
  bloqueada: { background: "#F7E2DF", color: "var(--danger)" },
};

const FILTROS = [
  { key: "todas", label: "Todas" },
  { key: "descoberta", label: "Descoberta" },
  { key: "analisada", label: "Analisada" },
  { key: "pendente_revisao", label: "Pendente revisão" },
  { key: "bloqueada", label: "Bloqueada" },
  { key: "pulada", label: "Pulada" },
];

const METODO_LABEL: Record<string, string> = {
  chrome: "Chrome",
  formulario: "Formulário",
  email: "E-mail",
  linkedin: "LinkedIn",
  manual: "Manual",
};

const RESULTADO_OPCOES = [
  { value: null, label: "—", color: "var(--text-tertiary)" },
  { value: "sem_resposta", label: "Sem resposta", color: "var(--text-tertiary)" },
  { value: "rejeitada", label: "Rejeitada", color: "var(--danger)" },
  { value: "entrevista", label: "Entrevista", color: "var(--warning)" },
  { value: "oferta", label: "Oferta", color: "var(--success)" },
];

const ResultadoPicker: React.FC<{
  candidaturaId: number;
  resultado: string | null;
  onUpdate: () => void;
}> = ({ candidaturaId, resultado, onUpdate }) => {
  const [busy, setBusy] = useState(false);
  const atual = RESULTADO_OPCOES.find(o => o.value === resultado) ?? RESULTADO_OPCOES[0];

  const selecionar = async (valor: string | null) => {
    setBusy(true);
    await invoke("marcar_resultado_candidatura", { id: candidaturaId, resultado: valor })
      .catch(console.error);
    setBusy(false);
    onUpdate();
  };

  return (
    <select
      value={resultado ?? ""}
      onChange={(e) => selecionar(e.target.value || null)}
      disabled={busy}
      style={{
        fontSize: 12, fontFamily: "inherit",
        padding: "2px 4px", borderRadius: 4,
        border: "1px solid var(--border)", background: "var(--bg-surface)",
        color: atual.color, cursor: "pointer",
        outline: "none",
      }}
    >
      {RESULTADO_OPCOES.map(o => (
        <option key={o.value ?? ""} value={o.value ?? ""}>{o.label}</option>
      ))}
    </select>
  );
};

export const Vagas: React.FC = () => {
  const [subView, setSubView] = useState<"todas" | "historico">("todas");
  const [filtro, setFiltro] = useState("todas");
  const [vagas, setVagas] = useState<Vaga[]>([]);
  const [candidaturas, setCandidaturas] = useState<Candidatura[]>([]);
  const [loading, setLoading] = useState(true);

  const carregarVagas = () => {
    setLoading(true);
    const f = filtro === "todas" ? null : filtro;
    invoke<Vaga[]>("listar_vagas", { filtro: f })
      .then(setVagas)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  const carregarCandidaturas = () => {
    setLoading(true);
    invoke<Candidatura[]>("listar_candidaturas")
      .then(setCandidaturas)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (subView === "historico") {
      carregarCandidaturas();
    } else {
      carregarVagas();
    }
  }, [subView, filtro]);

  useEffect(() => {
    const unsub = listen("db-atualizada", () => {
      if (subView === "historico") carregarCandidaturas();
      else carregarVagas();
    });
    return () => { unsub.then((f) => f()); };
  }, [subView, filtro]);

  const pill = (active: boolean, onClick: () => void, label: string) => (
    <button
      key={label}
      onClick={onClick}
      style={{
        padding: "4px 14px",
        borderRadius: 6,
        border: "none",
        cursor: "pointer",
        fontSize: 13,
        fontWeight: active ? 500 : 400,
        fontFamily: "inherit",
        background: active ? "var(--accent-soft)" : "transparent",
        color: active ? "var(--accent-strong)" : "var(--text-secondary)",
        transition: "background 0.15s, color 0.15s",
      }}
    >
      {label}
    </button>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Header */}
      <div style={{
        padding: "16px 24px 0",
        borderBottom: "1px solid var(--border)",
        background: "var(--bg-surface)",
        flexShrink: 0,
      }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", marginBottom: 12 }}>
          Vagas
        </h1>

        <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
          {pill(subView === "todas", () => setSubView("todas"), "Todas as vagas")}
          {pill(subView === "historico", () => setSubView("historico"), "Histórico de candidaturas")}
        </div>

        {subView === "todas" && (
          <div style={{ display: "flex", gap: 4, paddingBottom: 12, flexWrap: "wrap" }}>
            {FILTROS.map((f) =>
              pill(filtro === f.key, () => setFiltro(f.key), f.label)
            )}
          </div>
        )}
      </div>

      {/* Content */}
      <div style={{ flex: 1, overflow: "auto", padding: "0 24px 24px" }}>
        {loading ? (
          <div style={{ padding: "24px 0", color: "var(--text-tertiary)", fontSize: 14 }}>
            A carregar…
          </div>
        ) : subView === "todas" ? (
          vagas.length === 0 ? (
            <div style={{ padding: "24px 0", color: "var(--text-secondary)", fontSize: 14 }}>
              Nenhuma vaga encontrada.
            </div>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 14 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border)" }}>
                  {["Título", "Empresa", "Plataforma", "Status", "Descoberta", ""].map((h) => (
                    <th key={h} style={{
                      padding: "10px 12px",
                      textAlign: "left",
                      fontSize: 12,
                      fontWeight: 500,
                      color: "var(--text-secondary)",
                      whiteSpace: "nowrap",
                    }}>
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {vagas.map((v) => {
                  const s = STATUS_STYLE[v.status] ?? STATUS_STYLE.descoberta;
                  const data = new Date(v.descoberta_em).toLocaleDateString("pt-PT", {
                    day: "2-digit", month: "short",
                  });
                  return (
                    <tr key={v.id} style={{ borderBottom: "1px solid var(--border)" }}>
                      <td style={{ padding: "10px 12px", maxWidth: 260 }}>
                        <div style={{
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                          fontWeight: 500, color: "var(--text-primary)",
                        }}>
                          {v.titulo}
                        </div>
                      </td>
                      <td style={{ padding: "10px 12px", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>
                        {v.empresa}
                      </td>
                      <td style={{ padding: "10px 12px", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>
                        {v.plataforma}
                      </td>
                      <td style={{ padding: "10px 12px" }}>
                        <span
                          title={v.motivo_status ?? undefined}
                          style={{ fontSize: 12, fontWeight: 500, padding: "2px 8px", borderRadius: 6, ...s }}
                        >
                          {STATUS_LABELS[v.status] ?? v.status}
                        </span>
                        {v.motivo_status && (
                          <div style={{
                            fontSize: 11, color: "var(--text-tertiary)", marginTop: 3,
                            maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                          }} title={v.motivo_status}>
                            {v.motivo_status}
                          </div>
                        )}
                      </td>
                      <td style={{ padding: "10px 12px", color: "var(--text-tertiary)", whiteSpace: "nowrap", fontSize: 12 }}>
                        {data}
                      </td>
                      <td style={{ padding: "10px 12px" }}>
                        <button
                          onClick={() => openUrl(v.url).catch(console.error)}
                          title="Abrir vaga"
                          style={{
                            background: "transparent", border: "none", cursor: "pointer",
                            padding: 4, color: "var(--text-tertiary)", display: "flex", alignItems: "center",
                          }}
                        >
                          <ExternalLink size={14} />
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )
        ) : (
          candidaturas.length === 0 ? (
            <div style={{ padding: "24px 0", color: "var(--text-secondary)", fontSize: 14 }}>
              Nenhuma candidatura enviada ainda.
            </div>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 14 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border)" }}>
                  {["Título", "Empresa", "Plataforma", "Método", "Resultado", "Enviada", "", ""].map((h, i) => (
                    <th key={i} style={{
                      padding: "10px 12px",
                      textAlign: "left",
                      fontSize: 12,
                      fontWeight: 500,
                      color: "var(--text-secondary)",
                      whiteSpace: "nowrap",
                    }}>
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {candidaturas.map((c) => {
                  const data = new Date(c.enviada_em).toLocaleDateString("pt-PT", {
                    day: "2-digit", month: "short", year: "2-digit",
                  });
                  return (
                    <tr key={c.id} style={{ borderBottom: "1px solid var(--border)" }}>
                      <td style={{ padding: "10px 12px", maxWidth: 240 }}>
                        <div style={{
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                          fontWeight: 500, color: "var(--text-primary)",
                        }}>
                          {c.titulo}
                        </div>
                      </td>
                      <td style={{ padding: "10px 12px", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>
                        {c.empresa}
                      </td>
                      <td style={{ padding: "10px 12px", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>
                        {c.plataforma}
                      </td>
                      <td style={{ padding: "10px 12px" }}>
                        <span style={{
                          fontSize: 12, fontWeight: 500, padding: "2px 8px", borderRadius: 6,
                          background: "var(--accent-soft)", color: "var(--accent-strong)",
                        }}>
                          {METODO_LABEL[c.metodo] ?? c.metodo}
                        </span>
                      </td>
                      <td style={{ padding: "10px 12px" }}>
                        <ResultadoPicker
                          candidaturaId={c.id}
                          resultado={c.resultado}
                          onUpdate={carregarCandidaturas}
                        />
                      </td>
                      <td style={{ padding: "10px 12px", color: "var(--text-tertiary)", whiteSpace: "nowrap", fontSize: 12 }}>
                        {data}
                      </td>
                      <td style={{ padding: "10px 12px" }}>
                        {c.pasta_arquivos && (
                          <button
                            onClick={() => invoke("abrir_pasta", { caminho: c.pasta_arquivos }).catch(console.error)}
                            title="Abrir pasta de ficheiros"
                            style={{
                              background: "transparent", border: "none", cursor: "pointer",
                              padding: 4, color: "var(--text-tertiary)", display: "flex", alignItems: "center",
                            }}
                          >
                            <Folder size={14} />
                          </button>
                        )}
                      </td>
                      <td style={{ padding: "10px 12px" }}>
                        <button
                          onClick={() => openUrl(c.url).catch(console.error)}
                          title="Abrir vaga"
                          style={{
                            background: "transparent", border: "none", cursor: "pointer",
                            padding: 4, color: "var(--text-tertiary)", display: "flex", alignItems: "center",
                          }}
                        >
                          <ExternalLink size={14} />
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )
        )}
      </div>
    </div>
  );
};
