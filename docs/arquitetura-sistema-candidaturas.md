# Arquitetura do sistema de candidaturas automáticas

> Documento de referência para implementação. Descreve o sistema completo: app Tauri, sessão Claude Code de execução, camada de estratégia, terminal embutido, perfil do candidato, e regras de governação. Este documento é a fonte de verdade para qualquer trabalho de codificação subsequente.

## 1. Visão geral

O sistema é uma aplicação desktop (Windows, construída em Tauri) que automatiza a descoberta e candidatura a vagas de emprego, com o objetivo de processar cerca de dez vagas por dia, cobrindo tanto candidaturas "fáceis" (Easy Apply, formulários padronizados) como candidaturas manuais que exigem navegação por sites de terceiros.

A aplicação não tenta resolver isto com automação puramente scriptada (Playwright determinístico). Em vez disso, orquestra sessões do Claude Code ligadas ao Chrome através da extensão oficial Claude in Chrome (via native messaging, ativada com a flag `--chrome` ou o comando `/chrome`), deixando o modelo interpretar cada página, decidir o que fazer, e quando parar. O Tauri funciona como casca: interface, configuração, histórico, e o agente que dispara essas sessões — nunca como motor de decisão.

A ideia central do sistema pode ser resumida assim: existe uma camada lenta e conversacional, onde o usuário e o Claude discutem estratégia, perfil e prioridades; e existe uma camada rápida e autónoma, onde uma sessão de execução aplica essa estratégia vaga a vaga, de forma visível, parando sempre que encontra algo fora do que foi acordado.

## 2. Princípios de design

Estes princípios não são negociáveis e devem orientar qualquer decisão de implementação que não esteja explicitamente coberta neste documento.

**Visibilidade sobre ocultação.** O Chrome controlado pela sessão de execução nunca é minimizado, escondido, ou corre em modo headless. O usuário deve poder ver, em qualquer momento, o que a IA está a fazer. Isto não é negociável mesmo que prejudique performance ou elegância técnica.

**Honestidade sobre o currículo é absoluta.** O sistema nunca inventa experiência, certificações, tecnologias, ou inflaciona competências do candidato. Qualquer conteúdo gerado (CV, carta, respostas em formulário) deve ser rastreável a algo que existe no perfil ou no currículo mestre. Isto é herdado diretamente da skill `tailor-application` original e estende-se a todo o sistema.

**Pausa é o comportamento seguro por padrão.** Sempre que a sessão de execução encontra ambiguidade real — um campo fora do perfil, uma pergunta sem resposta clara, um captcha, uma vaga que não se encaixa nos critérios — o comportamento correto é parar e tornar isso visível, nunca adivinhar e seguir.

**O perfil é a única fonte de verdade sobre o candidato.** A sessão de execução nunca decide sozinha o que o candidato quer ou aceita. Se uma decisão depende de uma preferência que não está no perfil, isso é, por definição, um motivo de pausa — não um espaço para a IA exercer julgamento próprio.

**Etiqueta de plataforma é uma prioridade de produto, não só de risco.** O sistema deve comportar-se com um ritmo que pareça humano nas plataformas que visita (LinkedIn, Indeed, Jobindex, sites de empresas), não porque a conta do usuário precisa de proteção a todo o custo, mas porque um sistema que se comporta de forma agressiva é, por construção, um sistema de pior qualidade.

**Tudo é auditável depois do facto.** Cada candidatura enviada, cada vaga pulada, cada decisão de pausa, fica registada com o motivo. O usuário deve poder, semanas depois, reconstruir porque é que uma vaga específica foi tratada de uma certa forma.

## 3. Componentes do sistema

O sistema divide-se em cinco componentes com responsabilidades estritamente separadas. Nenhum componente deve assumir a responsabilidade de outro — é esta separação que torna o sistema depurável e que permite a cada peça evoluir de forma independente.

### 3.1 App Tauri (casca)

Aplicação nativa Windows, construída em Tauri (Rust no backend, webview HTML/CSS/JS no frontend). Responsabilidades:

- Renderizar o dashboard: histórico de vagas, estado atual, pendências de revisão.
- Gerir configurações: credenciais de plataformas (LinkedIn e outras) e caminho da pasta de aplicações. Os dados do candidato não se editam aqui — vivem na aba Perfil.
- Expor os dois mecanismos de disparo da sessão de execução: botão manual ("procurar vagas agora") e deteção de inatividade do sistema.
- Spawnar e gerir o processo `claude` como child process, através de um pseudo-terminal (PTY), e renderizar o seu output num terminal embutido.
- Gerir o segundo modo de sessão Claude Code: a conversa da aba Perfil, invocada sem `--chrome`, com interface de chat (não PTY/terminal). Essa sessão lê e escreve `candidate_base.yaml` e `search_variants.yaml`; nunca submete candidaturas.
- Posicionar e (opcionalmente) gerir a janela do Chrome aberta pela sessão de execução, sem nunca a esconder.
- Ler o estado persistido (SQLite, arquivos `candidate_base.yaml`, `search_variants.yaml` e estratégia) e refletir isso na interface. O Tauri nunca escreve diretamente nesse estado como resultado de uma decisão de candidatura — apenas como resultado de ação direta do usuário (ex: editar o perfil, aprovar uma pendência).

O Tauri é deliberadamente "burro" em relação a decisões sobre vagas. Ele observa e expõe controle; não decide.

### 3.2 Sessão de execução (Claude Code + extensão Claude in Chrome)

Processo `claude` invocado pelo Tauri com a integração Chrome ativa (flag `--chrome`), responsável por todo o trabalho de descoberta, análise, geração de conteúdo e candidatura. Esta sessão:

- Lê, no arranque, `candidate_base.yaml`, `search_variants.yaml`, a estratégia do dia (se existir), e a memória das últimas execuções.
- Navega as plataformas configuradas (job boards e, com restrições próprias, o LinkedIn), com um Chrome sempre visível.
- Para cada vaga candidata, avalia o match contra o perfil, decide se vale a pena candidatar, gera o material necessário (CV adaptado, carta, ou resposta a um formulário) e executa a candidatura — ou pausa e pede intervenção humana, segundo as regras descritas na seção 6.
- Escreve o resultado de cada vaga processada (aplicada, pulada, pendente, bloqueada) no estado partilhado, para que o Tauri possa refletir isso na interface em tempo real.
- Decide internamente, vaga a vaga, se deve continuar na mesma sessão ou sinalizar ao Tauri que é preferível reiniciar o processo antes da próxima vaga (ver seção 7).

Esta é a única parte do sistema com poder de decisão real. Tudo o resto existe para lhe dar contexto ou para lhe impor limites.

**Pré-requisitos confirmados desta integração:** exige um plano direto pago da Anthropic (Pro, Max, Team, ou Enterprise) — não funciona através de Amazon Bedrock, Google Cloud Vertex AI, ou Microsoft Foundry; se a conta usada for de um destes provedores, é necessária uma conta claude.ai separada apenas para esta funcionalidade. Funciona apenas com Google Chrome ou Microsoft Edge — Brave, Arc, e outros browsers baseados em Chromium não são suportados, nem o WSL no Windows. Estes pré-requisitos devem ser verificados e comunicados ao usuário na tela de configurações antes de qualquer tentativa de disparo da sessão de execução.

### 3.3 Camada de estratégia

Um modo de operação distinto da execução — mais lento, conversacional, iniciado pelo usuário quando este o desejar, nunca automaticamente. Nesta camada:

- O usuário e o Claude discutem e atualizam o perfil do candidato.
- O usuário pode pedir ao Claude que proponha uma estratégia para a próxima janela de execução (ex: "hoje foco em vagas internacionais", "esta semana só candidaturas no Brasil").
- O resultado desta conversa é persistido como um arquivo de estratégia que a sessão de execução lê no arranque. A camada de estratégia nunca executa candidaturas diretamente.

### 3.4 Terminal embutido

Um terminal funcional (não decorativo) construído com `xterm.js` no frontend e um pseudo-terminal real (`portable-pty`, via ConPTY no Windows) no backend Rust, ligado ao processo da sessão de execução. Começa bloqueado para input do usuário (modo apenas-visualização) e pode ser desbloqueado por um botão "assumir controle", momento em que o teclado do usuário passa a escrever diretamente no processo Claude Code subjacente. Detalhado na seção 8.

### 3.5 Estado partilhado (persistência)

SQLite como base de dados principal (histórico de vagas, candidaturas, pendências), mais um pequeno conjunto de arquivos de configuração em formato YAML ou JSON (`candidate_base.yaml`, `search_variants.yaml`, estratégia ativa). Detalhado na seção 5. É este estado — não a janela de contexto de nenhuma sessão — que carrega a memória do sistema entre execuções.

## 4. Fluxo de disparo da sessão de execução

Existem dois mecanismos de disparo, ambos invocando exatamente o mesmo caminho de código no Tauri — a única diferença é o que origina a chamada.

**Disparo manual.** O usuário clica em "procurar vagas agora" no dashboard. O Tauri invoca imediatamente a sessão de execução.

**Disparo automático por inatividade.** O Tauri monitoriza o tempo de inatividade do sistema operativo (via `GetLastInputInfo` no Windows, acessível através de uma crate Rust como `user-idle` ou chamada direta à Win32 API). Quando o tempo de inatividade excede um limiar configurável (sugestão de default: 15 minutos), e desde que ainda existam vagas no orçamento diário por processar, o Tauri invoca a sessão de execução automaticamente. Se o usuário retomar atividade enquanto a sessão está a correr, a sessão **não é interrompida** — o princípio de visibilidade já garante que o usuário vê o que está a acontecer e pode intervir manualmente se quiser parar.

Em ambos os casos, o passo seguinte é idêntico:

1. O Tauri monta o prompt de invocação, injetando: conteúdo de `candidate_base.yaml` e `search_variants.yaml`, caminho para o arquivo de estratégia ativa (se existir), e um resumo da memória recente (últimos N dias de atividade, lido da base de dados).
2. O Tauri spawna o processo `claude` com a flag de skip de permissões e a flag `--chrome` (que ativa a integração com a extensão Claude in Chrome via native messaging), ligado ao PTY do terminal embutido.
3. O terminal embutido começa a mostrar o output em tempo real; o Chrome abre como janela visível separada.
4. A sessão processa vagas até esgotar o orçamento do dia, encontrar um motivo de pausa total, ou decidir que deve encerrar e ser reaberta pelo Tauri (ver seção 7).
5. Ao terminar (por qualquer motivo), o Tauri atualiza o dashboard a partir do estado persistido.

Não existe um terceiro caminho de disparo (ex: cron job independente do Tauri). Isto é deliberado: manter o Tauri como único ponto de entrada simplifica a depuração e evita condições de corrida entre dois processos a tentar abrir sessões simultaneamente.

## 5. Perfil do candidato, estratégia e memória

Estes três artefactos são o contexto que transforma uma sessão Claude Code genérica numa sessão que representa este candidato específico, com esta estratégia específica, sabendo o que já foi feito.

### 5.1 Perfil do candidato (`candidate_base.yaml` + `search_variants.yaml`)

O modelo antigo de um único `profile.yaml` foi substituído. Existem dois arquivos com responsabilidades distintas. Nenhuma variante pode afirmar um fato que não seja rastreável a `candidate_base.yaml` (regra de honestidade da seção 2).

#### 5.1.1 `candidate_base.yaml` — banco de dados pessoal

Tudo o que é verdadeiro sobre o candidato, independente de qualquer candidatura ou variante. Editado pela aba Perfil (conversa Claude Code sem `--chrome`), nunca pela sessão de execução.

```yaml
dados_pessoais:
  nome_completo: ""
  email: ""
  telefone: ""
  localizacao_atual: ""
  links:
    - tipo: ""                   # "github" | "linkedin" | "portfolio" | "outro"
      url: ""

experiencia:
  - empresa: ""
    cargo: ""
    inicio: ""
    fim: ""                      # "" ou null se atual
    descricao: ""
    conquistas: []
    tecnologias: []

projetos:
  - nome: ""
    descricao: ""
    tecnologias: []
    url: ""
    origem: ""                   # "github" | "manual" | "linkedin"

formacao:
  - instituicao: ""
    curso: ""
    inicio: ""
    fim: ""

competencias:
  - ""

idiomas:
  - idioma: ""
    nivel: ""

gaps_conhecidos:
  - competencia: ""
    contexto: ""
    como_abordar: ""

respostas_modelo:
  porque_esta_vaga: ""
  pretensao_salarial_texto: ""
  notice_period: ""

ultima_atualizacao: ""            # ISO 8601
fontes_usadas:
  - tipo: ""                      # "github" | "linkedin" | "cv_existente" | "conversa"
    referencia: ""
    consultado_em: ""
```

#### 5.1.2 `search_variants.yaml` — variantes de busca/CV

Cada variante é uma combinação de área profissional e recorte geográfico/modelo de trabalho, com peso relativo, e gera o próprio CV a partir de `candidate_base.yaml`. Não há um único CV mestre; há um por variante (`cv_gerado_path`).

```yaml
variantes:
  - id: "backend"
    nome_exibicao: "Backend"
    peso: 50
    ativa: true
    foco_competencias: []
    foco_experiencia: []
    regioes_aceitas: []
    modelos_trabalho: []
    idiomas_aplicacao: []
    cv_gerado_path: ""
    cv_gerado_em: ""

preferencias_globais:
  faixa_salarial:
    minimo: null
    moeda: ""
    flexivel: false
  setores_evitar: []
  empresas_evitar: []

red_lines:
  - "pedido de salário fora da faixa definida"
  - "qualquer campo de dados sensíveis de saúde"
  - "perguntas abertas sobre motivação sem resposta-modelo no perfil"

perguntas_pendentes:
  - pergunta: ""
    origem_vaga: ""
    variante_relacionada: ""
    data: ""
```

A soma dos pesos das variantes ativas é normalizada no uso (peso / soma dos ativos). O algoritmo de distribuição diária por peso está fora de escopo; até existir, `peso` é só ordem de prioridade aproximada.

O campo `perguntas_pendentes` é o mecanismo de crescimento orgânico da seção 9 — a sessão de execução nunca escreve direto em `preferencias_globais` nem em `red_lines`; só propõe.

### 5.2 Estratégia ativa (`strategy.md`)

Texto livre, gerado em conversa na camada de estratégia, com uma estrutura leve sugerida (não imposta por schema):

```markdown
# Estratégia ativa

Definida em: <data>
Válida até: <data ou "até nova instrução">

## Foco
<ex: "Esta semana, priorizar vagas internacionais em inglês, full remoto.">

## Notas para a sessão de execução
<qualquer instrução específica, ex: "não aplicar a startups em estágio seed">
```

Se este arquivo não existir ou estiver vazio, a sessão de execução usa apenas o perfil como guia, sem foco temático adicional.

### 5.3 Memória de execução

Vive inteiramente na base de dados SQLite (ver seção 10), nunca na janela de contexto de uma sessão. No arranque de cada sessão de execução, o Tauri lê um resumo (não o histórico completo) e injeta-o no prompt: número de candidaturas dos últimos 7 dias, vagas puladas recentemente e motivo, pendências ainda não resolvidas pelo usuário. Isto é o que permite à sessão "lembrar-se" sem depender de continuidade de processo.

## 6. Regras de pausa

Esta é a seção mais importante do documento. O risco identificado pelo usuário — a IA "assumir tanto esse posto que fugisse do que foi definido", enviando uma candidatura errada ou fora do perfil — é mitigado inteiramente através destas regras, e elas devem ser tratadas como restrições rígidas no prompt de sistema da sessão de execução, não como sugestões que o modelo pode pesar contra outras considerações.

### 6.1 Categorias de pausa total (a sessão para e notifica imediatamente)

Estas condições interrompem o processamento da vaga atual e disparam notificação de alta prioridade. A sessão não tenta resolver, não adivinha, não avança para a vaga seguinte até o usuário responder a esta pendência especificamente — embora possa, em paralelo, continuar a trabalhar noutras vagas se a arquitetura de execução permitir (ver nota abaixo).

- **Qualquer campo de pretensão salarial fora da faixa definida em `preferencias_globais.faixa_salarial`**, ou qualquer campo salarial quando a faixa não está definida.
- **Qualquer campo listado em `red_lines`** no perfil do candidato, sem excepção.
- **Captcha, "prove que não é um robot", ou qualquer desafio de verificação humana.** A sessão nunca tenta contornar isto — apenas marca a vaga como bloqueada e notifica. Isto aplica-se a qualquer plataforma, incluindo o LinkedIn — a única diferença no LinkedIn é que o login em si é automatizado (credenciais geridas via keyring, seção 3.3 do prompt de construção), nunca causando pausa por si só; um captcha que apareça depois do login segue a regra geral. Nota técnica: esta é também a política nativa da extensão Claude in Chrome — ao encontrar uma página de login ou captcha, ela já pausa e espera intervenção manual, o que está alinhado com esta regra e não exige lógica adicional para a detetar.
- **Diálogo JavaScript bloqueante** (`alert`, `confirm`, `prompt`) na página. Estes diálogos bloqueiam todos os eventos do browser e a sessão não os consegue dispensar programaticamente — exigem fecho manual pelo usuário antes de a sessão poder continuar. Trata-se de uma limitação técnica da extensão, não de uma escolha de design, mas o efeito prático é o mesmo de uma pausa total: a sessão fica bloqueada até intervenção humana.
- **Pergunta aberta de formulário sem correspondência em `respostas_modelo`** e sem informação suficiente no perfil ou no CV da variante correspondente para responder com honestidade. Exemplo: "porque queres trabalhar aqui especificamente" quando não há nenhuma resposta-modelo equivalente.
- **Pedido de dados pessoais sensíveis** não cobertos pelo perfil (dados de saúde, informação familiar, número de identificação nacional, e equivalentes).
- **Qualquer situação em que a sessão precise de inventar informação** para preencher um campo obrigatório. Isto é absoluto: a honestidade definida na seção 2 não é uma preferência, é uma regra de bloqueio.

### 6.2 Categoria de pausa local (pula a vaga, regista, continua)

Estas condições não exigem intervenção imediata do usuário — a sessão regista a vaga como pendente de baixa prioridade ou simplesmente pulada, com o motivo explícito, e segue para a próxima vaga candidata.

- A vaga não atinge um limiar mínimo de match com o perfil (critério a refinar em conjunto com o usuário, mas nunca abaixo de "há pelo menos um must-have coberto").
- A vaga viola um critério de `setores_evitar` ou `empresas_evitar`.
- A plataforma onde a vaga está hospedada não é suportada pela navegação atual da sessão.

A distinção entre pausa total e pausa local é a peça central de design desta seção: pausa total protege o usuário de uma candidatura errada ou de uma decisão fora do perfil; pausa local mantém o sistema produtivo nos casos em que a resposta certa é simplesmente "não vale a pena, seguir em frente" — e não exige que o usuário seja interrompido por algo que não precisa da sua atenção.

### 6.3 O que nunca é decidido pela sessão de execução

Para eliminar ambiguidade, esta lista enumera explicitamente decisões que pertencem sempre ao usuário, nunca à sessão:

- Alterar a faixa salarial aceitável.
- Adicionar ou remover uma `red_line`.
- Decidir que um gap é aceitável quando o perfil não o classificou como tal previamente.
- Inventar ou aproximar uma resposta a uma pergunta de motivação ou fit cultural sem base no perfil.
- Prosseguir após um captcha ou desafio de verificação.

Qualquer situação que pareça exigir uma destas decisões é, por definição, uma pausa total.

### 6.4 Queda de ligação da extensão Chrome

A extensão Claude in Chrome usa um service worker que pode ficar inativo em sessões longas, quebrando a ligação entre a sessão de execução e o browser. Quando isto acontece, a sessão de execução deve primeiro tentar reconectar automaticamente (equivalente ao comando `/chrome` seguido de "Reconnect extension") antes de notificar o usuário — esta é uma falha técnica recuperável na maioria dos casos, não uma decisão que exige julgamento humano, e por isso não deve gerar uma notificação só por tentar. Se a tentativa de reconexão automática falhar, ou se falhar repetidamente dentro de um curto espaço de tempo, a situação é então tratada como pausa total e notificada normalmente, com a descrição a indicar explicitamente que se trata de uma falha de ligação, não de uma decisão sobre uma vaga.

### 6.5 Mecanismo de notificação

Quando ocorre uma pausa total, a sessão de execução escreve o estado de pendência na base de dados (tabela `pendencias`, ver seção 10) e o Tauri, através de um listener no estado partilhado, dispara uma notificação nativa do Windows (toast). Se a notificação não for atendida — definido como o usuário não abrir o dashboard nem interagir com a pendência — dentro de um intervalo configurável (sugestão: 5, 10, ou 15 minutos, à escolha do usuário nas configurações), a notificação repete-se. Isto continua indefinidamente até o usuário resolver a pendência ou cancelar explicitamente a vaga em questão.

## 7. Etiqueta de plataforma e granularidade de sessão

### 7.1 Comportamento de ritmo

Como a navegação é feita pela própria sessão Claude Code através da extensão Claude in Chrome e não por um script Playwright determinístico, a etiqueta de plataforma é implementada como instrução de comportamento no prompt de sistema, não como lógica de código com `sleep()` calculados. O prompt de sistema da sessão de execução deve instruir explicitamente:

- Introduzir pausas de leitura entre ações (ex: "antes de clicar num botão, considera o tempo que uma pessoa levaria a ler o conteúdo da página").
- Não preencher campos de formulário instantaneamente em sequência; preencher um, fazer uma pequena pausa, preencher o seguinte.
- Fazer scroll pela página antes de interagir com elementos fora do viewport inicial, como uma pessoa faria.
- Limitar o número de candidaturas processadas por hora, distribuindo o trabalho do dia em vez de processar tudo em sequência rápida.
- Nunca tentar resolver ou contornar um desafio de verificação humana (ver seção 6.1) — isto é tratado como pausa total, não como obstáculo a superar.

Estas instruções não garantem indetetabilidade — são orientação de comportamento para um agente que raciocina, não controle mecânico — mas alinham-se com o princípio de que um sistema bem comportado é, por si, um sistema de melhor qualidade, independentemente de considerações de deteção.

### 7.2 Diversidade de fontes

A sessão de execução deve distribuir a descoberta de vagas entre múltiplas fontes (job boards dedicados, boards de empresas, LinkedIn) em vez de concentrar todo o volume diário numa única plataforma. Job boards com termos de uso mais permissivos para automação devem ser preferidos como fonte primária; o LinkedIn é tratado com o ritmo mais conservador de todas as fontes, dado ser historicamente a plataforma mais agressiva na deteção de automação.

### 7.3 Granularidade da sessão

Em vez de um número fixo de vagas por sessão, a sessão de execução avalia a complexidade de cada vaga e decide, vaga a vaga, se deve continuar no mesmo processo ou sinalizar ao Tauri que prefere ser reiniciada antes da próxima. O sinal técnico para isto é um marcador específico escrito no estado partilhado (ex: linha `SESSION_CHECKPOINT_REQUESTED` ou equivalente lido pelo Tauri através do stream de output do PTY) — ao detetá-lo, o Tauri encerra o processo atual de forma limpa e invoca um novo, que retoma a partir do estado persistido (não da janela de contexto anterior).

Casos típicos em que isto é esperado:

- Vagas com Easy Apply simples (poucos campos, sem upload de documentos customizados): múltiplas vagas processadas confortavelmente na mesma sessão.
- Vagas que exigem navegação extensa num site de terceiro, preenchimento de formulário longo, ou múltiplas etapas: a sessão pode optar por processar essa vaga isoladamente e pedir reinício a seguir, para manter o seu próprio raciocínio focado e a janela de contexto limpa.

Esta decisão pertence ao modelo, não a uma regra fixa de código — é precisamente o tipo de julgamento que motivou a escolha de Claude Code como motor em vez de Playwright scriptado.

## 8. Terminal embutido

### 8.1 Objetivo

Substituir a janela de terminal padrão do sistema operativo (que abriria ao spawnar o processo `claude`) por uma vista integrada na interface do Tauri, com aparência consistente com o resto da aplicação, mantendo a fidelidade total de um terminal real — incluindo a capacidade de o usuário assumir controle direto da sessão quando necessário.

### 8.2 Stack técnica

- **Backend (Rust):** crate `portable-pty`, que usa ConPTY no Windows (API moderna de pseudo-terminal, disponível desde o Windows 10), para criar o pseudo-terminal e spawnar o processo `claude` dentro dele.
- **Frontend (webview):** `xterm.js`, com o addon `xterm-addon-fit` para ajuste automático ao tamanho do painel, e um tema customizado (paleta de cores, tipografia) definido no objeto de configuração do terminal para substituir a aparência padrão de terminal preto.
- **Comunicação:** eventos Tauri (`emit`/`listen`) para transportar cada chunk de output do PTY para o frontend; comandos Tauri (`invoke`) para o frontend enviar input de volta ao PTY quando desbloqueado.

### 8.3 Estado de bloqueio

O terminal arranca sempre em modo bloqueado (`locked: true`). Neste modo:

- O listener `onData` do xterm.js, que captura cada tecla premida pelo usuário, não envia nada ao backend.
- Qualquer input automático necessário (ex: confirmações que o próprio Tauri já sabe responder por regra, se existirem) é enviado exclusivamente pelo orquestrador, nunca pelo usuário.

Um botão "assumir controle", visível permanentemente no painel do terminal, alterna `locked` para `false`. A partir desse momento, o teclado do usuário escreve diretamente no PTY, tal como um terminal normal. Quando o usuário desbloqueia a sessão, o orquestrador do Tauri deve parar de enviar qualquer input automático, para evitar colisão entre os dois canais de escrita. Um indicador visual claro (ex: borda colorida do painel, etiqueta de estado) deve sempre comunicar se o terminal está bloqueado ou sob controle do usuário.

### 8.4 Validação incremental

Antes de integrar a complexidade da sessão de candidaturas completa, a integração do terminal deve ser validada isoladamente: spawnar um comando simples (`claude --version`, ou mesmo `cmd /c dir`) através do PTY e confirmar que o output aparece corretamente no `xterm.js`, sem problemas de encoding ou de buffering. Só depois disso a sessão real de execução deve ser ligada a este mesmo caminho.

## 9. Crescimento orgânico do perfil

O perfil do candidato (seção 5.1) não é estático. O sistema deve ficar progressivamente mais capaz de operar com autonomia dentro do que o usuário realmente quer, sem nunca decidir isso sozinho.

### 9.1 Mecanismo

Sempre que a sessão de execução encontra uma situação que resulta em pausa total (seção 6.1) e a resolução, depois de o usuário intervir, revela uma preferência ou regra que não estava no perfil, a sessão registra uma entrada em `perguntas_pendentes` em `search_variants.yaml` — nunca escreve direto em `preferencias_globais` nem em `red_lines`. Exemplo: se o usuário resolve manualmente uma pausa sobre disponibilidade para viajar e essa pergunta tende a se repetir, a sessão propõe uma entrada nova em `perguntas_pendentes` em vez de assumir uma resposta-padrão.

### 9.2 Apresentação ao usuário

As propostas acumuladas em `perguntas_pendentes` não exigem resposta diária. Ao iniciar uma sessão de execução (ou ao abrir o dashboard), se existirem propostas acumuladas, o Tauri apresenta-as de forma discreta (ex: um contador ou badge no dashboard, "3 perguntas novas para refinar o perfil"), sem bloquear o início da sessão. O usuário decide quando quer entrar na camada de estratégia para as resolver, atualizando o perfil de forma deliberada.

### 9.3 Garantia de não-regressão

Este mecanismo nunca deve resultar em o sistema tornar-se mais permissivo sem intervenção explícita do usuário. Uma proposta em `perguntas_pendentes` é apenas uma sugestão até ser promovida manualmente — pelo usuário, na camada de estratégia — a uma entrada real em `preferencias_globais`, `red_lines`, ou `respostas_modelo`.

## 10. Schema de dados (SQLite)

```sql
-- Vagas descobertas, independentemente do que aconteceu a elas
CREATE TABLE vagas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    titulo TEXT NOT NULL,
    empresa TEXT NOT NULL,
    plataforma TEXT NOT NULL,           -- 'linkedin', 'jobindex', 'indeed', 'company_board', etc.
    url TEXT NOT NULL UNIQUE,
    localizacao TEXT,
    modelo_trabalho TEXT,               -- 'remoto', 'hibrido', 'presencial'
    idioma TEXT,
    descoberta_em TEXT NOT NULL,        -- ISO 8601
    status TEXT NOT NULL DEFAULT 'descoberta',
        -- valores: 'descoberta', 'analisada', 'candidatando',
        --          'aplicada', 'pulada', 'pendente_revisao', 'bloqueada'
    motivo_status TEXT,                 -- preenchido quando pulada/pendente/bloqueada
    match_score TEXT                    -- resumo textual do match (must-haves/nice-to-haves)
    , variante_id TEXT                 -- id da variante em search_variants.yaml, se associada
);

-- Candidaturas efetivamente enviadas
CREATE TABLE candidaturas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER NOT NULL REFERENCES vagas(id),
    enviada_em TEXT NOT NULL,
    pasta_arquivos TEXT NOT NULL,        -- caminho para CV, carta, etc. gerados
    metodo TEXT NOT NULL                 -- 'easy_apply', 'formulario_manual', 'email_outreach'
);

-- Pendências que aguardam decisão do usuário
CREATE TABLE pendencias (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER NOT NULL REFERENCES vagas(id),
    criada_em TEXT NOT NULL,
    categoria TEXT NOT NULL,             -- corresponde às categorias da seção 6.1
    descricao TEXT NOT NULL,             -- detalhe legível do que travou
    resolvida BOOLEAN NOT NULL DEFAULT 0,
    resolvida_em TEXT,
    resolucao TEXT                       -- o que o usuário decidiu
);

-- Propostas de evolução do perfil (seção 9)
CREATE TABLE propostas_perfil (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER REFERENCES vagas(id),
    criada_em TEXT NOT NULL,
    pergunta TEXT NOT NULL,
    contexto TEXT,
    promovida BOOLEAN NOT NULL DEFAULT 0,
    promovida_em TEXT
);

-- Log de sessões de execução, para o resumo de memória injetado no prompt
CREATE TABLE sessoes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    iniciada_em TEXT NOT NULL,
    terminada_em TEXT,
    motivo_disparo TEXT NOT NULL,        -- 'manual', 'inatividade'
    motivo_termino TEXT,                 -- 'orcamento_esgotado', 'pausa_total', 'checkpoint_solicitado', 'erro'
    vagas_processadas INTEGER DEFAULT 0
);
```

Este schema deve ser tratado como ponto de partida, não como definição congelada — a fase de implementação pode revelar necessidade de campos adicionais (ex: índices para performance, ou uma tabela de log de gaps equivalente ao `_gaps.jsonl` da skill original, ver seção 11).

## 11. Relação com a skill `tailor-application` original

A skill original (análise de posting, match assessment, geração de CV/carta/outreach/talking points/study guide, convenções de nomenclatura de arquivos, estilo de carta dinamarquesa, registo de gaps em `_gaps.jsonl`) não é substituída por este sistema — é absorvida como a lógica de conteúdo que a sessão de execução invoca para cada vaga individual. A diferença está inteiramente naquilo que envolve essa lógica:

- A skill original assume que o usuário traz a vaga; este sistema acrescenta a camada de descoberta autónoma.
- A skill original produz os arquivos e reporta de volta ao usuário em texto; este sistema acrescenta a decisão de quando avançar automaticamente para a candidatura versus pausar.
- O registo de gaps em `_gaps.jsonl`, pensado para alimentar um futuro `/study-plan`, mantém-se exatamente como está — é complementar, não conflitante, com a tabela `propostas_perfil` deste sistema, que serve um propósito diferente (evolução das preferências do candidato, não acompanhamento de lacunas de competência).
- As regras de honestidade, a estrutura de pastas por empresa, e a separação entre arquivos para o empregador (nome do candidato) e arquivos internos (nomes descritivos) aplicam-se sem alteração.

Em suma: este documento descreve a máquina que decide quando e como invocar a skill; não substitui o que a skill já faz bem.

