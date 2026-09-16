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
4. **Histórico** (tabela de vagas processadas — seção 6)
5. **Feedback** (item da sidebar; layout desta aba não é especificado neste ciclo)
6. **Pendências** — com badge numérico vermelho (`--danger`) sempre que existir pelo menos uma pendência não resolvida. O badge mostra a contagem; acima de 9, mostra "9+".
7. **Terminal**
8. Espaço flexível (empurra o item seguinte para o fundo)
9. **Configurações** + **ícone de engrenagem** (configurações rápidas/globais — tema, toggle de notificações), no fundo da sidebar, visualmente distinto dos itens de navegação principal.

Cada item de navegação principal (2–7) é uma linha de 40px de altura, ícone (Lucide ou Tabler outline, 18px) mais label, padding horizontal de 16px. O item ativo tem fundo `--accent-soft` e texto/ícone em `--accent-strong`; os restantes têm texto `--text-secondary`, mudando para `--text-primary` no hover.

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
4. **Pré-requisitos do sistema** — bloco de estado (não editável), mostrando se a extensão Claude in Chrome está instalada e ativa, a versão do Claude Code detectada, e se a conta tem o plano necessário. Cada item com um indicador verde/vermelho e, se vermelho, uma frase curta de como resolver.

## 11. Configurações rápidas (engrenagem)

Acionada pelo ícone de engrenagem no fundo da sidebar (item 9). Abre como um popover pequeno (280px), ancorado à engrenagem, não uma tela cheia — estas são definições que se ajustam com frequência, por isso não merecem o custo de navegação de uma aba inteira:

- **Tema**: claro / escuro / seguir o sistema (ainda que o padrão visual deste documento seja claro, o toggle de tema fica disponível desde já como preferência pessoal, não como decisão de design pendente).
- **Notificações**: toggle geral de notificações Windows, e o input do intervalo de repetição (5 / 10 / 15 minutos, conforme seção 6.5 do documento de arquitetura).
- Nenhuma outra opção vive aqui — qualquer coisa que precise de mais espaço ou contexto pertence à aba Configurações (item 9), não a este popover.

## 12. Resolução das referências provisórias

Este documento substitui, de forma definitiva, as seguintes referências marcadas como "provisório, aguardar documento de design" no prompt de construção histórico:

- A estrutura de componentes em `src/components/` deve ser reorganizada para refletir as seis telas principais desta seção (Dashboard, Perfil, Histórico, Terminal, Pendências, Configurações) mais o popover de configurações rápidas, em vez da divisão genérica originalmente sugerida.
- A tela de configurações da fase 2 do prompt de construção corresponde à seção 10 deste documento.
- O componente `PendenciaCard` da fase 6 corresponde à seção 9 deste documento.
- O dashboard e histórico da fase 7 correspondem às seções 4 e 6 deste documento.
- Qualquer identidade visual (cores, tipografia) a partir de agora segue exclusivamente a seção 2 deste documento — não a paleta padrão de nenhuma biblioteca de componentes usada.
