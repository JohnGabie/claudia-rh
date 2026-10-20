import type { Strings } from "../../i18n/en";
// Explicit .ts extension so the node test runner can resolve it (tsconfig allows it).
import { diasDesde, type Site } from "./logica.ts";

/** "semlogin" = green dot, the site does not ask for login. */
export type CorEstado = "logada" | "precisa" | "semlogin";

export const semLogin = (s: Site): boolean => s.regra_login === "nenhuma";

/** May the AI be released? Only no-login sites, or sites with a confirmed login (the backend refuses anything else). */
export const pronto = (s: Site): boolean => semLogin(s) || s.estado === "logada";

/** A site that asks for login (manual or session cookie) and is not logged in cannot be opened to the AI. */
export const bloqueado = (s: Site): boolean => !pronto(s);

export type SituacaoFaixa = "falta" | "fechada" | "ligada";
export const situacaoFaixa = (s: Site): SituacaoFaixa => (!pronto(s) ? "falta" : s.ia_pode_usar ? "ligada" : "fechada");

export type Passo1 = "pendente" | "logado" | "semLogin";
export const passo1 = (s: Site): Passo1 => (semLogin(s) ? "semLogin" : s.estado === "logada" ? "logado" : "pendente");

/** The "does not ask for login" button is offered only where it is safe (spec P5). */
export const podeAlternarSemLogin = (s: Site): boolean => s.regra_login !== "cookie_sessao";

/** "Already logged in" appears only after the site was opened from the page in this visit. */
export const mostraJaEntrei = (jaAbriu: boolean, s: Site): boolean => jaAbriu && passo1(s) === "pendente";

/** Dot color and text of the login state: green logged in or no login, red only when the login is known to be required
 *  (`precisa_login`). A manual-login site nobody configured (`desconhecido`) has no state: null, nothing is drawn. */
export function estadoVisual(s: Site, agora: Date, t: Strings): { cor: CorEstado; texto: string } | null {
  if (s.estado === "logada") {
    const d = diasDesde(s.verificada_em, agora);
    return { cor: "logada", texto: d === null ? t.sites.state.logada : t.sites.loggedAgo(d) };
  }
  if (semLogin(s)) return { cor: "semlogin", texto: t.sites.noLogin };
  if (s.estado === "precisa_login") return { cor: "precisa", texto: t.sites.state.precisa_login };
  return null;
}
