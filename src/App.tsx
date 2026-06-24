import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles/tokens.css";
import { TitleBar } from "./components/TitleBar";
import { Sidebar, type View } from "./components/Sidebar";
import { Dashboard } from "./components/Dashboard";
import { Feedback } from "./components/Feedback";
import { Perfil } from "./components/Perfil";
import { TerminalView } from "./components/Terminal";
import { Vagas } from "./components/Vagas";
import { Configuracoes } from "./components/Configuracoes";
import { Pendencias } from "./components/Pendencias";

function App() {
  const [view, setView] = useState<View>("dashboard");
  const [sugerirFeedback, setSugerirFeedback] = useState(false);
  const [pendenciasCount, setPendenciasCount] = useState(0);
  const [perfilSection, setPerfilSection] = useState<string | null>(null);
  const handlePerfilSectionHandled = useCallback(() => setPerfilSection(null), []);

  const refreshFeedbackSugestao = () =>
    invoke<{ sugerir: boolean }>("sugerir_feedback").then(s => setSugerirFeedback(s.sugerir)).catch(() => {});

  const refreshPendenciasCount = () =>
    invoke<number>("contar_pendencias").then(setPendenciasCount).catch(() => {});

  useEffect(() => {
    refreshFeedbackSugestao();
    refreshPendenciasCount();

    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen("feedback-output-done", () => setSugerirFeedback(false)),
      listen("nova-pendencia", refreshPendenciasCount),
      listen("pendencia-resolvida", refreshPendenciasCount),
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
      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        <Sidebar active={view} onChange={setView} sugerirFeedback={sugerirFeedback} pendenciasCount={pendenciasCount} />
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
            <Pendencias />
          </div>

          {/* Configurações */}
          <div style={{ display: view === "configuracoes" ? "block" : "none", height: "100%", overflow: "auto" }}>
            <Configuracoes />
          </div>

        </main>
      </div>
    </div>
  );
}

export default App;
