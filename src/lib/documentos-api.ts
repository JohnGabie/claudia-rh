// The only place the front talks to the document commands (spec section 6). In `vite dev` outside
// Tauri (a plain browser), it answers from an in-memory mock so the screens can be built before the
// backend lands; production builds drop the mock (import.meta.env.DEV is false there).
import { invoke } from "@tauri-apps/api/core";
import type { DocLang } from "../components/perfil/types";
import type { Documento, DocumentoDetalhe, Tag, TipoDocumento, VarianteResumo } from "../types";

const semTauri = () => typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

// Fired after any write, so screens that keep a copy of the library (Profile selects, the grid) reload.
export const DOCUMENTOS_MUDARAM = "documentos-mudaram";
const ESCRITAS = new Set([
  "gerar_curriculo", "importar_documento", "renomear_documento", "atualizar_tags_documento",
  "definir_preferido", "remover_preferido", "apagar_documento",
]);

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  let r: T;
  if (import.meta.env.DEV && semTauri()) {
    const { mockInvoke } = await import("./documentos.mock");
    r = await mockInvoke<T>(cmd, args ?? {});
  } else {
    r = await invoke<T>(cmd, args);
  }
  if (ESCRITAS.has(cmd)) window.dispatchEvent(new Event(DOCUMENTOS_MUDARAM));
  return r;
}

export const docsApi = {
  listar: (tipo: TipoDocumento) => call<Documento[]>("listar_documentos", { tipo }),
  obter: (id: number) => call<DocumentoDetalhe>("obter_documento", { id }),
  gerarCurriculo: (a: { templateId: string; corPrimaria: string; idioma: DocLang; varianteId: string | null }) =>
    call<Documento>("gerar_curriculo", a),
  gerarCarta: (a: { empresa: string; cargo: string; descricaoVaga: string | null; notaExtra: string | null; idioma: DocLang; corPrimaria: string; varianteId: string | null }) =>
    call<void>("gerar_cover_letter", a),
  importar: (tipo: TipoDocumento, caminhoOrigem: string, nome: string | null) =>
    call<Documento>("importar_documento", { tipo, caminhoOrigem, nome }),
  renomear: (id: number, nome: string) => call<void>("renomear_documento", { id, nome }),
  atualizarTags: (id: number, tags: Tag[]) => call<void>("atualizar_tags_documento", { id, tags }),
  definirPreferido: (id: number, varianteId: string | null) => call<void>("definir_preferido", { id, varianteId }),
  removerPreferido: (tipo: TipoDocumento, varianteId: string | null) => call<void>("remover_preferido", { tipo, varianteId }),
  abrir: (id: number) => call<void>("abrir_documento", { id }),
  apagar: (id: number) => call<void>("apagar_documento", { id }),
  listarVariantes: () => call<VarianteResumo[]>("listar_variantes"),
};
