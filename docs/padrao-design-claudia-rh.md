# Padrão de design — Claudia RH

> Documento de referência visual para implementação. Especifica identidade, tokens de design (cor, tipografia, espaçamento), estrutura de navegação, e o layout de cada tela. Complementa `arquitetura-sistema-candidaturas.md` (lógica). O prompt de construção das fases Tauri é histórico (pasta pai do workspace) — onde ele dizia "provisório, aguardar o documento de design", é este documento que resolve essas referências.

## 1. Identidade

**Nome do produto:** Claudia RH.

**Tom:** ferramenta de trabalho pessoal, não produto comercial. Claudia RH existe para uma pessoa controlar o seu próprio processo de candidaturas — a voz da interface deve ser direta e funcional, nunca promocional. Não há copy de marketing em lado nenhum da aplicação; há apenas instruções claras, estados claros, e o que está acontecendo agora.

**Sensação geral:** um painel de controle leve, não um terminal escuro de operações. O usuário já fica com Chrome e terminal visíveis durante a execução — a própria app Tauri não precisa de competir com essa intensidade visual. É claro, calmo, e deixa o trabalho real (vagas, candidaturas, decisões) ser o conteúdo, não a casca.

### 1.1 Logotipo — óculos

O elemento de assinatura da identidade é um par de óculos redondos, em traço aberto (sem lente preenchida) — o trocadilho de "Claude" para "Claudia" associado à imagem clássica de uma profissional de RH experiente, criteriosa, que já viu currículos de mais. É o único elemento gráfico de personalidade da aplicação; todo o resto do sistema visual (seção 2) permanece deliberadamente neutro precisamente para que este símbolo se destaque sem competir com decoração.

**Geometria de referência** (viewBox 660×360, validada visualmente em várias iterações):

```svg
<g fill="none" stroke-linecap="round">
  <circle cx="160" cy="195" r="135"/>
  <circle cx="500" cy="195" r="135"/>
  <path d="M295 180 Q330 130 365 180"/>
  <path d="M10 195 L35 192"/>
  <path d="M650 195 L625 192"/>
</g>
```

Pontos que não devem ser alterados ao reimplementar: as duas lentes (`circle`) têm o mesmo raio e ficam próximas o suficiente para a ponte (`path` central) as conectar visivelmente nas bordas — a ponte não pode flutuar isolada nem as lentes podem tocar-se diretamente. A ponte sobe acima do nível dos centros das lentes (curva para cima, não reta), e as duas hastes laterais (os dois `path` finais) são curtas e discretas, não um elemento de destaque.

**Espessura do traço:** escala com o tamanho de uso — `stroke-width` 24–26 no tamanho de referência (logo grande, ex: tela de splash ou favicon grande), 26–28 em tamanhos pequenos como o item da sidebar (traços finos demais desaparecem em ícones pequenos), nunca abaixo de 22 nem acima de 32.

**Cor e aplicações:**

| Contexto | Cor do traço | Fundo |
|---|---|---|
| Cabeçalho da sidebar, junto ao nome "Claudia RH" | `--accent` (`#D97757`) | `--bg-surface` (branco) |
| Ícone da aplicação (barra de tarefas, atalho) | Branco (`#FFFFFF`) | `--accent` (`#D97757`), `border-radius` 14px sobre um quadrado |
| Marca-de-água opcional dentro do cartão "Agora" (seção 4.2), apenas quando há sessão ativa | `--accent`, opacidade 0.5, escala reduzida, posicionado no canto superior direito do cartão | transparente sobre `--bg-surface` |

O logotipo nunca aparece preenchido (sem fill), nunca aparece com lentes de tamanhos diferentes, e nunca é usado como padrão repetido ou textura de fundo — é um símbolo único, não um motivo decorativo espalhado pela interface. A sua única repetição admitida é a marca-de-água discreta da tabela acima, e mesmo essa é opcional, a aplicar com critério, não em todos os cartões.

## 2. Tokens de design

### 2.1 Paleta

| Token | Hex | Uso |
|---|---|---|
| `--bg-base` | `#FAF9F7` | fundo principal da aplicação, leve quebra do branco puro |
| `--bg-surface` | `#FFFFFF` | cartões, painéis, sidebar |
| `--bg-sunken` | `#F1EFEA` | áreas de recuo (ex: fundo do terminal antes de ativo, blocos de código) |
| `--border` | `#E4E1D9` | divisórias, contornos de cartão |
| `--text-primary` | `#1F1D18` | texto principal |
| `--text-secondary` | `#6B6759` | texto de apoio, legendas, timestamps |
| `--text-tertiary` | `#9D9889` | texto desativado, placeholders |
| `--accent` | `#D97757` | accent principal — o laranja característico do Claude |
| `--accent-strong` | `#B85C3E` | hover/active sobre accent, texto sobre fundos accent claros |
| `--accent-soft` | `#F5E4DB` | fundos suaves com o accent (ex: item de sidebar ativo) |
| `--success` | `#3B7A52` | candidatura aplicada com sucesso |
| `--warning` | `#B8862E` | pendência de baixa urgência, aviso |
| `--danger` | `#B8473D` | pendência crítica, bloqueio, badge de notificação |

Esta paleta usa um único accent (o laranja do Claude, conforme pedido) e reserva verde/âmbar/vermelho exclusivamente para estado semântico (sucesso, aviso, perigo) — nunca para decoração. Isto significa que o laranja nunca compete com o vermelho de uma pendência: o usuário aprende rapidamente que vermelho = precisa da sua atenção, e o resto da interface é neutro com o accent a marcar apenas o que está ativo ou é a ação principal.

### 2.2 Tipografia

- **Interface (títulos, labels, corpo):** Inter — face utilitária, neutra, com excelente legibilidade em densidades de informação altas (tabelas de vagas, listas de pendências). Pesos usados: 400 (corpo), 500 (labels, títulos de cartão), 600 (títulos de seção, apenas).
- **Dados monoespaçados (terminal, IDs de vaga, timestamps técnicos):** JetBrains Mono ou, na ausência desta, Consolas/Cascadia Code como fallback no Windows.
- **Escala tipográfica:**
  - Título de seção (ex: "Dashboard", "Histórico"): 20px / 600
  - Título de cartão: 15px / 500
  - Corpo: 14px / 400
  - Legenda/metadado (timestamps, contadores): 12px / 400, cor `--text-secondary`

Não há face serifada em nenhum lugar da aplicação — isto não é um produto editorial, é uma ferramenta de dados e controle, e a tipografia deve servir densidade e escaneabilidade antes de personalidade.

### 2.3 Espaçamento e forma

- Unidade base: 4px. Espaçamentos usados: 4, 8, 12, 16, 24, 32px.
- `border-radius`: 8px em cartões e botões, 6px em badges e pills, 4px em inputs.
- Sombra: apenas uma, sutil, para elevar modais/drawers sobre o conteúdo (`0 4px 16px rgba(31,29,24,0.08)`). Cartões na superfície normal não têm sombra — distinguem-se por borda de 1px em `--border`, não por elevação.

## 3. Estrutura de navegação

Sidebar fixa à esquerda, 220px de largura, fundo `--bg-surface`, borda direita de 1px em `--border`. Do topo para baixo:

1. **Logotipo (óculos, seção 1.1) + nome** ("Claudia RH"), área de cabeçalho da sidebar, 56px de altura, logo à esquerda do nome com 10px de gap.
2. **Dashboard**
3. **Perfil** (aba conversacional — seção 5)
4. **Currículos** (ícone `FileText`; biblioteca de currículos e cartas — seção 13)
5. **Sites** (ícone `Globe`; "Sites" nos dois idiomas; lista de sites que a IA pode abrir — seção 15). Sem badge.
6. **Histórico** (tabela de vagas processadas — seção 6)
7. **Feedback** (item da sidebar; layout desta aba não é especificado neste ciclo)
8. **Pendências** — com badge numérico vermelho (`--danger`) sempre que existir pelo menos uma pendência não resolvida. O badge mostra a contagem; acima de 9, mostra "9+".
9. **Terminal**
10. Espaço flexível (empurra o item seguinte para o fundo)
11. **Configurações** + **ícone de engrenagem** (configurações rápidas/globais — tema, toggle de notificações), no fundo da sidebar, visualmente distinto dos itens de navegação principal.

Cada item de navegação principal (2–9) é uma linha de 40px de altura, ícone (Lucide ou Tabler outline, 18px) mais label, padding horizontal de 16px. O item ativo tem fundo `--accent-soft` e texto/ícone em `--accent-strong`; os restantes têm texto `--text-secondary`, mudando para `--text-primary` no hover.

## 4. Dashboard

A primeira tela que o usuário vê. Tem dois estados distintos, dependendo de existir ou não uma sessão de execução ativa neste momento — isto é o requisito central desta tela, conforme pedido: mostrar o que está acontecendo agora, não apenas números acumulados.

### 4.1 Estado sem sessão ativa

- **Cartão de resumo do dia**, no topo: candidaturas enviadas hoje / orçamento diário (ex: "4 / 10"), com uma barra de progresso fina em `--accent`.
- **Botão principal "Procurar vagas agora"**, em destaque (`--accent` como fundo, texto branco), abaixo do cartão de resumo.
- **Atividade recente**, lista compacta das últimas 5–8 vagas processadas (qualquer status), cada linha com: nome da vaga, empresa, status (badge colorido conforme seção 7), tempo relativo ("há 12 min").
- Se existirem pendências não resolvidas, um cartão de aviso (`--warning` ou `--danger` conforme severidade) aparece imediatamente abaixo do cartão de resumo, com contagem e link direto para a aba Pendências.

### 4.2 Estado com sessão ativa

Substitui o botão "Procurar vagas agora" por um indicador de estado ativo: ponto pulsante em `--accent` mais o texto "A trabalhar...".

- **Cartão "Agora"**, em destaque visual (borda em `--accent`, não apenas `--border`): mostra a vaga que a sessão está processando neste instante — título, empresa, e a etapa atual em texto curto (ex: "a preencher formulário", "a analisar correspondência com o perfil", "à espera de resposta da página"). Este texto vem de uma leitura periódica do estado compartilhado (a sessão de execução atualiza isto a cada mudança relevante, não apenas no fim de cada vaga). **O link da vaga (campo `url` da tabela `vagas`) é sempre visível neste cartão**, como uma linha de texto truncado com ícone de link externo (`ti-external-link`), nunca escondido atrás de um clique — o usuário deve poder abrir a vaga original a qualquer momento para acompanhar o que a sessão está vendo.
- Abaixo do cartão "Agora", a fila de vagas ainda por processar nesta sessão, em lista compacta e mais discreta (texto `--text-secondary`, sem badges de status, já que ainda não foram avaliadas).
- O cartão de resumo do dia e a atividade recente continuam visíveis, mas abaixo do cartão "Agora" — a prioridade visual vai para o que está acontecendo neste momento.
- Um botão secundário "Abrir terminal" neste cartão leva diretamente à aba Terminal, para quem quiser ver o detalhe completo do raciocínio.

## 5. Perfil

Dois estados, usando só os tokens da seção 2. Sem paleta nova.

### 5.1 Estado resumo (padrão, quando já existe pelo menos uma variante)

Cartões das seções de `candidate_base.yaml` (dados pessoais, experiência, projetos, formação, competências, idiomas) e, abaixo, um cartão por variante de `search_variants.yaml` com nome e peso visível (barra). Cada cartão tem um botão "Editar".

Botão "Atualizar perfil" abre o chat sem foco. Botão "Nova variante" abre o chat focado em criar variante.

### 5.2 Estado chat

Bolhas de mensagem, usuário à direita, Claude à esquerda. Markdown básico (listas, negrito, código). Indicador de streaming (cursor piscante) enquanto a sessão responde. Enter envia.

Entrada geral ("Atualizar perfil") ou focada ("Editar" de um cartão — o chat já recebe qual seção/variante está em foco). A reabertura nunca começa do zero se já existirem dados: o prompt de sistema inclui os YAML atuais.

## 6. Histórico

Tabela única de tudo o que foi descoberto/processado, qualquer status. Sem as duas sub-vistas antigas ("Todas as vagas" / "Histórico") como navegação principal.

Colunas: título, empresa, plataforma, status (badge, seção 7), data, variante (`variante_id` como texto discreto, não badge de cor nova), ação (abrir detalhe / link externo / pasta de arquivos quando existir candidatura).

Filtro por status como pills acima da tabela (Todas, Descoberta, Analisada, Aplicada, Pendente revisão, Bloqueada, Pulada). Filtro por variante quando houver mais de uma variante.

Clicar numa linha abre drawer de detalhe (420px, da direita): match, motivo de status, link da vaga original, pasta de arquivos gerados se `status = aplicada`.

## 7. Badges de status

Usados em qualquer tabela ou cartão que mostre o `status` de uma vaga (campo definido na seção 10 do documento de arquitetura). Pill pequena, 12px de texto, peso 500, padding 2px 8px, `border-radius` 6px:

| Status | Fundo | Texto |
|---|---|---|
| `descoberta` | `--bg-sunken` | `--text-secondary` |
| `analisada` | `--bg-sunken` | `--text-secondary` |
| `candidatando` | `--accent-soft` | `--accent-strong` |
| `aplicada` | `#E3EFE7` | `--success` |
| `pulada` | `--bg-sunken` | `--text-tertiary` |
| `pendente_revisao` | `#FBEFD9` | `--warning` |
| `bloqueada` | `#F7E2DF` | `--danger` |

`pulada` é deliberadamente o estado mais discreto da tabela — não é um problema, é apenas "decidiu não avançar", e a hierarquia visual deve refletir isso. `pendente_revisao` e `bloqueada` são as únicas cores quentes fora do accent, reservadas para chamar atenção real.

## 8. Terminal

Tela dedicada (não painel fixo, conforme decidido). Ocupa toda a área de conteúdo à direita da sidebar, sem padding lateral — o terminal deve sentir-se como uma superfície de trabalho, não um cartão dentro de outro cartão.

- **Barra superior fina** (36px), fundo `--bg-sunken`, contendo: indicador de estado da sessão (ponto verde/cinza + "Sessão ativa" / "Sem sessão"), e o botão "Assumir controle" alinhado à direita — conforme a seção 8.3 do documento de arquitetura, este botão alterna o estado de bloqueio do terminal.
- Quando bloqueado (padrão), a barra superior tem uma etiqueta discreta "Apenas visualização" em `--text-secondary`. Quando desbloqueado, a etiqueta muda para "Sob o seu controle" em `--accent-strong`, e a borda externa do painel do terminal (2px) muda de `--border` para `--accent`, para que a diferença seja inequívoca à distância, sem precisar de ler texto.
- O terminal em si (`xterm.js`) usa fundo `--bg-sunken` (não preto), texto `--text-primary`, com a paleta ANSI ajustada para permanecer legível sobre fundo claro — cores ANSI padrão tendem a assumir fundo escuro, por isso os tons devem ser escurecidos o suficiente para contraste em fundo `#F1EFEA`. Fonte JetBrains Mono, 13px, `line-height` 1.5.

## 9. Pendências

Lista de cartões, um por pendência não resolvida, ordenados da mais antiga para a mais recente (a mais antiga é a que está mais perto de re-disparar notificação, por isso fica em primeiro). Cada cartão:

- Cabeçalho: nome da vaga + empresa, badge de categoria (correspondendo às categorias da seção 6.1 do documento de arquitetura — ex: "Salário fora da faixa", "Captcha", "Pergunta sem resposta"), tempo desde que foi criada.
- Corpo: a descrição legível do que travou, escrita pela sessão de execução.
- Ações, alinhadas à direita do cartão: "Resolver" (abre um formulário inline ou modal, dependendo da categoria — ex: para pergunta sem resposta, um campo de texto; para captcha, apenas um botão "Já resolvi no Chrome, continuar"), e "Pular esta vaga" (ação secundária, texto, sem fundo).

Pendências resolvidas não aparecem mais nesta lista — o registro fica no detalhe da vaga em Histórico.

## 10. Configurações

Organizada em seções dentro de uma única tela com scroll, cada seção com um título de 15px/500 e separada da seguinte por um espaço de 32px (não por divisórias visuais — o espaço já é suficiente):

1. **Credenciais** — uma linha por plataforma configurada (LinkedIn, e espaço para adicionar outras), com campos de usuário/password geridos via keyring — a password nunca é mostrada em texto simples depois de guardada, apenas um botão "Substituir".
2. **Disparo automático** — toggle "Ativar disparo por inatividade", e, quando ativo, um input numérico para o limiar em minutos (default 15).
3. **Pasta de aplicações** — caminho onde os arquivos gerados são guardados, com botão para escolher pasta via diálogo nativo do Windows.
4. **Pré-requisitos do sistema** — bloco de estado (não editável), mostrando se o browser da Claudia está pronto (baixado e aberto pelo app, sem extensão), a versão do Claude Code detectada, e se a conta tem o plano necessário. Cada item com um indicador verde/vermelho e, se vermelho, uma frase curta de como resolver.
5. **Activity log** — uma descrição curta e o botão "Ver registros", que abre a tela própria da seção 14.

## 11. Configurações rápidas (engrenagem)

Acionada pelo ícone de engrenagem no fundo da sidebar (item 11). Abre como um popover pequeno (280px), ancorado à engrenagem, não uma tela cheia — estas são definições que se ajustam com frequência, por isso não merecem o custo de navegação de uma aba inteira:

- **Tema**: claro / escuro / seguir o sistema (ainda que o padrão visual deste documento seja claro, o toggle de tema fica disponível desde já como preferência pessoal, não como decisão de design pendente).
- **Notificações**: toggle geral de notificações Windows, e o input do intervalo de repetição (5 / 10 / 15 minutos, conforme seção 6.5 do documento de arquitetura).
- Nenhuma outra opção vive aqui — qualquer coisa que precise de mais espaço ou contexto pertence à aba Configurações (item 11), não a este popover.

## 12. Resolução das referências provisórias

Este documento substitui, de forma definitiva, as seguintes referências marcadas como "provisório, aguardar documento de design" no prompt de construção histórico:

- A estrutura de componentes em `src/components/` deve ser reorganizada para refletir as seis telas principais desta seção (Dashboard, Perfil, Histórico, Terminal, Pendências, Configurações) mais o popover de configurações rápidas, em vez da divisão genérica originalmente sugerida.
- A tela de configurações da fase 2 do prompt de construção corresponde à seção 10 deste documento.
- O componente `PendenciaCard` da fase 6 corresponde à seção 9 deste documento.
- O dashboard e histórico da fase 7 correspondem às seções 4 e 6 deste documento.
- Qualquer identidade visual (cores, tipografia) a partir de agora segue exclusivamente a seção 2 deste documento — não a paleta padrão de nenhuma biblioteca de componentes usada.

## 13. Currículos

Item 4 da sidebar (`FileText`, label "Currículos"; "Résumés" em inglês). Código em `src/components/documentos/`. É a biblioteca de documentos (arquitetura 3.7).

**Tela principal.** Título de seção "Currículos e cover letters" (20px / 600), sobre uma barra de duas abas (**Currículos** e **Cover letters**): aba ativa com texto `--accent-strong` e sublinhado de 2px em `--accent`, as outras em `--text-secondary`. Padding de 24px.

- **Barra de ferramentas**, acima da grade: à esquerda, um `select` por tipo de tag (**Variante**, **Template**, **Cor**, **Idioma**), cada um com a opção "<Rótulo>: Todos" e os valores que existem nos documentos; aparecem só quando há pelo menos um documento. À direita, **Gerar** (botão cheio em `--accent`, texto branco) e **Adicionar o meu** (botão com borda, para importar um PDF ou DOCX pelo seletor do sistema).
- **Grade:** `repeat(auto-fill, minmax(190px, 1fr))`, 16px de espaço. Cada **cartão** é uma superfície com borda de 1px em `--border`, raio de 8px e padding de 12px; a borda vai a `--accent` no hover. Dentro: a **miniatura** (a folha A4 escalada, no mesmo HTML da pré-visualização, carregada só quando o cartão aparece; documento importado mostra o ícone de arquivo e o formato), o nome (14px / 500, uma linha com reticências), a origem ("Gerado" ou "Importado", 12px), uma estrela cheia em `--accent` quando é o **preferido geral**, a linha "Preferido da variante: ..." em `--accent-strong` quando é o preferido de alguma variante, e as **tags** como pills de 11px em `--bg-sunken` (a tag de cor leva uma bolinha da própria cor).
- **Estados:** carregando ("Carregando…"); erro com borda e texto `--danger` e botão "Tentar de novo"; vazio (cartão tracejado: "Nenhum currículo ainda." / "Nenhuma cover letter ainda." e a dica "Gere um com a Claudia ou adicione o seu PDF ou DOCX."); filtros sem resultado ("Nenhum documento combina com esses filtros.").
- **Gerar** abre um painel acima da grade (título "Gerar currículo" ou "Nova cover letter", com X para fechar): na cover letter, empresa e cargo (obrigatórios), descrição da vaga e nota extra (opcionais); nas duas, o idioma (PT ou EN), a cor de destaque, a variante e, no currículo, os cartões de template. Quando o perfil não tem e-mail, a carta é salva e o painel mostra o aviso "Carta salva. Aviso: ..." com um botão OK.

**Página do documento** (substitui a grade; botão **Voltar** no topo). Duas colunas que quebram em telas estreitas:
- **Folha A4** à esquerda, até 794px de largura, escalada à coluna, num `iframe` sem permissões (`sandbox` vazio: sem scripts), com sombra única do documento (seção 2.3). Documento importado: um cartão com ícone, nome, formato, tamanho, data, a frase "Arquivo importado. Abra no visualizador do sistema." e o botão **Abrir**.
- **Coluna lateral** (280 a 360px) com três painéis (borda de 1px, raio de 10px): **Detalhes** (origem, formato, tamanho, criado em), **Tags** (pills removíveis com "x", mais um `select` do tipo — variante, template, cor, idioma, livre — um campo e **Adicionar**) e **Preferido** (uma caixa "Preferido geral" e uma por variante ativa). Abaixo, os botões **Renomear** (edita o título no lugar), **Abrir** e **Excluir** (`--danger`); excluir pede confirmação e avisa quando o documento é um preferido.

A aba recarrega sozinha quando o backend muda os documentos (geração, migração, o agente salvando uma carta). No Perfil, um `select` escolhe a variante principal.

## 14. Activity log (tela de Configurações)

Não é item da sidebar: abre a partir de **Configurações → Activity log → Ver registros**, ocupando a área principal, com **← Voltar** no topo (volta às Configurações). Somente leitura: não existe "Limpar tudo" nem outra escrita; a retenção é automática (30 dias ou 1 GB).

- **Filtros** numa linha acima da lista: tipo (**todos, ação, falha, navegação, browser**), sessão e vaga.
- **Lista** cronológica, mais novo primeiro, em tabela de colunas fixas: hora local, ferramenta (ou o tipo, quando não há ferramenta), alvo, resultado e duração, com "Carregar mais" a cada 100 linhas. Falhas em `--danger`, só como estado.
- **Detalhe** ao clicar numa linha, ao lado: o print **antes** e o **depois** (carregados só do registro aberto), os campos nomeados (hora, tipo, passo, alvo, papel, texto, endereço, resultado, erro, duração, sessão, vaga) e os dados técnicos num bloco secundário. As linhas do tipo **browser** são os eventos do próprio browser (download, lançamento, queda, queda de aba, diálogo).
- Os prints podem mostrar dados pessoais; ficam só nesta máquina.
## 15. Sites

Item 5 da sidebar (`Globe`, label "Sites" nos dois idiomas, sem badge). Título da tela: "Sites de vagas" ("Job sites" em inglês). Código em `src/components/sites/`, estilos em `src/components/sites/estilos.ts` (CSS injetado uma vez dentro de `.sites-tela`). É a lista de sites que a IA pode abrir (arquitetura 3.8). Padding de 24px. Só tokens da seção 2; nada de cor fixa além do branco sobre `--accent`.

**Barra de ferramentas**, acima da galeria ou da lista, numa linha que quebra: busca (por nome ou domínio, sem acento e sem diferença de maiúsculas, "orsted" acha "Ørsted"), três `select` de filtro (**Tipo**, **Região**, **País**, cada um com "<Rótulo>: Todos" e só os valores que existem na lista; o país aparece pelo nome por extenso), **Ordenar por** (Nome, Tipo, Região, País, Estado de login) com um botão que inverte a ordem, o seletor **Galeria | Lista** (o modo fica lembrado em `localStorage`, chave `sites-modo`; sem armazenamento, volta para a galeria), o contador "N de TOTAL sites" em `--text-tertiary` e, à direita, **Adicionar site** (botão cheio em `--accent`). Sites sem país vão por último na ordenação por país, nas duas direções.

**Formulário "Adicionar site"** (painel acima da lista, com borda de 1px): nome, endereço, tipo (`select`) e país (opcional, código de 2 letras; vai para maiúsculas ao enviar). **Salvar** e **Cancelar**. A validação é do backend; o formulário só mostra a mensagem devolvida em `--danger` (por exemplo `http://` recusado, domínio repetido ou sobreposto).

**Cartão da galeria.** Retangular, em colunas de 300px: grade `repeat(auto-fill, minmax(min(300px, 100%), 300px))`, 16px de espaço; cartão com borda de 1px em `--border`, raio de 8px, borda mais escura no hover. O corpo do cartão abre a página do site; **só a faixa inferior liga e desliga a IA**.
- **Capa** de 52px em `--bg-sunken`, gerada (sem favicon): a inicial do nome bem grande e quase transparente ao fundo, o **nome** do site (15px / 500, uma linha com reticências; é um botão que abre a página do site dentro da tela e estica o clique ao cartão inteiro) e o botão **Abrir ↗** à direita (abre o site no browser da Claudia).
- **Bloco branco:** o domínio como link (abre o site no browser da Claudia, não no navegador do app); linha de pills com o **país pelo nome por extenso** (ou a região quando o site não tem país) e o **tipo**, mais a pill "descoberta" quando o papel é só descoberta de vagas; e o **estado de login** numa linha própria: bolinha de 8px mais texto. "não é mais padrão" aparece em 11px quando o site saiu do seed.
- **Estado de login** (não existe âmbar nem "Sem confirmação"): verde (`--success`) "Logada · há N dias" ou "Logada · hoje" para `logada`; verde "Não pede login" para site com `regra_login = nenhuma` (salvo se o estado ainda for `logada`: então aparece "Logada · há N dias"); **vermelho** (`--danger`) "Precisa de login" ("Login required") **só** para `precisa_login` (parede de login, "Este site pede login", "Não estou mais logado"). **Site que ninguém configurou** (login manual e `desconhecido`, nunca confirmado) **aparece limpo: nenhum rótulo, nenhuma bolinha**; a galeria mostra só a faixa "Faça o passo 1…", a página não mostra rótulo no cabeçalho e a linha da lista usa o risco neutro (`--border`). Site que exige login e não está logado continua bloqueado para a IA, vermelho ou não.
- **Faixa inferior** de 42px, um botão de verdade, com três situações: **site não pronto** (exige login e não está logado): marca tracejada e "Faça o passo 1 para liberar" com "Entrar →" à direita, em `--text-secondary` sobre `--bg-sunken`; é um botão comum que **abre a página do site**, não um interruptor. **Pronto e fechado:** "Fechada · clique para liberar", `role="switch"` com `aria-checked="false"`. **Pronto e liberado:** fundo `--accent-soft`, marca cheia em `--accent`, "Liberada para a IA", `role="switch"` com `aria-checked="true"`. Teclado: a faixa é um botão (Espaço e Enter); o foco do cartão aparece num anel desenhado no cartão inteiro.
- **Rótulos dos tipos** (as chaves não mudam): Portal de vagas, Portal regional, Vagas de tecnologia, Vagas remotas, Plataforma freelance, Site da empresa, Portal especializado.

**Linha da lista.** Voltou ao comportamento antigo (o usuário desfez a lista nova em 2026-10-15). Linha de no mínimo 52px, borda de 1px em `--border`, raio de 8px, em colunas: um risco vertical de 4px na cor do estado de login (`--success`, `--danger` ou `--border` quando não há estado), um monograma de 28px, nome (14px / 500, botão que abre a página) com "domínio · país · tipo" embaixo em 12px, o botão **Abrir ↗** e o texto final ("✓ Liberada", "Fechada" ou "Entre antes"). **A linha inteira é o interruptor** (`role="switch"`, `aria-checked`, foco visível, Espaço e Enter quando o foco está na própria linha; o nome e "Abrir ↗" não alternam). Liberada ganha borda `--accent` e fundo `--accent-soft`. Linha bloqueada (site vermelho) não é `switch` (é `group`); o clique chega ao backend como alternância: se a IA estava ligada (uma parede de login pode deixá-la ligada com o site vermelho), o clique a desliga; ligar é recusado e a mensagem aparece sob a linha em `--danger`. Na galeria e na página, um site vermelho não pode ser alternado.

**Página do site** (substitui a lista; **← Sites** no topo; a lista continua montada por baixo, então filtros, ordem e modo sobrevivem, e o foco e a rolagem voltam ao cartão de onde se saiu). Cabeçalho com nome, pills de local e tipo e o estado de login. Logo abaixo do cabeçalho, uma linha `role="alert"` em `--danger` mostra a recusa de qualquer ação (liberar, abrir, remover, marcar login); fica fora da zona de risco, então aparece mesmo com ela recolhida. Duas colunas de **mesma altura**; a grade da página é uma consulta de contêiner: duas colunas só quando a área de conteúdo passa de 700px, senão uma.
- **Sobre o site** (painel de 1px, três blocos; os dois últimos abrem com um divisor com título):
  - **Cadastro** (primeiro bloco, sem divisor com título): endereço (link que abre o site), tipo, região, país ("Sem país (vale para vários)" quando não há), papel ("Candidatura de ponta a ponta" ou "Só descoberta de vagas"), origem ("Veio com o app" ou "Adicionado por você") e domínios.
  - **Conta:** "Confirmado em" com a data (ou "Ainda não entrou") e "Relogins" com a contagem; para site que não pede login, só "Login: Não pede".
  - **Uso pela IA:** só "Aberturas", com a data da última ("última dd/mm/aaaa"), ou "A IA ainda não abriu este site." quando a contagem é zero; "Não foi possível carregar o uso deste site." se a leitura falhar. **Vagas, candidaturas e taxa por site não são mostradas**: a vaga não guarda a chave do site, só a URL, e o casamento por domínio não é confiável (arquitetura 3.8).
- **O que a IA pode fazer aqui:** três cartões de passo, sempre na ordem 1, 2, 3.
  1. **Entrar no site:** **Abrir o site ↗** (cheio em `--accent` enquanto o passo está pendente); **Já entrei** só aparece depois que o site foi aberto por ali nesta visita (antes disso, a dica de que o botão aparece depois de abrir); **Este site não pede login** (escondido para site `cookie_sessao`). Quando feito: selo "Feito" mais, se logado, **Abrir o site ↗** e **Não estou mais logado**; para site sem login, o selo e **Este site pede login** (escondido em `cookie_sessao`).
  2. **Configurar o perfil no site:** botão desabilitado e selo "Em breve"; opcional, não bloqueia a liberação.
  3. **Liberar para a IA:** travado ("Disponível depois do passo 1", selo "Bloqueado") até o passo 1 estar feito; depois mostra a mesma faixa do cartão, solta (borda de 1px, raio de 8px, até 300px) e `switch`.
- **Zona de risco** abaixo das colunas: `<details>` recolhido por padrão, com título em `--danger`. Dentro, **Remover site** (`--danger`) com confirmação inline ("Remover este site da lista?", **Remover da lista** e **Cancelar**); fica **desabilitado** para site que veio com o app (com a dica "Só sites que você adicionou podem ser removidos. Este veio com o app.").

**Estados.** Carregando ("Carregando…" em `--text-tertiary`). Erro (cartão com borda e texto em `--danger`, "Não foi possível carregar os sites: <erro>" e **Tentar de novo**). Vazio por filtro ("Nenhum site combina com esses filtros." e o botão **Limpar filtros**). Recusas e falhas de uma ação (liberar, abrir, remover, marcar login) aparecem junto do site (sob o cartão ou a linha, e na linha de alerta da página), em `--danger`, e a lista recarrega.

**Faixa de avisos.** No topo da tela, acima da barra, uma linha por site cujo login foi perdido: ícone de alerta e "Entre de novo em <site>" em vermelho (borda `--danger`, fundo de 10% de `--danger` sobre `--bg-surface`) e o botão **Abrir**, que abre o site. Some quando o usuário marca "Já entrei" ou "Este site não pede login" (o aviso é resolvido). Voltar a "Este site pede login" cria um aviso, uma vez. Cada linha tem um X que dispensa só o aviso; o site continua vermelho.

**Pendências: item "Site pede login".** A tela Pendências (seção 9) mostra cada aviso de site como um cartão do mesmo molde, com filete esquerdo de 3px em `--warning`, o badge de categoria "Site pede login" em `--warning`, o tempo desde a criação, o texto "Entre de novo em <site>" e as ações **Abrir o site ↗** e, só depois de abrir por ali, **Já entrei**. O badge do item Pendências da sidebar conta esses avisos junto com as pendências e as propostas de perfil.

**Atualização.** A tela recarrega sozinha em `db-atualizada`, no evento local `sites-mudaram` e em `novo-aviso-site` / `aviso-site-resolvido` (ouvidos em `src/components/sites/ouvir.ts`). As aberturas contadas pelo porteiro e as paredes de login registradas pelo processo do MCP chegam à tela por esses eventos ou no próximo recarregamento.
