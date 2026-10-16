# Spec: fluxo guiado da tela Sites (card, página do site e passos)

Data: 2026-10-15. Estado: **rascunho, aguardando aprovação do usuário**. Nada desta spec foi implementado.

Complementa `2026-10-14-lista-de-sites-design.md` (catálogo, banco, bloqueio no MCP). Onde as duas divergirem, **esta vence só nos trechos de tela e de comportamento do cartão** (§4.3 e a parte "Já estou logado" de §4.4 da spec anterior). Dados, segurança e MCP não mudam.

O desenho visual foi aprovado pelo usuário num mock em HTML, guardado em `docs/superpowers/specs/assets/2026-10-15-sites-fluxo-guiado-mock.html` (e `.css`). O mock é referência visual (medidas, cores, estrutura); o que vale é o texto desta spec. Os números do painel "Uso pela IA" no mock são inventados.

## 1. Pedido do usuário

Nas palavras dele, em ordem:

- "o problema não é o comportamento da IA, mas da interface."
- "clica abre sobre do site e vai ser algo guiado, como primeiro passo eu logando e criando, volto e coloco que loguei, (ai se vai ter verificação se realmente foi ai é outra história), dps de volto clico na segunda etapa para o agente usar a aba e configurar o perfil. [...] ai nisso se eu voltar eu consigo selecionar para ia. E eu acho que só pode selecionar para ia usar na parte de baixo cinza onde está escrito closed. click allow, no resto abre para dar mexer nas config. O botão de login do card some e vai para interno, junto dessas etapas."
- "Basta o passo 1" (para liberar a IA) e "difícil um site não pedir login. add uma config escondida acessar sem login."
- "esse fluxo mesmo mas usa os cards no design antigo, pode manter retangulares ao invés de ser quadrados" e "basicamente trás de volta o about site", com os passos dentro de "O que a IA pode fazer aqui".
- "move Este site não pede login para dentro do entrar no site como um botão comum igual não estou logado, porque faz sentido."
- "coloca mais dados com uma divisórias ali no sobre, como quantas vezes eu fiz relogin, quantas as vezes a IA abriu esses site, quantas vezes ela aplicou vagas nesse site. Se achar que tem que colocar outra coisa pode colocar"
- "muda as tags que são confusas, de geral para um nome mais específico para sites de aplicar vagas. e empresas também coloca site de empresas ou algo mais específico."
- Ajustes de aparência: cards na largura de ~300 px, "não pede login" com ícone verde, estado de login sempre em linha própria.

## 2. Leitura do pedido

1. **Card (galeria).** Retangular, com capa (nome, letra grande de marca d'água, botão "Abrir ↗"), domínio como link, selos de país (nome por extenso) e tipo, estado de login em linha própria e a faixa cinza de baixo. Clicar no corpo do card **abre a página do site**. Só a faixa de baixo liga ou desliga a IA. Não há botão de login no card.
2. **Faixa de baixo, três situações.** Site pronto e fechado: "Fechada · clique para liberar". Site pronto e liberado: "Liberada para a IA". Site **não pronto** (precisa de login): "Faça o passo 1 para liberar · Entrar →", e clicar leva à página, não liga nada.
3. **Pronto** = `regra_login = nenhuma` **ou** `estado = logada`.
4. **Página do site.** Topo com "← Sites", nome, selo local/tipo e estado. Duas colunas de **mesma altura**: à esquerda "Sobre o site"; à direita "O que a IA pode fazer aqui", com três passos em cartões:
   - **1 Entrar no site:** "Abrir o site ↗"; o botão "Já entrei" só aparece depois de abrir o site; botão comum "Este site não pede login". Depois de feito: selo "Feito", "Abrir o site ↗" e "Não estou mais logado".
   - **2 Configurar o perfil no site:** desabilitado, selo "Em breve", não bloqueia nada.
   - **3 Liberar para a IA:** bloqueado até o passo 1 ficar pronto; depois mostra a mesma faixa do card.
5. **Site que não pede login:** o passo 1 aparece como "Feito" ("Este site não pede login.") com o botão "Este site pede login" para desfazer.
6. **Zona de risco** recolhida por padrão, com "Remover da lista", habilitado só para sites adicionados pelo usuário.
7. **Sobre o site**, com divisórias: **Cadastro** (endereço, tipo, região, país, papel, origem, domínios), **Conta** e **Uso pela IA** (§4.3).
8. **Nomes de tipo** mais específicos (§4.4).

"Já entrei" continua sendo a palavra do usuário. A verificação real é "outra história" (fora do escopo, §7).

## 3. Propostas do assistente (NÃO DECIDIDAS)

| # | Proposta | Recomendação |
|---|---|---|
| P1 | Itens extras em "Sobre o site" que o usuário não listou: **Confirmado em** (data do "Já entrei"), **Vagas analisadas** e **Taxa** (candidaturas ÷ vagas analisadas). | Incluir "Confirmado em" e "Vagas analisadas"; a "Taxa" só se ambos os números forem confiáveis (ver §8). |
| P2 | **Aberturas** guardadas em coluna própria de `contas_browser`, somada pelo bloqueio do `browser_open`, em vez de lidas do Activity log. | Coluna própria: o Activity log apaga com 30 dias ou 1 GB, e a contagem do site deve ser a vida inteira. |
| P3 | Nomes dos 7 tipos: Portal de vagas, Portal regional, Vagas de tecnologia, Vagas remotas, Plataforma freelance, Site da empresa, Portal especializado. Só muda o **rótulo** (pt e en); a chave no banco não muda. | Aceitar. |
| P4 | Visão em **lista**: a linha abre a página; só o texto de status (à direita) liga a IA. | Aceitar. Ainda não foi desenhada no mock. |
| P5 | O botão "Este site não pede login" **não aparece** em sites com `regra_login = cookie_sessao` (hoje só o LinkedIn), porque desfazer devolveria `manual` e perderia a detecção por cookie. | Aceitar. |

## 4. Design

### 4.1 Comportamento do card e da faixa

- Corpo do card = um botão que abre a página. "Abrir ↗" e o link do domínio abrem o site no browser do app e **não** abrem a página (param a propagação).
- A faixa é o **único** controle de IA da galeria. É um `role="switch"` quando o site está pronto, e um botão que leva à página quando não está.
- A faixa nunca recusa em silêncio: sem o passo 1, ela leva ao passo 1.

### 4.2 Página do site e passos

Conforme §2.4 a §2.6. Regras:

- "Já entrei" só é oferecido depois de "Abrir o site ↗" nesta visita à página. Grava `estado = logada` e `verificada_em` (comando que já existe).
- "Não estou mais logado" grava `precisa_login` **e fecha a IA** do site.
- "Este site não pede login" grava `regra_login = nenhuma`. "Este site pede login" grava `regra_login = manual`, **marca o site como `precisa_login`, fecha a IA (`ia_pode_usar = 0`) e cria o aviso "relogar em X"** (aparece nas pendências e no selo da sidebar), tudo na mesma transação. **Mudança de 2026-10-15, pedida pelo usuário** ("muda algo em vermelho Exige login, e vai para as notificações"): antes a spec mantinha a IA aberta se o site estava `logada`; o login confirmado antes de "não pede login" não vale depois dele.
- **Aparência do estado `precisa_login`:** bolinha **vermelha** (`--danger`) e o texto **"Exige login"** (en: "Login required"), no cartão, na página, na lista e na lateral da linha. `desconhecido` continua âmbar "Sem confirmação".
- O passo 3 só liga a IA se o site está pronto. O backend continua recusando (falha fechada) mesmo que a tela erre.

### 4.3 "Sobre o site": Conta e Uso pela IA

**Conta:** "Confirmado em" (data de `verificada_em`; "Ainda não entrou" se não houver) e **Relogins** (quantidade de avisos "relogar em X" já criados para o site, resolvidos ou não; tabela `avisos_sites`). Site que não pede login mostra só "Login: Não pede".

**Uso pela IA:** **Aberturas** (contagem de `browser_open` aceitos no site, com a data da última), **Vagas analisadas**, **Candidaturas** (com a data da última) e **Taxa**. Sem nenhuma abertura: "A IA ainda não abriu este site."

O mock mostrava também "Recusadas". **Foi retirado**: o bloqueio guarda recusas por **domínio fora da lista** (`dominios_recusados`), então não existe "recusadas por site".

### 4.4 Rótulos dos tipos

Conforme P3. Rótulos em `src/i18n/pt.ts` e `en.ts`; filtro, selos, ordenação e formulário de adicionar usam os mesmos.

## 5. Impacto

- **Dados:** coluna nova `aberturas INTEGER NOT NULL DEFAULT 0` e `ultima_abertura TEXT` em `contas_browser`, com migração idempotente (P2). Nenhuma outra tabela muda.
- **Backend:** (a) comandos novos `definir_site_sem_login(chave, valor)` e `marcar_site_nao_logado(chave)` (este grava `precisa_login` e fecha a IA, conforme §4.2; antes só existia a função interna) com as regras de §4.2; (b) comando de leitura das estatísticas de um site (relogins, aberturas, vagas, candidaturas) ou campos novos em `listar_sites`; (c) o bloqueio do `browser_open` soma a abertura aceita.
- **Frontend:** `Cartao` e `Faixa` (faixa única, corpo abre a página), `Detalhe` (duas colunas, passos, zona de risco recolhível), componente novo de passos, `Lista` (P4), rótulos de tipo, textos pt/en, `sites.mock.ts` e `dev-mock.ts`.
- **Segurança:** nenhuma regra do MCP muda. A regra "voltar para `manual` sem login fecha a IA" é de segurança e tem teste próprio. O que sai da tela (contagens) não traz URL nem cookie.
- **Docs:** `padrao-design-claudia-rh.md` §15 (cartão e página) e `arquitetura-sistema-candidaturas.md` §3.8 (comando novo e coluna).
- **Não toca** `prompt_sistema_runtime.md`.

## 6. Testes

- **Back (Sandbox):** `definir_site_sem_login` liga e desliga; voltar a "pede login" sempre marca `precisa_login`, fecha a IA e cria o aviso (inclusive com `logada`; sem duplicar o aviso se já houver um aberto); recusa chave inexistente; recusa `cookie_sessao` (P5); migração idempotente; o bloqueio soma só aberturas aceitas (recusa e site `precisa_login` não somam). Prova de mutação em cada regra.
- **Front (node `--test`):** lógica pura de "pronto", da faixa (3 situações), do passo atual e da ordem dos passos; rótulos dos tipos nos dois idiomas.
- **Revisor de front (app aberto):** galeria e lista; página com site logado, sem login, não logado e recém-adicionado; estados vazio, erro e com dados; pt e en; interruptor e zona de risco recolhível; teclado (Enter na faixa, foco visível); largura estreita; prints.
- **Smoke de uso real:** `/claudia-teste-e2e`, sem site real e sem enviar candidatura.

## 7. Fora do escopo

- **Verificação real do login** (o app confirmar sozinho que a sessão vale). "Já entrei" continua sendo a palavra do usuário.
- **Passo 2 funcional** (configurar o perfil no site): fica visível e desabilitado.
- Curadoria dos ~100 sites do seed e a decisão sobre termos de uso (tarefa à parte, do usuário).
- Edição do prompt de runtime.
- Merge, push e PR (decisão do usuário).

## 8. Em aberto / não verificado

- **Não verificado:** se **Vagas analisadas** e **Candidaturas** por site podem ser derivadas de `vagas.url` pelo casamento de domínio. `vagas` não guarda a chave do site, só `plataforma` (texto) e `url`; vagas em ATS compartilhado ou com link redirecionado podem cair no site errado. Se não for confiável, a spec propõe **ocultar** esses números e a Taxa, não mostrar valor aproximado.
- **Não verificado:** o que `plataforma` costuma conter (nome livre escrito pela IA).
- **Em aberto:** "Relogins" conta avisos criados; um site que caiu duas vezes sem o aviso ser resolvido entre elas conta uma ou duas. Hoje assumo uma por aviso criado.
- **Em aberto:** em sites `cookie_sessao`, mostrar ou não o botão de "pede login" (P5 propõe ocultar).
- **Em aberto:** a visão em lista (P4) ainda não foi desenhada; o usuário aprova o desenho antes da implementação da lista.

## 9. Emenda de 2026-10-15 (pedido do usuário, por cima de §2.7, P4 e §4.2)

Pedido, nas palavras dele: "é muito fácil, volta isso, e somente faz o que eu disser. Onde está escrito logado, ou não precisa de login, muda para algo em vermelho escrito algo do tipo, Precisa de login, já fica impossibilitado de usar; caso o agent acesse o site usando o mcp, o mcp devolve dizendo que precisa de login, e pula e continua. Simples."

- **Lista (volta ao comportamento antigo):** a visão em lista volta a ser como era antes da Tarefa 7: **a linha inteira liga e desliga a IA** (o nome abre a página, "Abrir ↗" abre o site). **Substitui P4** ("a linha abre a página; só o status liga a IA"). A galeria **não muda**.
- **Rótulo de login simplificado:** todo site que exige login e **não está logado** (`precisa_login` **ou** `desconhecido`) mostra **vermelho "Precisa de login"** (en "Login required") e fica **bloqueado para a IA**. Some o âmbar "Sem confirmação". Verde continua só para `logada` e para "Não pede login". **Substitui o texto "Exige login" e a regra do âmbar de §4.2.**
- **MCP:** nada muda no backend. O bloqueio já recusa o site que não está logado, com a mensagem "RECUSADO: o site ... não está com login confirmado. Pule esta fonte; o usuário precisa entrar nele de novo.", e o agente segue para a próxima fonte.
