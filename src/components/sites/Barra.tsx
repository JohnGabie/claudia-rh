import React from "react";
import { ArrowDown, ArrowUp, LayoutGrid, List, Plus, Search } from "lucide-react";
import { useLocale, useT } from "../../i18n";
import { nomePais } from "./paises";
import { valoresDe, type Campo, type Filtros, type Ordem, type Site } from "./logica";
import { rotulo } from "./rotulos";

export type Modo = "galeria" | "lista";

interface Props {
  filtros: Filtros; onFiltros: (f: Filtros) => void;
  ordem: Ordem; onOrdem: (o: Ordem) => void;
  modo: Modo; onModo: (m: Modo) => void;
  sites: Site[]; visiveis: number; onAdicionar: () => void;
}

const campo: React.CSSProperties = {
  padding: "6px 8px", borderRadius: 6, border: "1px solid var(--border)", background: "var(--bg-surface)",
  fontSize: 12, color: "var(--text-secondary)", fontFamily: "inherit",
};

export const Barra: React.FC<Props> = ({ filtros, onFiltros, ordem, onOrdem, modo, onModo, sites, visiveis, onAdicionar }) => {
  const t = useT();
  const { locale } = useLocale();
  const mudar = (parcial: Partial<Filtros>) => onFiltros({ ...filtros, ...parcial });

  const filtro = (chave: "tipo" | "regiao" | "pais", rotuloCampo: string, mapa?: object) => (
    <select key={chave} aria-label={rotuloCampo} value={filtros[chave]} onChange={(e) => mudar({ [chave]: e.target.value })} style={campo}>
      <option value="">{rotuloCampo}: {t.sites.all}</option>
      {valoresDe(sites, chave)
        .map((v) => ({ v, nome: mapa ? rotulo(mapa, v) : nomePais(v, locale) }))
        .sort((a, b) => a.nome.localeCompare(b.nome, locale === "pt" ? "pt-BR" : "en"))
        .map(({ v, nome }) => <option key={v} value={v}>{nome}</option>)}
    </select>
  );

  const ordens: { id: Campo; texto: string }[] = [
    { id: "nome", texto: t.sites.sortName }, { id: "tipo", texto: t.sites.sortType }, { id: "regiao", texto: t.sites.sortRegion },
    { id: "pais", texto: t.sites.sortCountry }, { id: "estado", texto: t.sites.sortState },
  ];

  const botaoModo = (m: Modo, texto: string, Icone: typeof List) => (
    <button
      type="button"
      aria-pressed={modo === m}
      aria-label={texto}
      title={texto}
      onClick={() => onModo(m)}
      style={{
        display: "flex", alignItems: "center", gap: 5, padding: "6px 10px", border: "none", cursor: "pointer", fontFamily: "inherit", fontSize: 12,
        background: modo === m ? "var(--accent-soft)" : "var(--bg-surface)", color: modo === m ? "var(--accent-strong)" : "var(--text-secondary)",
      }}
    >
      <Icone size={14} /> {texto}
    </button>
  );

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap", marginBottom: 16 }}>
      <label style={{ ...campo, display: "flex", alignItems: "center", gap: 6, flex: "1 1 200px", maxWidth: 320, padding: "6px 10px" }}>
        <Search size={14} aria-hidden="true" />
        <input
          type="search"
          aria-label={t.sites.searchLabel}
          placeholder={t.sites.searchPlaceholder}
          value={filtros.busca}
          onChange={(e) => mudar({ busca: e.target.value })}
          style={{ border: "none", outline: "none", background: "transparent", flex: 1, minWidth: 0, fontSize: 12, fontFamily: "inherit", color: "var(--text-primary)" }}
        />
      </label>
      {filtro("tipo", t.sites.type, t.sites.types)}
      {filtro("regiao", t.sites.region, t.sites.regions)}
      {filtro("pais", t.sites.country)}
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        <select aria-label={t.sites.sortBy} value={ordem.campo} onChange={(e) => onOrdem({ ...ordem, campo: e.target.value as Campo })} style={campo}>
          {ordens.map((o) => <option key={o.id} value={o.id}>{t.sites.sortBy}: {o.texto}</option>)}
        </select>
        <button
          type="button" aria-label={t.sites.sortDirection} title={t.sites.sortDirection}
          onClick={() => onOrdem({ ...ordem, desc: !ordem.desc })}
          style={{ ...campo, display: "flex", alignItems: "center", padding: "6px 7px", cursor: "pointer" }}
        >
          {ordem.desc ? <ArrowDown size={14} /> : <ArrowUp size={14} />}
        </button>
      </div>
      <div role="group" aria-label={t.sites.view} style={{ display: "flex", border: "1px solid var(--border)", borderRadius: 6, overflow: "hidden" }}>
        {botaoModo("galeria", t.sites.gallery, LayoutGrid)}
        {botaoModo("lista", t.sites.list, List)}
      </div>
      <span style={{ fontSize: 12, color: "var(--text-tertiary)" }}>{t.sites.counter(visiveis, sites.length)}</span>
      <button
        type="button" onClick={onAdicionar}
        style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 6, padding: "7px 14px", borderRadius: 8, fontSize: 13, fontFamily: "inherit", cursor: "pointer", background: "var(--accent)", border: "none", color: "#fff" }}
      >
        <Plus size={14} /> {t.sites.add}
      </button>
    </div>
  );
};
