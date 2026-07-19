import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useT } from "../../i18n";
import { Plus, FileText, MailOpen, Lightbulb } from "lucide-react";
import { SearchVariant } from "../../types";
import { CandidateBase, ChatFocus, EditTarget, Proposta } from "./types";
import { ProfileHeader, ExperienciaSection, ProjetosSection, FormacaoSection, CompetenciasSection, IdiomasSection, VariantCard } from "./sections";

// ── Resumo view ────────────────────────────────────────────────────────────

export const ResumoView: React.FC<{
  data: CandidateBase | null;
  variants: SearchVariant[];
  onOpenChat: (focus?: ChatFocus) => void;
  onOpenCurriculos: () => void;
  onOpenCoverLetters: () => void;
  onDirectEdit: (target: EditTarget) => void;
  onReloadData: () => void;
}> = ({ data, variants, onOpenChat, onOpenCurriculos, onOpenCoverLetters, onDirectEdit, onReloadData }) => {
  const t = useT();
  const [localPesos, setLocalPesos] = useState<Record<string, number>>({});
  const committedPesosRef = useRef<Record<string, number>>({});

  const [propostas, setPropostas] = useState<Proposta[]>([]);

  const carregarPropostas = useCallback(() => {
    invoke<Proposta[]>("listar_propostas").then(setPropostas).catch(console.error);
  }, []);

  useEffect(() => {
    carregarPropostas();
    const unsubs = [
      listen("nova-proposta", carregarPropostas),
      listen("proposta-resolvida", carregarPropostas),
    ];
    return () => { unsubs.forEach((p) => p.then((f) => f())); };
  }, [carregarPropostas]);

  const handleAplicarProposta = (p: Proposta) => {
    onOpenChat({
      section: "sugestao_perfil",
      label: t.profile.applySuggestion,
      preMessage: p.pergunta + (p.contexto ? `\n\nContexto: ${p.contexto}` : ""),
    });
  };

  const handleIgnorarProposta = async (id: number) => {
    await invoke("ignorar_proposta", { id }).catch(console.error);
    carregarPropostas();
  };

  useEffect(() => {
    const map: Record<string, number> = {};
    variants.forEach(v => { map[v.id] = v.peso; });
    setLocalPesos(map);
    committedPesosRef.current = map;
  }, [variants]);

  const activeVariants = variants.filter(v => v.ativa);
  const totalPeso = activeVariants.reduce((sum, v) => sum + (localPesos[v.id] ?? v.peso), 0);
  const maxPct = activeVariants.length > 1 ? 100 - (activeVariants.length - 1) * 5 : 95;

  const handleDragBar = (variantId: string, newPct: number) => {
    setLocalPesos(prev => {
      const curTotal = activeVariants.reduce((s, v) => s + (prev[v.id] ?? v.peso), 0);
      const newPesoForId = (newPct / 100) * curTotal;
      const others = activeVariants.filter(v => v.id !== variantId);
      const sumOthers = others.reduce((s, v) => s + (prev[v.id] ?? v.peso), 0);
      const remaining = curTotal - newPesoForId;
      const minPeso = (5 / 100) * curTotal;
      const next: Record<string, number> = { ...prev, [variantId]: newPesoForId };
      others.forEach(v => {
        const oldPeso = prev[v.id] ?? v.peso;
        next[v.id] = sumOthers > 0 ? Math.max(minPeso, (oldPeso / sumOthers) * remaining) : remaining / others.length;
      });
      committedPesosRef.current = next;
      return next;
    });
  };

  const handleDragEnd = async () => {
    const pesos: Record<string, number> = {};
    variants.forEach(v => { pesos[v.id] = committedPesosRef.current[v.id] ?? v.peso; });
    await invoke("guardar_pesos_variantes", { pesos }).catch(console.error);
    onReloadData();
  };

  const handleToggleAtiva = async (variantId: string) => {
    const v = variants.find(x => x.id === variantId);
    if (!v) return;
    await invoke("guardar_variante_unica", { variante: { ...v, ativa: !v.ativa } }).catch(console.error);
    onReloadData();
  };

  const atualizadoEm = data?.ultima_atualizacao
    ? new Date(data.ultima_atualizacao).toLocaleDateString("pt-PT", { day: "numeric", month: "long", year: "numeric" })
    : null;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      {/* Header row */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 4 }}>
        <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: 0, flex: 1 }}>{t.profile.title}</h1>
        <button
          onClick={() => onDirectEdit({ kind: "nova_variante" })}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <Plus size={14} />
          {t.profile.newVariant}
        </button>
        <button
          onClick={onOpenCurriculos}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <FileText size={14} />
          {t.profile.resumes}
        </button>
        <button
          onClick={onOpenCoverLetters}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "transparent",
            border: "1px solid var(--border)", borderRadius: 8,
            fontSize: 13, color: "var(--text-secondary)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          <MailOpen size={14} />
          {t.profile.coverLetters}
        </button>
        <button
          onClick={() => onOpenChat()}
          style={{
            display: "flex", alignItems: "center", gap: 6,
            padding: "7px 14px", background: "var(--accent)",
            border: "none", borderRadius: 8,
            fontSize: 13, color: "#fff", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          {t.profile.updateProfile}
        </button>
      </div>
      {atualizadoEm ? (
        <div style={{ fontSize: 12, color: "var(--text-tertiary)", marginBottom: 20 }}>
          {t.profile.lastUpdated}{atualizadoEm}
        </div>
      ) : (
        <div style={{ marginBottom: 20 }} />
      )}

      {/* Propostas de perfil */}
      {propostas.length > 0 && (
        <div style={{
          background: "var(--accent-soft)",
          border: "1px solid var(--accent)",
          borderRadius: 10,
          padding: "14px 16px",
          marginBottom: 20,
        }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <Lightbulb size={15} style={{ color: "var(--accent)", flexShrink: 0 }} />
            <span style={{ fontSize: 13, fontWeight: 600, color: "var(--accent-strong)" }}>
              {propostas.length === 1 ? t.profile.suggestions_one : `${propostas.length}${t.profile.suggestions_many}`}
            </span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {propostas.map((p) => (
              <div key={p.id} style={{
                background: "var(--bg-surface)",
                border: "1px solid var(--border)",
                borderRadius: 8,
                padding: "10px 12px",
              }}>
                {(p.titulo_vaga || p.empresa_vaga) && (
                  <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginBottom: 4 }}>
                    {[p.titulo_vaga, p.empresa_vaga].filter(Boolean).join(" · ")}
                  </div>
                )}
                <div style={{ fontSize: 13, color: "var(--text-primary)", lineHeight: 1.5, marginBottom: 8 }}>
                  {p.pergunta}
                </div>
                <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                  <button
                    onClick={() => handleIgnorarProposta(p.id)}
                    style={{
                      padding: "4px 12px", borderRadius: 6, fontSize: 12,
                      background: "transparent", border: "1px solid var(--border)",
                      color: "var(--text-secondary)", cursor: "pointer", fontFamily: "inherit",
                    }}
                  >
                    {t.profile.ignore}
                  </button>
                  <button
                    onClick={() => handleAplicarProposta(p)}
                    style={{
                      padding: "4px 12px", borderRadius: 6, fontSize: 12, fontWeight: 500,
                      background: "var(--accent)", border: "none",
                      color: "#fff", cursor: "pointer", fontFamily: "inherit",
                    }}
                  >
                    {t.profile.applySuggestion}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* CV preview */}
      {data && (
        <>
          <ProfileHeader
            data={data}
            onEdit={() => onDirectEdit({ kind: "dados_pessoais" })}
          />
          {(data.experiencia?.length ?? 0) > 0 && (
            <ExperienciaSection
              items={data.experiencia}
              onEdit={() => onDirectEdit({ kind: "experiencia" })}
            />
          )}
          {(data.projetos?.length ?? 0) > 0 && (
            <ProjetosSection
              items={data.projetos}
              onEdit={() => onDirectEdit({ kind: "projetos" })}
            />
          )}
          {(data.formacao?.length ?? 0) > 0 && (
            <FormacaoSection
              items={data.formacao}
              onEdit={() => onDirectEdit({ kind: "formacao" })}
            />
          )}
          {(data.idiomas?.length ?? 0) > 0 && (
            <IdiomasSection
              items={data.idiomas}
              onEdit={() => onDirectEdit({ kind: "idiomas" })}
            />
          )}
          {(data.competencias?.length ?? 0) > 0 && (
            <CompetenciasSection
              items={data.competencias}
              onEdit={() => onDirectEdit({ kind: "competencias" })}
            />
          )}
        </>
      )}

      {/* Variants section */}
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
          <span style={{ fontSize: 15, fontWeight: 500, color: "var(--text-primary)" }}>{t.profile.searchVariants}</span>
          <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>
            {variants.filter(v => v.ativa).length} {variants.filter(v => v.ativa).length === 1 ? t.profile.activeCount_one : t.profile.activeCount_many}
          </span>
          <button
            onClick={() => onDirectEdit({ kind: "nova_variante" })}
            style={{
              marginLeft: "auto",
              display: "flex", alignItems: "center", gap: 5,
              padding: "5px 12px", background: "var(--accent)",
              border: "none", borderRadius: 7,
              fontSize: 12, fontWeight: 500, color: "#fff",
              cursor: "pointer", fontFamily: "inherit",
            }}
          >
            <Plus size={12} />
            {t.profile.addNewSearch}
          </button>
        </div>

        {variants.length === 0 ? (
          <div style={{
            background: "var(--bg-surface)", border: "1px dashed var(--border)",
            borderRadius: 8, padding: "20px 16px", textAlign: "center",
          }}>
            <div style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 10 }}>
              {t.profile.noVariantsDesc}
            </div>
            <button
              onClick={() => onDirectEdit({ kind: "nova_variante" })}
              style={{
                display: "inline-flex", alignItems: "center", gap: 6,
                padding: "7px 14px", background: "var(--accent-soft)",
                border: "none", borderRadius: 8, fontSize: 13,
                color: "var(--accent-strong)", cursor: "pointer", fontFamily: "inherit",
              }}
            >
              <Plus size={14} />
              {t.profile.createVariant}
            </button>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {variants.map(v => (
              <VariantCard
                key={v.id}
                variant={v}
                pct={totalPeso > 0 ? ((localPesos[v.id] ?? v.peso) / totalPeso) * 100 : 0}
                maxPct={maxPct}
                onEdit={onDirectEdit}
                onDragBar={(newPct) => handleDragBar(v.id, newPct)}
                onDragEnd={handleDragEnd}
                onToggleAtiva={() => handleToggleAtiva(v.id)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
