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
7. **Sobre o site**, com divisórias: **Cadastro** (endereço, tipo, região, país, papel, origem, domínios), **Conta** e **Uso pela IA** (§4.3) (ver §10: o divisor titulado "Cadastro" não foi desenhado).
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
(Atualizado na T15, §10: o texto agora é "precisa de login e o usuário ainda não entrou nele. Pule esta fonte e siga para a próxima." e o login é conferido antes da liberação.)

## 10. Estado da implementação (2026-10-15)

Branch `feat/lista-de-sites-impl`, T1 a T7, T9 e T14 a T16 passaram pelo ciclo executor e revisor ("Looks good to me" escrito); a T8, esta documentação, é revisada à parte. A revisão final da branch inteira (Opus, 2026-10-16) deu "sim, com correções", aplicadas na T17. O registro por tarefa está em `.superpowers/sdd/2026-10-15-sites-fluxo-guiado/progress.md`.

**O que foi construído**

- **T1, aberturas:** colunas `contas_browser.aberturas` e `ultima_abertura` (migração idempotente); o porteiro soma uma abertura a cada `browser_open` que **aceita**, nunca nas recusas.
- **T2, estatísticas:** comando `estatisticas_site` (relogins, aberturas, última abertura, vagas, candidaturas, última candidatura). Vagas e candidaturas por site saem do casamento de domínio da URL da vaga e **não são confiáveis**; ver "não verificado" abaixo.
- **T3 e T3b, regra de login:** `definir_site_sem_login` e `marcar_site_nao_logado`. "Não pede login" vira `regra_login = nenhuma` e resolve os avisos (recusado em `cookie_sessao`). Voltar a "pede login" marca `precisa_login`, fecha a IA e cria **um** aviso, numa transação, também quando o site estava `logada` (decisão do usuário, §4.2 emendado; a T3b substituiu o comportamento da T3, em que `logada` mantinha a IA aberta).
- **T4 e T4b, lógica e rótulos:** lógica pura de "pronto", da faixa e dos passos; rótulos dos tipos (Portal de vagas, Portal regional, Vagas de tecnologia, Vagas remotas, Plataforma freelance, Site da empresa, Portal especializado).
- **T5, cartão:** cartão retangular de 300px, capa de 52px, faixa de 42px com três situações; só a faixa liga a IA.
- **T6, página do site:** "Sobre o site" (Cadastro, Conta, Uso pela IA), os três passos, zona de risco recolhível, linha de alerta das recusas, grade em consulta de contêiner (duas colunas acima de 700px). Vagas, candidaturas e taxa por site ficaram **ocultas** (§8). O texto "Não pede login" (antes "Sem login") também é desta tarefa (commit `dca40ca`).
- **T7, lista nova:** construída e aprovada no mock, depois **revertida a pedido do usuário** (§9); `Lista.tsx` e `Interruptor.tsx` voltaram byte a byte à versão da T4b.
- **T9, reversão e vermelho:** a lista volta ao comportamento antigo (a linha inteira liga a IA) e todo site que exige login e não está logado mostra vermelho "Precisa de login", bloqueado para a IA. Some o âmbar "Sem confirmação".
- **T14, seed de 163 sites:** o seed passou de 9 para 163 sites (9 + 154, `src-tauri/sites-padrao.json`). Ficaram **fora** os sites que proíbem automação, e também a VTEX e o Hugging Face (pergunta em aberto: entram ou não).
- **T14b, login manual:** seis empresas do seed pedem login manual: redhat, suse, booking, orsted, accenture e wise.
- **T15:** o bloqueio do MCP confere o login antes da liberação (a IA recebe "precisa de login"), e a mensagem de liberar a IA cita o passo 1 da página.
- **T16:** a página do site abre no topo, com o foco no título.
- **T17, correções da revisão final:** a tela Sites também recarrega em `novo-aviso-site` e `aviso-site-resolvido` (antes só em `db-atualizada` e `sites-mudaram`); `browser_contas` só lista o site que o porteiro deixaria abrir (`regra_login = nenhuma` ou `logada`, com a IA ligada); textos de docs corrigidos.

**Decisões do usuário**

- Site que **proíbe automação** fica **fora** do seed (Tarefa 14).
- **"Precisa de login" para todo site não logado** (`precisa_login` e nunca confirmado), em vermelho; o porteiro já recusa os dois com "Pule esta fonte" e o agente segue.
- **A lista volta ao comportamento antigo** (§9). Com isso, a dúvida da §4.2 sobre um `logada` antigo que sobrevive ao ciclo "não pede login" e volta é **moot**: voltar a "pede login" sempre fica vermelho e fecha a IA.

**Smoke e testes**

- **Smoke de uso real no Sandbox (2026-10-15):** feito no app Tauri de verdade, com o relatório em `.superpowers/sdd/2026-10-15-sites-fluxo-guiado/smoke-report.md`: 27 PASS, 9 achados e 6 itens não verificados. Tratados na T15 e na T16: F3 (texto antigo "Já estou logado" na recusa), F6 (ordem e mensagem do porteiro do MCP) e F4 (a página do site abria rolada). **Não tratados** (rótulos do próprio relatório): F1, LEGO aparece em vermelho (pergunta em aberto ao usuário); F5, "Ordenar por: Estado de login" não agrupa nada enquanto os 163 sites estão não confirmados (pergunta em aberto); F7, a dica de "browser_open repetido" conta URLs diferentes (já existia antes); F8, "Confirmado em" mostra data num site que nunca teve login (e "Relogins" conta alternâncias, ver itens menores); F9, a recusa antiga continua no cartão depois de trocar modo ou idioma; e o espaço que falta depois do domínio na linha da lista ("domínio· País", comportamento antigo restaurado pela reversão do usuário).
- **T19: faixa de avisos em vermelho (decisão do usuário em 2026-10-16).** A faixa "Entre de novo em <site>" no topo da tela Sites passou de `--warning` para `--danger` (borda, ícone e fundo de 10%); o texto segue em `--text-primary`. Era o achado F2 do smoke.
- **`cargo test --lib`:** 689 testes passaram no Sandbox no HEAD `a96b1e3` (clippy sem avisos novos na branch); a T17 acrescenta 2 testes de `browser_contas` (691 passaram no Sandbox depois dela).

**Não verificado / pendente**

- **Não verificado no smoke** (nada disto foi visto no app real): a contagem de abertura aceita (`aberturas` e `ultima_abertura`; só os testes unitários cobrem); o botão "Já entrei" depois de abrir um site; o `definir_site_sem_login` em site `cookie_sessao` (nenhum site assim no seed nem no mock); a tela do Activity log; as colunas da mesma altura em janelas largas reais (só emuladas por CDP).
- A recarga da tela pelos eventos Tauri `novo-aviso-site` e `aviso-site-resolvido` é coberta por teste unitário; **fora do Tauri não dá para vê-la** (o mock só dispara o evento local).

**Decisão pendente do usuário** (não implementada)

- **Parede de login deixa a IA ligada num site vermelho.** `marcar_precisa_login` não zera `ia_pode_usar`: o site fica vermelho, mas a permissão continua gravada e volta a valer sozinha depois de "Já entrei". Decidir: zerar a IA ao registrar a parede, ou manter como está (hoje o porteiro e o `browser_contas` já tratam o site como fechado).
- **Amplitude de alguns domínios do seed.** `google.com` alcança e-mail, documentos, drive e sites; `personio.com` cobre `*.jobs.personio.com`; `gupy.io` cobre o portal de toda empresa. O usuário já disse "deixar como está" uma vez; fica registrado aqui como **risco explícito aceito** até nova decisão.

**Outros pendentes**

- **Vagas, candidaturas e taxa por site: ocultas.** Voltam só quando a vaga guardar a chave do site (`register_vaga`); isso é mudança de dados e exige **spec nova** e aprovação. Hoje o desejo "quantas vezes me candidatei neste site" **não é atendido**.
- **ATS compartilhado sem `caminho_ats`:** um site assim casa **toda** vaga daquele host e pode autorizar, no porteiro, páginas de outras empresas. O seed e `adicionar_site` deveriam recusar domínio de ATS compartilhado sem caminho (o seed de hoje é seguro). **A registrar no Linear.**
- **Rótulo "Cadastro":** a chave `sectionRegistry` existe em `src/i18n/pt.ts` mas a página não desenha divisor com esse título (o primeiro bloco abre sem título); decidir se entra.

**Itens menores adiados (ainda verdadeiros)**

- "Relogins" conta avisos criados (um por queda), não quedas de fato; dizer isso na tela se confundir.
- `jaAbriu` na página vale mesmo se `abrir_site` falhar; no passo 1, os botões somem após o clique e o foco cai no corpo.
- `.st-painel .vazio` sobrescreve a cor de `.st-erro`, então "Não foi possível carregar o uso deste site." não sai em vermelho; "2vezes" na leitura de tela (o espaço de "Relogins" é só CSS).
- `marcar_site_nao_logado` aceita site `nenhuma` (a UI não oferece); a verificação de `cookie_sessao` em `definir_site_sem_login` é lida antes da transação (sem exploração possível, escritor único); a atomicidade do `definir_site_sem_login` é protegida por teste do revisor, não por teste commitado; a asserção do porteiro em `repo.rs` é fraca (a IA já está em 0 antes de `decidir`).
- `resolver` consulta `listar` antes de ler a URL (URL inválida mais banco quebrado dá "não foi possível consultar" em vez de "fora da lista"; continua sendo recusa).
- Teste de migração aplica uma vez só; `schema.sql` não declara as colunas novas (só `ALTER`); nenhum teste commitado conta aberturas pelo `DbSiteGate` de arquivo.
- Linha da lista: risco vermelho sem texto de estado (desenho antigo); o domínio da linha não é link.
- `sites.mock.ts`: não atualiza `verificada_em` ao voltar a "pede login" (o backend atualiza).
- Na página de um site vermelho, "Confirmado em" mostra `verificada_em`, que é a data em que o login foi perdido (o backend grava now ao marcar `precisa_login`); cosmético.

## 11. Emenda de 2026-10-16 (pedido do usuário): a lista padrão não decide quem pede login

Pedido, nas palavras dele, olhando os cartões no Sandbox: "tu definiu como padrão quais sites são para login e quais não são. Isso é um problema, porque a ideia é que o usuário, a gente deu uma lista para ele, como um início, mas que ele vai adicionando site, removendo sites que ele quiser. E aí ele configure. [...] o cara vai clicar, vai abrir o browser e vai preencher o perfil já logado. [...] meio que eu vou criar um ranqueamento de coisas recomendadas, que são padrão, sites bons [...] tipo uma setlist, o cara vai clicando, ele configurou esse, aí próximo pro próximo [...] tu configura uma vez e só deixa. E por isso que a tua pré-configuração é ruim. Mesmo que seja um adiantamento, o usuário tem que checar isso manualmente."

- **Decisão:** a lista padrão **não pré-define** o login de nenhum site. Todo site do seed nasce com `regra_login = manual` (aparece vermelho "Precisa de login"), inclusive as páginas de carreiras de empresas e os agregadores que antes vinham como `nenhuma`. Quem diz que um site "não pede login" é o **usuário**, pelo botão do passo 1 da página do site. **Substitui** a regra do seed "`nenhuma` para páginas de carreiras públicas" (plano de 2026-10-14, Task 14, e a Task 14 de 2026-10-15).
- **Efeito em bancos que já existem:** o mesclador do seed só grava `regra_login` ao **inserir** um site; nunca sobrescreve a escolha de quem já tem o site. Portanto, num banco que já recebeu a lista antiga, os sites de empresas continuam `nenhuma` até o usuário mudar (ou até o banco de desenvolvimento ser recriado). Não há migração que reescreva escolhas.
- **Fora do escopo desta emenda (visão do usuário, propostas para depois, cada uma com spec própria):** (a) o **passo 2** (abrir o browser, o usuário já logado, e a IA preencher o perfil no site); (b) um **fluxo de configuração em sequência** ("setlist": configurou este, vai para o próximo, sem voltar à lista); (c) um **ranking de recomendação** (sites bons e padrão, por exemplo LinkedIn como ótima recomendação) que ordena essa sequência; (d) a **mistura da lista de aplicação** entre tipos de site.
