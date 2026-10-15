// CSS of the Sites screen, ported from the approved mockup (option A cards, option C rows, detail page).
// Tokens come from the app (src/styles/tokens.css). Injected once by index.tsx inside `.sites-tela`.
export const ESTILOS = `
.sites-tela :focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.sites-tela .st-card:focus-visible, .sites-tela .st-linha:focus-visible, .sites-tela .st-switch-faixa:focus-visible { outline-color: var(--text-primary); }
.sites-tela label:focus-within { outline: 2px solid var(--accent); outline-offset: 2px; }

.st-pill { display: inline-block; font-size: 11px; font-weight: 500; background: var(--bg-sunken); color: var(--text-secondary); border-radius: 6px; padding: 2px 8px; white-space: nowrap; max-width: 100%; overflow: hidden; text-overflow: ellipsis; }
.st-estado { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); min-width: 0; }
.st-estado::before { content: ""; flex: 0 0 auto; width: 8px; height: 8px; border-radius: 50%; background: var(--text-tertiary); }
.st-estado.logada::before { background: var(--success); }
.st-estado.sem::before { background: var(--warning); }
.st-estado.precisa::before { background: var(--text-tertiary); }
.st-estado.semlogin::before { background: var(--success); }
.st-abrir { font: inherit; font-size: 12px; font-weight: 500; color: var(--text-primary); background: var(--bg-surface); border: 1px solid var(--border); border-radius: 6px; padding: 3px 8px; cursor: pointer; white-space: nowrap; }
.st-abrir:hover { border-color: var(--accent); color: var(--accent-strong); }
.sites-tela a.dominio { color: var(--text-secondary); font-size: 12px; text-decoration: none; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: block; }
.sites-tela a.dominio.inline { display: inline; overflow-wrap: anywhere; white-space: normal; font-size: 13px; }
.sites-tela a.dominio:hover { color: var(--accent-strong); text-decoration: underline; }
.st-nota { font-size: 11px; color: var(--text-tertiary); }
.st-erro { font-size: 12px; color: var(--danger); overflow-wrap: anywhere; }

.st-faixa { display: flex; align-items: center; flex-wrap: nowrap; gap: 8px; white-space: nowrap; padding: 8px 12px; font-size: 12px; font-weight: 500; border-top: 1px solid var(--border); background: var(--bg-sunken); color: var(--text-secondary); }
.st-faixa.solta { border: 1px solid var(--border); border-radius: 8px; }
.st-faixa > span:not(.marca) { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.st-faixa .marca { width: 16px; height: 16px; border-radius: 4px; border: 1.5px solid var(--text-tertiary); flex: 0 0 auto; box-sizing: border-box; }
.st-faixa.aberta { background: var(--accent-soft); color: var(--accent-strong); }
.st-faixa.aberta .marca { background: var(--accent); border-color: var(--accent); position: relative; }
.st-faixa.aberta .marca::after { content: ""; position: absolute; left: 4px; top: 0; width: 4px; height: 9px; border: solid #fff; border-width: 0 2px 2px 0; transform: rotate(45deg); }
.st-faixa.bloqueada { color: var(--text-tertiary); }
.st-faixa.bloqueada .marca { border-style: dashed; }
.st-faixa .logou { margin-left: auto; font: inherit; font-size: 12px; font-weight: 500; color: var(--accent-strong); background: none; border: 0; cursor: pointer; padding: 0; text-decoration: underline; }

.st-grade { display: grid; grid-template-columns: repeat(auto-fill, minmax(232px, 1fr)); gap: 16px; }
.st-card { position: relative; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; overflow: hidden; cursor: pointer; display: flex; flex-direction: column; min-width: 0; transition: border-color .12s; }
.st-card:hover { border-color: var(--accent); }
.st-card.aberta { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
.st-card.bloqueada { cursor: default; }
.st-card.bloqueada:hover { border-color: var(--border); }
.st-card .capa { position: relative; height: 84px; background: var(--bg-sunken); display: flex; align-items: flex-end; padding: 10px 12px; overflow: hidden; }
.st-card .inicial { position: absolute; right: -6px; top: -18px; font-size: 84px; font-weight: 600; line-height: 1; color: rgba(31, 29, 24, 0.07); user-select: none; }
.st-card .st-abrir { position: absolute; top: 8px; left: 8px; }
.st-card .nome { position: relative; min-width: 0; max-width: 100%; text-align: left; font: inherit; font-size: 15px; font-weight: 500; line-height: 1.2; color: var(--text-primary); background: none; border: 0; padding: 0; cursor: pointer; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow-wrap: anywhere; }
.st-card .nome:hover { color: var(--accent-strong); text-decoration: underline; }
.st-card .corpo { padding: 10px 12px; display: grid; gap: 6px; min-width: 0; }
.st-card .linha { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
.st-card .st-faixa { margin-top: auto; }

.st-lista { display: flex; flex-direction: column; gap: 8px; }
.st-linha { display: grid; grid-template-columns: 4px 28px minmax(0, 1fr) auto auto; align-items: center; gap: 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; padding: 0 10px 0 0; min-height: 52px; cursor: pointer; overflow: hidden; }
.st-linha:hover { border-color: var(--accent); }
.st-linha .risco { align-self: stretch; background: var(--text-tertiary); }
.st-linha .risco.logada { background: var(--success); }
.st-linha .risco.semlogin { background: var(--success); }
.st-linha .risco.sem { background: var(--warning); }
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

.st-pagina { background: var(--bg-surface); border: 1px solid var(--border); border-radius: 8px; padding: 16px; max-width: 920px; }
.st-pagina .topo-p { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 12px; margin-bottom: 14px; }
.st-pagina h2 { font-size: 15px; font-weight: 500; margin: 0; min-width: 0; overflow-wrap: anywhere; }
.st-pagina .grade-p { display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr); gap: 12px; }
.st-painel { border: 1px solid var(--border); border-radius: 8px; padding: 12px; min-width: 0; }
.st-painel h3 { margin: 0 0 8px; font-size: 15px; font-weight: 500; }
.st-painel dl { margin: 0; display: grid; grid-template-columns: 110px minmax(0, 1fr); gap: 6px 10px; font-size: 13px; }
.st-painel dt { color: var(--text-secondary); }
.st-painel dd { margin: 0; overflow-wrap: anywhere; }
.st-acoes-p { display: flex; flex-direction: column; gap: 8px; }
.st-switch-faixa { cursor: pointer; border-radius: 8px; }
.st-btn { font: inherit; font-size: 13px; font-weight: 500; border-radius: 8px; padding: 7px 12px; cursor: pointer; border: 1px solid var(--border); background: var(--bg-surface); color: var(--text-primary); text-align: left; }
.st-btn.primario { background: var(--accent); border-color: var(--accent); color: #fff; }
.st-btn.perigo { color: var(--danger); }
.st-btn[disabled] { color: var(--text-tertiary); cursor: not-allowed; }
.st-confirma { display: flex; flex-direction: column; gap: 8px; font-size: 13px; color: var(--text-secondary); }
@media (max-width: 760px) { .st-pagina .grade-p { grid-template-columns: 1fr; } }
`;
