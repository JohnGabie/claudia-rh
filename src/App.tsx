import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles/tokens.css";
import { TitleBar } from "./components/TitleBar";
import { Sidebar, type View } from "./components/Sidebar";
import { useT } from "./i18n";
import { Dashboard } from "./components/Dashboard";
import { Feedback } from "./components/Feedback";
import { Perfil } from "./components/Perfil";
import { TerminalView } from "./components/Terminal";
import { Vagas } from "./components/Vagas";
import { Configuracoes } from "./components/Configuracoes";
import { Pendencias } from "./components/Pendencias";
import { Welcome } from "./components/Welcome";

function App() {
  const t = useT();
  const [view, setView] = useState<View>("dashboard");
  const [sugerirFeedback, setSugerirFeedback] = useState(false);
  const [pendenciasCount, setPendenciasCount] = useState(0);
  const [propostasCount, setPropostasCount] = useState(0);
  const [perfilSection, setPerfilSection] = useState<string | null>(null);
  const handlePerfilSectionHandled = useCallback(() => setPerfilSection(null), []);
  const [updateInfo, setUpdateInfo] = useState<{ version: string; body: string } | null>(null);
  const [updateDismissed, setUpdateDismissed] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [showWelcome, setShowWelcome] = useState(false);

  const refreshFeedbackSugestao = () =>
    invoke<{ sugerir: boolean }>("sugerir_feedback").then(s => setSugerirFeedback(s.sugerir)).catch(() => {});

  const refreshPendenciasCount = () =>
    invoke<number>("contar_pendencias").then(setPendenciasCount).catch(() => {});

  const refreshPropostasCount = () =>
    invoke<number>("contar_propostas").then(setPropostasCount).catch(() => {});

  useEffect(() => {
    invoke<boolean>("welcome_necessario")
      .then(needed => { if (needed) setShowWelcome(true); })
      .catch(() => {});
    invoke<{ version: string; body: string } | null>("verificar_atualizacao")
      .then(info => { if (info) setUpdateInfo(info); })
      .catch(() => {});
  }, []);

  useEffect(() => {
    refreshFeedbackSugestao();
    refreshPendenciasCount();
    refreshPropostasCount();

    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen("feedback-output-done", () => setSugerirFeedback(false)),
      listen("nova-pendencia", refreshPendenciasCount),
      listen("pendencia-resolvida", refreshPendenciasCount),
      listen("nova-proposta", refreshPropostasCount),
      listen("proposta-resolvida", refreshPropostasCount),
      listen("db-atualizada", () => { refreshPendenciasCount(); refreshPropostasCount(); }),
    ]).then((fns) => {
      if (active) {
        unlisteners.push(...fns);
      } else {
        fns.forEach((f) => f());
      }
    });

    return () => {
      active = false;
      unlisteners.forEach((f) => f());
    };
  }, []);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <TitleBar />
      {updateInfo && !updateDismissed && (
        <div style={{
          display: "flex", alignItems: "center", gap: 12,
          padding: "8px 16px",
          background: "color-mix(in srgb, var(--warning) 12%, var(--bg-surface))",
          borderBottom: "1px solid color-mix(in srgb, var(--warning) 30%, var(--border))",
          fontSize: 13, color: "var(--text-primary)", flexShrink: 0,
        }}>
          <span style={{ flex: 1 }}>
            {t.app.updateAvailable}<strong>v{updateInfo.version}</strong>
          </span>
          <button
            onClick={() => {
              setInstalling(true);
              invoke("instalar_atualizacao")
                .then(() => setInstalling(false))
                .catch(() => setInstalling(false));
            }}
            disabled={installing}
            style={{
              padding: "4px 14px", borderRadius: 5, border: "none",
              background: "var(--warning)", color: "#fff",
              fontSize: 12, fontWeight: 500, fontFamily: "inherit",
              cursor: installing ? "default" : "pointer", opacity: installing ? 0.7 : 1,
            }}
          >
            {installing ? t.app.installing : t.app.installNow}
          </button>
          <button
            onClick={() => setUpdateDismissed(true)}
            style={{
              padding: "4px 10px", borderRadius: 5,
              border: "1px solid var(--border)", background: "transparent",
              color: "var(--text-secondary)", fontSize: 12, fontFamily: "inherit", cursor: "pointer",
            }}
          >
            {t.app.later}
          </button>
        </div>
      )}
      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        {showWelcome ? (
          <Welcome onComplete={(v) => { setShowWelcome(false); setView(v); }} />
        ) : (
          <>
            <Sidebar active={view} onChange={setView} sugerirFeedback={sugerirFeedback} pendenciasCount={pendenciasCount + propostasCount} />
            <main style={{ flex: 1, minWidth: 0, position: "relative", background: "var(--bg-base)" }}>

              {/* Dashboard */}
              <div style={{ display: view === "dashboard" ? "flex" : "none", flexDirection: "column", height: "100%", overflow: "auto" }}>
                <Dashboard onNavigate={(tab, section) => { setView(tab as View); if (section) setPerfilSection(section); }} />
              </div>

              {/* Perfil */}
              <div style={{ display: view === "perfil" ? "flex" : "none", flexDirection: "column", height: "100%", overflow: "hidden" }}>
                <Perfil initialSection={perfilSection} onSectionHandled={handlePerfilSectionHandled} />
              </div>

              {/* Terminal — nunca desmontado */}
              <div style={{ display: view === "terminal" ? "flex" : "none", flexDirection: "column", height: "100%", overflow: "hidden" }}>
                <TerminalView />
              </div>

              {/* Histórico */}
              <div style={{ display: view === "historico" ? "flex" : "none", flexDirection: "column", height: "100%", overflow: "hidden" }}>
                <Vagas />
              </div>

              {/* Feedback */}
              <div style={{ display: view === "feedback" ? "block" : "none", height: "100%", overflow: "auto" }}>
                <Feedback />
              </div>

              {/* Pendências */}
              <div style={{ display: view === "pendencias" ? "flex" : "none", flexDirection: "column", height: "100%", overflow: "auto" }}>
                <Pendencias onNavigateToPerfil={() => setView("perfil")} />
              </div>

              {/* Configurações */}
              <div style={{ display: view === "configuracoes" ? "block" : "none", height: "100%", overflow: "auto" }}>
                <Configuracoes onShowWelcome={() => setShowWelcome(true)} />
              </div>

            </main>
          </>
        )}
      </div>
    </div>
  );
}

export default App;
