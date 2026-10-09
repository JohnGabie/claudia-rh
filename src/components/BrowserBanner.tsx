import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useT } from "../i18n";

interface Status {
  stage: string;
  downloaded: number;
  total: number | null;
  message: string | null;
}

// Shows the preparation of the Claudia browser (download, start) and nothing once it is ready or waiting to be called.
// The "open the browser" button: launches it (it only runs when the AI is called) so the user can log into sites.
export const OpenBrowserButton: React.FC = () => {
  const t = useT().settings.browser;
  const [s, setS] = useState<Status | null>(null);
  useEffect(() => {
    invoke<Status>("browser_status").then(setS).catch(() => {});
    const un = listen<Status>("browser-progress", (e) => setS(e.payload));
    return () => {
      un.then((f) => f());
    };
  }, []);
  const busy = s?.stage === "downloading" || s?.stage === "launching";
  const running = s?.stage === "ready";
  return (
    <button
      onClick={() => invoke("browser_abrir").catch(() => {})}
      disabled={busy || running}
      style={{
        padding: "7px 20px", background: "var(--accent)", color: "#fff", border: "none", borderRadius: 6,
        fontSize: 13, fontWeight: 500, fontFamily: "inherit",
        cursor: busy || running ? "default" : "pointer", opacity: busy || running ? 0.55 : 1,
      }}
    >
      {busy ? t.opening : t.open}
    </button>
  );
};

export const BrowserBanner: React.FC = () => {
  const t = useT().browserSetup;
  const [s, setS] = useState<Status | null>(null);

  useEffect(() => {
    invoke<Status>("browser_status").then(setS).catch(() => {});
    const un = listen<Status>("browser-progress", (e) => setS(e.payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  // "installed" = on disk, launches when the AI is called: nothing to show.
  if (!s || s.stage === "idle" || s.stage === "ready" || s.stage === "installed") return null;
  const pct = s.total ? Math.min(100, Math.round((s.downloaded * 100) / s.total)) : null;
  const label = s.stage === "downloading" ? t.downloading : s.stage === "launching" ? t.launching : t.error;
  return (
    <div
      role="status"
      style={{
        padding: "8px 16px",
        background: s.stage === "error" ? "var(--bg-sunken)" : "var(--accent-soft)",
        color: "var(--accent-strong)",
        fontSize: 13,
        display: "flex",
        gap: 12,
        alignItems: "center",
      }}
    >
      <span>
        {label}
        {pct !== null ? ` — ${pct}%` : ""}
        {s.stage === "error" && s.message ? `: ${s.message}` : ""}
      </span>
      {s.stage === "error" && <button onClick={() => invoke("browser_retry")}>{t.retry}</button>}
    </div>
  );
};
