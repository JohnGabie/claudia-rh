import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { X, CheckCircle2, AlertTriangle, ShieldAlert, MousePointerClick, HelpCircle } from "lucide-react";

interface Pendencia {
  id: number;
  vaga_id: number;
  titulo_vaga: string;
  empresa_vaga: string;
  criada_em: string;
  categoria: string;
  descricao: string;
  resolvida: boolean;
  resolvida_em: string | null;
  resolucao: string | null;
}

const CATEGORIA_META: Record<string, { label: string; color: string; bg: string; Icon: React.FC<{ size: number }> }> = {
  captcha:               { label: "Captcha",            color: "var(--danger)",  bg: "rgba(184,71,61,0.08)",  Icon: MousePointerClick },
  dados_sensiveis:       { label: "Dados sensíveis",    color: "var(--danger)",  bg: "rgba(184,71,61,0.08)",  Icon: ShieldAlert },
  inventar_informacao:   { label: "Informação em falta",color: "var(--danger)",  bg: "rgba(184,71,61,0.08)",  Icon: ShieldAlert },
  red_line:              { label: "Red line",            color: "var(--danger)",  bg: "rgba(184,71,61,0.08)",  Icon: ShieldAlert },
  salario:               { label: "Salário fora da faixa",color:"var(--warning)",bg: "rgba(184,134,46,0.08)", Icon: AlertTriangle },
  pergunta_sem_resposta: { label: "Pergunta sem resposta",color:"var(--warning)",bg: "rgba(184,134,46,0.08)", Icon: HelpCircle },
  dialogo_bloqueante:    { label: "Diálogo bloqueante",  color:"var(--warning)",bg: "rgba(184,134,46,0.08)", Icon: AlertTriangle },
  extensao_chrome:       { label: "Extensão Chrome",     color:"var(--warning)",bg: "rgba(184,134,46,0.08)", Icon: AlertTriangle },
};

function resolveMeta(categoria: string) {
  const key = Object.keys(CATEGORIA_META).find((k) => categoria.toLowerCase().includes(k));
  return CATEGORIA_META[key ?? "red_line"] ?? CATEGORIA_META.red_line;
}

function tempoDesde(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "agora";
  if (mins < 60) return `${mins} min`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

interface CardProps {
  p: Pendencia;
  onResolved: () => void;
}

const NotifItem: React.FC<CardProps> = ({ p, onResolved }) => {
  const [expanded, setExpanded] = useState(false);
  const [resolving, setResolving] = useState(false);
  const [resolucaoText, setResolucaoText] = useState("");
  const [loading, setLoading] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  const meta = resolveMeta(p.categoria);
  const isCaptcha = p.categoria.toLowerCase().includes("captcha");

  const handleResolver = async () => {
    if (isCaptcha) {
      setLoading(true);
      try {
        await invoke("resolver_pendencia", { id: p.id, resolucao: "Captcha resolvido manualmente no Chrome" });
        onResolved();
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    } else {
      setResolving(true);
    }
  };

  const handleSubmit = async () => {
    if (!resolucaoText.trim()) return;
    setLoading(true);
    try {
      await invoke("resolver_pendencia", { id: p.id, resolucao: resolucaoText.trim() });
      onResolved();
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const handlePular = async () => {
    setLoading(true);
    try {
      await invoke("pular_pendencia", { id: p.id });
      setDismissed(true);
      onResolved();
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      style={{
        display: "flex",
        background: "var(--bg-surface)",
        border: "1px solid var(--border)",
        borderRadius: 10,
        overflow: "hidden",
        opacity: loading || dismissed ? 0.4 : 1,
        transition: "opacity 0.2s",
        pointerEvents: loading || dismissed ? "none" : "auto",
      }}
    >
      {/* Left accent bar */}
      <div style={{ width: 3, flexShrink: 0, background: meta.color, borderRadius: "10px 0 0 10px" }} />

      {/* Body */}
      <div style={{ flex: 1, padding: "12px 14px", minWidth: 0 }}>
        {/* Top row: category pill + time + dismiss */}
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
          <span style={{
            display: "inline-flex", alignItems: "center", gap: 4,
            fontSize: 11, fontWeight: 600,
            padding: "2px 7px", borderRadius: 5,
            color: meta.color, background: meta.bg,
            flexShrink: 0,
          }}>
            <meta.Icon size={10} />
            {meta.label}
          </span>
          <span style={{ fontSize: 11, color: "var(--text-tertiary)", marginLeft: "auto", flexShrink: 0 }}>
            {tempoDesde(p.criada_em)}
          </span>
          <button
            onClick={handlePular}
            title="Ignorar"
            style={{
              width: 20, height: 20,
              display: "flex", alignItems: "center", justifyContent: "center",
              background: "transparent", border: "none",
              borderRadius: 4, cursor: "pointer",
              color: "var(--text-tertiary)",
              flexShrink: 0,
              padding: 0,
              transition: "color 0.1s, background 0.1s",
            }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.color = "var(--text-primary)"; (e.currentTarget as HTMLElement).style.background = "var(--bg-sunken)"; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.color = "var(--text-tertiary)"; (e.currentTarget as HTMLElement).style.background = "transparent"; }}
          >
            <X size={13} />
          </button>
        </div>

        {/* Title + company */}
        <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", lineHeight: 1.3, marginBottom: 3 }}>
          {p.titulo_vaga}
        </div>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
          {p.empresa_vaga}
        </div>

        {/* Description — collapsed by default, expandable */}
        <div
          onClick={() => !resolving && setExpanded((e) => !e)}
          style={{
            fontSize: 12,
            color: "var(--text-secondary)",
            lineHeight: 1.5,
            cursor: resolving ? "default" : "pointer",
            display: "-webkit-box",
            WebkitBoxOrient: "vertical",
            WebkitLineClamp: expanded ? undefined : 2,
            overflow: expanded ? "visible" : "hidden",
            marginBottom: resolving ? 10 : 8,
          } as React.CSSProperties}
        >
          {p.descricao}
        </div>

        {/* Inline resolve input */}
        {resolving && (
          <div style={{ marginBottom: 10 }}>
            <textarea
              autoFocus
              value={resolucaoText}
              onChange={(e) => setResolucaoText(e.target.value)}
              placeholder="Como resolveste esta pendência?"
              rows={2}
              style={{
                width: "100%",
                padding: "7px 10px",
                borderRadius: 6,
                border: "1px solid var(--border)",
                background: "var(--bg-base)",
                color: "var(--text-primary)",
                fontSize: 12,
                fontFamily: "inherit",
                resize: "none",
                outline: "none",
                boxSizing: "border-box",
              }}
              onKeyDown={(e) => { if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) handleSubmit(); }}
            />
          </div>
        )}

        {/* Actions */}
        <div style={{ display: "flex", alignItems: "center", gap: 4, justifyContent: "flex-end" }}>
          {resolving ? (
            <>
              <button
                onClick={() => setResolving(false)}
                style={ghostBtnStyle}
              >
                Cancelar
              </button>
              <button
                onClick={handleSubmit}
                disabled={!resolucaoText.trim()}
                style={{ ...solidBtnStyle, opacity: resolucaoText.trim() ? 1 : 0.45 }}
              >
                Confirmar
              </button>
            </>
          ) : (
            <button
              onClick={handleResolver}
              style={solidBtnStyle}
            >
              {isCaptcha ? "Já resolvi no Chrome" : "Resolver"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

const ghostBtnStyle: React.CSSProperties = {
  background: "transparent",
  border: "none",
  cursor: "pointer",
  fontSize: 12,
  color: "var(--text-secondary)",
  fontFamily: "inherit",
  padding: "5px 10px",
  borderRadius: 6,
};

const solidBtnStyle: React.CSSProperties = {
  background: "var(--accent)",
  border: "none",
  borderRadius: 6,
  cursor: "pointer",
  fontSize: 12,
  fontWeight: 500,
  color: "#fff",
  fontFamily: "inherit",
  padding: "5px 12px",
};

export const Pendencias: React.FC<{ noHeader?: boolean }> = ({ noHeader = false }) => {
  const [pendencias, setPendencias] = useState<Pendencia[]>([]);
  const [loading, setLoading] = useState(true);

  const carregar = () => {
    invoke<Pendencia[]>("listar_pendencias", { apenasNaoResolvidas: true })
      .then(setPendencias)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    carregar();
    const unsub = listen("nova-pendencia", carregar);
    const unsubR = listen("pendencia-resolvida", carregar);
    return () => {
      unsub.then((f) => f());
      unsubR.then((f) => f());
    };
  }, []);

  return (
    <div style={{ padding: noHeader ? "12px 16px" : "24px 24px" }}>
      {!noHeader && (
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 20 }}>
          <h1 style={{ fontSize: 18, fontWeight: 600, color: "var(--text-primary)", flex: 1 }}>
            Notificações
          </h1>
          {pendencias.length > 0 && (
            <span style={{
              background: "var(--danger)",
              color: "#fff",
              fontSize: 11,
              fontWeight: 700,
              padding: "2px 8px",
              borderRadius: 6,
            }}>
              {pendencias.length}
            </span>
          )}
        </div>
      )}

      {loading ? (
        <div style={{ color: "var(--text-tertiary)", fontSize: 13 }}>A carregar…</div>
      ) : pendencias.length === 0 ? (
        <div style={{
          display: "flex", flexDirection: "column", alignItems: "center",
          padding: "48px 24px", gap: 10,
          color: "var(--text-tertiary)",
        }}>
          <CheckCircle2 size={32} strokeWidth={1.5} color="var(--success)" />
          <span style={{ fontSize: 13 }}>Sem pendências por resolver</span>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {pendencias.map((p) => (
            <NotifItem key={p.id} p={p} onResolved={carregar} />
          ))}
        </div>
      )}
    </div>
  );
};
