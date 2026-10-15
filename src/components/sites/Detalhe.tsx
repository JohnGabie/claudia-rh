import React from "react";
import { useLocale, useT } from "../../i18n";
import { Acoes } from "./Acoes";
import { estadoVisual } from "./estado";
import type { Site } from "./logica";
import { localDoSite } from "./paises";
import { EstadoLinha, Pill } from "./Pill";
import { rotulo } from "./rotulos";
import type { AcoesSite } from "./tipos";

type Props = { site: Site; agora: Date; erro?: string; onVoltar: () => void } & AcoesSite;

// Site detail page (state-based inside the Sites screen).
export const Detalhe: React.FC<Props> = ({ site, agora, erro, onVoltar, ...acoes }) => {
  const t = useT();
  const { locale } = useLocale();
  const { cor, texto } = estadoVisual(site, agora, t);
  const local = localDoSite(site, locale, t.sites.regions);
  const link = (
    <a className="dominio inline" href={site.url_login} onClick={(e) => { e.preventDefault(); acoes.onAbrir(site); }}>{site.url_login}</a>
  );
  const linhas: [string, React.ReactNode][] = [
    [t.sites.address, link],
    [t.sites.type, rotulo(t.sites.types, site.tipo)],
    [t.sites.region, rotulo(t.sites.regions, site.regiao)],
    [t.sites.country, site.pais ? local : t.sites.noCountry],
    [t.sites.role, site.papel === "descoberta" ? t.sites.roleDiscovery : t.sites.roleApplication],
    [t.sites.origin, site.embutida ? t.sites.originBundled : t.sites.originUser],
    [t.sites.domains, site.dominios.join(", ")],
  ];
  return (
    <section className="st-pagina" aria-labelledby="st-titulo">
      <header className="topo-p">
        <button type="button" className="st-abrir" onClick={onVoltar}>{t.sites.back}</button>
        <h2 id="st-titulo" title={site.nome}>{site.nome}</h2>
        <Pill>{local}</Pill>
        <Pill>{rotulo(t.sites.types, site.tipo)}</Pill>
        <span style={{ marginLeft: "auto" }}><EstadoLinha cor={cor} texto={texto} /></span>
      </header>
      <div className="grade-p">
        <div className="st-painel">
          <h3>{t.sites.aboutTitle}</h3>
          <dl>
            {linhas.map(([k, v]) => <React.Fragment key={k}><dt>{k}</dt><dd>{v}</dd></React.Fragment>)}
          </dl>
        </div>
        <Acoes site={site} erro={erro} {...acoes} />
      </div>
    </section>
  );
};
