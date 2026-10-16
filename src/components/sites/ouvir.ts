// When the Sites screen must reload: the Tauri events (see ouvirEventos) and the local write event raised
// by sites-api. Reuses the debounced listener of the documents screen. Use inside useEffect.
import { listen } from "@tauri-apps/api/event";
import { SITES_MUDARAM } from "../../lib/sites-api";
import { criarOuvinte, type Assinar, type Desassinar } from "../documentos/ouvirCore";
import { fontesTauri } from "./ouvirEventos";

const local: Assinar = (cb) => {
  window.addEventListener(SITES_MUDARAM, cb);
  return () => window.removeEventListener(SITES_MUDARAM, cb);
};

export const ouvirSites = (recarregar: () => void): Desassinar =>
  criarOuvinte(recarregar, [...fontesTauri((ev, cb) => listen(ev, cb)), local]);
