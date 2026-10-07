# Issues do fechamento de `feat/browser-cdp-mcp` no Linear

Criadas em 2026-10-07. Os ids estão entre colchetes.

Criar com a skill `linear-claudia`. Depois de criar, anotar o id ao lado de cada item.

1. **[CLA-53] Dados falsos completos para teste (cenário mock do Sandbox).** Gerador determinístico, versionado fora do Sandbox (o Sandbox só recebe cópia), cobrindo todas as tabelas (vagas em todos os status, candidaturas, pendências de todas as categorias incluindo `sem_curriculo`/`sem_carta`, documentos gerados e importados, preferidos, anexos, migrados, sessões, Activity log), currículos e cartas antigos em HTML para a migração, PDFs e DOCX, variantes com principal e inativa, campos desconhecidos no YAML e casos de borda (acentos, nomes longos, banco grande, datas antigas). Pedido do usuário: "esse banco falso seja bem cheio mesmo, com todo tipo de dado pq vai ser para um uso geral". Branch `feat/dados-falsos-teste`, spec `docs/superpowers/specs/2026-10-07-dados-falsos-teste-design.md` (a escrever). Bloqueia a Tarefa 5 do plano de fechamento.
2. **[CLA-54] Tirar os caminhos absolutos de máquina do repositório.** Em 2026-10-07 havia uns 41 arquivos com `C:Usersjoaog` (`git grep -l "C:[\/]Users[\/]joaog"`): código, testes, skills, protótipo, docs, planos. Trocar por variáveis, caminhos relativos ou APIs. A regra está no `CLAUDE.md` (Regras duras).
3. **[CLA-55] Renomear os parâmetros das tools do MCP para nomes uniformes** (`id`, `variante_id`, `documento`), em branch curta depois do merge de `feat/browser-cdp-mcp`. O erro diz o nome certo e de onde tirar o valor. Não é refazer o MCP.
4. **[CLA-56] Evals das skills de teste (`rodar-evals.mjs`) rodam `claude -p` com as permissões do usuário.** Rodar em modo `plan`, sem ferramentas de escrita, e matar a árvore de processos. Só manual (B6 da auditoria).
5. **[CLA-52] Urgent: nunca mais perder trabalho.** O incidente de 2026-10-05 (12 h perdidas), os 158 commits sem `dev`, o `dev` 55 commits à frente do `origin`, a evidência guardada em pasta ignorada pelo git e o CI que não roda front, clippy nem live. Solução depois de `feat/browser-cdp-mcp`, com spec aprovada; a primeira proposta foi recusada por ser burocrática.
6. **[CLA-57] CI.** Roda só `cargo test --lib` e `npm run build`. Os testes que apagam vão para o Sandbox, então o CI precisa decidir o que roda onde.

7. **[CLA-58] Migração da pasta antiga (`com.joaog.claudia-rh`) apaga subpastas que não copiou.** `migration.rs`: o app confere só o banco e os YAML antes de apagar a pasta antiga. Quem atualizar de uma versão antiga com `curriculos\` lá perde esses arquivos. Também aceita caminho relativo. Corrigir: só apagar se tudo foi copiado. (P2 da auditoria; a máquina do usuário não tem a pasta antiga.)
8. **[CLA-59] `app_data_dir().unwrap_or_default()` em `feedback.rs`, `perfil.rs` (2x) e `cover_letter.rs`.** Hoje só leem; se um dia gravarem ou apagarem, o caminho vazio vira a pasta atual. Trocar por erro explícito. (P4 da auditoria.)

As demais linhas "L" do inventário do plano entram aqui quando a Tarefa 10 rodar.
