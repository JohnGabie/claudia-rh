import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
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

// `voltarSinal` changes each time the sidebar item is clicked: the detail page closes and the list shows.
export const Sites: React.FC<{ voltarSinal?: number }> = ({ voltarSinal = 0 }) => {
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
  const telaRef = useRef<HTMLDivElement>(null);
  const tituloRef = useRef<HTMLHeadingElement>(null);
  const rolagem = useRef(0);
  const ultimo = useRef<string | null>(null);

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
    onSemLogin: (s: Site, v: boolean) => agir(s.chave, () => sitesApi.definirSemLogin(s.chave, v)),
    onNaoLogado: (s: Site) => agir(s.chave, () => sitesApi.marcarNaoLogado(s.chave)),
    onDetalhe: (s: Site) => { rolagem.current = telaRef.current?.scrollTop ?? 0; ultimo.current = s.chave; setAberto(s.chave); },
  };
  const detalhe = sites?.find((s) => s.chave === aberto) ?? null;
  const chaveAberta = detalhe?.chave ?? null;

  // The list stays mounted (hidden) under the detail page, so filters, sort and view survive. When the
  // detail closes (back button, sidebar, or the site was removed) restore the scroll and move focus
  // to the card that was open, or to the heading when that card is gone.
  const antes = useRef<string | null>(null);
  useLayoutEffect(() => {
    if (antes.current !== null && chaveAberta === null) {
      if (telaRef.current) telaRef.current.scrollTop = rolagem.current;
      const alvo = telaRef.current?.querySelector<HTMLElement>(`[data-chave="${CSS.escape(antes.current)}"]`) ?? tituloRef.current;
      alvo?.focus({ preventScroll: true });
    }
    antes.current = chaveAberta;
  }, [chaveAberta]);
  useEffect(() => { if (aberto !== null && sites !== null && detalhe === null) setAberto(null); }, [aberto, sites, detalhe]);
  useEffect(() => { setAberto(null); }, [voltarSinal]);

  const visiveis = useMemo(() => (sites ? ordenar(filtrar(sites, filtros), ordem) : []), [sites, filtros, ordem]);

  let corpo: React.ReactNode;
  if (erro && sites === null) {
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
    <div ref={telaRef} className="sites-tela" style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <div style={{ display: detalhe ? "none" : undefined }}>
        <h1 ref={tituloRef} tabIndex={-1} style={{ fontSize: 20, fontWeight: 600, color: "var(--text-primary)", margin: "0 0 16px", outline: "none" }}>{t.sites.title}</h1>
        <AvisosFaixa avisos={avisos} onAbrir={(chave) => agir(chave, () => sitesApi.abrir(chave))} />
        {corpo}
      </div>
      {detalhe && (
        <Detalhe
          key={detalhe.chave} site={detalhe} agora={agora} erro={erros[detalhe.chave]} onVoltar={() => setAberto(null)}
          onAbrir={acoes.onAbrir} onLogado={acoes.onLogado} onIa={acoes.onIa} onRemover={acoes.onRemover}
          onSemLogin={acoes.onSemLogin} onNaoLogado={acoes.onNaoLogado}
        />
      )}
      <style>{ESTILOS}</style>
    </div>
  );
};
