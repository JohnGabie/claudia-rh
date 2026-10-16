import React from "react";
import { useLocale, useT } from "../../i18n";
import { Avatar } from "./Avatar";
import { bloqueado as estaBloqueado, estadoVisual } from "./estado";
import { Interruptor } from "./Interruptor";
import type { Site } from "./logica";
import { localDoSite } from "./paises";
import { rotulo } from "./rotulos";
import type { AcoesSite, ListaProps } from "./tipos";

type LinhaProps = { site: Site; agora: Date; erro?: string; onDetalhe: (s: Site) => void } & AcoesSite;

// Option C: dense row. Stripe = login state, whole row toggles the AI, status text at the end.
const Linha: React.FC<LinhaProps> = ({ site, agora, erro, onAbrir, onIa, onDetalhe }) => {
  const t = useT();
  const { locale } = useLocale();
  const bloq = estaBloqueado(site);
  const cor = estadoVisual(site, agora, t)?.cor ?? "nenhum";
  const fim = bloq ? t.sites.rowBlocked : site.ia_pode_usar ? t.sites.rowOpen : t.sites.rowClosed;
  const situacao = bloq ? t.sites.aiBlocked : site.ia_pode_usar ? t.sites.aiOpen : t.sites.aiClosed;
  return (
    <div>
      <Interruptor
        className="st-linha"
        chave={site.chave}
        ligado={site.ia_pode_usar}
        bloqueado={bloq}
        nome={`${site.nome}: ${situacao}`}
        onMudar={() => onIa(site, !site.ia_pode_usar)}
      >
        <span className={`risco ${cor}`} aria-hidden="true" />
        <Avatar nome={site.nome} />
        <div className="quem">
          <button type="button" className="n" title={site.nome} onClick={(e) => { e.stopPropagation(); onDetalhe(site); }}>{site.nome}</button>
          <div className="d">
            <span className="dom" title={site.dominios[0]}>{site.dominios[0]}</span>
            <span className="resto">
              {" · "}{localDoSite(site, locale, t.sites.regions)} · {rotulo(t.sites.types, site.tipo)}
              {!site.padrao_atual && ` · ${t.sites.notDefault}`}
            </span>
          </div>
        </div>
        <button type="button" className="st-abrir" onClick={(e) => { e.stopPropagation(); onAbrir(site); }}>{t.sites.openArrow}</button>
        <span className="ia">{fim}</span>
      </Interruptor>
      {erro && <div role="alert" className="st-erro" style={{ marginTop: 4 }}>{erro}</div>}
    </div>
  );
};

export const Lista: React.FC<ListaProps> = ({ sites, erros, ...resto }) => (
  <div className="st-lista">
    {sites.map((s) => <Linha key={s.chave} site={s} erro={erros[s.chave]} {...resto} />)}
  </div>
);
