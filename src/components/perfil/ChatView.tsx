import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useT } from "../../i18n";
import { ArrowLeft, Paperclip, Send, Pencil, Square, AlertCircle, X } from "lucide-react";
import { renderMarkdown } from "../../lib/markdown";
import { CandidateBase, ChatFocus, Message } from "./types";
import { GlassesAvatar } from "./sections";

// ── Chat view ──────────────────────────────────────────────────────────────

export const ChatView: React.FC<{
  focus: ChatFocus | null;
  onBack: () => void;
  data: CandidateBase | null;
}> = ({ focus, onBack, data }) => {
  const t = useT();
  const isChrome = focus?.chromeSessao === true;
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<string[]>([]);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Chrome selection state — shown before the session starts
  const [selectionDone, setSelectionDone] = useState(false);
  const [importLinkedin, setImportLinkedin] = useState(false);
  const [importGithub, setImportGithub] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const perfilSavedRef = useRef(false);
  const onBackRef = useRef(onBack);
  useEffect(() => { onBackRef.current = onBack; }, [onBack]);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  useEffect(() => {
    let active = true;
    const unlisteners: (() => void)[] = [];

    Promise.all([
      listen<string>("perfil-output", (event) => {
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last && last.role === "assistant" && last.streaming) {
            return [...prev.slice(0, -1), { ...last, content: last.content + event.payload }];
          }
          return [...prev, { id: crypto.randomUUID(), role: "assistant", content: event.payload, streaming: true }];
        });
      }),
      listen("perfil-atualizado", () => {
        perfilSavedRef.current = true;
      }),
      listen("perfil-output-done", () => {
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last?.streaming) return [...prev.slice(0, -1), { ...last, streaming: false }];
          return prev;
        });
        setSending(false);

        if (isChrome && perfilSavedRef.current) {
          setTimeout(() => onBackRef.current(), 2000);
        }
      }),
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
  }, [isChrome]);

  useEffect(() => {
    if (isChrome) {
      setMessages([{
        id: crypto.randomUUID(),
        role: "assistant",
        content: t.profile.importQuestion,
        streaming: false,
      }]);
      // Don't start the session yet — wait for the user's selection
    } else if (focus?.preMessage) {
      setMessages([{ id: crypto.randomUUID(), role: "user", content: focus.preMessage, streaming: false }]);
      startSession(focus.preMessage);
    } else {
      const greeting = data
        ? t.profile.greetingWithProfile
        : t.profile.greetingNoProfile;
      setMessages([{ id: crypto.randomUUID(), role: "assistant", content: greeting, streaming: false }]);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleImport = async () => {
    const sources: string[] = [];
    if (importLinkedin) sources.push("LinkedIn");
    if (importGithub) sources.push("GitHub");
    if (sources.length === 0) return;

    const primeiraMsg = `Quero importar o meu perfil do ${sources.join(" e ")}. Acede com a minha sessão autenticada e extrai toda a informação profissional.`;
    const userMsg: Message = { id: crypto.randomUUID(), role: "user", content: primeiraMsg, streaming: false };

    setMessages(prev => [...prev, userMsg]);
    setSelectionDone(true);
    setSending(true);
    setError(null);

    try {
      await invoke("iniciar_sessao_perfil_chrome", { primeiraMensagem: primeiraMsg });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setSending(false);
    }
  };

  const startSession = async (firstMessage: string) => {
    setSending(true);
    setError(null);
    try {
      await invoke("iniciar_sessao_perfil", {
        contexto: focus?.section ?? "geral",
        primeiraMessage: firstMessage,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setSending(false);
    }
  };

  const attachFiles = async () => {
    const selected = await openFileDialog({ multiple: true, title: t.profile.attachFiles });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    setAttachments(prev => [...prev, ...paths.filter(p => !prev.includes(p))]);
  };

  const fileName = (path: string) => path.split(/[/\\]/).pop() ?? path;

  const sendMessage = async () => {
    const text = input.trim();
    if ((!text && attachments.length === 0) || sending) return;

    // Bubble shows only the text + file names; the invoke gets the full paths
    // with an instruction so Claude reads them with the Read tool.
    const displayText = attachments.length > 0
      ? `${text}${text ? "\n\n" : ""}📎 ${attachments.map(fileName).join(", ")}`
      : text;
    const promptText = attachments.length > 0
      ? `${text}${text ? "\n\n" : ""}${t.profile.attachedNote}\n${attachments.map(p => `- ${p}`).join("\n")}`
      : text;

    const userMsg: Message = { id: crypto.randomUUID(), role: "user", content: displayText, streaming: false };
    setMessages(prev => [...prev, userMsg]);
    setInput("");
    setAttachments([]);
    if (inputRef.current) inputRef.current.style.height = "auto";
    setSending(true);
    setError(null);

    try {
      if (isChrome) {
        await invoke("escrever_para_perfil_chrome", { input: promptText });
      } else {
        await invoke("enviar_mensagem_perfil", { mensagem: promptText });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  const stopGeneration = async () => {
    try {
      await invoke("interromper_perfil");
    } catch {
      // the done event resets the UI either way
    }
  };

  const lastUserIdx = messages.map(m => m.role).lastIndexOf("user");

  const editLastMessage = async () => {
    if (sending || lastUserIdx === -1) return;
    const msg = messages[lastUserIdx];
    try {
      await invoke("remover_ultima_troca_perfil");
    } catch {
      // history stays as-is; resending will still work, just with extra context
    }
    setMessages(prev => prev.slice(0, lastUserIdx));
    setInput(msg.content);
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (el) {
        el.style.height = "auto";
        el.style.height = Math.min(el.scrollHeight, 160) + "px";
        el.focus();
      }
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Chat header */}
      <div style={{
        height: 48, flexShrink: 0,
        background: "var(--bg-surface)", borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center", padding: "0 16px", gap: 10,
      }}>
        <button
          onClick={onBack}
          style={{
            background: "none", border: "none", cursor: "pointer",
            display: "flex", alignItems: "center", gap: 6,
            color: "var(--text-secondary)", fontSize: 13, fontFamily: "inherit",
            padding: "4px 0",
          }}
        >
          <ArrowLeft size={15} />
          {t.profile.title}
        </button>

        {focus && focus.section !== "geral" && (
          <>
            <span style={{ color: "var(--border)" }}>|</span>
            <span style={{
              fontSize: 12, padding: "2px 8px", borderRadius: 6,
              background: "var(--accent-soft)", color: "var(--accent-strong)", fontWeight: 500,
            }}>
              {focus.label}
            </span>
          </>
        )}

      </div>

      {/* Messages area */}
      <div style={{ flex: 1, overflowY: "auto", padding: "16px" }}>
        {error && (
          <div style={{
            display: "flex", alignItems: "center", gap: 8,
            padding: "10px 14px", background: "#F7E2DF",
            border: "1px solid var(--danger)", borderRadius: 8,
            fontSize: 13, color: "var(--danger)", marginBottom: 12,
          }}>
            <AlertCircle size={14} />
            <span>
              {t.profile.sessionError.replace("{error}", error)}
            </span>
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: 12, maxWidth: 680, margin: "0 auto" }}>
          {messages.map((msg, i) => (
            <div
              key={msg.id}
              style={{
                display: "flex",
                flexDirection: msg.role === "user" ? "row-reverse" : "row",
                alignItems: "flex-start",
                gap: 10,
              }}
            >
              {msg.role === "assistant" && <GlassesAvatar />}

              <div style={{
                maxWidth: "82%",
                padding: "10px 14px",
                borderRadius: msg.role === "user" ? "12px 12px 4px 12px" : "12px 12px 12px 4px",
                background: msg.role === "user" ? "var(--accent-soft)" : "var(--bg-surface)",
                border: msg.role === "user" ? "none" : "1px solid var(--border)",
                fontSize: 13,
                color: msg.role === "user" ? "var(--accent-strong)" : "var(--text-primary)",
                lineHeight: "1.55",
              }}>
                {msg.role === "user"
                  ? <span style={{ whiteSpace: "pre-wrap" }}>{msg.content}</span>
                  : renderMarkdown(msg.content)
                }
                {msg.streaming && (
                  <span style={{
                    display: "inline-block", width: 6, height: 13, marginLeft: 2,
                    background: "var(--accent)", borderRadius: 1, verticalAlign: "middle",
                    animation: "blink 0.8s step-end infinite",
                  }} />
                )}
              </div>

              {msg.role === "user" && i === lastUserIdx && !sending && (
                <button
                  onClick={editLastMessage}
                  title={t.profile.editMessage}
                  style={{
                    background: "none", border: "none", cursor: "pointer",
                    padding: 4, marginTop: 4, flexShrink: 0,
                    display: "flex", alignItems: "center",
                    color: "var(--text-tertiary)", transition: "color 0.15s",
                  }}
                  onMouseEnter={e => e.currentTarget.style.color = "var(--accent)"}
                  onMouseLeave={e => e.currentTarget.style.color = "var(--text-tertiary)"}
                >
                  <Pencil size={13} />
                </button>
              )}
            </div>
          ))}

          {sending && !messages[messages.length - 1]?.streaming && (
            <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
              <GlassesAvatar />
              <div style={{
                padding: "12px 14px",
                borderRadius: "12px 12px 12px 4px",
                background: "var(--bg-surface)",
                border: "1px solid var(--border)",
                display: "flex", alignItems: "center", gap: 4,
              }}>
                {[0, 1, 2].map(n => (
                  <span key={n} style={{
                    width: 6, height: 6, borderRadius: "50%",
                    background: "var(--text-tertiary)",
                    animation: `typingDot 1.2s ease-in-out ${n * 0.18}s infinite`,
                  }} />
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Chrome source selection — shown before session starts */}
        {isChrome && !selectionDone && (
          <div style={{ maxWidth: 680, margin: "0 auto", paddingLeft: 40, paddingBottom: 8 }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
              {(["LinkedIn", "GitHub"] as const).map(src => {
                const active = src === "LinkedIn" ? importLinkedin : importGithub;
                const toggle = src === "LinkedIn"
                  ? () => setImportLinkedin(v => !v)
                  : () => setImportGithub(v => !v);
                return (
                  <button key={src} onClick={toggle} style={{
                    padding: "7px 18px", borderRadius: 20, cursor: "pointer",
                    border: `1px solid ${active ? "var(--accent)" : "var(--border)"}`,
                    background: active ? "var(--accent-soft)" : "var(--bg-surface)",
                    color: active ? "var(--accent-strong)" : "var(--text-secondary)",
                    fontSize: 13, fontWeight: active ? 500 : 400,
                    fontFamily: "inherit", transition: "all 0.15s",
                  }}>
                    {active ? "✓ " : ""}{src}
                  </button>
                );
              })}
            </div>
            <button
              onClick={handleImport}
              disabled={!importLinkedin && !importGithub}
              style={{
                padding: "8px 20px", borderRadius: 8,
                background: (importLinkedin || importGithub) ? "var(--accent)" : "var(--bg-sunken)",
                color: (importLinkedin || importGithub) ? "#fff" : "var(--text-tertiary)",
                border: "none",
                cursor: (importLinkedin || importGithub) ? "pointer" : "default",
                fontSize: 13, fontWeight: 500, fontFamily: "inherit",
                transition: "background 0.15s, color 0.15s",
              }}
            >
              {t.profile.importBtn}
            </button>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input area — hidden while chrome selection is pending */}
      <div style={{
        flexShrink: 0,
        borderTop: "1px solid var(--border)",
        background: "var(--bg-surface)",
        padding: "12px 16px",
        display: isChrome && !selectionDone ? "none" : undefined,
      }}>
        {attachments.length > 0 && (
          <div style={{
            display: "flex", flexWrap: "wrap", gap: 6,
            maxWidth: 680, margin: "0 auto 8px",
          }}>
            {attachments.map(path => (
              <span
                key={path}
                title={path}
                style={{
                  display: "inline-flex", alignItems: "center", gap: 5,
                  padding: "3px 8px", borderRadius: 6,
                  background: "var(--bg-sunken)", border: "1px solid var(--border)",
                  fontSize: 12, color: "var(--text-secondary)", maxWidth: 220,
                }}
              >
                <Paperclip size={11} style={{ flexShrink: 0 }} />
                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {fileName(path)}
                </span>
                <button
                  onClick={() => setAttachments(prev => prev.filter(p => p !== path))}
                  title={t.profile.removeAttachment}
                  style={{
                    background: "none", border: "none", cursor: "pointer", padding: 0,
                    display: "flex", alignItems: "center", color: "var(--text-tertiary)",
                  }}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
          </div>
        )}
        <div style={{
          display: "flex", alignItems: "flex-end", gap: 10,
          maxWidth: 680, margin: "0 auto",
        }}>
          <button
            onClick={attachFiles}
            disabled={sending}
            title={t.profile.attachFiles}
            style={{
              width: 36, height: 36, flexShrink: 0,
              background: "none", border: "none", borderRadius: 8,
              cursor: sending ? "default" : "pointer",
              display: "flex", alignItems: "center", justifyContent: "center",
              color: "var(--text-secondary)", transition: "color 0.15s",
            }}
            onMouseEnter={e => { if (!sending) e.currentTarget.style.color = "var(--accent)"; }}
            onMouseLeave={e => e.currentTarget.style.color = "var(--text-secondary)"}
          >
            <Paperclip size={16} />
          </button>
          <textarea
            ref={inputRef}
            value={input}
            onChange={e => {
              setInput(e.target.value);
              const el = e.target;
              el.style.height = "auto";
              el.style.height = Math.min(el.scrollHeight, 160) + "px";
            }}
            onKeyDown={handleKeyDown}
            placeholder={t.profile.chatPlaceholder}
            disabled={sending}
            rows={1}
            style={{
              flex: 1,
              resize: "none",
              border: "1px solid var(--border)",
              borderRadius: 8,
              padding: "9px 12px",
              fontFamily: "inherit",
              fontSize: 13,
              color: "var(--text-primary)",
              background: sending ? "var(--bg-sunken)" : "var(--bg-base)",
              outline: "none",
              lineHeight: "1.5",
              maxHeight: 160,
              overflow: "auto",
              transition: "border-color 0.15s",
            }}
            onFocus={e => e.target.style.borderColor = "var(--accent)"}
            onBlur={e => e.target.style.borderColor = "var(--border)"}
          />
          {sending ? (
            <button
              onClick={stopGeneration}
              title={t.profile.stopGeneration}
              style={{
                width: 36, height: 36, flexShrink: 0,
                background: "var(--accent)",
                border: "none", borderRadius: 8, cursor: "pointer",
                display: "flex", alignItems: "center", justifyContent: "center",
                transition: "background 0.15s",
              }}
            >
              <Square size={12} fill="#fff" color="#fff" />
            </button>
          ) : (
            <button
              onClick={sendMessage}
              disabled={!input.trim() && attachments.length === 0}
              title="Enviar (Enter)"
              style={{
                width: 36, height: 36, flexShrink: 0,
                background: (!input.trim() && attachments.length === 0) ? "var(--bg-sunken)" : "var(--accent)",
                border: "none", borderRadius: 8, cursor: (!input.trim() && attachments.length === 0) ? "default" : "pointer",
                display: "flex", alignItems: "center", justifyContent: "center",
                transition: "background 0.15s",
              }}
            >
              <Send size={15} color={(!input.trim() && attachments.length === 0) ? "var(--text-tertiary)" : "#fff"} />
            </button>
          )}
        </div>
        <div style={{ fontSize: 11, color: "var(--text-tertiary)", textAlign: "center", marginTop: 6, maxWidth: 680, margin: "6px auto 0" }}>
          {t.profile.enterHint}
        </div>
      </div>

      <style>{`
        @keyframes spin { to { transform: rotate(360deg); } }
        @keyframes blink { 50% { opacity: 0; } }
        @keyframes typingDot {
          0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
          30% { opacity: 1; transform: translateY(-3px); }
        }
      `}</style>
    </div>
  );
};
