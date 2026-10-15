// The only place the front talks to the Sites commands. In `vite dev` outside Tauri (a plain
// browser) it answers from an in-memory mock; production builds drop the mock path because
// import.meta.env.DEV is false there.
import { invoke } from "@tauri-apps/api/core";
import type { Site } from "../components/sites/logica";

export interface AvisoSite { id: number; chave: string; nome: string; criado_em: string }

const semTauri = () => typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

// Fired after any write, so the screen reloads even when the backend event never arrives (mock).
export const SITES_MUDARAM = "sites-mudaram";
const ESCRITAS = new Set(["definir_ia_pode_usar", "marcar_site_logado", "adicionar_site", "remover_site"]);

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  let r: T;
  if (import.meta.env.DEV && semTauri()) {
    const { mockInvoke } = await import("./sites.mock");
    r = await mockInvoke<T>(cmd, args ?? {});
  } else {
    r = await invoke<T>(cmd, args);
  }
  if (ESCRITAS.has(cmd)) window.dispatchEvent(new Event(SITES_MUDARAM));
  return r;
}

export const sitesApi = {
  listar: () => call<Site[]>("listar_sites"),
  definirIa: (chave: string, valor: boolean) => call<void>("definir_ia_pode_usar", { chave, valor }),
  marcarLogado: (chave: string) => call<void>("marcar_site_logado", { chave }),
  adicionar: (nome: string, url: string, tipo: string, pais: string | null) =>
    call<string>("adicionar_site", { nome, url, tipo, pais }),
  remover: (chave: string) => call<void>("remover_site", { chave }),
  abrir: (chave: string) => call<void>("abrir_site", { chave }),
  avisos: () => call<AvisoSite[]>("listar_avisos_sites"),
  contarAvisos: () => call<number>("contar_avisos_sites"),
};
