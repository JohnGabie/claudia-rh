import React from "react";
import { useLocale, useT } from "../../i18n";
import type { SiteStats } from "../../lib/sites-api";
import { semLogin } from "./estado";
import { dataLocal, type Site } from "./logica";
import { localDoSite } from "./paises";
import { rotulo } from "./rotulos";

/** Stats of the page: not loaded yet, loaded, or the load failed (only the usage block reacts to it). */
export type EstadoStats = { tipo: "carregando" } | { tipo: "ok"; stats: SiteStats } | { tipo: "erro" };

const Linha: React.FC<{ rotulo: string; sub?: string | null; children: React.ReactNode }> = ({ rotulo: r, sub, children }) => (
  <>
    <dt>{r}</dt>
    <dd>{children}{sub ? <span className="sub">{sub}</span> : null}</dd>
  </>
);

const Divisoria: React.FC<{ children: string }> = ({ children }) => <div className="st-divisoria"><span>{children}</span></div>;

// "About the site": registry, account and AI usage, separated by titled dividers.
export const Sobre: React.FC<{ site: Site; stats: EstadoStats; onAbrir: () => void }> = ({ site, stats, onAbrir }) => {
  const t = useT();
  const { locale } = useLocale();
  const s = t.sites;
  const local = localDoSite(site, locale, s.regions);
  const confirmado = dataLocal(site.verificada_em, locale);

  let uso: React.ReactNode;
  if (stats.tipo === "carregando") {
    uso = <p className="vazio" aria-busy="true">{s.loading}</p>;
  } else if (stats.tipo === "erro") {
    uso = <p className="vazio st-erro" role="status">{s.statsUnavailable}</p>;
  } else if (stats.stats.aberturas === 0) {
    uso = <p className="vazio">{s.neverOpened}</p>;
  } else {
    const ultima = dataLocal(stats.stats.ultima_abertura, locale);
    uso = <dl><Linha rotulo={s.opens} sub={ultima ? s.lastOn(ultima) : null}>{stats.stats.aberturas}</Linha></dl>;
  }

  return (
    <section className="st-painel" aria-labelledby="st-sobre-t">
      <h3 id="st-sobre-t">{s.aboutTitle}</h3>
      <dl>
        <Linha rotulo={s.address}>
          <a className="dominio inline" href={site.url_login} onClick={(e) => { e.preventDefault(); onAbrir(); }}>{site.url_login}</a>
        </Linha>
        <Linha rotulo={s.type}>{rotulo(s.types, site.tipo)}</Linha>
        <Linha rotulo={s.region}>{rotulo(s.regions, site.regiao)}</Linha>
        <Linha rotulo={s.country}>{site.pais ? local : s.noCountry}</Linha>
        <Linha rotulo={s.role}>{site.papel === "descoberta" ? s.roleDiscovery : s.roleApplication}</Linha>
        <Linha rotulo={s.origin}>{site.embutida ? s.originBundled : s.originUser}</Linha>
        <Linha rotulo={s.domains}>{site.dominios.join(", ")}</Linha>
      </dl>
      <Divisoria>{s.sectionAccount}</Divisoria>
      {semLogin(site) ? (
        <dl><Linha rotulo={s.loginRow}>{s.loginNotAsked}</Linha></dl>
      ) : (
        <dl>
          <Linha rotulo={s.confirmedOn}>{confirmado ?? s.notEntered}</Linha>
          {stats.tipo === "ok" && (
            <Linha rotulo={s.relogins} sub={stats.stats.relogins === 1 ? s.timesOne : s.timesMany}>{stats.stats.relogins}</Linha>
          )}
        </dl>
      )}
      <Divisoria>{s.sectionUsage}</Divisoria>
      {uso}
    </section>
  );
};
