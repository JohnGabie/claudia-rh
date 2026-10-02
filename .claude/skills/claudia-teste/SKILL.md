---
name: claudia-teste
description: Vê e controla o Claudia RH (Tauri) rodando, por CDP no WebView2, para testar frontend e backend e observar o que os agentes fazem dentro dele. Use SEMPRE que o usuário pedir para testar o app rodando, ver uma tela, conferir se a UI bate com o banco, chamar um comando Tauri, reproduzir um bug, ou reclamar do layout ("não gostei do X", "o bug tá acontecendo", "tá estranho aqui"). Use também antes de dar por concluída qualquer mudança em UI ou em comandos Tauri, e quando um bug for citado no código — reproduzir antes de propor correção. NÃO use para cargo test puro, automação de sites de vagas, nem registrar no Linear.
---

# claudia-teste

Dá a quem desenvolve o olhar de um dev sobre o app rodando. **Nunca interage com sites de vagas** e nunca clica em algo que submeta candidatura: quem interage com sites são os agentes dentro da Claudia.

Todos os scripts ficam em `.claude/skills/claudia-teste/scripts/` e rodam com Node ≥ 22, sem instalar nada.

## Fluxo

1. **Conectar:** `node scripts/subir.mjs`. Devolve JSON com `acao`: `subiu`, `reutilizar`, `perguntar` ou `abortar`. Se `perguntar`, o app já roda sem CDP: peça ao usuário antes de reiniciar.
2. **Ver:** `node scripts/ver.mjs arvore | tela [arq] | clicar "<rótulo>"`.
3. **Backend:** `node scripts/invocar.mjs <comando> [json]` e `node scripts/banco.mjs "SELECT …"`. Compare o que a UI mostra com o banco.
4. **Observar agentes:** `node scripts/observar.mjs`. Iniciar uma sessão é apertar o botão na UI do app, nunca rodar um agente por fora.

Defina `CDP_PORT` com a porta devolvida por `subir.mjs` (padrão 9333).

## Modo reconhecimento

Quando a queixa é vaga, não suponha. Conecte, abra a tela citada, leia a árvore, tire screenshot e **descreva o que vê**. Só depois pergunte ou proponha. Nesse modo nada é alterado. Se o bug depende de dados ou sequência, pergunte como chegar lá.

## Bug citado

Primeiro reproduza no app rodando. Só feche depois de reproduzido antes e confirmado depois.

## O que nunca fazer

- Escrever direto no banco ou nos YAMLs do perfil. Escrita só por `invocar.mjs --escrever`, avisando antes.
- Editar `prompt_sistema_runtime.md` ou o `identifier` Tauri.
- Reiniciar o app do usuário sem perguntar, ou matar processo que não subiu.
- Clicar em rótulo destrutivo, de envio, ou sem nome (o `ver.mjs` recusa).
- Dar teste por passado porque o `cargo test` ficou verde.

## Saída

Cada teste devolve: o que foi feito, o que foi visto (árvore ou screenshot) e o veredito, com evidência.

## Mais detalhe

- `references/limites.md` — o que foi medido e o que não funciona
- `references/padroes-cdp.md` — armadilhas
- `references/comandos.md` — comandos Tauri (gerado por `scripts/gerar-comandos.mjs`)
