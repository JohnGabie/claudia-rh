import React, { useCallback, useEffect, useRef, useState } from "react";
import { useLocale, useT } from "../../i18n";
import { sitesApi } from "../../lib/sites-api";
import { estadoVisual } from "./estado";
import type { Site } from "./logica";
import { ouvirSites } from "./ouvir";
import { localDoSite } from "./paises";
import { Passos } from "./Passos";
import { EstadoLinha, Pill } from "./Pill";
import { rotulo } from "./rotulos";
import { Sobre, type EstadoStats } from "./Sobre";
import type { AcoesSite } from "./tipos";
import { ZonaRisco } from "./ZonaRisco";

type Props = { site: Site; agora: Date; erro?: string; onVoltar: () => void } & AcoesSite;

// Site detail page (state-based inside the Sites screen): header, two equal-height columns
// (About the site, steps) and the collapsible danger zone.
export const Detalhe: React.FC<Props> = ({ site, agora, erro, onVoltar, ...acoes }) => {
  const t = useT();
  const { locale } = useLocale();
  const visual = estadoVisual(site, agora, t);
  const local = localDoSite(site, locale, t.sites.regions);
  // Local to this visit: leaving and re-entering the page resets it.
  const [jaAbriu, setJaAbriu] = useState(false);
  const [riscoAberto, setRiscoAberto] = useState(false);
  const [stats, setStats] = useState<EstadoStats>({ tipo: "carregando" });
  const pedido = useRef(0);

  const chave = site.chave;
  const carregar = useCallback(async () => {
    const meu = ++pedido.current;
    try {
      const r = await sitesApi.estatisticas(chave);
      if (meu === pedido.current) setStats({ tipo: "ok", stats: r });
    } catch {
      if (meu === pedido.current) setStats({ tipo: "erro" });
    }
  }, [chave]);

  useEffect(() => { carregar(); return () => { pedido.current++; }; }, [carregar]);
  useEffect(() => ouvirSites(carregar), [carregar]);

  return (
    <section className="st-pagina" aria-labelledby="st-titulo">
      <header className="topo-p">
        <button type="button" className="st-abrir" onClick={onVoltar}>{t.sites.back}</button>
        <h2 id="st-titulo" tabIndex={-1} style={{ outline: "none" }} title={site.nome}>{site.nome}</h2>
        <Pill>{local}</Pill>
        <Pill>{rotulo(t.sites.types, site.tipo)}</Pill>
        {visual && <span style={{ marginLeft: "auto" }}><EstadoLinha cor={visual.cor} texto={visual.texto} /></span>}
      </header>
      {erro && <div role="alert" className="st-erro" style={{ marginBottom: 12 }}>{erro}</div>}
      <div className="grade-p">
        <Sobre site={site} stats={stats} onAbrir={() => acoes.onAbrir(site)} />
        <Passos
          site={site} jaAbriu={jaAbriu}
          onAbrirSite={() => { setJaAbriu(true); acoes.onAbrir(site); }}
          onLogado={acoes.onLogado} onIa={acoes.onIa} onSemLogin={acoes.onSemLogin} onNaoLogado={acoes.onNaoLogado}
        />
      </div>
      <ZonaRisco site={site} aberta={riscoAberto} onAlternar={setRiscoAberto} onRemover={acoes.onRemover} />
    </section>
  );
};
