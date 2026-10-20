import React from "react";
import { useLocale, useT } from "../../i18n";
import { estadoVisual, situacaoFaixa } from "./estado";
import { Faixa } from "./Faixa";
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

// Rectangular card: the whole card opens the detail page (stretched link on the name button);
// "Open" and the domain open the site; only the bottom strip toggles the AI (or opens the page when not ready).
export const Cartao: React.FC<Props> = ({ site, agora, erro, onAbrir, onIa, onDetalhe }) => {
  const t = useT();
  const { locale } = useLocale();
  const abrirSite = (e: React.SyntheticEvent) => { e.stopPropagation(); e.preventDefault(); onAbrir(site); };
  const visual = estadoVisual(site, agora, t);
  const situacao = situacaoFaixa(site);

  return (
    <article className="st-card" data-chave={site.chave}>
      <div className="capa">
        <span className="fundo" aria-hidden="true">
          <span className="inicial">{Array.from(site.nome.trim())[0]?.toUpperCase() ?? "?"}</span>
        </span>
        <button type="button" className="nome" title={site.nome} onClick={() => onDetalhe(site)}>{site.nome}</button>
        <button type="button" className="st-abrir" onClick={abrirSite}>{t.sites.openArrow}</button>
      </div>
      <div className="corpo">
        <a className="dominio" href={site.url_login} title={site.dominios[0]} onClick={abrirSite}>{site.dominios[0]}</a>
        <div className="linha">
          <Pill>{localDoSite(site, locale, t.sites.regions)}</Pill>
          <Pill>{rotulo(t.sites.types, site.tipo)}</Pill>
          {site.papel === "descoberta" && <PapelPill />}
          {visual && <EstadoLinha cor={visual.cor} texto={visual.texto} />}
        </div>
        {!site.padrao_atual && <span className="st-nota">{t.sites.notDefault}</span>}
        {erro && <div role="alert" className="st-erro">{erro}</div>}
      </div>
      <Faixa
        situacao={situacao}
        nome={site.nome}
        onClick={() => (situacao === "falta" ? onDetalhe(site) : onIa(site, !site.ia_pode_usar))}
      />
    </article>
  );
};
