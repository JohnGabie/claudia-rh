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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <LocaleProvider>
      <App />
    </LocaleProvider>
  </React.StrictMode>,
);
