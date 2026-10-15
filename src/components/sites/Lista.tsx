import React from "react";
import { ArrowDown, ArrowUp } from "lucide-react";
import { useT } from "../../i18n";
import { Acoes } from "./Acoes";
import { Avatar } from "./Avatar";
import { Interruptor } from "./Interruptor";
import { diasDesde, type Campo, type Ordem, type Site } from "./logica";
import { EstadoBadge, Pill } from "./Pill";
import { rotulo } from "./rotulos";
import type { AcoesSite, ListaProps } from "./tipos";

const COLUNAS = "minmax(0, 1fr) 112px 124px 48px 118px";
const elipse: React.CSSProperties = { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" };

const Linha: React.FC<{ site: Site; agora: Date; erro?: string } & AcoesSite> = ({ site, agora, erro, ...acoes }) => {
  const t = useT();
  const dias = diasDesde(site.verificada_em, agora);
  return (
    <div style={{ padding: "10px 12px", borderTop: "1px solid var(--border)" }}>
      <div style={{ display: "grid", gridTemplateColumns: COLUNAS, gap: 12, alignItems: "center" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, minWidth: 0 }}>
          <Avatar nome={site.nome} />
          <div style={{ minWidth: 0 }}>
            <div title={site.nome} style={{ ...elipse, fontSize: 14, fontWeight: 500, color: "var(--text-primary)" }}>{site.nome}</div>
            <div title={site.dominios[0]} style={{ ...elipse, fontSize: 12, color: "var(--text-tertiary)" }}>{site.dominios[0]}</div>
            {!site.padrao_atual && <div style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{t.sites.notDefault}</div>}
          </div>
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
          <Pill>{rotulo(t.sites.types, site.tipo)}</Pill>
          {site.pais && <Pill>{site.pais}</Pill>}
          {site.papel === "descoberta" && <Pill>{t.sites.discovery}</Pill>}
        </div>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 3 }}>
          <EstadoBadge estado={site.estado} />
          {dias !== null && <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{t.sites.confirmedDaysAgo(dias)}</span>}
        </div>
        <Interruptor ligado={site.ia_pode_usar} nome={`${t.sites.aiCanUse}: ${site.nome}`} onMudar={(v) => acoes.onIa(site, v)} />
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          <Acoes site={site} {...acoes} />
        </div>
      </div>
      {erro && <div role="alert" style={{ fontSize: 12, color: "var(--danger)", marginTop: 6, overflowWrap: "anywhere" }}>{erro}</div>}
    </div>
  );
};

const Cabecalho: React.FC<{ ordem: Ordem; onOrdem: (o: Ordem) => void }> = ({ ordem, onOrdem }) => {
  const t = useT();
  // First click sorts by the column, a second click on the same column reverses it.
  const ordenar = (campo: Campo) => onOrdem({ campo, desc: ordem.campo === campo ? !ordem.desc : false });
  const col = (campo: Campo, texto: string) => (
    <button
      type="button"
      onClick={() => ordenar(campo)}
      aria-label={`${t.sites.sortBy}: ${texto}`}
      style={{
        display: "inline-flex", alignItems: "center", gap: 3, padding: 0, background: "transparent", border: "none", cursor: "pointer",
        fontFamily: "inherit", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: ".04em",
        color: ordem.campo === campo ? "var(--accent-strong)" : "var(--text-tertiary)",
      }}
    >
      {texto}
      {ordem.campo === campo && (ordem.desc ? <ArrowDown size={11} /> : <ArrowUp size={11} />)}
    </button>
  );
  return (
    <div style={{ display: "grid", gridTemplateColumns: COLUNAS, gap: 12, padding: "8px 12px" }}>
      <div>{col("nome", t.sites.sortName)}</div>
      <div style={{ display: "flex", gap: 8 }}>{col("tipo", t.sites.type)}{col("pais", t.sites.country)}</div>
      <div>{col("estado", t.sites.sortState)}</div>
      <div /><div />
    </div>
  );
};

export const Lista: React.FC<ListaProps & { ordem: Ordem; onOrdem: (o: Ordem) => void }> = ({ sites, agora, erros, ordem, onOrdem, ...acoes }) => (
  <div style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, overflow: "hidden" }}>
    <Cabecalho ordem={ordem} onOrdem={onOrdem} />
    {sites.map((s) => <Linha key={s.chave} site={s} agora={agora} erro={erros[s.chave]} {...acoes} />)}
  </div>
);
