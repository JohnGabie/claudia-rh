import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useT } from "../../i18n";
import { sitesApi, type AvisoSite } from "../../lib/sites-api";
import { AvisosFaixa } from "./AvisosFaixa";
import { Barra, type Modo } from "./Barra";
import { Detalhe } from "./Detalhe";
import { ESTILOS } from "./estilos";
import { FormAdicionar } from "./FormAdicionar";
import { Galeria } from "./Galeria";
import { Lista } from "./Lista";
import { filtrar, ordenar, type Filtros, type Ordem, type Site } from "./logica";
import { ouvirSites } from "./ouvir";

const CHAVE_MODO = "sites-modo";
const SEM_FILTROS: Filtros = { busca: "", tipo: "", regiao: "", pais: "" };

// The remembered view lives in localStorage, which may be missing or throw (private window, blocked data).
function lerModo(): Modo {
  try {
    return localStorage.getItem(CHAVE_MODO) === "lista" ? "lista" : "galeria";
  } catch {
    return "galeria";
  }
}
function gravarModo(m: Modo) {
  try { localStorage.setItem(CHAVE_MODO, m); } catch { /* the choice just is not remembered */ }
}

const btn: React.CSSProperties = {
  padding: "7px 14px", borderRadius: 8, fontSize: 13, fontFamily: "inherit", cursor: "pointer",
  background: "transparent", border: "1px solid var(--border)", color: "var(--text-secondary)",
};

export const Sites: React.FC = () => {
  const t = useT();
  const [sites, setSites] = useState<Site[] | null>(null);
  const [avisos, setAvisos] = useState<AvisoSite[]>([]);
  const [erro, setErro] = useState<string | null>(null);
  const [erros, setErros] = useState<Record<string, string>>({});
  const [filtros, setFiltros] = useState<Filtros>(SEM_FILTROS);
  const [ordem, setOrdem] = useState<Ordem>({ campo: "nome", desc: false });
  const [modo, setModo] = useState<Modo>(lerModo);
  const [adicionando, setAdicionando] = useState(false);
  const [aberto, setAberto] = useState<string | null>(null);
  const [agora, setAgora] = useState(() => new Date());

  const carregar = useCallback(async () => {
    try {
      const [lista, av] = await Promise.all([sitesApi.listar(), sitesApi.avisos()]);
      setSites(lista);
      setAvisos(av);
      setAgora(new Date());
      setErro(null);
    } catch (e) {
      setErro(String(e));
    }
  }, []);

  useEffect(() => { carregar(); }, [carregar]);
  useEffect(() => ouvirSites(carregar), [carregar]);

  const trocarModo = (m: Modo) => { setModo(m); gravarModo(m); };

  // Runs a backend action; a refusal is shown on the site it belongs to, then the list is reloaded.
  const agir = async (chave: string, acao: () => Promise<unknown>) => {
    setErros((e) => { const { [chave]: _, ...resto } = e; return resto; });
    try {
      await acao();
    } catch (e) {
      setErros((x) => ({ ...x, [chave]: String(e) }));
    }
    await carregar();
  };

  const acoes = {
    onAbrir: (s: Site) => agir(s.chave, () => sitesApi.abrir(s.chave)),
    onLogado: (s: Site) => agir(s.chave, () => sitesApi.marcarLogado(s.chave)),
    onIa: (s: Site, v: boolean) => agir(s.chave, () => sitesApi.definirIa(s.chave, v)),
    onRemover: (s: Site) => agir(s.chave, () => sitesApi.remover(s.chave)),
    onDetalhe: (s: Site) => setAberto(s.chave),
  };
  const detalhe = sites?.find((s) => s.chave === aberto) ?? null;

  const visiveis = useMemo(() => (sites ? ordenar(filtrar(sites, filtros), ordem) : []), [sites, filtros, ordem]);

  let corpo: React.ReactNode;
  if (detalhe) {
    const { onDetalhe: _d, ...porSite } = acoes;
    corpo = <Detalhe site={detalhe} agora={agora} erro={erros[detalhe.chave]} onVoltar={() => setAberto(null)} {...porSite} />;
  } else if (erro && sites === null) {
    corpo = (
      <div role="alert" style={{ border: "1px solid var(--danger)", background: "var(--bg-surface)", borderRadius: 8, padding: "14px 16px", display: "flex", alignItems: "center", gap: 12 }}>
        <span style={{ flex: 1, fontSize: 13, color: "var(--danger)", overflowWrap: "anywhere" }}>{t.sites.error}: {erro}</span>
        <button type="button" onClick={carregar} style={btn}>{t.sites.retry}</button>
      </div>
    );
  } else if (sites === null) {
    corpo = <div style={{ fontSize: 13, color: "var(--text-tertiary)" }}>{t.sites.loading}</div>;
  } else {
    corpo = (
      <>
        <Barra
          filtros={filtros} onFiltros={setFiltros} ordem={ordem} onOrdem={setOrdem} modo={modo} onModo={trocarModo}
          sites={sites} visiveis={visiveis.length} onAdicionar={() => setAdicionando(true)}
        />
        {adicionando && (
          <FormAdicionar
            onCancelar={() => setAdicionando(false)}
            onSalvar={async (nome, url, tipo, pais) => { await sitesApi.adicionar(nome, url, tipo, pais); setAdicionando(false); await carregar(); }}
          />
        )}
        {visiveis.length === 0 ? (
          <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: 13, color: "var(--text-secondary)" }}>
            <span>{t.sites.noMatch}</span>
            <button type="button" onClick={() => setFiltros(SEM_FILTROS)} style={btn}>{t.sites.clearFilters}</button>
          </div>
        ) : modo === "galeria" ? (
          <Galeria sites={visiveis} agora={agora} erros={erros} {...acoes} />
        ) : (
          <Lista sites={visiveis} agora={agora} erros={erros} {...acoes} />
        )}
      </>
    );
  }

  return (
    <div className="sites-tela" style={{ padding: 24, overflow: "auto", height: "100%" }}>
      {!detalhe && <h1 style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: "0 0 16px" }}>{t.sites.title}</h1>}
      <AvisosFaixa avisos={avisos} onAbrir={(chave) => agir(chave, () => sitesApi.abrir(chave))} />
      {corpo}
      <style>{ESTILOS}</style>
    </div>
  );
};
