// The only place the front talks to the Sites commands. In `vite dev` outside Tauri, dev-mock.ts
// answers these commands from sites.mock.ts, so there is no mock path here.
import { invoke } from "@tauri-apps/api/core";
import type { Site } from "../components/sites/logica";

export interface AvisoSite { id: number; chave: string; nome: string; criado_em: string }

export interface SiteStats { relogins: number; aberturas: number; ultima_abertura: string | null; vagas: number; candidaturas: number; ultima_candidatura: string | null }

// Fired after any write, so the screen reloads even when the backend event never arrives (mock).
export const SITES_MUDARAM = "sites-mudaram";
const ESCRITAS = new Set(["definir_ia_pode_usar", "marcar_site_logado", "adicionar_site", "remover_site", "definir_site_sem_login", "marcar_site_nao_logado", "dispensar_aviso_site"]);

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const r = await invoke<T>(cmd, args);
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
  estatisticas: (chave: string) => call<SiteStats>("estatisticas_site", { chave }),
  definirSemLogin: (chave: string, valor: boolean) => call<void>("definir_site_sem_login", { chave, valor }),
  marcarNaoLogado: (chave: string) => call<void>("marcar_site_nao_logado", { chave }),
  abrir: (chave: string) => call<void>("abrir_site", { chave }),
  avisos: () => call<AvisoSite[]>("listar_avisos_sites"),
  dispensarAviso: (id: number) => call<void>("dispensar_aviso_site", { id }),
  contarAvisos: () => call<number>("contar_avisos_sites"),
};
