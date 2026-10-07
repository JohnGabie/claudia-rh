# Issues a criar no Linear (o MCP do Linear está sem autenticação nesta sessão)

Criar com a skill `linear-claudia`. Depois de criar, anotar o id ao lado de cada item.

1. **Dados falsos completos para teste (cenário mock do Sandbox).** Gerador determinístico, versionado fora do Sandbox (o Sandbox só recebe cópia), cobrindo todas as tabelas (vagas em todos os status, candidaturas, pendências de todas as categorias incluindo `sem_curriculo`/`sem_carta`, documentos gerados e importados, preferidos, anexos, migrados, sessões, Activity log), currículos e cartas antigos em HTML para a migração, PDFs e DOCX, variantes com principal e inativa, campos desconhecidos no YAML e casos de borda (acentos, nomes longos, banco grande, datas antigas). Pedido do usuário: "esse banco falso seja bem cheio mesmo, com todo tipo de dado pq vai ser para um uso geral". Branch `feat/dados-falsos-teste`, spec `docs/superpowers/specs/2026-10-07-dados-falsos-teste-design.md` (a escrever). Bloqueia a Tarefa 5 do plano de fechamento.
2. **Tirar os caminhos absolutos de máquina do repositório.** Em 2026-10-07 havia uns 41 arquivos com `C:Usersjoaog` (`git grep -l "C:[\/]Users[\/]joaog"`): código, testes, skills, protótipo, docs, planos. Trocar por variáveis, caminhos relativos ou APIs. A regra está no `CLAUDE.md` (Regras duras).
3. **Renomear os parâmetros das tools do MCP para nomes uniformes** (`id`, `variante_id`, `documento`), em branch curta depois do merge de `feat/browser-cdp-mcp`. O erro diz o nome certo e de onde tirar o valor. Não é refazer o MCP.
4. **Evals das skills de teste (`rodar-evals.mjs`) rodam `claude -p` com as permissões do usuário.** Rodar em modo `plan`, sem ferramentas de escrita, e matar a árvore de processos. Só manual (B6 da auditoria).
5. **Urgent: nunca mais perder trabalho.** O incidente de 2026-10-05 (12 h perdidas), os 158 commits sem `dev`, o `dev` 55 commits à frente do `origin`, a evidência guardada em pasta ignorada pelo git e o CI que não roda front, clippy nem live. Solução depois de `feat/browser-cdp-mcp`, com spec aprovada; a primeira proposta foi recusada por ser burocrática.
6. **CI.** Roda só `cargo test --lib` e `npm run build`. Os testes que apagam vão para o Sandbox, então o CI precisa decidir o que roda onde.

As demais linhas "L" do inventário do plano entram aqui quando a Tarefa 10 rodar.
