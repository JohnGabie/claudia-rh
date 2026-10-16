// The Tauri events that make the Sites screen reload. Kept free of runtime imports so a node test can load it.
// The gate runs in the MCP process, so a login wall or a counted open only reaches the app through the
// notice events raised by the notifications poller, besides the DB-change event.
import type { Assinar, Desassinar } from "../documentos/ouvirCore";

export const EVENTOS_TAURI = ["db-atualizada", "novo-aviso-site", "aviso-site-resolvido"] as const;

type Listen = (evento: string, cb: () => void) => Promise<Desassinar>;

// One subscription per event; `listen` is injected so a test can cover each one.
export const fontesTauri = (escutar: Listen): Assinar[] => EVENTOS_TAURI.map((ev) => (cb) => escutar(ev, cb));
