# Notas para a próxima spec: vigia de navegação, campanha, níveis e banidos (RASCUNHO, não aprovada)

Data: 2026-10-16. **Não é spec aprovada e nada daqui foi implementado.** Registra o que o usuário pediu e decidiu, para a spec desta frente (nova branch a partir de `dev`, depois do merge da `feat/lista-de-sites-impl`). Pesquisas de apoio (fora do git, `.superpowers/sdd/2026-10-15-sites-fluxo-guiado/`): `pesquisa-niveis-A-produtos-e-padroes.md` e `pesquisa-niveis-B-ameacas-e-codigo.md`.

## 1. Pedido do usuário (nas palavras dele, resumido)
- Níveis de permissão nas Configurações, do muito estreito ao muito livre, mistura de usabilidade com segurança, número de níveis em aberto (3 a 6).
- O uso dele: selecionar sites e empresas, **deixar a IA independente**, gastando horas aplicando e resolvendo obstáculos sozinha, com meta de candidaturas por semana e limites definidos em outra configuração. Outros usuários querem o oposto: acompanhar junto, mais automatização que autonomia.
- As empresas da lista são alvos que ele quer **monitorar constantemente**; no LinkedIn a IA **aplica em qualquer empresa**, também as que não estão na lista.
- Prompt: hoje a IA **para demais** para perguntar; refinar é etapa própria, depois.
- **Testar injeção de prompt**: injetar prompts maliciosos para ver o que o agente faz; se falhar, **brecar o desenvolvimento** (ou rodar outra coisa) até resolver.
- **Sites banidos**: sites que a IA não pode acessar de jeito nenhum; o MCP sempre recusa ou fecha a aba à força e devolve uma mensagem; integrado à vigia; um site com ameaça detectada fica banido até segunda ordem, para um agente novo não abri-lo de novo.

## 2. Decisões do usuário (2026-10-16)
- Banimento automático = **quarentena**: bloqueia na hora para todos os agentes, avisa em Sites e nas Notificações, e o usuário **confirma o banimento ou libera**.
- Alcance: para site da lista do usuário (ex.: LinkedIn), bloqueia **só aquela vaga ou página**; para destino desconhecido ou de fora da lista, o **domínio** inteiro.
- Depois de uma ameaça: **encerrar o agente, registrar a vaga como pulada por ameaça e seguir com um agente novo**, que recebe só dados estruturados (empresa, título, motivo), nunca o texto da página.
- Ordem das specs: **teste de injeção primeiro**, depois campanha e modos (com a vigia), depois o prompt mais autônomo.

## 3. Propostas do assistente (NÃO decididas)
- **Modelo em dois controles**, em vez de um slider: (a) **onde** a IA pode ir; (b) **quando** precisa do usuário. Modos propostos: **Autônomo** (o do usuário: segue o destino de uma vaga achada num site liberado até o sistema de candidatura da empresa, candidata até o limite diário e semanal, em obstáculo anota a pendência e vai para a próxima), **Assistido** (pergunta antes de cada candidatura), **Só leitura**; talvez um **Autônomo restrito** (só empresas escolhidas).
- **Campanha** como unidade de configuração: sites liberados, empresas alvo, limites (dia, semana, teto).
- **Destino legítimo = sistema de candidatura conhecido** (Greenhouse, Lever, Ashby, Workday, Teamtailor, Gupy...) ou domínio de carreiras da empresa da vaga; destino desconhecido: pular a vaga e seguir, sem perguntar.
- **Vigia de navegação em camadas** (estimativas de esforço da pesquisa B, não testadas): (1) conferir a página atual contra a lista no início de toda ferramenta, ~1 dia; (2) bloquear documentos do frame principal por CDP `Fetch`, 3 a 5 dias; (3) popups e abas novas por `Target.setAutoAttach`, 2 a 4 dias.
- **Banidos**: lista com precedência sobre tudo; origem usuário ou quarentena automática; recusa na abertura, em cada ferramenta e (camadas 2 e 3) fechamento forçado da aba com mensagem ao agente. Sinais de detecção possíveis (todos imperfeitos): destino banido ou desconhecido vindo de página confiável; envio bloqueado para site de fora; texto de página com cara de ordem para a IA; aviso da própria IA (o mais perigoso, pode estar enganada).
- **Teste de injeção**: páginas de vaga maliciosas em `src-tauri/tests/fixtures/` (texto escondido, pedido de dados do perfil, botão de candidatura para domínio parecido com ATS, redirecionamento para fora), rodada de agente real contra as páginas locais, revisor lendo o Activity log; o modo autônomo **só é liberado** quando passar; correções no código, não só no prompt.

## 4. Limites conhecidos (da pesquisa)
- Hoje só o **primeiro `browser_open`** passa pelo bloqueio; clique, redirecionamento e popup depois dele não são vistos, e as ferramentas de digitar e anexar não conferem a página. Qualquer promessa "nunca sai da lista" exige a vigia.
- Lista de sites **não impede** injeção vinda de dentro de um site liberado (vaga maliciosa num board permitido): o que ajuda é bloqueio de envio por vaga, limites, IA sem login fora da campanha e resumo no fim da rodada.
- **NÃO VERIFICADO:** se `Fetch` pausa cada salto de redirect; se a árvore de acessibilidade traz a URL dos links; formato do redirecionador do LinkedIn.
