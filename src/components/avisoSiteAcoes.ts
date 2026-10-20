// Which actions a "log in again" notice card offers. "I logged in" only becomes available after
// the site was opened from this card, and everything is locked while a command is in flight.
export interface AvisoSiteEstado { abriu: boolean; ocupado: boolean }

export function acoesAvisoSite(e: AvisoSiteEstado): { podeAbrir: boolean; mostrarJaEntrei: boolean; podeJaEntrei: boolean } {
  return { podeAbrir: !e.ocupado, mostrarJaEntrei: e.abriu, podeJaEntrei: e.abriu && !e.ocupado };
}
