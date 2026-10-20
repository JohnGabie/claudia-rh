// CSS of the Sites screen, ported from the approved mockup (option A cards, option C rows, detail page).
// Tokens come from the app (src/styles/tokens.css). Injected once by index.tsx inside `.sites-tela`.
export const ESTILOS = `
.sites-tela :focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.sites-tela .st-linha:focus-visible, .sites-tela .st-switch-faixa:focus-visible { outline-color: var(--text-primary); }
.sites-tela label:focus-within { outline: 2px solid var(--accent); outline-offset: 2px; }

.st-pill { display: inline-block; font-size: 11px; font-weight: 500; background: var(--bg-sunken); color: var(--text-secondary); border-radius: 6px; padding: 2px 8px; white-space: nowrap; max-width: 100%; overflow: hidden; text-overflow: ellipsis; }
.st-estado { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); min-width: 0; }
.st-estado::before { content: ""; flex: 0 0 auto; width: 8px; height: 8px; border-radius: 50%; background: var(--text-tertiary); }
.st-estado.logada::before { background: var(--success); }
.st-estado.precisa::before { background: var(--danger); }
.st-estado.semlogin::before { background: var(--success); }
.st-abrir { font: inherit; font-size: 12px; font-weight: 500; color: var(--text-primary); background: var(--bg-surface); border: 1px solid var(--border); border-radius: 6px; padding: 3px 8px; cursor: pointer; white-space: nowrap; }
.st-abrir:hover { border-color: var(--accent); color: var(--accent-strong); }
.sites-tela a.dominio { color: var(--text-secondary); font-size: 12px; text-decoration: none; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: block; }
.sites-tela a.dominio.inline { display: inline; overflow-wrap: anywhere; white-space: normal; font-size: 13px; }
.sites-tela a.dominio:hover { color: var(--accent-strong); text-decoration: underline; }
.st-nota { font-size: 11px; color: var(--text-tertiary); }
.st-erro { font-size: 12px; color: var(--danger); overflow-wrap: anywhere; }

.st-faixa { width: 100%; min-height: 42px; display: flex; align-items: center; flex-wrap: nowrap; gap: 8px; white-space: nowrap; padding: 8px 12px; font: inherit; font-size: 13px; font-weight: 500; text-align: left; border: 0; border-top: 1px solid var(--border); background: var(--bg-sunken); color: var(--text-primary); cursor: pointer; }
.st-faixa.solta { border: 1px solid var(--border); border-radius: 8px; }
.st-faixa .txt { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.st-faixa .marca { width: 16px; height: 16px; border-radius: 4px; border: 1.5px solid var(--text-tertiary); background: var(--bg-surface); flex: 0 0 auto; box-sizing: border-box; position: relative; }
.st-faixa.ligada { background: var(--accent-soft); }
.st-faixa.ligada .marca { background: var(--accent); border-color: var(--accent); }
.st-faixa.ligada .marca::after { content: ""; position: absolute; left: 4px; top: 0; width: 4px; height: 9px; border: solid #fff; border-width: 0 2px 2px 0; transform: rotate(45deg); }
.st-faixa.falta { font-size: 12px; gap: 6px; color: var(--text-secondary); }
.st-faixa.falta .marca { border-style: dashed; }
.st-faixa.falta .seta { margin-left: auto; flex: 0 0 auto; color: var(--accent-strong); }

.st-grade { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 300px)); gap: 16px; }
.st-card { position: relative; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; overflow: hidden; display: flex; flex-direction: column; min-width: 0; transition: border-color .12s; }
.st-card:hover { border-color: var(--text-tertiary); }
.st-card .capa { height: 52px; flex: 0 0 auto; background: var(--bg-sunken); display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 0 12px; }
.st-card .fundo { position: absolute; top: 0; left: 0; right: 0; height: 52px; overflow: hidden; pointer-events: none; }
.st-card .inicial { position: absolute; right: 70px; top: -22px; font-size: 84px; font-weight: 600; line-height: 1; color: rgba(31, 29, 24, 0.07); user-select: none; }
.st-card .nome { min-width: 0; text-align: left; font: inherit; font-size: 15px; font-weight: 500; color: var(--text-primary); background: none; border: 0; padding: 0; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.st-card .nome::after { content: ""; position: absolute; inset: 0; }
.st-card .nome:focus-visible { outline: none; }
.st-card:has(.nome:focus-visible) { outline: 2px solid var(--text-primary); outline-offset: -2px; }
.st-card .st-faixa:focus-visible { outline-offset: -2px; }
.st-card .st-abrir, .st-card a.dominio, .st-card .st-faixa { position: relative; z-index: 1; }
.st-card .st-abrir { flex: 0 0 auto; color: var(--text-secondary); }
.st-card a.dominio { justify-self: start; max-width: 100%; }
.st-card .corpo { flex: 1 1 auto; padding: 16px 14px 18px; display: grid; align-content: start; gap: 10px; min-width: 0; }
.st-card .linha { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
.st-card .linha .st-estado { flex-basis: 100%; margin-top: 2px; }

.st-lista { display: flex; flex-direction: column; gap: 8px; }
.st-linha { display: grid; grid-template-columns: 4px 28px minmax(0, 1fr) auto auto; align-items: center; gap: 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; padding: 0 10px 0 0; min-height: 52px; cursor: pointer; overflow: hidden; }
.st-linha:hover { border-color: var(--accent); }
.st-linha .risco { align-self: stretch; background: var(--text-tertiary); }
.st-linha .risco.logada { background: var(--success); }
.st-linha .risco.semlogin { background: var(--success); }
.st-linha .risco.precisa { background: var(--danger); }
.st-linha .risco.nenhum { background: var(--border); }
.st-mono { width: 28px; height: 28px; border-radius: 6px; background: var(--bg-sunken); display: grid; place-items: center; font-size: 12px; font-weight: 600; color: var(--text-secondary); }
.st-linha .quem { min-width: 0; padding: 6px 0; }
.st-linha .n { display: block; max-width: 100%; text-align: left; font: inherit; font-size: 14px; font-weight: 500; color: var(--text-primary); background: none; border: 0; padding: 0; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.st-linha .n:hover { color: var(--accent-strong); text-decoration: underline; }
.st-linha .d { display: flex; font-size: 12px; color: var(--text-secondary); white-space: nowrap; min-width: 0; }
.st-linha .d .dom { min-width: 0; flex: 0 1 auto; overflow: hidden; text-overflow: ellipsis; }
.st-linha .d .resto { flex: 0 0 auto; }
.st-linha .ia { font-size: 12px; font-weight: 500; color: var(--text-secondary); white-space: nowrap; min-width: 84px; text-align: right; }
.st-linha.aberta { border-color: var(--accent); background: linear-gradient(0deg, var(--accent-soft), var(--accent-soft)), var(--bg-surface); }
.st-linha.aberta .ia { color: var(--accent-strong); }
.st-linha.bloqueada { cursor: default; }
.st-linha.bloqueada:hover { border-color: var(--border); }
.st-linha.bloqueada .ia { color: var(--text-tertiary); }

.st-pagina { background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; padding: 16px; max-width: 1000px; container-type: inline-size; }
.st-pagina .topo-p { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 12px; margin-bottom: 14px; }
.st-pagina h2 { font-size: 15px; font-weight: 500; margin: 0; min-width: 0; overflow-wrap: anywhere; }
.st-pagina .grade-p { display: grid; grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr); gap: 16px; align-items: stretch; }
.st-painel { border: 1px solid var(--border); border-radius: 8px; padding: 14px; min-width: 0; height: 100%; box-sizing: border-box; }
.st-painel h3 { margin: 0 0 10px; font-size: 15px; font-weight: 500; }
.st-painel dl { margin: 0; display: grid; grid-template-columns: 92px minmax(0, 1fr); gap: 6px 10px; font-size: 13px; }
.st-painel dt { color: var(--text-secondary); }
.st-painel dd { margin: 0; overflow-wrap: anywhere; }
.st-painel dd .sub { color: var(--text-secondary); font-size: 12px; margin-left: 4px; }
.st-painel .vazio { margin: 0; font-size: 13px; color: var(--text-secondary); }
.st-divisoria { display: flex; align-items: center; gap: 10px; margin: 16px 0 10px; font-size: 12px; font-weight: 500; color: var(--text-secondary); }
.st-divisoria::after { content: ""; flex: 1; height: 1px; background: var(--border); }
.st-passos { display: grid; gap: 10px; }
.st-passo { display: grid; grid-template-columns: 28px minmax(0, 1fr); gap: 12px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; padding: 14px; }
.st-passo.atual { border-color: var(--accent); }
.st-passo.travado { opacity: .75; }
.st-passo .num { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; font-size: 13px; font-weight: 600; background: var(--bg-sunken); color: var(--text-secondary); }
.st-passo.atual .num { background: var(--accent); color: #fff; }
.st-passo.feito .num { background: var(--success); color: #fff; }
.st-passo h4 { margin: 0 0 2px; font-size: 15px; font-weight: 500; }
.st-passo p { margin: 0 0 10px; font-size: 13px; color: var(--text-secondary); max-width: 60ch; }
.st-passo .acoes { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.st-passo .meta { font-size: 12px; color: var(--text-secondary); }
.st-passo .st-faixa { border: 1px solid var(--border); border-radius: 8px; max-width: 300px; }
.st-selo { font-size: 12px; font-weight: 500; border-radius: 6px; padding: 3px 8px; background: var(--bg-sunken); color: var(--text-secondary); }
.st-selo.ok { background: color-mix(in srgb, var(--success) 14%, var(--bg-surface)); color: var(--success); }
.st-risco { margin-top: 16px; border: 1px solid color-mix(in srgb, var(--danger) 30%, var(--border)); border-radius: 8px; background: var(--bg-surface); }
.st-risco > summary { list-style: none; cursor: pointer; display: flex; align-items: center; gap: 8px; padding: 12px 16px; font-size: 15px; font-weight: 500; color: var(--danger); border-radius: 8px; }
.st-risco > summary::-webkit-details-marker { display: none; }
.st-risco > summary::before { content: ""; width: 7px; height: 7px; border: solid var(--danger); border-width: 0 2px 2px 0; transform: rotate(-45deg); transition: transform .12s; margin-right: 4px; }
.st-risco[open] > summary::before { transform: rotate(45deg); }
.st-risco[open] > summary { border-bottom: 1px solid color-mix(in srgb, var(--danger) 18%, var(--border)); border-radius: 8px 8px 0 0; }
.st-risco .linha-risco { display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; padding: 14px 16px; }
.st-risco .linha-risco strong { display: block; font-size: 14px; font-weight: 500; }
.st-risco .linha-risco small { display: block; margin-top: 2px; font-size: 12px; color: var(--text-secondary); max-width: 62ch; }
.st-risco .st-erro { padding: 0 16px 14px; }
.st-switch-faixa { cursor: pointer; border-radius: 8px; }
.st-btn { font: inherit; font-size: 13px; font-weight: 500; border-radius: 8px; padding: 7px 12px; cursor: pointer; border: 1px solid var(--border); background: var(--bg-surface); color: var(--text-primary); text-align: left; }
.st-btn.primario { background: var(--accent); border-color: var(--accent); color: #fff; }
.st-btn.perigo { color: var(--danger); }
.st-btn[disabled] { color: var(--text-tertiary); cursor: not-allowed; }
.st-confirma { display: flex; flex-direction: column; gap: 8px; font-size: 13px; color: var(--text-secondary); }
@container (max-width: 700px) { .st-pagina .grade-p { grid-template-columns: 1fr; } }
`;
