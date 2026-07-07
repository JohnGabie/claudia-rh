import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ExternalLink, Folder } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useT } from "../i18n";

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

const STATUS_STYLE: Record<string, { background: string; color: string }> = {
  descoberta: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  analisada: { background: "var(--bg-sunken)", color: "var(--text-secondary)" },
  candidatando: { background: "var(--accent-soft)", color: "var(--accent-strong)" },
  aplicada: { background: "#E3EFE7", color: "var(--success)" },
  pulada: { background: "var(--bg-sunken)", color: "var(--text-tertiary)" },
  pendente_revisao: { background: "#FBEFD9", color: "var(--warning)" },
  bloqueada: { background: "#F7E2DF", color: "var(--danger)" },
};

const ResultadoPicker: React.FC<{
  candidaturaId: number;
  resultado: string | null;
  onUpdate: () => void;
}> = ({ candidaturaId, resultado, onUpdate }) => {
  const t = useT();
  const RESULTADO_OPCOES = [
    { value: null, label: t.jobs.resultOptions.none, color: "var(--text-tertiary)" },
    { value: "sem_resposta", label: t.jobs.resultOptions.sem_resposta, color: "var(--text-tertiary)" },
    { value: "rejeitada", label: t.jobs.resultOptions.rejeitada, color: "var(--danger)" },
    { value: "entrevista", label: t.jobs.resultOptions.entrevista, color: "var(--warning)" },
    { value: "oferta", label: t.jobs.resultOptions.oferta, color: "var(--success)" },
  ];
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
  const t = useT();
  const FILTROS = [
    { key: "todas", label: t.jobs.filters.todas },
    { key: "descoberta", label: t.jobs.filters.descoberta },
    { key: "analisada", label: t.jobs.filters.analisada },
    { key: "pendente_revisao", label: t.jobs.filters.pendente_revisao },
    { key: "bloqueada", label: t.jobs.filters.bloqueada },
    { key: "pulada", label: t.jobs.filters.pulada },
  ];
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
    let active = true;
    let unlisten: (() => void) | undefined;

    listen("db-atualizada", () => {
      if (subView === "historico") carregarCandidaturas();
      else carregarVagas();
    }).then((fn) => {
      if (active) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      active = false;
      unlisten?.();
    };
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
          {t.jobs.title}
        </h1>

        <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
          {pill(subView === "todas", () => setSubView("todas"), t.jobs.allJobs)}
          {pill(subView === "historico", () => setSubView("historico"), t.jobs.applicationHistory)}
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
            {t.common.loading}
          </div>
        ) : subView === "todas" ? (
          vagas.length === 0 ? (
            <div style={{ padding: "24px 0", color: "var(--text-secondary)", fontSize: 14 }}>
              {t.jobs.noJobsFound}
            </div>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 14 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border)" }}>
                  {[t.jobs.tableHeaders.title, t.jobs.tableHeaders.company, t.jobs.tableHeaders.platform, t.jobs.tableHeaders.status, t.jobs.tableHeaders.discovered, ""].map((h) => (
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
                          {t.jobs.statusLabels[v.status as keyof typeof t.jobs.statusLabels] ?? v.status}
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
                          title={t.jobs.openJob}
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
              {t.jobs.noApplications}
            </div>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 14 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border)" }}>
                  {[t.jobs.tableHeaders.title, t.jobs.tableHeaders.company, t.jobs.tableHeaders.platform, t.jobs.tableHeaders.method, t.jobs.tableHeaders.result, t.jobs.tableHeaders.sent, "", ""].map((h, i) => (
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
                          {(t.jobs.methodLabels as Record<string, string>)[c.metodo] ?? c.metodo}
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
                            title={t.jobs.openFiles}
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
                          title={t.jobs.openJob}
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
