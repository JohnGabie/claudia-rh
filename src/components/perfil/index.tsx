import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SearchVariant } from "../../types";
import { CandidateBase, ChatFocus, EditTarget, Mode } from "./types";
import { ResumoView } from "./ResumoView";
import { ChatView } from "./ChatView";
import { CurriculosView } from "./CurriculosView";
import { CoverLettersView } from "./CoverLettersView";
import { SectionEditModal } from "./EditModal";
import { EmptyState } from "./EmptyState";

// ── Main component ─────────────────────────────────────────────────────────

export const Perfil: React.FC<{ initialSection?: string | null; onSectionHandled?: () => void }> = ({ initialSection, onSectionHandled }) => {
  const [mode, setMode] = useState<Mode>("resumo");
  const [chatFocus, setChatFocus] = useState<ChatFocus | null>(null);
  const [profileData, setProfileData] = useState<CandidateBase | null>(null);
  const [variants, setVariants] = useState<SearchVariant[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasProfile, setHasProfile] = useState(false);
  const [editTarget, setEditTarget] = useState<EditTarget | null>(null);
  const hasLoadedOnce = useRef(false);

  const loadData = useCallback(async () => {
    try {
      const [base, vars] = await Promise.all([
        invoke<CandidateBase>("ler_candidato_base"),
        invoke<SearchVariant[]>("ler_search_variants"),
      ]);
      setProfileData(base);
      setVariants(vars ?? []);
      hasLoadedOnce.current = true;
      setHasProfile(
        Boolean(base?.dados_pessoais?.nome_completo) ||
        Boolean(base?.dados_pessoais?.email) ||
        (base?.experiencia?.length ?? 0) > 0 ||
        (base?.competencias?.length ?? 0) > 0 ||
        (base?.projetos?.length ?? 0) > 0 ||
        (vars?.length ?? 0) > 0
      );
    } catch (e) {
      console.error("[Perfil] loadData error:", e);
      // Se já tínhamos dados carregados, manter o estado — não apagar o perfil por erro de parse
      if (!hasLoadedOnce.current) setHasProfile(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();

    let active = true;
    let unlisten: (() => void) | undefined;

    listen("perfil-atualizado", loadData).then((fn) => {
      if (active) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [loadData]);

  useEffect(() => {
    if (!initialSection) return;
    const sectionTargets: Record<string, EditTarget> = {
      dados_pessoais: { kind: "dados_pessoais" },
      experiencia:    { kind: "experiencia" },
      projetos:       { kind: "projetos" },
      formacao:       { kind: "formacao" },
      competencias:   { kind: "competencias" },
      idiomas:        { kind: "idiomas" },
    };
    const target = sectionTargets[initialSection];
    if (!target) return;
    setEditTarget(target);
    onSectionHandled?.();
  }, [initialSection, onSectionHandled]);

  const openChat = (focus?: ChatFocus) => {
    setChatFocus(focus ?? null);
    setMode("chat");
  };

  const openCurriculos = () => setMode("curriculos");
  const openCoverLetters = () => setMode("cover_letters");

  const backToResumo = () => {
    setMode("resumo");
    setChatFocus(null);
    loadData();
  };

  if (loading) {
    return (
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <span style={{ fontSize: 13, color: "var(--text-tertiary)" }}>A carregar perfil…</span>
      </div>
    );
  }

  if (mode === "chat") {
    return <ChatView focus={chatFocus} onBack={backToResumo} data={profileData} />;
  }

  if (mode === "curriculos") {
    return <CurriculosView onBack={backToResumo} />;
  }

  if (mode === "cover_letters") {
    return <CoverLettersView onBack={backToResumo} />;
  }

  if (!hasProfile) {
    return <EmptyState onStart={openChat} />;
  }

  return (
    <>
      <ResumoView
        data={profileData}
        variants={variants}
        onOpenChat={openChat}
        onOpenCurriculos={openCurriculos}
        onOpenCoverLetters={openCoverLetters}
        onDirectEdit={setEditTarget}
        onReloadData={loadData}
      />
      {editTarget && profileData && (
        <SectionEditModal
          target={editTarget}
          profileData={profileData}
          variants={variants}
          onSaved={(updatedData) => {
            if (updatedData) setProfileData(updatedData);
            setEditTarget(null);
            loadData();
          }}
          onClose={() => setEditTarget(null)}
        />
      )}
    </>
  );
};
