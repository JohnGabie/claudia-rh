# Spec: skill `claudia-teste`

> Spec de design. Não é o plano de implementação.

**Data:** 2026-10-01
**Branch:** `feat/skill-claudia-teste` (a partir de `dev`)
**Origem:** conversa de 2026-10-01 sobre não conseguir ver nem controlar a janela do app Tauri

## 1. Problema

Quem desenvolve o Claudia RH com um agente de IA não consegue ver o app rodando. As provas de que algo funciona são `cargo test`, `clippy` e `tsc`, e nenhum deles mostra a tela.

Casos concretos:

- **CLA-25:** um comentário em `handleModoDragEnd` diz "sem backend por agora"; os sliders mexem, a escolha nunca chega ao agente. Só apareceu porque o usuário usou a tela.
- **CLA-11:** fechada com 128 testes verdes. O badge de notificações foi conferido pelo banco, não pelo badge.
- **`sessao.rs` sem compilar:** o app morreu e a causa só foi notada pelo processo ausente, horas depois.

O código fica mais propenso a bug porque o ciclo de desenvolvimento não tem um momento de ver o resultado.

## 2. O que foi verificado

Em 2026-10-01, nesta máquina:

- O WebView2 do Tauri aceita `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta>` e abre CDP em ~6 s (`Edg/154`, protocolo 1.3), na página `localhost:1420`.
- Com o cliente de `prototipo-playwright/cdp/cdp.mjs` foi possível ler a árvore de acessibilidade (23 nós úteis, ~132 tokens no Dashboard), tirar screenshot (1100×700) e chamar um comando Rust: `__TAURI_INTERNALS__.invoke("obter_modo_autonomo")` devolveu `true`.
- Duas armadilhas ao subir: a porta 9222 já era do Chrome pessoal do usuário, e um vite antigo segurava a 1420.

**Não verificado:** clique na UI do app, e leitura da aba Terminal (como ela renderiza o PTY).

Trocar para Electron foi descartado: o ganho (CDP oficial) é o mesmo que a flag do WebView2 já dá, e o custo é reescrever ~9,5 mil linhas de Rust, perder o `identifier` congelado e multiplicar o instalador.

## 3. Objetivo

Uma skill que dá ao dev a visão e o controle de um dev sobre o app rodando, e a leitura do que os agentes fazem lá dentro. Ela entra como passo obrigatório do ciclo de desenvolvimento.

**Nunca interage com sites de vagas.** Quem interage com o LinkedIn são só os agentes dentro da Claudia.

## 4. Desenho

Skill única, `claudia-rh/.claude/skills/claudia-teste/`, ao lado de `linear-claudia`.

```
SKILL.md                   curto: quando usar, fluxo, o que nunca fazer
scripts/
  cdp.mjs                  cliente WebSocket, zero dependências (de prototipo-playwright/cdp/)
  subir.mjs                tauri dev com CDP, valida a porta, libera a 1420
  ver.mjs                  árvore, screenshot, recorte, clique por rótulo
  invocar.mjs              invoke("comando", args) e retorno
  banco.mjs                consultas somente leitura em claudia_rh.db
  observar.mjs             terminal da sessão, log, tabelas de sessão
references/
  padroes-cdp.md           armadilhas
  comandos.md              lista dos comandos Tauri, gerada por grep
```

### 4.1 Gatilho

Dispara em:

- pedido de teste: "testa o Dashboard rodando", "o badge bate com o banco?", "chama o comando X";
- **bug ou problema citado no código**: o primeiro movimento é reproduzir no app rodando, antes de propor correção;
- **reconhecimento**: "não gostei do layout do Perfil", "o bug está acontecendo", "tá estranho aqui". Mesmo sem a palavra "testar";
- observar agentes: "o agente parou onde devia?", "compara Claude e Grok".

Não dispara em: `cargo test` puro, automação de LinkedIn, registro no Linear, explicação de código sem tela para ver.

### 4.2 Quatro capacidades

**a) Conectar.** Sobe `tauri dev` com a flag numa porta livre. Antes de assumir sucesso, confirma que a porta é do app: `/json/version` com `Edg/…` e página `localhost:1420`. Libera a 1420 se um vite antigo a segura, matando só esse processo. Se o app já roda sem CDP, avisa e pergunta antes de reiniciar.

**b) Ver o frontend.** Árvore de acessibilidade, screenshot, recorte e clique por rótulo. Segue o padrão validado: ativar a aba, `scrollIntoViewIfNeeded`, `elementFromPoint` antes de clique por coordenada, reler a árvore para confirmar a mudança.

**c) Exercitar o backend.** Chama comandos pela janela com `invoke`. Compara o retorno com o banco e os arquivos de dados, para pegar divergência entre o que a UI mostra e o que o backend sabe. Comandos que escrevem só rodam se o teste pedir, com aviso prévio.

**d) Observar agentes.** Lê o terminal da sessão, o log e as tabelas `sessoes`, `vagas`, `pendencias`, e compara com o comportamento esperado declarado pelo teste. Disparar sessão é apertar o botão de iniciar na UI da Claudia, nunca rodar um agente por fora. Comparar Claude e Grok depende da CLA-31; até lá, só observa o que o app permite.

### 4.3 Modo reconhecimento

Quando a queixa é vaga, a skill conecta, abre a tela citada, lê a árvore, tira screenshot e **descreve o que vê** antes de supor. Só depois pergunta ou propõe. Nesse modo nada é alterado.

A saída é screenshot, o que a árvore diz, e uma hipótese rotulada como hipótese. Se o bug depende de dados ou sequência específica, pergunta como chegar lá em vez de adivinhar.

### 4.4 Saída padrão

Cada teste devolve: o que foi feito, o que foi visto (árvore ou screenshot) e o veredito, com evidência. É o que entra no passo de teste do ciclo.

## 5. Mudança no ciclo de desenvolvimento

O `claudia-rh/CLAUDE.md` passa a dizer:

> spec → plano → branch → implementação → **teste no app rodando (`claudia-teste`)** → review → merge

Regras do passo:

- roda o app de verdade (`tauri dev` com CDP) e exercita o que mudou; `cargo test` e `tsc` não bastam;
- o resultado entra no ledger ou na descrição do PR, com evidência;
- mudança só de backend, sem efeito visível: chama o comando por `invoke` e confere o banco;
- bug corrigido só fecha depois de reproduzido antes e confirmado depois.

**Esta alteração do `CLAUDE.md` só é feita na implementação**, depois da spec aprovada.

## 6. O que a skill nunca faz

- Automatizar LinkedIn ou qualquer site de vagas, ou clicar em algo que submeta candidatura.
- Escrever direto no banco ou nos YAMLs do perfil. Escrita só por `invoke`.
- Editar `prompt_sistema_runtime.md` ou o `identifier` Tauri.
- Reiniciar o app do usuário sem perguntar, ou matar processo que não subiu.
- Deixar a porta de debug fora do `tauri dev`. A flag vai só em build de debug, nunca no instalador, o que respeita a regra de nenhum passo novo para o usuário.
- Dar teste por passado porque o `cargo test` ficou verde.

## 7. Evals

Para o `skill-creator`, ajustar a descrição e medir o gatilho.

**Devem disparar:** "testa o Dashboard rodando", "o badge de notificações bate com o banco?", "tem um bug no Perfil, reproduz", "chama o comando X e vê o retorno", "o agente parou onde devia?", "compara Claude e Grok", "não gostei do layout do Dashboard", "o bug do Perfil tá acontecendo de novo", "olha o que tá errado nessa tela", "só mexi no CSS" (teste visual), "corrige este bug" (primeiro reproduzir).

**Não devem disparar:** "roda `cargo test`", "candidata nessa vaga do LinkedIn", "registra uma issue no Linear", "explica o `pty_manager.rs`", "qual a cor do tema?".

## 8. Verificação antes de dar por pronta

Contra casos reais:

1. **Conectar:** subir o app com CDP e provar que a porta é do app.
2. **Ver:** ler o Dashboard e clicar em Notifications, confirmando a mudança pela árvore.
3. **Backend:** chamar `obter_modo_autonomo` e conferir com `modo_autonomo.json`.
4. **Bug real:** reproduzir a CLA-25 pela UI, sem depender dos olhos do usuário. É o teste que mostra se a skill fecha o buraco que motivou tudo.
5. **Observar:** abrir a aba Terminal e ler a saída.

## 9. Riscos e limites

- **`observar.mjs` é o item mais incerto.** Ler o terminal por CDP depende de como a aba renderiza o PTY, e isso não foi testado. Se falhar, a limitação é registrada na skill, não escondida.
- **Clique na UI do app não foi testado**, só leitura e `invoke`.
- **Comparar agentes** depende da CLA-31.
- Reconhecimento só vê o estado atual do app.

## 10. Fora de escopo

Escrever o plano de implementação, e criar a skill. Esta spec só fixa o desenho. A skill será criada com o `skill-creator` depois da revisão desta spec.
