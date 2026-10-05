import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { LocaleProvider } from "./i18n";
import { warn, error } from "@tauri-apps/plugin-log";

function forwardConsole(fnName: "warn" | "error", logger: (m: string) => Promise<void>) {
  const original = console[fnName];
  console[fnName] = (...args: unknown[]) => {
    original(...args);
    logger(args.map(String).join(" ")).catch(() => {});
  };
}
forwardConsole("warn", warn);
forwardConsole("error", error);

async function start() {
  // Dev-only: in a plain browser (no Tauri) the screens run against an in-memory mock. Dead code in production.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    (await import("./lib/dev-mock")).instalarMockTauri();
  }
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <LocaleProvider>
        <App />
      </LocaleProvider>
    </React.StrictMode>,
  );
}
void start();
