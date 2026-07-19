import React, { useEffect, useRef, useState } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import { useT } from "../i18n";
import { ToggleSwitch } from "./ui/ToggleSwitch";

const LIGHT_THEME = {
  background: "#F1EFEA",
  foreground: "#1F1D18",
  cursor: "#D97757",
  cursorAccent: "#FFFFFF",
  selectionBackground: "#D9775740",
  black: "#1F1D18",
  red: "#B8473D",
  green: "#3B7A52",
  yellow: "#B8862E",
  blue: "#3B5EA6",
  magenta: "#7A3B7A",
  cyan: "#2B7A78",
  white: "#6B6759",
  brightBlack: "#9D9889",
  brightRed: "#D97757",
  brightGreen: "#5A9E70",
  brightYellow: "#D9A84E",
  brightBlue: "#5A7EC4",
  brightMagenta: "#9D5A9D",
  brightCyan: "#4A9E9C",
  brightWhite: "#1F1D18",
};

export const TerminalView: React.FC = () => {
  const t = useT();
  const containerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [locked, setLocked] = useState(true);
  const lockedRef = useRef(true);

  // Keep ref in sync so the xterm onData callback always reads the current value
  const handleToggle = () => {
    const next = !lockedRef.current;
    lockedRef.current = next;
    setLocked(next);
    // Return focus to terminal after toggle so user can type immediately
    if (!next) xtermRef.current?.focus();
  };

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new XTerm({
      theme: LIGHT_THEME,
      fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
      fontSize: 13,
      lineHeight: 1.5,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: false,
      // windowsPty substitui windowsMode em xterm.js v5+.
      // Activa as correcções de wrapped lines e scrollback específicas do ConPTY.
      // buildNumber 26200 = Windows 11 (build real desta máquina).
      windowsPty: { backend: 'conpty', buildNumber: 26200 },
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    fitAddon.fit();
    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    term.onData((data) => {
      if (!lockedRef.current) {
        invoke("escrever_pty", { input: data }).catch(console.error);
      }
    });

    let active = true;
    let unlisten: (() => void) | undefined;

    listen<string>("pty-output", (event) => {
      xtermRef.current?.write(event.payload);
    }).then((fn) => {
      if (active) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    const observer = new ResizeObserver(() => {
      fitAddon.fit();
      invoke("redimensionar_pty", { rows: term.rows, cols: term.cols }).catch(() => {});
    });
    if (containerRef.current) observer.observe(containerRef.current);

    return () => {
      active = false;
      unlisten?.();
      observer.disconnect();
      term.dispose();
      xtermRef.current = null;
    };
  }, []);

  const launchTest = () => {
    const term = xtermRef.current;
    if (!term) return;
    // Passa as dimensões reais do xterm para que o PTY não divergia → sem ghost text
    invoke("iniciar_pty", {
      comando: "cmd",
      args: [],
      rows: term.rows,
      cols: term.cols,
    }).catch(console.error);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Toolbar — z-index garante que fica sobre o xterm canvas */}
      <div style={{
        height: 40,
        background: "var(--bg-sunken)",
        borderBottom: "1px solid var(--border)",
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        gap: 12,
        flexShrink: 0,
        position: "relative",
        zIndex: 10,
      }}>
        {/* Status */}
        <div style={{ display: "flex", alignItems: "center", gap: 6, flex: 1 }}>
          <span style={{
            width: 8, height: 8, borderRadius: "50%",
            background: locked ? "var(--text-tertiary)" : "var(--success)",
            display: "inline-block",
            transition: "background 0.2s",
          }} />
          <span style={{
            fontSize: 12,
            color: locked ? "var(--text-secondary)" : "var(--accent-strong)",
            fontWeight: locked ? 400 : 500,
            transition: "color 0.2s",
          }}>
            {locked ? t.terminal.viewOnly : t.terminal.underYourControl}
          </span>
        </div>

        {/* Test button */}
        <button
          onMouseDown={(e) => { e.preventDefault(); launchTest(); }}
          style={{
            background: "transparent",
            border: "1px solid var(--border)",
            borderRadius: 4,
            padding: "2px 10px",
            fontSize: 12,
            cursor: "pointer",
            color: "var(--text-secondary)",
            fontFamily: "inherit",
          }}
        >
          {t.terminal.testPty}
        </button>

        {/* Toggle switch */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{t.terminal.takeControl}</span>
          <ToggleSwitch checked={!locked} onChange={handleToggle} />
        </div>
      </div>

      {/* Terminal surface — clicking aqui garante focus no xterm */}
      <div
        ref={containerRef}
        onClick={() => xtermRef.current?.focus()}
        style={{
          flex: 1,
          overflow: "hidden",
          outline: locked ? "2px solid transparent" : "2px solid var(--accent)",
          transition: "outline-color 0.15s",
          cursor: "text",
        }}
      />
    </div>
  );
};

