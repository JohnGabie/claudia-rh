import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { Check, ExternalLink, FolderOpen, RefreshCw } from "lucide-react";

// ── sub-components ────────────────────────────────────────────────────────────

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 16 }}>
    {children}
  </div>
);

const Section: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ marginBottom: 32 }}>{children}</div>
);

const ToggleSwitch: React.FC<{ checked: boolean; onChange: () => void }> = ({ checked, onChange }) => (
  <button
    role="switch"
    aria-checked={checked}
    onMouseDown={(e) => { e.preventDefault(); onChange(); }}
    style={{
      width: 40, height: 22, borderRadius: 11, flexShrink: 0, padding: 0,
      background: checked ? "var(--accent)" : "var(--bg-sunken)",
      border: `1px solid ${checked ? "var(--accent)" : "var(--border)"}`,
      cursor: "pointer", position: "relative", transition: "background 0.2s, border-color 0.2s",
    }}
  >
    <span style={{
      position: "absolute", top: 2, left: checked ? 20 : 2, width: 16, height: 16,
      borderRadius: "50%", background: checked ? "#fff" : "var(--text-tertiary)",
      transition: "left 0.2s, background 0.2s", display: "block",
    }} />
  </button>
);

// ── constants ─────────────────────────────────────────────────────────────────

const PROMPTS = [
  {
    id: "runtime",
    label: "Sessão de execução",
    desc: "Prompt injetado em cada sessão automática de candidaturas. Contém o perfil, estratégia e regras de comportamento do agente.",
  },
  {
    id: "perfil",
    label: "Assistente de Perfil",
    desc: "Instruções para o chat de construção de perfil. Define como a Claudia interpreta e edita candidate_base.yaml e search_variants.yaml.",
  },
  {
    id: "feedback",
    label: "Análise de Feedback",
    desc: "Instruções para geração de relatórios de feedback. Define estrutura, tom e regras de inferência a partir dos dados de candidaturas.",
  },
  {
    id: "cover_letter_pt",
    label: "Cover Letter — Português",
    desc: "Template para cartas de apresentação em PT. Inclui regras de estilo, estrutura obrigatória e palavras proibidas.",
  },
  {
    id: "cover_letter_en",
    label: "Cover Letter — English",
    desc: "Template for cover letters in EN. Same rules as PT variant but in English, with substitution-test enforcement.",
  },
];

// ── main component ────────────────────────────────────────────────────────────

export const Configuracoes: React.FC = () => {
  const [estrategia, setEstrategia] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const savedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [modoAutonomo, setModoAutonomo] = useState(false);
  const [iniciarComSistema, setIniciarComSistema] = useState(false);

  const [appVersion, setAppVersion] = useState("");
  const [updateStatus, setUpdateStatus] = useState<"idle" | "checking" | "found" | "latest">("idle");
  const [updateVersion, setUpdateVersion] = useState("");
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    invoke<string>("ler_estrategia")
      .then((e) => setEstrategia(e ?? ""))
      .catch(console.error)
      .finally(() => setLoading(false));

    invoke<boolean>("obter_modo_autonomo").then((v) => setModoAutonomo(!!v)).catch(() => {});
    invoke<boolean>("obter_iniciar_com_sistema").then((v) => setIniciarComSistema(!!v)).catch(() => {});
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  const verificarAtualizacao = async () => {
    setUpdateStatus("checking");
    try {
      const info = await invoke<{ version: string; body: string } | null>("verificar_atualizacao");
      if (info) {
        setUpdateVersion(info.version);
        setUpdateStatus("found");
      } else {
        setUpdateStatus("latest");
      }
    } catch {
      setUpdateStatus("idle");
    }
  };

  const instalarAtualizacao = async () => {
    setInstalling(true);
    try {
      await invoke("instalar_atualizacao");
    } finally {
      setInstalling(false);
    }
  };

  const salvarModoAutonomo = (ativo: boolean) => {
    invoke("configurar_modo_autonomo", { ativo }).catch(console.error);
  };

  const salvarIniciarComSistema = (ativo: boolean) => {
    invoke("configurar_iniciar_com_sistema", { ativo }).catch(console.error);
  };

  const guardar = async () => {
    setSaving(true);
    try {
      await invoke("guardar_estrategia", { conteudo: estrategia });
      setSaved(true);
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
      savedTimerRef.current = setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error(e);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <div style={{ padding: 24, color: "var(--text-tertiary)", fontSize: 14 }}>A carregar…</div>;
  }


  return (
    <div style={{ padding: 24, paddingBottom: 80 }}>
      <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", marginBottom: 24 }}>
        Configurações
      </h1>

      {/* 1. Iniciar com o sistema */}
      <Section>
        <SectionTitle>Iniciar com o sistema</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          Quando ativo, a Claudia RH abre automaticamente ao iniciar o Windows.
        </p>
        <label style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer" }}>
          <ToggleSwitch
            checked={iniciarComSistema}
            onChange={() => { const v = !iniciarComSistema; setIniciarComSistema(v); salvarIniciarComSistema(v); }}
          />
          <span style={{ fontSize: 14, color: "var(--text-primary)" }}>
            Iniciar Claudia RH ao ligar o computador
          </span>
        </label>
      </Section>

      {/* 2. Modo autónomo */}
      <Section>
        <SectionTitle>Modo autónomo do agente</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          Quando <strong>inativo</strong> (recomendado), o Claude pede confirmação no terminal antes de cada ação. Quando <strong>ativo</strong>, age sem interrupções — necessário para sessões completamente não supervisionadas.
        </p>
        <label style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer" }}>
          <ToggleSwitch
            checked={modoAutonomo}
            onChange={() => { const v = !modoAutonomo; setModoAutonomo(v); salvarModoAutonomo(v); }}
          />
          <span style={{ fontSize: 14, color: "var(--text-primary)" }}>
            Pular confirmações de permissão (--dangerously-skip-permissions)
          </span>
        </label>
        {modoAutonomo && (
          <p style={{ fontSize: 12, color: "var(--warning)", marginTop: 10, lineHeight: 1.5 }}>
            ⚠️ Com isto ativo, o agente age sem travas técnicas. As únicas restrições passam a ser as regras do prompt — certifica-te de que a estratégia e o perfil estão bem configurados.
          </p>
        )}
      </Section>

      {/* 4. Estratégia de busca */}
      <Section>
        <SectionTitle>Estratégia de busca</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 12, lineHeight: 1.5 }}>
          Texto livre injetado no prompt de execução em cada sessão. Usa para definir foco, restrições temporárias, ou notas para o agente.
        </p>
        <textarea
          value={estrategia}
          onChange={(e) => setEstrategia(e.target.value)}
          placeholder={"# Estratégia ativa\n\nFoco atual, restrições, notas para o agente…"}
          rows={7}
          style={{
            width: "100%", padding: "7px 10px", borderRadius: 4,
            border: "1px solid var(--border)", background: "var(--bg-surface)",
            color: "var(--text-primary)", fontSize: 14, fontFamily: "inherit",
            boxSizing: "border-box", resize: "vertical", outline: "none", lineHeight: 1.5,
          }}
        />
        <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 8 }}>
          <button
            onClick={guardar}
            disabled={saving}
            style={{
              padding: "7px 20px",
              background: saved ? "var(--success)" : "var(--accent)",
              color: "#fff", border: "none", borderRadius: 6, fontSize: 13, fontWeight: 500,
              cursor: saving ? "default" : "pointer", fontFamily: "inherit",
              display: "flex", alignItems: "center", gap: 6, transition: "background 0.2s",
            }}
          >
            {saved ? <><Check size={13} /> Salvo</> : saving ? "Salvando…" : "Salvar"}
          </button>
        </div>
      </Section>

      {/* 4. Prompts do sistema */}
      <Section>
        <SectionTitle>Prompts do sistema</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          Todos os prompts usados pelo Claude estão em arquivos de texto na pasta de dados. Edite com qualquer editor — as alterações entram em vigor na próxima invocação, sem reiniciar a aplicação.
        </p>

        <div style={{
          border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden",
        }}>
          {PROMPTS.map((p, i) => (
            <div
              key={p.id}
              style={{
                display: "flex", alignItems: "center", gap: 16,
                padding: "12px 16px",
                borderBottom: i < PROMPTS.length - 1 ? "1px solid var(--border)" : "none",
                background: "var(--bg-surface)",
              }}
            >
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-primary)", marginBottom: 2 }}>
                  {p.label}
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.4 }}>
                  {p.desc}
                </div>
              </div>
              <button
                onClick={() => invoke("abrir_ficheiro_prompt", { id: p.id }).catch(console.error)}
                style={{
                  flexShrink: 0,
                  display: "flex", alignItems: "center", gap: 6,
                  padding: "6px 14px", borderRadius: 6,
                  border: "1px solid var(--border)",
                  background: "var(--bg-base)",
                  color: "var(--text-secondary)",
                  fontSize: 12, fontFamily: "inherit", cursor: "pointer",
                  transition: "color 0.1s, border-color 0.1s",
                }}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.color = "var(--text-primary)";
                  (e.currentTarget as HTMLElement).style.borderColor = "var(--text-tertiary)";
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.color = "var(--text-secondary)";
                  (e.currentTarget as HTMLElement).style.borderColor = "var(--border)";
                }}
              >
                <ExternalLink size={12} />
                Abrir
              </button>
            </div>
          ))}
        </div>

        <button
          onClick={() => invoke("abrir_pasta_dados").catch(console.error)}
          style={{
            marginTop: 12,
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 16px", borderRadius: 6,
            border: "1px solid var(--border)",
            background: "transparent",
            color: "var(--text-secondary)",
            fontSize: 13, fontFamily: "inherit", cursor: "pointer",
          }}
          onMouseEnter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-primary)")}
          onMouseLeave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-secondary)")}
        >
          <FolderOpen size={14} />
          Abrir pasta de dados
        </button>
      </Section>

      {/* 5. Atualizações */}
      <Section>
        <SectionTitle>Atualizações</SectionTitle>
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "12px 16px", borderRadius: 8,
          border: "1px solid var(--border)", background: "var(--bg-surface)",
        }}>
          <div>
            <div style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>
              Versão atual{appVersion ? `: ${appVersion}` : ""}
            </div>
            {updateStatus === "latest" && (
              <div style={{ fontSize: 12, color: "var(--success)", marginTop: 3 }}>
                Estás na versão mais recente
              </div>
            )}
            {updateStatus === "found" && (
              <div style={{ fontSize: 12, color: "var(--warning)", marginTop: 3 }}>
                Nova versão disponível: <strong>v{updateVersion}</strong>
              </div>
            )}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            {updateStatus === "found" && (
              <button
                onClick={instalarAtualizacao}
                disabled={installing}
                style={{
                  padding: "6px 16px", borderRadius: 6, border: "none",
                  background: "var(--warning)", color: "#fff",
                  fontSize: 12, fontWeight: 500, fontFamily: "inherit",
                  cursor: installing ? "default" : "pointer", opacity: installing ? 0.7 : 1,
                }}
              >
                {installing ? "A instalar…" : "Instalar e reiniciar"}
              </button>
            )}
            <button
              onClick={verificarAtualizacao}
              disabled={updateStatus === "checking"}
              style={{
                display: "flex", alignItems: "center", gap: 6,
                padding: "6px 14px", borderRadius: 6,
                border: "1px solid var(--border)", background: "var(--bg-base)",
                color: "var(--text-secondary)", fontSize: 12, fontFamily: "inherit",
                cursor: updateStatus === "checking" ? "default" : "pointer",
              }}
            >
              <RefreshCw size={12} style={{ animation: updateStatus === "checking" ? "spin 1s linear infinite" : "none" }} />
              {updateStatus === "checking" ? "A verificar…" : "Verificar atualizações"}
            </button>
          </div>
        </div>
      </Section>
    </div>
  );
};
