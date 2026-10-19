# Estado do projeto

Inventário do que o repositório já é e dívida conhecida. Movido do `CLAUDE.md` em 2026-10-19 (spec `2026-10-19-regras-agentes-design.md`, P9) sem alterar o texto. Atualize aqui quando o estado mudar; o `CLAUDE.md` guarda só regras e fluxo.

## Dívida conhecida

- **Browser, não provado no mundo real:** nada rodou ainda contra o LinkedIn com login e envio reais; os testes são em páginas locais e headless. O login do LinkedIn no browser do app precisa ser refeito pelo usuário (botão "Abrir o browser"). A persistência do login entre duas aberturas e o desligamento do Chrome ao fechar o app pela janela também não foram vistos de ponta a ponta.
- **Testes live instáveis:** CLA-36 (viewport), CLA-38 (rolagem, falha só na suíte inteira) e CLA-39 (Chrome de teste sobe sem página e fica órfão) impedem o merge. Os testes live criam e apagam a pasta do perfil, então rodam só no Sandbox.
- **Sandbox, ferramentas do PC:** `testes` usa as ferramentas do PC só leitura e sobe em minutos (decisão do usuário, 2026-10-08, spec `2026-10-07-sandbox-ambiente-declarativo-design.md` §9); `testes-limpo` instala tudo no guest e leva cerca de 3 h (o gargalo é o Windows SDK). Ainda **não verificado ao vivo**: a subida do `testes` com as montagens só leitura e o `cargo test --lib` com elas.
- **Sandbox, não verificado:** `claude -p` respondendo dentro do guest (a conta estava no limite de sessão na rodada de 2026-10-08) e a renovação do token da conta copiada para o guest. Ver `.claude/skills/claudia-teste-sandbox/references/limites.md`, itens 13 a 17.
- **Smoke de uso real da aba Currículos e do PDF dentro do app real:** pendente (bloqueado até a branch de dados falsos completos); o PDF só foi provado num binário à parte.
- **Skill `tailor-application`:** o prompt de execução manda segui-la, mas ela não existe nesta máquina (nem em `~/.claude/skills`, nem nos plugins, nem no repositório) e não se sabe onde ela grava os arquivos. Manter ou trocar a menção é decisão do usuário (o prompt não se edita sem pedido).
- **Coordenadas:** `browser_crop` mede a partir do topo do documento e `browser_click_at` a partir da tela visível; depois de rolar, divergem.
- Página de erro do Chrome (ex.: 404) aparece como `estado: ok`; a descrição dentro de iframe não aparece em `browser_text`.
- **Guarda de envio, limites conhecidos:** botão feito só de JavaScript fora de `<form>` e envio implícito de campo único não são pegos; uma página hostil pode enviar pelos próprios handlers; a porta de debug local não tem autenticação (CLA-30). A guarda só libera com **uma** vaga em `candidatando`; com duas, recusa todo envio. CLA-35 (vagas antigas presas em `candidatando`) ficou de fora desta branch por decisão do usuário.
- **Janela invisível do PDF** roda com JavaScript (sem acesso ao app): risco aceito por decisão do usuário em 2026-10-08. `PrintToPdf` pode gravar depois do timeout.
- Desinstalação real nunca foi testada (só em Sandbox/VM; nunca nesta máquina); só vale a caixa padrão do instalador, não existe a opção "só o browser" nem backup antes de apagar.
- Licença do binário do Chrome for Testing não estabelecida e sem auto-update (CLA-34, antes de qualquer release público).
- Toast nativo Windows: `notificacoes.rs` emite eventos Tauri mas não chama o plugin de notificação — o toast do sistema não dispara.
- Pré-requisitos Chrome / plano da conta: não validados na UI.
- Pesquisa pontual da aba Feedback: não implementada.
- `submit_claims` é criada em uso, fora das migrations.
- Nomes de parâmetro misturados nas tools do MCP (`id`, `variante_id`, `documento`): renomear em branch curta depois do merge (CLA-55).

## O que este repo já é

v0.2.0. Perfil = `candidate_base.yaml` + `search_variants.yaml` (módulo `src/components/perfil/`). Dashboard em `src/components/dashboard/`. MCP embutido em `src-tauri/src/mcp/`.

**Browser próprio** (`src-tauri/src/browser/`): Chrome for Testing fixado em `browser-pin.json`, baixado pelo app na primeira abertura; perfil persistente (só cookies, sem senhas); tema e ícone laranja. **Só roda quando a IA é chamada** (início de sessão) ou pelo botão "Abrir o browser" nas Configurações; na abertura do app só baixa. Fecha com o app, e o app tem instância única. Garante viewport mínimo de 1200×800 a cada ferramenta.

**Ferramentas do agente** (MCP `claudia`, registradas em `mcp/server.rs`, implementadas em `mcp/tools/`):
- **Browser** (`mcp/tools/browser.rs`): `browser_open`, `browser_read`, `browser_text`, `browser_click`, `browser_type`, `browser_key`, `browser_scroll`, `browser_crop`, `browser_click_at`, `browser_upload`. Todo resultado de leitura termina na linha `estado:` (ok, login_wall, captcha, vazio, carregando). `browser_upload` anexa o documento que o app escolhe (variante gravada da vaga, depois o preferido geral, depois o gerado da variante) e **nunca recebe caminho de arquivo**; só funciona com uma vaga em `candidatando` e nunca envia a candidatura.
- **Documentos e variantes:** `list_documentos`, `get_documento`, `save_cover_letter`, `list_variantes`, `get_variante`, `set_vaga_variante` (nenhuma devolve ou aceita caminho de arquivo). `update_variante` e `set_documento_preferido` são listadas nas duas sessões e **recusadas na de execução**: só a sessão de Perfil escreve.
- **Vagas, perfil e pendências:** `register_vaga`, `update_vaga_status`, `register_candidatura`, `create_pendencia`, `list_pendencias`, `get_pendencia_vaga`, `close_pendencia`, `close_pendencias_vaga`, `get_candidate_profile`, `get_search_variants`, `get_strategy`, `get_memory_summary`, `update_profile`, `propose_profile_change`.
- **Sites** (`mcp/tools/sites.rs`): `browser_contas` (só leitura) lista os sites que o usuário liberou para a IA, com o estado de login de cada um. O `browser_open` consulta a lista antes de abrir: endereço fora dela, site fechado ou sem login confirmado é recusado.

A **guarda de envio** (`browser/guard.rs`, lista em `src-tauri/submit-patterns.txt`) vive no código: um nome de envio ("Enviar candidatura", "Submit application"...) exige `confirm_submit` = id da única vaga em `candidatando`, dentro do orçamento diário, e gasta o envio da vaga no dia (uma vez por vaga por dia). Nomes fracos ("Apply", "Candidatar-se") só contam quando o botão envia um `<form>`. **Todo botão que envia um formulário e cujo nome não é de navegação** ("Próximo", "Next", "Voltar"... as linhas `!` do arquivo) também exige a confirmação, mas **não gasta** o envio. Alvo sem nome, em iframe ou ilegível conta como envio sem gastar. O mesmo vale para Enter e Espaço em `browser_key` e para `browser_click_at` (que olha o elemento sob o ponto).

**Diálogos nativos** (`alert`, `confirm`, `prompt`, `beforeunload`): uma tarefa própria os responde na hora, sem travar a aba (política P2: `alert` OK; `prompt` Cancelar; `confirm` Cancelar, exceto durante um clique que a guarda aprovou; `beforeunload` sai só durante `browser_open`). O resultado da tool traz a linha "Diálogo da página (...)" com o texto da página entre aspas. A queda da aba vira o erro "a aba do browser travou e foi perdida" até o próximo `browser_open`.

**Documentos:** biblioteca em `<dados>/documentos/` (tabelas `documentos`, `preferidos`, `anexos` e `migrados`; `candidaturas.documento_id`), aba **Currículos** na sidebar (sub-abas "Currículos" e "Cover letters", grade com miniaturas A4, página do documento com tags, preferido geral e por variante, renomear, abrir e excluir, importar PDF ou DOCX, gerar), PDF gerado pelo WebView2 do próprio app numa janela invisível, e migração dos currículos e cartas antigos no início do app (idempotente; os originais não são apagados). Código em `src-tauri/src/documentos/` e `src/components/documentos/`.

**Lista de sites** (`src-tauri/src/sites/`, `src-tauri/src/commands/sites.rs`, `src/components/sites/`): catálogo de sites de vagas com a escolha do usuário sobre onde a IA pode abrir páginas (tabelas `contas_browser`, `dominios_recusados`, `avisos_sites`). O seed `src-tauri/sites-padrao.json` entra por merge que nunca sobrescreve `ia_pode_usar` nem `estado`; `sites/gate.rs` é o porteiro do `browser_open` (fecha por padrão); `login_wall` vira `precisa_login` e um aviso em Pendências. Tela **Sites** na sidebar. Detalhes em `docs/arquitetura-sistema-candidaturas.md` (3.8) e na spec `2026-10-14-lista-de-sites-design.md`.

**Activity log**: sempre ligado, banco separado, retenção de 30 dias ou 1 GB, tela própria somente leitura (Configurações → Activity log), sem senhas. Além das ações das tools, grava os eventos do próprio browser (`kind = browser`): download iniciado, instalado, falha de instalação, lançado, falha de lançamento, queda, religado, não religado, desistência de religar, queda de aba e diálogo.

Sessão de execução = `claude --no-chrome` no PTY; a sessão espera o browser abrir.
