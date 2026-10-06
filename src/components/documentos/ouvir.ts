// One place that decides when a screen showing the document library must reload: the backend event
// `documentos-atualizados` (migration, cover-letter queue), the DB-change event `db-atualizada`, and
// the local write event raised by documentos-api. Use inside useEffect and return the result.
import { listen } from "@tauri-apps/api/event";
import { DOCUMENTOS_MUDARAM } from "../../lib/documentos-api";
import { criarOuvinte, type Assinar, type Desassinar } from "./ouvirCore";

export const EVENTO_DOCUMENTOS_ATUALIZADOS = "documentos-atualizados";
export const EVENTO_DB_ATUALIZADA = "db-atualizada";

const tauri = (evento: string): Assinar => (cb) => listen(evento, cb);
const local: Assinar = (cb) => {
  window.addEventListener(DOCUMENTOS_MUDARAM, cb);
  return () => window.removeEventListener(DOCUMENTOS_MUDARAM, cb);
};

export function ouvirDocumentos(recarregar: () => void): Desassinar {
  return criarOuvinte(recarregar, [tauri(EVENTO_DOCUMENTOS_ATUALIZADOS), tauri(EVENTO_DB_ATUALIZADA), local]);
}
