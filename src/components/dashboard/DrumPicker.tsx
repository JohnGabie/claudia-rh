import React, { useRef, useEffect } from "react";

// ── DrumPicker ────────────────────────────────────────────────────────────────
const ITEM_H = 52;

export const DrumPicker: React.FC<{
  values: number[];
  selectedValue: number;
  onChange: (v: number) => void;
  formatValue: (v: number) => string;
}> = ({ values, selectedValue, onChange, formatValue }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const selectedIdx = Math.max(0, values.indexOf(selectedValue));

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = selectedIdx * ITEM_H;
    }
  }, []);

  const handleScroll = () => {
    if (scrollTimeout.current) clearTimeout(scrollTimeout.current);
    scrollTimeout.current = setTimeout(() => {
      if (!containerRef.current) return;
      const idx = Math.round(containerRef.current.scrollTop / ITEM_H);
      const clamped = Math.max(0, Math.min(values.length - 1, idx));
      onChange(values[clamped]);
    }, 80);
  };

  return (
    <div style={{ position: "relative", height: ITEM_H * 5, userSelect: "none" }}>
      {/* Selection band */}
      <div style={{
        position: "absolute", top: ITEM_H * 2, height: ITEM_H,
        left: 0, right: 0, pointerEvents: "none", zIndex: 0,
        background: "var(--accent-soft)",
        borderTop: "2px solid var(--accent)",
        borderBottom: "2px solid var(--accent)",
        borderRadius: 8,
      }} />
      {/* Top fade */}
      <div style={{
        position: "absolute", top: 0, left: 0, right: 0,
        height: ITEM_H * 2.4,
        background: "linear-gradient(to bottom, var(--bg-surface) 15%, transparent)",
        pointerEvents: "none", zIndex: 2,
      }} />
      {/* Bottom fade */}
      <div style={{
        position: "absolute", bottom: 0, left: 0, right: 0,
        height: ITEM_H * 2.4,
        background: "linear-gradient(to top, var(--bg-surface) 15%, transparent)",
        pointerEvents: "none", zIndex: 2,
      }} />
      {/* Scroll container */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        style={{
          height: "100%",
          overflowY: "scroll",
          scrollSnapType: "y mandatory",
          scrollbarWidth: "none",
        }}
      >
        <div style={{ height: ITEM_H * 2 }} />
        {values.map((v) => {
          const isSel = v === selectedValue;
          return (
            <div
              key={v}
              onClick={() => {
                onChange(v);
                const i = values.indexOf(v);
                containerRef.current?.scrollTo({ top: i * ITEM_H, behavior: "smooth" });
              }}
              style={{
                height: ITEM_H,
                scrollSnapAlign: "center",
                display: "flex", alignItems: "center", justifyContent: "center",
                cursor: "pointer", position: "relative", zIndex: 1,
              }}
            >
              <span style={{
                fontSize: isSel ? 30 : 20,
                fontWeight: isSel ? 700 : 400,
                color: isSel ? "var(--accent-strong)" : "var(--text-tertiary)",
                letterSpacing: isSel ? "-0.04em" : "-0.01em",
                fontVariantNumeric: "tabular-nums",
                transition: "font-size 0.12s ease, color 0.12s ease",
              }}>
                {formatValue(v)}
              </span>
            </div>
          );
        })}
        <div style={{ height: ITEM_H * 2 }} />
      </div>
    </div>
  );
};
