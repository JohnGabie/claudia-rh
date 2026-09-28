# Coexistência dos dois servidores MCP

**Data:** 2026-09-28
**Config usada:** `mcp-config-teste.json` (cópia do que o app gera + bloco `playwright`)

| Teste | Resultado |
|---|---|
| Tools do `claudia` visíveis | **sim, 14** |
| Tools do `playwright` visíveis | **sim, 25** |
| `get_candidate_profile` responde | **sim** — devolveu "John" |

## As 14 tools do claudia, listadas pela sessão

```
close_pendencia          get_pendencia_vaga       register_candidatura
close_pendencias_vaga    get_search_variants      register_vaga
create_pendencia         get_strategy             update_profile
get_candidate_profile    list_pendencias          update_vaga_status
get_memory_summary       propose_profile_change
```

`propose_profile_change` aparece na lista — é a tool adicionada na CLA-11 e
confirmada em produção a 25/09.

## Como foi testado

Uma sessão `claude --mcp-config ./mcp-config-teste.json --strict-mcp-config`, sem
`--dangerously-skip-permissions`. Os dois servidores arrancaram e responderam.

**Nenhum arquivo do app foi modificado.** A config é passada à mão a uma sessão de
teste. O app monta a sua própria config em tempo de execução (`commands/perfil.rs:447`);
acrescentar o bloco `playwright` lá é uma mudança de poucas linhas e pertence à Fase 2.

## Uma descoberta sobre permissões

A primeira tentativa de chamar `get_candidate_profile` **foi bloqueada** — não pelo
servidor, mas pelo modo de permissões do Claude Code: a tool não estava na allowlist e
foi negada automaticamente.

Isso importa para a Fase 2 e para a **CLA-30** (confinar a sessão). Hoje o app resolve
isto passando `--dangerously-skip-permissions`, que liberta tudo. O teste mostra que
existe o meio-termo: `--allowedTools "mcp__claudia__get_candidate_profile"` libertou
exatamente uma tool e nada mais.

**É o mecanismo que a CLA-30 procura.** Confinar a sessão não precisa de infraestrutura
nova — precisa de trocar uma flag que liberta tudo por uma lista do que é preciso.

## Resposta

**A Fase 2 tem caminho.** Os dois servidores coexistem sem interferência, o do app
continua funcional, e a mudança necessária é acrescentar um bloco à config que o app
já escreve.
