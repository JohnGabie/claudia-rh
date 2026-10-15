import React from "react";
import { useT } from "../../i18n";
import { mostraJaEntrei, passo1, podeAlternarSemLogin, pronto, situacaoFaixa } from "./estado";
import { Faixa } from "./Faixa";
import type { Site } from "./logica";
import type { AcoesSite } from "./tipos";

type Estado = "atual" | "feito" | "travado";

const Passo: React.FC<{ n: number; estado: Estado; titulo: string; children: React.ReactNode }> = ({ n, estado, titulo, children }) => (
  <li className={`st-passo ${estado}`} aria-current={estado === "atual" ? "step" : undefined}>
    <div className="num" aria-hidden="true">{estado === "feito" ? "✓" : n}</div>
    <div>
      <h4>{titulo}</h4>
      {children}
    </div>
  </li>
);

interface Props extends Pick<AcoesSite, "onLogado" | "onIa" | "onSemLogin" | "onNaoLogado"> {
  site: Site;
  jaAbriu: boolean;
  onAbrirSite: () => void;
}

// Panel "What the AI can do here": three steps, always in the order 1, 2, 3.
export const Passos: React.FC<Props> = ({ site, jaAbriu, onAbrirSite, onLogado, onIa, onSemLogin, onNaoLogado }) => {
  const t = useT();
  const p1 = passo1(site);
  const liberavel = pronto(site);
  const abrir = <button type="button" className={`st-btn ${p1 === "pendente" ? "primario" : ""}`} onClick={onAbrirSite}>{t.sites.openSite}</button>;
  const situacao = situacaoFaixa(site);

  return (
    <section className="st-painel" aria-labelledby="st-ia-t">
      <h3 id="st-ia-t">{t.sites.aiTitle}</h3>
      <ol className="st-passos" style={{ listStyle: "none", margin: 0, padding: 0 }}>
        {p1 === "pendente" && (
          <Passo n={1} estado="atual" titulo={t.sites.step1Title}>
            <p>{t.sites.step1Hint}</p>
            <div className="acoes">
              {abrir}
              {mostraJaEntrei(jaAbriu, site)
                ? <button type="button" className="st-btn" onClick={() => onLogado(site)}>{t.sites.enteredBtn}</button>
                : <span className="meta">{t.sites.afterOpenHint}</span>}
              {podeAlternarSemLogin(site) && <button type="button" className="st-btn" onClick={() => onSemLogin(site, true)}>{t.sites.noLoginBtn}</button>}
            </div>
          </Passo>
        )}
        {p1 === "logado" && (
          <Passo n={1} estado="feito" titulo={t.sites.step1Title}>
            <p>{t.sites.step1Done}</p>
            <div className="acoes">
              <span className="st-selo ok">{t.sites.doneBadge}</span>
              {abrir}
              <button type="button" className="st-btn" onClick={() => onNaoLogado(site)}>{t.sites.notLoggedBtn}</button>
            </div>
          </Passo>
        )}
        {p1 === "semLogin" && (
          <Passo n={1} estado="feito" titulo={t.sites.step1Title}>
            <p>{t.sites.step1NoLogin}</p>
            <div className="acoes">
              <span className="st-selo ok">{t.sites.doneBadge}</span>
              {podeAlternarSemLogin(site) && <button type="button" className="st-btn" onClick={() => onSemLogin(site, false)}>{t.sites.asksLoginBtn}</button>}
            </div>
          </Passo>
        )}
        <Passo n={2} estado="travado" titulo={t.sites.step2Title}>
          <p>{t.sites.step2Hint}</p>
          <div className="acoes">
            <button type="button" className="st-btn" disabled>{t.sites.step2Btn}</button>
            <span className="st-selo">{t.sites.soonBadge}</span>
          </div>
        </Passo>
        <Passo n={3} estado={liberavel ? "atual" : "travado"} titulo={t.sites.step3Title}>
          <p>{liberavel ? t.sites.step3Hint : t.sites.step3Locked}</p>
          {liberavel
            ? <Faixa situacao={situacao} nome={site.nome} onClick={() => onIa(site, !site.ia_pode_usar)} />
            : <div className="acoes"><span className="st-selo">{t.sites.lockedBadge}</span></div>}
        </Passo>
      </ol>
    </section>
  );
};
