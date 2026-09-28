# Spec: isolar o ambiente das sessões do claudia-rh

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-25
**Branch:** `feat/isolar-ambiente-sessao` (a partir de `dev`)
**Issue:** CLA-29

## 1. Problema

Os transcripts das sessões param a 16 de Setembro de 2026. Nove dias de sessões — que descobriram vagas, prepararam candidaturas e submeteram material em nome do usuário — não deixaram registro nenhum.

O terminal da app diz porquê:

> *"⚠ Transcript saving is off — inherited CLAUDE_CODE_CHILD_SESSION marker"*

### Causa

Quando um processo lança outro, o filho herda o ambiente do pai. A app lança o `claude`; se a app foi lançada de um terminal que já corria Claude Code — que é exatamente o fluxo de `npm run tauri dev` — o `claude` da app recebe quatro variáveis que não lhe pertencem:

| Variável | Valor observado | O que afirma |
|---|---|---|
| `CLAUDE_CODE_CHILD_SESSION` | `1` | esta é uma sessão-filha, não grava transcript |
| `CLAUDE_CODE_SESSION_ID` | UUID da sessão que lançou a app | esta sessão é aquela |
| `CLAUDE_CODE_ENTRYPOINT` | `cli` | foi lançada por um CLI |
| `CLAUDE_CODE_EXECPATH` | caminho do `claude.exe` | qual executável |

Três das quatro são falsas. A sessão de candidaturas não é filha de nada, não é a conversa que abriu a app, e não foi lançada por um CLI.

`CHILD_SESSION=1` é a que desliga a gravação. `SESSION_ID` é potencialmente pior: a sessão julga-se ser outra, e o que gravasse iria para o transcript errado.

Verificado: a app **não define** nenhuma variável `CLAUDE_CODE_*`. Todas são herança.

## 2. Objetivo

1. Nenhuma variável `CLAUDE_CODE_*` do processo que lançou a app chega ao processo `claude` que a app cria.
2. Isso vale para variáveis que ainda não existem.
3. Uma regressão futura falha de forma visível, não em silêncio.

## 3. Porque não corrigir só a variável que incomoda

Remover `CLAUDE_CODE_CHILD_SESSION` pelo nome resolve hoje. Quando a Anthropic acrescentar outra variável de herança, o problema volta — **em silêncio**, descoberto passados mais nove dias sem transcripts.

A escolha foi discutida com o usuário, que levantou precisamente esta questão: *"apagar agora, não vai fazer com que no futuro precise apagar?"*

Três opções consideradas:

| | Cobre as futuras | Como falha se errarmos |
|---|---|---|
| Por nome | não | silenciosamente, como hoje |
| **Por prefixo `CLAUDE_CODE_`** | **sim** | **o `claude` não sobe — visível no primeiro teste** |
| Allowlist do ambiente inteiro | sim | o `claude` não sobe, e o mínimo do sistema descobre-se por tentativa |

**Escolhida: por prefixo.** Cobre as futuras sem exigir que alguém se lembre, e o seu modo de falha é ruidoso — o oposto do defeito que estamos a corrigir.

A allowlist é o isolamento a sério e pertence à CLA-30, onde é decidida junto com o confinamento do sistema de arquivos e das ferramentas. Fazer prefixo agora não a impede depois.

## 4. Enquadramento maior

O usuário descreveu o princípio ao discutir esta issue:

> *"a claudia-rh e suas sessoes deveriam ser isolada de outras sessoes. apesar de ter bypass permission ela podia ser isolada para fazer somente coisas relacionadas com a claudia-rh, separado até do sistema."*

Esse princípio é maior do que esta spec e está registado na **CLA-30** — que documenta 11 scripts e 63 pacotes npm que a sessão instalou por não ter limites nenhuns.

Esta spec entrega **o primeiro pedaço e o único sem risco**: o ambiente. Confinar ferramentas e sistema de arquivos pode parar a sessão de trabalhar e precisa de ordem própria; limpar variáveis herdadas não.

## 5. Desenho

### 5.1 Um ponto único

`commands/mod.rs:30` já tem `claude_program()`, o ponto único que resolve **qual** executável usar. O ambiente é a outra metade da mesma pergunta — **com que ambiente** — e vive ao lado:

A função não constrói o ambiente do zero. Recebe o que o processo tem e remove o que começa por `CLAUDE_CODE_`.

A assinatura exata fica para o plano de implementação: depende de servir duas APIs de spawn sem as acoplar (§5.2), e essa é uma decisão de implementação, não de desenho. O que esta spec fixa é o comportamento — remoção por prefixo, num ponto único.

### 5.2 Dois mecanismos de spawn, um comportamento

O repositório cria processos `claude` de duas formas, e ambas precisam do mesmo tratamento:

| Mecanismo | Onde |
|---|---|
| `std::process::Command` | `cover_letter.rs:327`, `curriculos.rs:29`, `feedback.rs:452`, `perfil.rs:515`, `perfil.rs:716`, `welcome.rs:13` |
| `portable_pty::CommandBuilder` | `pty_manager.rs:57` e `:120`, usados por `sessao.rs:65` e `linkedin.rs:83` |

São oito call sites. Sete lançam sessões; `welcome.rs:13` corre `claude --version` para verificar a instalação — não precisa da limpeza, mas também não sofre com ela, e tratá-lo igual evita a pergunta "porque é que este é diferente?".

As duas APIs não compartilham tipo. `std::process::Command` tem `env_remove`; `CommandBuilder` tem `env`. A função de limpeza expõe-se de forma que sirva as duas sem as acoplar.

### 5.3 O teste que não envelhece

O código por prefixo cobre variáveis futuras. O teste tem de fazer o mesmo, ou a garantia depende de alguém se lembrar.

O teste verifica a **propriedade**, não os nomes: dado um ambiente que contém variáveis `CLAUDE_CODE_*`, nenhuma sobrevive à limpeza. Inclui um nome inventado que não existe hoje — se o código passar a listar nomes em vez de usar o prefixo, esse caso falha.

E verifica o inverso com a mesma força: variáveis que **não** começam por `CLAUDE_CODE_` — `PATH`, `APPDATA` — passam intactas. Uma limpeza zelosa demais impede o `claude` de subir, e esse teste é o que a apanha.

## 6. Fora de escopo

**Allowlist do ambiente.** Pertence à CLA-30, com o resto do confinamento.

**Retenção de transcripts.** Um transcript de 172 turnos ocupou 5.5 MB; `~/.claude/projects` já tem 275 MB, com o disco a 98%. Voltar a gravar vai fazer isso crescer. É problema real, mas é outro — e não gravar nada não é a forma de o resolver.

**Confinar ferramentas e sistema de arquivos.** CLA-30.

**Os 11 scripts que a sessão escreveu.** CLA-30. Esta spec não os remove nem os impede.

## 7. Arquivos

| Arquivo | Mudança |
|---|---|
| `src-tauri/src/commands/mod.rs` | a função de limpeza, ao lado de `claude_program()` |
| `src-tauri/src/pty_manager.rs` | aplicar nos dois `CommandBuilder` |
| `src-tauri/src/commands/cover_letter.rs` | aplicar |
| `src-tauri/src/commands/curriculos.rs` | aplicar |
| `src-tauri/src/commands/feedback.rs` | aplicar |
| `src-tauri/src/commands/perfil.rs` | aplicar nos dois spawns |
| `src-tauri/src/commands/welcome.rs` | aplicar |

Nenhum arquivo de frontend é tocado. `prompt_sistema_runtime.md` não é tocado.

## 8. Verificação

```text
limpeza aplicada a um ambiente com CLAUDE_CODE_CHILD_SESSION
→ a variável não sobrevive

limpeza aplicada a uma variável CLAUDE_CODE_* inventada
→ também não sobrevive (prova que é prefixo, não lista)

PATH e APPDATA
→ intactos

a app corre e inicia uma sessão
→ aparece um .jsonl novo em ~/.claude/projects/…claudia-rh-workspace/
```

A verificação final é a única que conta: **um transcript novo em disco.** Os testes unitários provam a limpeza; só a app rodando prova que era essa a causa.

## 9. Ressalva honesta

O diagnóstico é consistente com toda a evidência — a mensagem no terminal, a data em que os transcripts param, as variáveis presentes no ambiente. Mas a relação causa-efeito **não foi provada de ponta a ponta**: não vimos o `claude` a gravar depois da limpeza.

Se o transcript continuar a não aparecer, a causa é outra e esta spec corrigiu uma coisa verdadeira sem resolver o problema. O passo de verificação em 8 existe para isso se saber no próprio dia.

Nota de âmbito: o efeito depende de como a app foi iniciada. Lançada pelo atalho do Windows, provavelmente já grava; lançada de um terminal com Claude Code, não. A correcção vale para os dois casos e não depende de qual estava em causa.

## 10. Superpowers

Spec aprovada → `writing-plans` → implementação em `feat/isolar-ambiente-sessao` → review → merge em `dev`.
