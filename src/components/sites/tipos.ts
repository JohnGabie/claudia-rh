import type { Site } from "./logica";

// Actions a site row/card can raise; the screen (index.tsx) owns the calls and the error messages.
export interface AcoesSite {
  onAbrir: (s: Site) => void;
  onLogado: (s: Site) => void;
  onIa: (s: Site, valor: boolean) => void;
  onRemover: (s: Site) => void;
}

export interface ListaProps extends AcoesSite {
  sites: Site[];
  agora: Date;
  erros: Record<string, string>;
  onDetalhe: (s: Site) => void;
}
