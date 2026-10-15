import React from "react";
import { useLocale, useT } from "../../i18n";
import { estadoVisual, bloqueado as estaBloqueado } from "./estado";
import { Faixa } from "./Faixa";
import { Interruptor } from "./Interruptor";
import type { Site } from "./logica";
import { localDoSite } from "./paises";
import { EstadoLinha, PapelPill, Pill } from "./Pill";
import { rotulo } from "./rotulos";
import type { AcoesSite } from "./tipos";

interface Props extends AcoesSite {
  site: Site;
  agora: Date;
  erro?: string;
  onDetalhe: (s: Site) => void;
}

// Option A: generated cover with the name, domain link, pills, login state, and the AI strip.
// The whole card toggles the AI; the cover button, links and "I logged in" never do.
export const Cartao: React.FC<Props> = ({ site, agora, erro, onAbrir, onLogado, onIa, onDetalhe }) => {
  const t = useT();
  const { locale } = useLocale();
  const parar = (f: () => void) => (e: React.SyntheticEvent) => { e.stopPropagation(); e.preventDefault(); f(); };
  const bloq = estaBloqueado(site);
  const { cor, texto } = estadoVisual(site, agora, t);
  const situacao = bloq ? t.sites.aiBlocked : site.ia_pode_usar ? t.sites.aiOpen : t.sites.aiClosed;

  return (
    <Interruptor
      className="st-card"
      chave={site.chave}
      ligado={site.ia_pode_usar}
      bloqueado={bloq}
      nome={`${site.nome}: ${situacao}`}
      onMudar={() => onIa(site, !site.ia_pode_usar)}
    >
      <div className="capa">
        <span className="inicial" aria-hidden="true">{Array.from(site.nome.trim())[0]?.toUpperCase() ?? "?"}</span>
        <button type="button" className="st-abrir" onClick={parar(() => onAbrir(site))}>{t.sites.openArrow}</button>
        <button type="button" className="nome" title={site.nome} onClick={parar(() => onDetalhe(site))}>{site.nome}</button>
      </div>
      <div className="corpo">
        <a className="dominio" href={site.url_login} title={site.dominios[0]} onClick={parar(() => onAbrir(site))}>{site.dominios[0]}</a>
        <div className="linha">
          <Pill>{localDoSite(site, locale, t.sites.regions)}</Pill>
          <Pill>{rotulo(t.sites.types, site.tipo)}</Pill>
          {site.papel === "descoberta" && <PapelPill />}
        </div>
        <EstadoLinha cor={cor} texto={texto} />
        {!site.padrao_atual && <span className="st-nota">{t.sites.notDefault}</span>}
        {erro && <div role="alert" className="st-erro">{erro}</div>}
      </div>
      <Faixa ligado={site.ia_pode_usar} bloqueado={bloq} onLogou={() => onLogado(site)} />
    </Interruptor>
  );
};
