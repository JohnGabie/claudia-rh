import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, RefreshCw } from "lucide-react";
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

const PAGE = 100;

// The log stores UTC instants ("2026-10-03T12:34:56Z"); the screen shows them in the user's own time.
const formatTime = (ts: string): string => {
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return ts;
  return d.toLocaleString(undefined, { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit", second: "2-digit" });
};

const formatDuration = (ms: number | null): string => {
  if (ms === null) return "";
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;
};

const prettyExtra = (extra: string): string => {
  try {
    return JSON.stringify(JSON.parse(extra), null, 2);
  } catch {
    return extra;
  }
};

// Semantic colours only (design: green/amber/red are states, never decoration).
const isFailure = (r: EventRow) => r.kind === "failure";

const Shot: React.FC<{ path: string | null; label: string; none: string }> = ({ path, label, none }) => {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    setSrc(null);
    setFailed(false);
    if (path) invoke<string>("activity_log_image", { path }).then(setSrc).catch(() => setFailed(true));
  }, [path]);
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>{label}</div>
      {!path || failed ? (
        <div style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{none}</div>
      ) : src ? (
        <img src={src} alt={label} style={{ width: "100%", border: "1px solid var(--border)", borderRadius: 6, display: "block" }} />
      ) : (
        <div style={{ height: 80, background: "var(--bg-sunken)", borderRadius: 6 }} />
      )}
    </div>
  );
};

const fieldInput: React.CSSProperties = {
  width: 80, padding: "5px 8px", borderRadius: 6, border: "1px solid var(--border)", background: "var(--bg-surface)",
  color: "var(--text-primary)", fontSize: 13, fontFamily: "inherit",
};

// Read-only screen over the Activity log database. Nothing here writes: the app trims the log by itself.
export const ActivityLogScreen: React.FC<{ onBack: () => void }> = ({ onBack }) => {
  const t = useT().settings.activityLog;
  const [rows, setRows] = useState<EventRow[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [kind, setKind] = useState("");
  const [session, setSession] = useState("");
  const [vaga, setVaga] = useState("");
  const [selected, setSelected] = useState<EventRow | null>(null);
  const loadId = useRef(0);

  const toId = (v: string): number | null => (/^\d+$/.test(v.trim()) ? Number(v) : null);

  const load = useCallback(
    (append: boolean, beforeId: number | null) => {
      const mine = ++loadId.current;
      setLoading(true);
      invoke<EventRow[]>("activity_log_list", {
        sessionId: toId(session),
        vagaId: toId(vaga),
        kind: kind || null,
        beforeId,
        limit: PAGE,
      })
        .then((page) => {
          if (mine !== loadId.current) return; // a newer request replaced this one
          setRows((prev) => (append ? [...prev, ...page] : page));
          setHasMore(page.length === PAGE);
        })
        .catch(() => {
          if (mine === loadId.current) {
            setRows((prev) => (append ? prev : []));
            setHasMore(false);
          }
        })
        .finally(() => {
          if (mine === loadId.current) setLoading(false);
        });
    },
    [kind, session, vaga],
  );

  useEffect(() => {
    setSelected(null);
    load(false, null);
  }, [load]);

  const kinds: [string, string][] = [
    ["", t.filterAll],
    ["action", t.kindAction],
    ["failure", t.kindFailure],
    ["navigation", t.kindNavigation],
    ["browser", t.kindBrowser],
  ];
  const kindLabel = (k: string) => kinds.find(([v]) => v === k)?.[1] ?? k;
  const filtered = kind !== "" || session !== "" || vaga !== "";

  const field = (label: string, value: React.ReactNode) =>
    value === null || value === undefined || value === "" ? null : (
      <div style={{ display: "flex", gap: 12, padding: "4px 0", fontSize: 13 }}>
        <div style={{ width: 96, flexShrink: 0, color: "var(--text-secondary)" }}>{label}</div>
        <div style={{ minWidth: 0, wordBreak: "break-word", color: "var(--text-primary)" }}>{value}</div>
      </div>
    );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      {/* Header: back + title */}
      <div style={{ display: "flex", alignItems: "center", gap: 16, padding: "16px 24px 12px", borderBottom: "1px solid var(--border)" }}>
        <button
          onClick={onBack}
          style={{
            background: "none", border: "none", cursor: "pointer", display: "flex", alignItems: "center", gap: 6,
            color: "var(--text-secondary)", fontSize: 13, fontFamily: "inherit", padding: "4px 0",
          }}
        >
          <ArrowLeft size={15} />
          {t.back}
        </button>
        <h1 style={{ fontSize: 18, fontWeight: 500, margin: 0, color: "var(--text-primary)" }}>{t.title}</h1>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{t.readOnly}</span>
      </div>

      {/* Filters */}
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 14, padding: "12px 24px" }}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, color: "var(--text-secondary)" }}>
          {t.filterKind}
          <select value={kind} onChange={(e) => setKind(e.target.value)} style={{ ...fieldInput, width: "auto" }}>
            {kinds.map(([v, l]) => (
              <option key={v} value={v}>
                {l}
              </option>
            ))}
          </select>
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, color: "var(--text-secondary)" }}>
          {t.filterSession}
          <input value={session} onChange={(e) => setSession(e.target.value)} inputMode="numeric" placeholder="#" style={fieldInput} />
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, color: "var(--text-secondary)" }}>
          {t.filterVaga}
          <input value={vaga} onChange={(e) => setVaga(e.target.value)} inputMode="numeric" placeholder="#" style={fieldInput} />
        </label>
        <button
          onClick={() => load(false, null)}
          style={{
            display: "flex", alignItems: "center", gap: 6, padding: "5px 12px", borderRadius: 6,
            border: "1px solid var(--border)", background: "var(--bg-base)", color: "var(--text-secondary)",
            fontSize: 12, fontFamily: "inherit", cursor: "pointer",
          }}
        >
          <RefreshCw size={12} />
          {t.refresh}
        </button>
        <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{t.limitNote}</span>
      </div>

      {/* List + detail */}
      <div style={{ display: "flex", flex: 1, minHeight: 0, gap: 16, padding: "0 24px 16px" }}>
        <div style={{ flex: "1.6 1 0", minWidth: 0, overflowY: "auto", overflowX: "hidden", border: "1px solid var(--border)", borderRadius: 8, background: "var(--bg-surface)" }}>
          {rows.length === 0 ? (
            <div style={{ padding: 24, fontSize: 13, color: "var(--text-secondary)" }}>
              {loading ? "…" : filtered ? t.emptyFiltered : t.empty}
            </div>
          ) : (
            <>
              <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse", tableLayout: "fixed" }}>
                <thead>
                  <tr style={{ textAlign: "left", color: "var(--text-secondary)", fontSize: 12 }}>
                    <th style={{ padding: "8px 8px 8px 12px", width: 112, fontWeight: 500 }}>{t.colTime}</th>
                    <th style={{ padding: "8px 8px", width: 104, fontWeight: 500 }}>{t.colTool}</th>
                    <th style={{ padding: "8px 8px", fontWeight: 500 }}>{t.colTarget}</th>
                    <th style={{ padding: "8px 8px", width: 76, fontWeight: 500 }}>{t.colResult}</th>
                    <th style={{ padding: "8px 12px 8px 8px", width: 68, fontWeight: 500, textAlign: "right" }}>{t.colDuration}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r) => {
                    const sel = selected?.id === r.id;
                    return (
                      <tr
                        key={r.id}
                        onClick={() => setSelected(r)}
                        style={{
                          cursor: "pointer",
                          background: sel ? "var(--accent-soft)" : isFailure(r) ? "rgba(184,71,61,0.08)" : undefined,
                          borderTop: "1px solid var(--border)",
                          boxShadow: isFailure(r) ? "inset 3px 0 0 var(--danger)" : undefined,
                        }}
                      >
                        <td style={{ padding: "7px 8px 7px 12px", whiteSpace: "nowrap", color: "var(--text-secondary)" }}>{formatTime(r.ts)}</td>
                        <td style={{ padding: "7px 8px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.tool ?? kindLabel(r.kind)}</td>
                        <td style={{ padding: "7px 8px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={r.target ?? r.url ?? ""}>
                          {r.target ?? r.url}
                        </td>
                        <td
                          style={{
                            padding: "7px 8px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                            color: isFailure(r) ? "var(--danger)" : undefined,
                          }}
                          title={r.error ?? r.result ?? ""}
                        >
                          {r.result ?? r.error}
                        </td>
                        <td style={{ padding: "7px 12px 7px 8px", textAlign: "right", color: "var(--text-secondary)", whiteSpace: "nowrap" }}>
                          {formatDuration(r.duration_ms)}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "10px 12px", borderTop: "1px solid var(--border)", fontSize: 12, color: "var(--text-secondary)" }}>
                <span>{t.loaded.replace("{n}", String(rows.length))}</span>
                {hasMore && (
                  <button
                    onClick={() => load(true, rows[rows.length - 1].id)}
                    disabled={loading}
                    style={{
                      padding: "5px 12px", borderRadius: 6, border: "1px solid var(--border)", background: "var(--bg-base)",
                      color: "var(--text-secondary)", fontSize: 12, fontFamily: "inherit", cursor: loading ? "default" : "pointer",
                    }}
                  >
                    {t.loadMore}
                  </button>
                )}
              </div>
            </>
          )}
        </div>

        <div style={{ flex: "1 1 0", minWidth: 0, overflow: "auto", border: "1px solid var(--border)", borderRadius: 8, background: "var(--bg-surface)", padding: 16 }}>
          {!selected ? (
            <div style={{ fontSize: 13, color: "var(--text-secondary)" }}>{t.pickOne}</div>
          ) : (
            <>
              <div style={{ display: "flex", gap: 12 }}>
                <Shot path={selected.shot_before} label={t.before} none={t.noShot} />
                <Shot path={selected.shot_after} label={t.after} none={t.noShot} />
              </div>
              <div style={{ marginTop: 16 }}>
                {field(t.fieldTime, formatTime(selected.ts))}
                {field(t.fieldKind, kindLabel(selected.kind))}
                {field(t.fieldTool, selected.tool)}
                {field(t.fieldTarget, selected.target)}
                {field(t.fieldRole, selected.role)}
                {field(t.fieldText, selected.text)}
                {field(t.fieldUrl, selected.url)}
                {field(t.fieldResult, selected.result)}
                {field(t.fieldError, selected.error && <span style={{ color: "var(--danger)" }}>{selected.error}</span>)}
                {field(t.fieldDuration, formatDuration(selected.duration_ms))}
                {field(t.fieldSession, selected.session_id)}
                {field(t.fieldVaga, selected.vaga_id)}
              </div>
              {selected.extra && (
                <details style={{ marginTop: 12 }}>
                  <summary style={{ cursor: "pointer", fontSize: 12, color: "var(--text-secondary)" }}>{t.technical}</summary>
                  <pre style={{ fontSize: 12, whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-secondary)", marginTop: 8 }}>
                    {prettyExtra(selected.extra)}
                  </pre>
                </details>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};
