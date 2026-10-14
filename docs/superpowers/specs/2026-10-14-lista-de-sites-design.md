# Spec: lista de sites (catálogo de sites de vagas e contas do browser)

> Spec de design. Não é o plano de implementação. Substitui a spec de contas do browser (`2026-10-02-contas-do-browser-design.md`, ciclo 2), **removida do repositório em 2026-10-14** a pedido do usuário; o texto original continua no histórico do git.

> **Status: RASCUNHO PARA REVISÃO. Não aprovada.** Nenhum código foi escrito. Depois da aprovação do usuário: plano em `docs/superpowers/plans/`, depois implementação.

**Data:** 2026-10-14
**Branch:** `feat/lista-de-sites` (a partir de `dev` em `98da02b`)
**Issue:** a criar no Linear (time CLA), ligada à CLA-31

## 1. Pedido do usuário

Nas palavras dele, em 2026-10-14, sem reinterpretar:

- "vai ter uma lista de sites default no app, uns 100 (quanto conseguirmos pegar, mas somente aqueles confiaveis) sites internacionais e nacionais brasileiros. alem de um botao + para por um site."
- "Eu quero dois tipos de view, galeria e lista, alem de ordenacao e busca no tipop"
- Sobre o propósito, escolheu "A,D, E sites que o browser vai estar logado e perfil em dia/configurado".
- "eu queria fazer uma feature ou um botao que tipo eu loguei manutalmente, mas seria uma etapa 2 no site, voltava no app e clicava em um botao para configurar a conta, ai era preencher os dados expereinecia, aquela parte mais chata."
- Sobre escrever o perfil: "é baseado sempre no perfil do usuario, Ele preenche e dps o usuario avança."
- Sobre a rigidez do MCP: "Agente vai deixar rigido o mcp, agora a IA so vai acessar o site por um comando do MCP ou MCP vai devolver esses sites, pq apatir de agora, ela nao acessa mais sites estranhos que nao estejam na lista e nao estao configurados. Isso abre marge para podermos criar uma lista de applicacao por exemplo de 10 aplicacao 50% linkedin, 30%externos 20%stie de empresas. (mas isso fica para o final, a parte de ajustar a setlist, nao tenho um nome melhor para isso.)"
- Sobre a validade do login: "Nao sei sinceramente, gostaria que sempre ficasse online. Mas eu como usuario, so saberia quando a IA abrisse um desses sites e marcasse como X e tivesse que relogar ou algo do tipo"
- Sobre site fora da lista: "se eu deixar muito restritro pode ser que chege mais perto do zero, o que talvez pode ficar, é que se o site aparecer mt com vagas analisadas que fazem sentido, ele pode entrar na lista, pq faz sentido para aquele perfil. Nao falei isso na fase anteroior, mas podemos criar um Rankeamento com pesos, mas precisamos pensar e testar como vai ser esses pesos."
- Sobre o alcance da rigidez: "1 + salva a vaga, e dps sai do site e algo do mcp ou programa diz que ela nao pode acessar."
- Escolheu, entre as opções oferecidas: abordagem **A** para guardar o catálogo (seed versionado), escopo **C** (só catálogo agora; "Configurar conta" fica para depois), e aviso de relogar como **pendência + badge na tela Sites**.
- Autorizou explicitamente editar o prompt de runtime (`src-tauri/src/prompt_sistema_runtime.md`) para o agente saber da tool `browser_contas`.

## 2. Leitura do pedido

1. **Catálogo:** uma lista padrão de cerca de 100 sites de vagas (internacionais e brasileiros), embutida no app, mais o botão **+** para o usuário adicionar o seu.
2. **Telas:** a lista tem duas visualizações (galeria e lista), busca e ordenação, e filtro por tipo.
3. **Propósito:** sites onde o browser da Claudia está logado. O "perfil em dia" no site é a etapa seguinte ("Configurar conta"), **fora desta branch**.
4. **Rigidez:** a IA só acessa site da lista e configurado; o resto é recusado pelo MCP. Isso **contradiz** a spec de contas §6 ("domínios sem conta não são bloqueados"), que esta spec substitui.
5. **Login:** não expira por prazo. Fica `logada` até a IA ver uma parede de login; aí vira `precisa_login` e o usuário é avisado. Isso **contradiz** a spec de contas §4 ("o verde nunca aparece sem checagem recente"), que esta spec substitui por decisão do usuário.
6. **Site fora da lista:** a IA não acessa, **salva a vaga**, sai do site e segue. Os domínios recusados são contados, para o usuário estudar o custo da rigidez e, no futuro, promover sites por frequência.
7. **Fora do pedido, só anotado:** ranking com pesos, promoção automática e a "lista de aplicação" 50/30/20 ficam para uma fase posterior.

## 3. Propostas do assistente

Separadas do pedido. Cada uma indica o estado da decisão.

| # | Proposta | Estado |
|---|---|---|
| P1 | Item **"Sites"** na sidebar (ícone `Globe`), depois de Currículos; exige atualizar o design doc (§3 e nova §15) | Apresentada na Seção 2; o usuário seguiu ("segue"). **Confirmar na revisão.** |
| P2 | Filtro por **País** (BR, DK…, `GLOBAL`), além de busca por texto e filtro por tipo | Aceita pelo "sim" do usuário à taxonomia. **Confirmar na revisão.** |
| P3 | `precisa_login` em **cinza**, e não vermelho (com ~100 sites, vermelho deixaria de significar "atenção") | Apresentada na Seção 2. **Confirmar na revisão.** |
| P4 | Avatar de **letra inicial**, sem buscar favicon pela rede | Aprovada (abordagem A "com esses dois detalhes"). |
| P5 | Regra de login **`nenhuma`** ("não exige login") para sites públicos | Aprovada com a Seção 3. |
| P6 | Cartão mostra **"confirmada há X dias"** | Aprovada com a Seção 3. |
| P7 | Tabela de **domínios recusados** com contagem, só registro | Aprovada explicitamente. |
| P8 | Contador "N de 100 sites" e botão "Limpar filtros" | Apresentadas na Seção 2. **Confirmar na revisão.** |
| P9 | Sétimo tipo **`nicho`** (estágio, PCD, mulheres em tecnologia) | Decidida pelo usuário em 2026-10-14, depois da pesquisa da fase 1. |
| P10 | Campo **`papel`**: agregadores que só redirecionam entram no seed como `descoberta`, com um pill no cartão | Decidida pelo usuário em 2026-10-14, depois da pesquisa da fase 1. |
| P11 | Campo **`regiao`** (`SA`, `NA`, `EU`, `AS`, `AF`, `OC`, `GLOBAL`) e filtro Região, com `pais` opcional | Decidida pelo usuário em 2026-10-14 ("separar por cada região"). |
| P12 | Para `tipo = empresa`, modelar **domínios de carreiras, domínios do ATS e caminho do ATS** por empresa, em vez de uma lista única `dominios`. Motivo: Workday usa subdomínio por empresa e servidor; Greenhouse, Lever e Ashby usam caminho por empresa; há aliases (`careers.google.com`) | **Proposta que veio da pesquisa de empresas. Não decidida.** Muda o casamento de domínio de §4.5. |

## 4. Design

### 4.1 Modelo de dados

Tabela `contas_browser` (banco principal, em `src-tauri/src/db/schema.sql`, com migration):

| Campo | Significado |
|---|---|
| `chave` | identificador estável (`linkedin`, `jobindex`…; no site do usuário, gerado do domínio) |
| `nome`, `url_login` | rótulo e para onde "Abrir" leva (`https`) |
| `dominios` | domínios que pertencem ao site (usados para resolver URL → site) |
| `tipo` | `geral`, `regional`, `tecnologia`, `remoto`, `freelance`, `empresa`, `nicho` (um por site; `nicho` cobre estágio, PCD, mulheres em tecnologia e afins, decisão de 2026-10-14) |
| `papel` | `candidatura` (padrão: dá para candidatar de ponta a ponta) ou `descoberta` (agregador que só redireciona; a IA acha e salva a vaga, e candidatar no destino depende de o destino estar na lista; decisão de 2026-10-14) |
| `regiao` | `SA`, `NA`, `EU`, `AS`, `AF`, `OC` ou `GLOBAL` (decisão de 2026-10-14; filtro próprio na tela) |
| `pais` | código ISO do país, ou vazio quando o site cobre um bloco (Nórdicos, LATAM, UE) |
| `regra_login` | `manual` (padrão), `cookie_sessao` (só os testados) ou `nenhuma` |
| `ia_pode_usar` | permissão explícita; desligada por padrão; só pode ser ligada com o site `logada` ou com regra `nenhuma` |
| `embutida` | veio do seed ou foi adicionada pelo usuário |
| `estado` | `logada`, `precisa_login`, `desconhecido` |
| `verificada_em` | instante da última confirmação |

Tabela `dominios_recusados`: `dominio`, `contagem`, `ultima_vez`, e a ligação com as vagas salvas por causa dele. Só registro; nenhuma promoção automática.

### 4.2 Seed

`src-tauri/sites-padrao.json`, empacotado no instalador, mesclado por `chave` na abertura do app. Idempotente. Para site existente atualiza só `nome`, `url_login`, `dominios`, `tipo` e `pais`; **nunca** toca em `ia_pode_usar`, `estado` ou `regra_login`. Site que sai do seed numa versão nova não é apagado. Um seed inválido (`chave` repetida, `tipo` fora dos 7, `papel` desconhecido, `regiao` fora do conjunto, `url_login` sem `https`, `dominios` vazio) faz o **teste** falhar, não o app.

A lista é montada a partir do conhecimento do assistente, **sem visitar os sites**, e fica **não verificada** até o usuário revisar.

### 4.3 Telas

Item "Sites" na sidebar. Barra de ferramentas: busca por texto (nome e domínio), filtros Tipo, Região e País, ordenação (Nome A–Z padrão, Tipo, Região, País, Estado de login), alternador galeria | lista (preferência salva), botão "Adicionar site" (cheio em `--accent`) e contador.

- **Galeria:** `repeat(auto-fill, minmax(190px, 1fr))`, 16px, cartão com borda de 1px e raio de 8px; avatar de letra inicial, nome, domínio, pills de tipo e país (mais um pill "descoberta" quando `papel = descoberta`), badge de estado, "confirmada há X dias", interruptor "IA pode usar" e botão "Abrir".
- **Lista:** linha de 48px; cabeçalho das colunas ordena ao clicar e inverte no segundo clique.
- **Estado:** `logada` verde (`--success`), `desconhecido` âmbar (`--warning`), `precisa_login` cinza (`--text-tertiary`).
- **Adicionar site (+):** painel acima da grade, com nome, URL (`https`) e tipo (país opcional). Nasce `manual`, `embutida = falso`, `ia_pode_usar` desligado.
- **Ações:** "Abrir" (reaproveita `browser_abrir`), "Já estou logado" nos sites `manual`, e remover só nos adicionados pelo usuário, com confirmação.
- **Estados da tela:** carregando, erro com "Tentar de novo", e "Nenhum site combina com esses filtros." com "Limpar filtros". Textos em pt-BR e en.

### 4.4 Comportamento do login

- "Já estou logado" grava `logada` e `verificada_em`. O estado **não expira por prazo**.
- Quando uma leitura do browser devolve `estado: login_wall` num site da lista, o site vira `precisa_login`, a IA pula esse site e o app cria uma **pendência** "relogar em X" (badge vermelho na sidebar, notificação). O cartão fica cinza.
- Só o LinkedIn ganha detecção por cookie, e **só depois de testar** o nome do cookie (`li_at` **não verificado**). Só o **nome** do cookie é lido; o valor nunca é guardado nem registrado.

### 4.5 Rigidez no MCP

- `browser_open(url)` resolve o domínio para um site da lista. Passa só se o site existir, tiver `ia_pode_usar` ligado e estiver `logada` (ou com regra `nenhuma`). Caso contrário, **recusa** com texto claro.
- Domínio **fora da lista**: recusa, **salva a vaga** com o motivo e o domínio, incrementa `dominios_recusados` e a IA segue para a próxima.
- Nova tool **`browser_contas`**: devolve os sites liberados e o estado de cada um.
- **Limite conhecido:** navegação causada por clique ou redirecionamento depois de um `browser_open` aprovado **não é barrada**. Toda navegação para fora da lista vai para o Activity log.
- A recusa fica no Activity log (sempre ligado) e entra no resumo da rodada com o motivo.

### 4.6 Segurança

- Casamento de domínio por **fronteira de rótulo**: `linkedin.com` e `www.linkedin.com` passam; `evil-linkedin.com`, `linkedin.com.evil.com` e `https://linkedin.com@evil.com` não.
- Só `https`. A recusa de URL inválida se testa na **função de validação pura**.
- Mudança de ferramenta do agente: **revisão por Opus obrigatória**.
- **Prompt de runtime:** o usuário autorizou editar `src-tauri/src/prompt_sistema_runtime.md` para mencionar `browser_contas` e a regra de só acessar a lista. O **texto exato** da edição será descrito no plano e revisado pelo usuário antes de aplicar. Nada é editado antes disso.

## 5. Impacto

| Área | Mudança |
|---|---|
| Banco | `contas_browser` e `dominios_recusados`, nas migrations |
| Seed | `src-tauri/sites-padrao.json` |
| Backend | módulo de contas e comandos Tauri |
| Ferramentas do agente | `browser_open` recusa (mudança de comportamento); nova `browser_contas` |
| Pendências | pendência "relogar em X" (categoria nova, ou existente: ver §7) |
| Front | `src/components/sites/`, item na sidebar, i18n pt/en |
| Docs | design doc §3 e nova §15; arquitetura; spec de contas marcada como substituída |
| Prompt de runtime | edição autorizada, descrita no plano |

## 6. Testes

- **Unitários** (`cargo test --lib`, `npx tsc --noEmit`): validação do seed; mesclagem idempotente que preserva `ia_pode_usar` e `estado`; resolução de domínio, incluindo os casos de ataque de §4.6; transições de estado (`login_wall` → `precisa_login`); `browser_open` recusa fora da lista, sem permissão e sem login; `ia_pode_usar` não liga com o site `precisa_login`.
- **Mutação:** cada regra nova com prova de que o teste nomeado falha quando o código é quebrado.
- **Persistência:** contas, estados e permissões sobrevivem a reiniciar o app.
- **Smoke de uso real** (`/claudia-teste-e2e`): abrir a tela Sites, alternar galeria e lista, buscar, filtrar, ordenar, adicionar e remover um site, nos dois idiomas, com prints; recusa e salvamento da vaga contra **páginas locais**. Sem site real.
- **Onde roda:** `cargo test` só no Sandbox. No PC: `cargo check`, `clippy`, `tsc`, build. Nenhum teste usa caminho real.

## 7. Fora do escopo

- "Configurar conta" e o preenchimento do perfil no site (branch seguinte).
- Ranking com pesos, promoção automática de site e a "lista de aplicação" 50/30/20.
- Detecção de login por cookie nos demais sites.
- Favicons, favoritos, ocultar site e filtro por estado de login.
- Barrar navegação causada por clique ou redirecionamento.
- Guardar senha ou fazer login automático (descartado por segurança).

## 8. Em aberto / não verificado

**Estado da pesquisa do seed (2026-10-14, fechada por ora):** `docs/pesquisa/2026-10-14-lista-de-sites-pesquisa.md`. Decisões do usuário já aplicadas: empresas só entram se publicam as vagas no próprio site; no freelance, só plataformas de fora da América do Sul. **Fora do seed por falta de confirmação:** VTEX e Hugging Face (o site só linka direto ao ATS) e Creditas (sem página própria vista), até o usuário decidir. A lista tem cerca de 190 entradas e **precisa de curadoria até ~100** antes de virar `sites-padrao.json`.

- **Não verificado:** a lista do seed (sites e `url_login` não visitados); o nome do cookie de sessão do LinkedIn (`li_at`).
- **Em aberto:** qual `status` recebe a vaga salva por site fora da lista; se a pendência de relogar exige categoria nova (arquitetura §10 e §6.1); como a vaga se liga ao domínio recusado.
- **Risco aceito pelo usuário:** o verde pode estar velho, porque o login não expira por prazo. Mitigado só pelo "confirmada há X dias".
- **Risco para acompanhar:** a rigidez pode reduzir a taxa de candidaturas. A contagem de `dominios_recusados` é o dado para medir isso.
