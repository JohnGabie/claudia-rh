# Pesquisa: teste comportamental no LinkedIn

**Data:** 2026-09-28
**Motivo:** abrir um URL não testa nada — as defesas disparam no que se **faz**
**Como:** Playwright MCP, perfil dedicado autenticado, nunca o Chrome pessoal do usuário

> As três fases que interessam: **navegar**, **ver vagas**, **candidatar**.
> Dados crus em `prototipo-playwright/medicoes/raw2/` — pasta descartável, e o
> snapshot do modal contém dados pessoais do usuário (ver §Privacidade).

## Resumo

| Fase | Resultado | Bloqueios |
|---|---|---|
| **A. Navegação** — busca + 4 scrolls | 7 vagas visíveis | **0** |
| **B. Visualização** — 5 vagas em sequência | 5/5 carregaram, 5/5 com Easy Apply | **0** |
| **C. Candidatura** — abrir o formulário | **modal abriu**, 20 campos legíveis | **0** |

**Nenhuma candidatura foi submetida.** Verificado por três vias independentes, abaixo.

## A. Navegação

Busca com filtro Easy Apply, quatro `PageDown` com 1.8–3.0 s aleatórios entre eles.
Nenhum captcha, checkpoint, authwall ou rate-limit. Sete vagas extraídas do resultado.

## B. Visualização

Cinco vagas abertas em sequência, 2.5–4.5 s entre cada. Todas carregaram, todas com o
botão de Candidatura simplificada presente. Nenhum bloqueio.

## C. Candidatura — o modal abre, e o custo disso

### O clique do MCP não funciona; o clique do DOM funciona

Duas tentativas anteriores falharam por eu clicar no `ref` errado — a frase
"Candidatura simplificada" aparece **18 vezes** na página, quase toda em badges do
carrossel de vagas recomendadas. O botão real é o único com papel `button` e rótulo
completo:

```
- button "Usar a candidatura simplificada para esta vaga" [ref=e506] [cursor=pointer]
```

Com o `ref` certo, o `browser_click` do MCP **continuou a não abrir o modal**. O estado
do botão estava perfeito — visível, não desativado, dentro do viewport, não coberto:

```json
{ "disabled": false, "visivel": true, "noViewport": true, "coberto": false,
  "posicao": { "x": 137, "y": 294, "w": 202, "h": 32 } }
```

O que funcionou foi um `.click()` direto no DOM, via `browser_evaluate`. **O modal
abriu em 2 segundos.**

Não sei a causa. Pode ser a forma como o MCP entrega o evento, ou algo no handler do
LinkedIn. **É um facto de implementação que a Fase 2 tem de conhecer:** para este
botão, o caminho é DOM, não o click de alto nível.

### O que está dentro do modal

20 campos no primeiro passo:

```
combobox "Email*"
combobox "Código do país*"
textbox  "Phone*"                     (pré-preenchido)
radiogroup — escolha de currículo
  radio "cv_fullstack_engineer.pdf"   [checked]
  radio "cv_joao_gabriel_hire_feed.pdf"
```

Os PDFs no radiogroup são os que a sessão gerou por fora e carregou — a evidência da
[CLA-27](https://linear.app/claudia-rh/issue/CLA-27), visível do lado do LinkedIn.

### O achado que muda o desenho da segurança

Os botões presentes no **mesmo** passo:

```
Fechar | Carregar currículo | Enviar candidatura | Pular para pesquisa | ...
```

**"Enviar candidatura" está no primeiro passo.** Não há percurso de vários passos onde
se possa parar a meio com margem — o botão que submete está à vista desde o início.

Isso torna a fronteira de segurança estreita: **um clique errado é uma candidatura
enviada em nome do usuário, irreversível, para um recrutador real.**

O script deste teste foi escrito para ser estruturalmente incapaz de a atravessar: uma
lista de rótulos proibidos (`enviar|submit|revisar|próximo|continuar|avançar`) aborta o
run se algum for alvo de clique. Foi essa lista que tornou seguro chegar até aqui.

### Verificação de que nada foi submetido

Três vias independentes:

1. **O botão de candidatar continua na página.** O LinkedIn substitui-o por um estado
   "Candidatou-se" depois de submeter. Está lá.
2. **A vaga 4467095900 não aparece** na lista de candidaturas (`/my-items/saved-jobs/?cardType=APPLIED`).
3. **Zero candidaturas com data de hoje** nessa lista.

Uma quarta verificação deu falso positivo e vale registar: procurei `Applied\b` no
snapshot e deu "SIM" — porque apanhou o título de uma vaga recomendada, *"Software
Engineer (Applied AI)"*.

## O que o anti-bot fez: nada

Nenhuma das três fases disparou captcha, checkpoint ou limite. Isso é um dado, com duas
ressalvas honestas:

- **Uma sessão curta.** Seis páginas, um modal, poucos minutos. Não diz nada sobre uma
  sessão de uma hora a percorrer dezenas de vagas.
- **O perfil já estava autenticado** e tinha histórico da sessão anterior. Um perfil
  recém-criado a fazer isto no primeiro minuto pode ser tratado de outra forma.

Não mede risco de conta a prazo. Mede que a mecânica funciona.

## O padrão que atravessa todo o estudo

**Cinco heurísticas escritas, cinco falhas silenciosas:**

| # | Heurística | Falha |
|---|---|---|
| 1 | caminho começa por `/job` | 0 vagas no RemoteOK (`/remote-jobs/`) |
| 2 | slug com 4+ palavras | 0 no LinkedIn e Greenhouse (IDs numéricos) |
| 3 | seletores CSS para extrair | todos os campos vazios; reportou "63× melhor" |
| 4 | descrição = parágrafo mais longo | apanhou a promoção do Premium |
| 5 | `Applied\b` = candidatou-se | apanhou "Applied AI" num título |

**Nenhuma deu erro. Todas devolveram algo que parecia uma resposta.**

Três foram apanhadas por revisão ou verificação; duas por eu desconfiar de um número
estranho. Se qualquer uma tivesse passado, teria entrado num documento como facto.

Para o desenho "o app conduz", isto é o custo real — não o contexto. **Cada extrator
precisa de validação de que apanhou a coisa certa, não de que apanhou alguma coisa.**

## Privacidade

`medicoes/raw2/modal-aberto.yml` contém o telefone real do usuário, pré-preenchido pelo
LinkedIn no formulário. O ficheiro está coberto pelo `.gitignore` do protótipo e nunca
foi versionado. Apagar com a pasta.

## Escrever nos campos: resolvido, e a solução é o achado

A primeira versão deste documento dizia que os campos tinham sido lidos mas não
preenchidos. Foram, depois — e o caminho até lá é o resultado mais útil do teste.

### Quatro tentativas, e onde cada uma partiu

| # | Estratégia | Resultado |
|---|---|---|
| 1 | `browser_click` do MCP com o `ref` da árvore | **não faz nada** — nem no botão, nem no rádio |
| 2 | `.click()` no DOM, elemento por `aria-label` | abre o botão ✓, **não encontra os campos** |
| 3 | `.click()` no DOM, elemento por texto do rótulo | **"radio não localizado"** |
| 4 | `.click()` no DOM, **por índice** | **funciona** ✓ |

### Porque as três primeiras falham

O modal do Easy Apply tem uma assimetria que não esperava:

**A árvore de acessibilidade tem os nomes:**
```
- radio "cv_fullstack_engineer.pdf" [checked] [ref=e1273]
- radio "cv_joao_gabriel_hire_feed.pdf" [ref=e1291]
```

**O DOM não tem identificador nenhum:**
```json
{ "tag": "input", "type": "radio", "aria": "", "id": "«re»" }
{ "tag": "input", "type": "radio", "aria": "", "id": "«rf»" }
```

`aria-label` vazio. IDs gerados pelo React, com guilhemetes: `«re»`, `«rf»`, `«rg»`.
Nada estável para endereçar. E o `ref` que a árvore fornece não funciona com o
`browser_click` deste servidor, dentro deste modal.

**Consegue-se ver os campos e não se consegue tocar neles** — por nome.

### A ponte é a ordem

A árvore e o DOM listam os rádios na **mesma ordem**. Isso dá o endereço:

```
árvore (tem os nomes)                        DOM (tem os elementos)
  [0] ● cv_fullstack_engineer.pdf     ────►  radios[0]
  [1] ○ cv_joao_gabriel_hire_feed.pdf ────►  radios[1]   ← clicar aqui
```

Verificado: cliquei no índice 1, o snapshot passou a marcar o índice 1, restaurei o 0,
fechei o modal, **nada submetido**.

### O padrão para a Fase 2

```
LER    ← árvore de acessibilidade   (tem os rótulos; o DOM não)
ESCREVER → DOM por índice           (a árvore não consegue clicar aqui)
VERIFICAR ← árvore outra vez        (a única fonte de verdade)
```

Três superfícies diferentes para uma operação. **Nenhuma sozinha chega.**

E a fragilidade é evidente: se o LinkedIn reordenar os elementos, ou meter um campo
escondido no meio, o índice aponta para outro sítio — e o clique acontece na mesma, sem
erro. É a sexta falha silenciosa possível, e esta ainda não aconteceu.

## O que continua por testar

- **Escrever em campo de texto.** O rádio funcionou; o telefone é `input[type=tel]` com
  `aria-label` vazio, mesma classe de problema, não testado.
- **Sessão longa.** Ritmo e volume reais.
- **Outras plataformas.** O usuário não tem conta em todas; este teste é só LinkedIn.
- **Candidatura completa**, que exige atravessar a fronteira e só o usuário pode decidir.
