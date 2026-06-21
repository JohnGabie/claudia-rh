import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, AlertTriangle, Lightbulb, Pause, Play, Square } from "lucide-react";

interface Vaga {
  id: number;
  titulo: string;
  empresa: string;
  status: string;
  descoberta_em: string;
}

interface VagaAtual {
  id: number;
  titulo: string;
  empresa: string;
  url: string;
  etapa: string | null;
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

function tempoRelativo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "agora mesmo";
  if (mins < 60) return `há ${mins} min`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `há ${hours}h`;
  return `há ${Math.floor(hours / 24)}d`;
}

const BUDGET = 10;

export const Dashboard: React.FC = () => {
  const [candidaturasHoje, setCandidaturasHoje] = useState<number>(0);
  const [atividade, setAtividade] = useState<Vaga[]>([]);
  const [vagaAtual, setVagaAtual] = useState<VagaAtual | null>(null);
  const [pendenciasCount, setPendenciasCount] = useState<number>(0);
  const [propostas, setPropostas] = useState<number>(0);
  const [sessionActive, setSessionActive] = useState(false);
  const [paused, setPaused] = useState(false);
  const [loading, setLoading] = useState(true);
  const [disparando, setDisparando] = useState(false);
  const [disparado, setDisparado] = useState(false);

  const carregar = () =>
    Promise.all([
      invoke<number>("candidaturas_hoje"),
      invoke<Vaga[]>("atividade_recente"),
      invoke<VagaAtual | null>("vaga_atual_sessao"),
      invoke<number>("contar_pendencias"),
      invoke<number>("contar_propostas"),
    ]).then(([count, vagas, atual, pendencias, propsCount]) => {
      setCandidaturasHoje(count);
      setAtividade(vagas);
      setVagaAtual(atual);
      setPendenciasCount(pendencias);
      setPropostas(propsCount);
    }).catch(console.error).finally(() => setLoading(false));

  useEffect(() => {
    carregar();

    const unlistenCheckpoint = listen("session-checkpoint-requested", () => {
      setTimeout(() => {
        invoke("disparar_sessao", { motivo: "checkpoint" }).catch(console.error);
      }, 500);
    });

    const unlistenStarted = listen("session-started", () => {
      setSessionActive(true);
      carregar();
    });

    const unlistenEnded = listen<string>("session-ended", () => {
      setSessionActive(false);
      setPaused(false);
      carregar();
    });

    const unlistenDb = listen("db-atualizada", () => {
      carregar();
    });

    const unlistenChrome = listen("chrome-reconnect-failed", () => {
      console.warn("[Claudia RH] Chrome extension reconnection failed — check Pendências");
    });

    return () => {
      unlistenCheckpoint.then((f) => f());
      unlistenStarted.then((f) => f());
      unlistenEnded.then((f) => f());
      unlistenDb.then((f) => f());
      unlistenChrome.then((f) => f());
    };
  }, []);

  const disparar = async () => {
    setDisparando(true);
    try {
      await invoke("disparar_sessao", { motivo: "manual" });
      setDisparado(true);
      setTimeout(() => setDisparado(false), 3000);
      await carregar();
    } catch (e) {
      console.error(e);
    } finally {
      setDisparando(false);
    }
  };

  const pausar = async () => {
    try {
      await invoke("escrever_pty", { input: "\x03" });
      setPaused(true);
    } catch (e) { console.error(e); }
  };

  const retomar = async () => {
    try {
      await invoke("escrever_pty", { input: "continue\r" });
      setPaused(false);
    } catch (e) { console.error(e); }
  };

  const interromper = async () => {
    try {
      await invoke("parar_pty");
      setSessionActive(false);
      setPaused(false);
    } catch (e) { console.error(e); }
  };

  const pct = Math.min((candidaturasHoje / BUDGET) * 100, 100);

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 24 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0 }}>
          Dashboard
        </h1>
        {sessionActive && (
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{
              width: 8, height: 8, borderRadius: "50%",
              background: paused ? "var(--warning)" : "var(--success)",
              boxShadow: paused ? "none" : "0 0 0 0 rgba(34,197,94,0.4)",
              animation: paused ? "none" : "pulse 2s infinite",
              display: "inline-block",
            }} />
            <span style={{ fontSize: 13, color: paused ? "var(--warning)" : "var(--success)", fontWeight: 500 }}>
              {paused ? "Pausada" : "A trabalhar…"}
            </span>
          </div>
        )}
      </div>

      {/* Vaga em curso */}
      {vagaAtual && (
        <div style={{
          background: "var(--accent-soft)",
          border: "1px solid var(--accent)",
          borderRadius: 8,
          padding: "12px 16px",
          marginBottom: 12,
          display: "flex",
          alignItems: "center",
          gap: 12,
        }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 12, color: "var(--accent-strong)", fontWeight: 500, marginBottom: 2 }}>
              Agora a candidatar
            </div>
            <div style={{
              fontSize: 14, fontWeight: 500, color: "var(--text-primary)",
              overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
            }}>
              {vagaAtual.titulo}
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {vagaAtual.empresa}
              {vagaAtual.etapa && ` — ${vagaAtual.etapa}`}
            </div>
          </div>
          <button
            onClick={() => openUrl(vagaAtual.url).catch(console.error)}
            style={{
              background: "transparent", border: "none", cursor: "pointer",
              padding: 4, color: "var(--accent-strong)", flexShrink: 0,
              display: "flex", alignItems: "center",
            }}
          >
            <ExternalLink size={14} />
          </button>
        </div>
      )}

      {/* Pendências warning */}
      {pendenciasCount > 0 && (
        <div style={{
          background: "#FBEFD9",
          border: "1px solid var(--warning)",
          borderRadius: 8,
          padding: "10px 16px",
          marginBottom: 12,
          display: "flex",
          alignItems: "center",
          gap: 10,
          fontSize: 13,
          color: "var(--warning)",
          fontWeight: 500,
        }}>
          <AlertTriangle size={16} />
          {pendenciasCount === 1
            ? "1 pendência aguarda resolução"
            : `${pendenciasCount} pendências aguardam resolução`}
        </div>
      )}

      {/* Propostas de evolução */}
      {propostas > 0 && (
        <div style={{
          background: "var(--bg-surface)",
          border: "1px solid var(--border)",
          borderRadius: 8,
          padding: "10px 16px",
          marginBottom: 12,
          display: "flex",
          alignItems: "center",
          gap: 10,
          fontSize: 13,
          color: "var(--text-secondary)",
        }}>
          <Lightbulb size={16} style={{ color: "var(--accent)", flexShrink: 0 }} />
          <span>
            {propostas === 1
              ? "1 proposta de evolução do perfil disponível"
              : `${propostas} propostas de evolução do perfil disponíveis`}
          </span>
        </div>
      )}

      {/* Resumo do dia */}
      <div style={{
        background: "var(--bg-surface)",
        border: "1px solid var(--border)",
        borderRadius: 8,
        padding: 16,
        marginBottom: 16,
      }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
          Candidaturas hoje
        </div>
        <div style={{ fontSize: 28, fontWeight: 600, color: "var(--text-primary)", marginBottom: 10 }}>
          {loading ? "—" : candidaturasHoje}{" "}
          <span style={{ fontSize: 16, fontWeight: 400, color: "var(--text-secondary)" }}>
            / {BUDGET}
          </span>
        </div>
        <div style={{ height: 4, background: "var(--bg-sunken)", borderRadius: 2 }}>
          <div style={{
            width: `${pct}%`, height: "100%", background: "var(--accent)",
            borderRadius: 2, transition: "width 0.4s ease",
          }} />
        </div>
      </div>

      {/* Controlo de sessão */}
      {sessionActive ? (
        <div style={{ display: "flex", gap: 8, marginBottom: 24 }}>
          <button
            onClick={paused ? retomar : pausar}
            style={{
              flex: 1, padding: "11px 0", borderRadius: 8, fontSize: 14, fontWeight: 500,
              fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
              display: "flex", alignItems: "center", justifyContent: "center", gap: 7,
              background: paused ? "var(--accent)" : "var(--bg-sunken)",
              color: paused ? "#fff" : "var(--text-secondary)",
              border: paused ? "none" : "1px solid var(--border)",
            }}
          >
            {paused ? <><Play size={14} /> Retomar</> : <><Pause size={14} /> Pausar</>}
          </button>
          <button
            onClick={interromper}
            style={{
              padding: "11px 20px", borderRadius: 8, fontSize: 14, fontWeight: 500,
              fontFamily: "inherit", cursor: "pointer", transition: "background 0.15s",
              display: "flex", alignItems: "center", gap: 7,
              background: "#F7E2DF", color: "var(--danger)",
              border: "1px solid var(--danger)",
            }}
          >
            <Square size={13} fill="currentColor" /> Interromper
          </button>
        </div>
      ) : (
        <button
          onClick={disparar}
          disabled={disparando}
          style={{
            width: "100%", padding: "12px 0",
            background: disparado ? "var(--success)" : "var(--accent)",
            color: "#fff", border: "none", borderRadius: 8, fontSize: 15, fontWeight: 500,
            cursor: disparando ? "default" : "pointer", fontFamily: "inherit",
            marginBottom: 24, transition: "background 0.2s", opacity: disparando ? 0.8 : 1,
          }}
        >
          {disparando ? "A iniciar sessão…" : disparado ? "✓ Sessão iniciada — ver Terminal" : "Procurar vagas agora"}
        </button>
      )}

      {/* Atividade recente */}
      <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 12 }}>
        Atividade recente
      </div>

      {loading ? (
        <div style={{ color: "var(--text-tertiary)", fontSize: 14 }}>A carregar…</div>
      ) : atividade.length === 0 ? (
        <div style={{ color: "var(--text-secondary)", fontSize: 14 }}>Nenhuma atividade ainda.</div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {atividade.map((v) => {
            const s = STATUS_STYLE[v.status] ?? STATUS_STYLE.descoberta;
            return (
              <div key={v.id} style={{
                display: "flex", alignItems: "center", gap: 12,
                padding: "10px 0", borderBottom: "1px solid var(--border)",
              }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{
                    fontSize: 14, fontWeight: 500, color: "var(--text-primary)",
                    whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis",
                  }}>
                    {v.titulo}
                  </div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>
                    {v.empresa}
                  </div>
                </div>
                <span style={{
                  fontSize: 12, fontWeight: 500, padding: "2px 8px",
                  borderRadius: 6, whiteSpace: "nowrap", ...s,
                }}>
                  {STATUS_LABELS[v.status] ?? v.status}
                </span>
                <span style={{ fontSize: 12, color: "var(--text-tertiary)", whiteSpace: "nowrap" }}>
                  {tempoRelativo(v.descoberta_em)}
                </span>
              </div>
            );
          })}
        </div>
      )}

      <style>{`
        @keyframes pulse {
          0% { box-shadow: 0 0 0 0 rgba(34,197,94,0.4); }
          70% { box-shadow: 0 0 0 6px rgba(34,197,94,0); }
          100% { box-shadow: 0 0 0 0 rgba(34,197,94,0); }
        }
      `}</style>
    </div>
  );
};
