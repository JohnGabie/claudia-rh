import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { Check, ExternalLink, FolderOpen, RefreshCw } from "lucide-react";
import { useT, useLocale } from "../i18n";
import { ToggleSwitch } from "./ui/ToggleSwitch";

// ── sub-components ────────────────────────────────────────────────────────────

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)", marginBottom: 16 }}>
    {children}
  </div>
);

const Section: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div style={{ marginBottom: 32 }}>{children}</div>
);


// ── main component ────────────────────────────────────────────────────────────

export const Configuracoes: React.FC<{ onShowWelcome?: () => void }> = ({ onShowWelcome }) => {
  const t = useT();
  const { locale, setLocale } = useLocale();
  const prompts = [
    { id: "runtime", label: t.settings.prompts.runtime.label, desc: t.settings.prompts.runtime.desc },
    { id: "perfil", label: t.settings.prompts.perfil.label, desc: t.settings.prompts.perfil.desc },
    { id: "feedback", label: t.settings.prompts.feedback.label, desc: t.settings.prompts.feedback.desc },
    { id: "cover_letter_pt", label: t.settings.prompts.cover_letter_pt.label, desc: t.settings.prompts.cover_letter_pt.desc },
    { id: "cover_letter_en", label: t.settings.prompts.cover_letter_en.label, desc: t.settings.prompts.cover_letter_en.desc },
  ];
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
    return <div style={{ padding: 24, color: "var(--text-tertiary)", fontSize: 14 }}>{t.common.loading}</div>;
  }


  return (
    <div style={{ padding: 24, paddingBottom: 80 }}>
      <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", marginBottom: 24 }}>
        {t.settings.title}
      </h1>

      {/* 0. Language */}
      <Section>
        <SectionTitle>{t.settings.language}</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          {t.settings.languageDesc}
        </p>
        <div style={{ display: "flex", gap: 8 }}>
          {(["en", "pt"] as const).map((l) => {
            const label = l === "en" ? t.settings.languageEn : t.settings.languagePt;
            const active = locale === l;
            return (
              <button
                key={l}
                onClick={() => setLocale(l)}
                style={{
                  padding: "7px 18px", borderRadius: 6,
                  border: `1px solid ${active ? "var(--accent)" : "var(--border)"}`,
                  background: active ? "var(--accent-soft)" : "transparent",
                  color: active ? "var(--accent-strong)" : "var(--text-secondary)",
                  fontSize: 13, fontWeight: active ? 600 : 400,
                  fontFamily: "inherit", cursor: "pointer",
                  transition: "all 0.15s",
                }}
              >
                {label}
              </button>
            );
          })}
        </div>
      </Section>

      {/* 1. Iniciar com o sistema */}
      <Section>
        <SectionTitle>{t.settings.startWithSystem}</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          {t.settings.startWithSystemDesc}
        </p>
        <label style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer" }}>
          <ToggleSwitch
            checked={iniciarComSistema}
            onChange={() => { const v = !iniciarComSistema; setIniciarComSistema(v); salvarIniciarComSistema(v); }}
          />
          <span style={{ fontSize: 14, color: "var(--text-primary)" }}>
            {t.settings.startOnBoot}
          </span>
        </label>
      </Section>

      {/* 2. Modo autónomo */}
      <Section>
        <SectionTitle>{t.settings.autonomousMode}</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          {t.settings.autonomousModeDesc1} <strong>{t.settings.autonomousModeDescInactive}</strong> {t.settings.autonomousModeDescRecommended},{t.settings.autonomousModeDesc2} <strong>{t.settings.autonomousModeDescActive}</strong>{t.settings.autonomousModeDesc3}
        </p>
        <label style={{ display: "flex", alignItems: "center", gap: 10, cursor: "pointer" }}>
          <ToggleSwitch
            checked={modoAutonomo}
            onChange={() => { const v = !modoAutonomo; setModoAutonomo(v); salvarModoAutonomo(v); }}
          />
          <span style={{ fontSize: 14, color: "var(--text-primary)" }}>
            {t.settings.skipPermissions}
          </span>
        </label>
        {modoAutonomo && (
          <p style={{ fontSize: 12, color: "var(--warning)", marginTop: 10, lineHeight: 1.5 }}>
            {t.settings.autonomousWarning}
          </p>
        )}
      </Section>

      {/* 4. Estratégia de busca */}
      <Section>
        <SectionTitle>{t.settings.searchStrategy}</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 12, lineHeight: 1.5 }}>
          {t.settings.searchStrategyDesc}
        </p>
        <textarea
          value={estrategia}
          onChange={(e) => setEstrategia(e.target.value)}
          placeholder={t.settings.searchStrategyPlaceholder}
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
            {saved ? <><Check size={13} /> {t.settings.savedBtn}</> : saving ? t.settings.savingBtn : t.settings.saveBtn}
          </button>
        </div>
      </Section>

      {/* 4. Prompts do sistema */}
      <Section>
        <SectionTitle>{t.settings.systemPrompts}</SectionTitle>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.5 }}>
          {t.settings.systemPromptsDesc}
        </p>

        <div style={{
          border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden",
        }}>
          {prompts.map((p, i) => (
            <div
              key={p.id}
              style={{
                display: "flex", alignItems: "center", gap: 16,
                padding: "12px 16px",
                borderBottom: i < prompts.length - 1 ? "1px solid var(--border)" : "none",
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
                {t.common.open}
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
          {t.settings.openDataFolder}
        </button>
      </Section>

      {/* 5. Atualizações */}
      <Section>
        <SectionTitle>{t.settings.updates}</SectionTitle>
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "12px 16px", borderRadius: 8,
          border: "1px solid var(--border)", background: "var(--bg-surface)",
        }}>
          <div>
            <div style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>
              {t.settings.currentVersion}{appVersion ? `: ${appVersion}` : ""}
            </div>
            {updateStatus === "latest" && (
              <div style={{ fontSize: 12, color: "var(--success)", marginTop: 3 }}>
                {t.settings.upToDate}
              </div>
            )}
            {updateStatus === "found" && (
              <div style={{ fontSize: 12, color: "var(--warning)", marginTop: 3 }}>
                {t.settings.updateFound}<strong>v{updateVersion}</strong>
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
                {installing ? t.app.installing : t.settings.installAndRestart}
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
              {updateStatus === "checking" ? t.settings.checkingUpdates : t.settings.checkUpdates}
            </button>
          </div>
        </div>
      </Section>

      {onShowWelcome && (
        <Section>
          <SectionTitle>Getting started</SectionTitle>
          <button
            onClick={onShowWelcome}
            style={{
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
            Show welcome screen
          </button>
        </Section>
      )}
    </div>
  );
};
