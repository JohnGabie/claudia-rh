import React from "react";
import { useT } from "../../i18n";
import { Acoes } from "./Acoes";
import { Avatar } from "./Avatar";
import { Interruptor } from "./Interruptor";
import { diasDesde, type Site } from "./logica";
import { EstadoBadge, Pill } from "./Pill";
import { rotulo } from "./rotulos";
import type { AcoesSite, ListaProps } from "./tipos";

const elipse: React.CSSProperties = { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" };

const Cartao: React.FC<{ site: Site; agora: Date; erro?: string } & AcoesSite> = ({ site, agora, erro, ...acoes }) => {
  const t = useT();
  const dias = diasDesde(site.verificada_em, agora);
  return (
    <div
      style={{
        background: "var(--bg-surface)", border: "1px solid var(--border)", borderRadius: 8, padding: 12,
        display: "flex", flexDirection: "column", gap: 10, minWidth: 0,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 10, minWidth: 0 }}>
        <Avatar nome={site.nome} />
        <div style={{ minWidth: 0, flex: 1 }}>
          <div title={site.nome} style={{ ...elipse, fontSize: 14, fontWeight: 500, color: "var(--text-primary)" }}>{site.nome}</div>
          <div title={site.dominios[0]} style={{ ...elipse, fontSize: 12, color: "var(--text-tertiary)" }}>{site.dominios[0]}</div>
        </div>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        <Pill>{rotulo(t.sites.types, site.tipo)}</Pill>
        {site.pais && <Pill>{site.pais}</Pill>}
        {site.papel === "descoberta" && <Pill>{t.sites.discovery}</Pill>}
      </div>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 3 }}>
        <EstadoBadge estado={site.estado} />
        {dias !== null && <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{t.sites.confirmedDaysAgo(dias)}</span>}
        {!site.padrao_atual && <span style={{ fontSize: 11, color: "var(--text-tertiary)" }}>{t.sites.notDefault}</span>}
      </div>
      <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, color: "var(--text-secondary)" }}>
        <Interruptor ligado={site.ia_pode_usar} nome={`${t.sites.aiCanUse}: ${site.nome}`} onMudar={(v) => acoes.onIa(site, v)} />
        {t.sites.aiCanUse}
      </label>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center" }}>
        <Acoes site={site} {...acoes} />
      </div>
      {erro && <div role="alert" style={{ fontSize: 12, color: "var(--danger)", overflowWrap: "anywhere" }}>{erro}</div>}
    </div>
  );
};

export const Galeria: React.FC<ListaProps> = ({ sites, agora, erros, ...acoes }) => (
  <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(190px, 1fr))", gap: 16 }}>
    {sites.map((s) => <Cartao key={s.chave} site={s} agora={agora} erro={erros[s.chave]} {...acoes} />)}
  </div>
);
