import React from "react";

// Renderer de markdown leve para as respostas da Claudia (chat, feedback).
// Suporta: **bold**, *italic*, `code`, blocos ```, títulos #/##/###, bullets "- "/"• ".

export function applyInline(text: string): React.ReactNode[] {
  const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g);
  return parts.map((part, i) => {
    if (part.startsWith("**") && part.endsWith("**"))
      return <strong key={i}>{part.slice(2, -2)}</strong>;
    if (part.startsWith("*") && part.endsWith("*"))
      return <em key={i}>{part.slice(1, -1)}</em>;
    if (part.startsWith("`") && part.endsWith("`"))
      return (
        <code key={i} style={{
          fontFamily: "JetBrains Mono, Consolas, monospace",
          fontSize: 12, background: "var(--bg-sunken)",
          padding: "1px 4px", borderRadius: 3,
        }}>
          {part.slice(1, -1)}
        </code>
      );
    return part;
  });
}

// headingSize: "sm" para bolhas de chat (default), "md" para vistas de documento
export function renderMarkdown(text: string, opts?: { headingSize?: "sm" | "md" }): React.ReactNode {
  const md = opts?.headingSize === "md";
  const lines = text.split("\n");
  const nodes: React.ReactNode[] = [];
  let codeLines: string[] = [];
  let inCode = false;

  lines.forEach((line, i) => {
    if (line.startsWith("```")) {
      if (!inCode) { inCode = true; codeLines = []; return; }
      inCode = false;
      nodes.push(
        <pre key={i} style={{
          background: "var(--bg-sunken)", borderRadius: 6, padding: "8px 10px",
          margin: "6px 0", fontFamily: "JetBrains Mono, Consolas, monospace",
          fontSize: 12, overflow: "auto", whiteSpace: "pre-wrap",
        }}>
          <code>{codeLines.join("\n")}</code>
        </pre>
      );
      return;
    }
    if (inCode) { codeLines.push(line); return; }

    if (line === "") { nodes.push(<div key={i} style={{ height: 6 }} />); return; }

    if (line.match(/^#{1,3} /)) {
      const level = line.match(/^#+/)![0].length;
      const content = line.replace(/^#+\s/, "");
      nodes.push(
        <div key={i} style={{
          fontSize: level === 1 ? (md ? 16 : 15) : (md ? 14 : 13),
          fontWeight: 600,
          color: "var(--text-primary)",
          marginTop: md ? 12 : 8, marginBottom: md ? 4 : 2,
        }}>
          {content}
        </div>
      );
      return;
    }

    if (line.startsWith("- ") || line.startsWith("• ")) {
      nodes.push(
        <div key={i} style={{ display: "flex", gap: 6, marginLeft: 2, lineHeight: "1.5" }}>
          <span style={{ color: "var(--accent)", flexShrink: 0, marginTop: 1 }}>•</span>
          <span>{applyInline(line.slice(2))}</span>
        </div>
      );
      return;
    }

    nodes.push(
      <div key={i} style={{ lineHeight: "1.55" }}>{applyInline(line)}</div>
    );
  });

  return <>{nodes}</>;
}
