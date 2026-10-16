import React from "react";
import { useLocale, useT } from "../../i18n";
import { Avatar } from "./Avatar";
import { estadoVisual, situacaoFaixa } from "./estado";
import type { Site } from "./logica";
import { localDoSite } from "./paises";
import { EstadoLinha } from "./Pill";
import { rotulo } from "./rotulos";
import type { AcoesSite, ListaProps } from "./tipos";

type LinhaProps = { site: Site; agora: Date; erro?: string; onDetalhe: (s: Site) => void } & AcoesSite;

// Dense row: the whole row opens the detail page (stretched link on the name button).
// "Open" and the domain do nothing to the page; only the status button toggles the AI
// (or opens the page when the site is not ready yet).
const Linha: React.FC<LinhaProps> = ({ site, agora, erro, onAbrir, onIa, onDetalhe }) => {
  const t = useT();
  const { locale } = useLocale();
  const { cor, texto } = estadoVisual(site, agora, t);
  const situacao = situacaoFaixa(site);
  const nota = site.padrao_atual ? "" : `${t.sites.notDefault} · `;
  const rotuloIa = situacao === "ligada" ? t.sites.aiOpen : t.sites.aiClosed;
  return (
    <div>
      <div className="st-linha" data-chave={site.chave}>
        <span className={`risco ${cor}`} aria-hidden="true" />
        <Avatar nome={site.nome} />
        <div className="quem">
          <button type="button" className="n" title={site.nome} onClick={() => onDetalhe(site)}>{site.nome}</button>
          <div className="d">
            <span className="dom" title={site.dominios[0]}>{site.dominios[0]}</span>
            {/* white-space: pre keeps the spaces around each middle dot */}
            <span className="resto">{` · ${localDoSite(site, locale, t.sites.regions)} · ${rotulo(t.sites.types, site.tipo)} · ${nota}`}</span>
            <span className="est"><EstadoLinha cor={cor} texto={texto} /></span>
          </div>
        </div>
        <button type="button" className="st-abrir" onClick={() => onAbrir(site)}>{t.sites.openArrow}</button>
        {situacao === "falta" ? (
          <button type="button" className="st-status falta" aria-label={`${site.nome}: ${t.sites.needStep1}`} onClick={() => onDetalhe(site)}>{t.sites.rowStep1}</button>
        ) : (
          <button
            type="button"
            role="switch"
            aria-checked={situacao === "ligada"}
            aria-label={`${site.nome}: ${rotuloIa}`}
            className={`st-status ${situacao}`}
            onClick={() => onIa(site, !site.ia_pode_usar)}
          >
            {situacao === "ligada" ? t.sites.rowOpen : t.sites.rowClosed}
          </button>
        )}
      </div>
      {erro && <div role="alert" className="st-erro" style={{ marginTop: 4 }}>{erro}</div>}
    </div>
  );
};

export const Lista: React.FC<ListaProps> = ({ sites, erros, ...resto }) => (
  <div className="st-lista">
    {sites.map((s) => <Linha key={s.chave} site={s} erro={erros[s.chave]} {...resto} />)}
  </div>
);
