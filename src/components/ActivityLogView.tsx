import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "../i18n";

interface EventRow {
  id: number;
  ts: string;
  kind: string;
  session_id: number | null;
  vaga_id: number | null;
  tool: string | null;
  target: string | null;
  role: string | null;
  text: string | null;
  url: string | null;
  result: string | null;
  error: string | null;
  duration_ms: number | null;
  shot_before: string | null;
  shot_after: string | null;
  extra: string | null;
}

const Shot: React.FC<{ path: string | null; label: string }> = ({ path, label }) => {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    setSrc(null);
    if (path) invoke<string>("activity_log_image", { path }).then(setSrc).catch(() => setSrc(null));
  }, [path]);
  if (!path) return null;
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>{label}</div>
      {src ? (
        <img src={src} alt={label} style={{ maxWidth: "100%", border: "1px solid var(--border)", borderRadius: 6 }} />
      ) : null}
    </div>
  );
};

export const ActivityLogView: React.FC = () => {
  const t = useT().settings.devMode;
  const [rows, setRows] = useState<EventRow[]>([]);
  const [kind, setKind] = useState<string>("");
  const [open, setOpen] = useState<EventRow | null>(null);

  const load = useCallback(() => {
    invoke<EventRow[]>("activity_log_list", { sessionId: null, vagaId: null, kind: kind || null })
      .then(setRows)
      .catch(() => setRows([]));
  }, [kind]);
  useEffect(load, [load]);

  const clear = async () => {
    if (!window.confirm(t.clearConfirm)) return;
    await invoke("activity_log_clear");
    setOpen(null);
    load();
  };

  const kinds: [string, string][] = [
    ["", t.filterAll],
    ["action", t.kindAction],
    ["failure", t.kindFailure],
    ["navigation", t.kindNavigation],
    ["browser", t.kindBrowser],
  ];

  return (
    <div style={{ marginTop: 12 }}>
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
        <strong style={{ fontSize: 14 }}>{t.logTitle}</strong>
        <select value={kind} onChange={(e) => setKind(e.target.value)}>
          {kinds.map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </select>
        <button onClick={load}>{t.refresh}</button>
        <button onClick={clear}>{t.clear}</button>
      </div>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{t.limitNote}</div>
      {rows.length === 0 ? (
        <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>{t.empty}</div>
      ) : (
        <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ textAlign: "left" }}>
              <th>{t.colTime}</th>
              <th>{t.colTool}</th>
              <th>{t.colTarget}</th>
              <th>{t.colResult}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr
                key={r.id}
                onClick={() => setOpen(r)}
                style={{ cursor: "pointer", background: r.kind === "failure" ? "var(--accent-soft)" : undefined }}
              >
                <td>{r.ts.replace("T", " ").replace("Z", "")}</td>
                <td>{r.tool}</td>
                <td>{r.target ?? r.url}</td>
                <td>{r.result}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {open && (
        <div style={{ marginTop: 12, padding: 12, border: "1px solid var(--border)", borderRadius: 8 }}>
          <div style={{ display: "flex", gap: 12 }}>
            <Shot path={open.shot_before} label={t.before} />
            <Shot path={open.shot_after} label={t.after} />
          </div>
          <div style={{ marginTop: 8, fontSize: 12 }}>{t.details}</div>
          <pre style={{ fontSize: 12, whiteSpace: "pre-wrap" }}>{JSON.stringify(open, null, 2)}</pre>
        </div>
      )}
    </div>
  );
};
