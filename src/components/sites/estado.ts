import type { Strings } from "../../i18n/en";
import { diasDesde, type Site } from "./logica";

export type CorEstado = "logada" | "sem" | "precisa";

/** A manual-login site that is not logged in cannot be opened to the AI (the backend refuses it too). */
export const bloqueado = (s: Site): boolean => s.regra_login === "manual" && s.estado !== "logada";

/** Dot color and text of the login state: green logged in, amber unconfirmed, grey needs login / no login. */
export function estadoVisual(s: Site, agora: Date, t: Strings): { cor: CorEstado; texto: string } {
  if (s.estado === "logada") {
    const d = diasDesde(s.verificada_em, agora);
    return { cor: "logada", texto: d === null ? t.sites.state.logada : t.sites.loggedAgo(d) };
  }
  if (s.regra_login !== "manual") return { cor: "precisa", texto: t.sites.noLogin };
  if (s.estado === "desconhecido") return { cor: "sem", texto: t.sites.state.desconhecido };
  return { cor: "precisa", texto: t.sites.state.precisa_login };
}
