---
name: linear-claudia
description: Registar e fechar trabalho do claudia-rh no Linear (time CLA). Use sempre que o utilizador disser "anota isso", "anota uma task", "abre um bug", "regista isso", "guarda para depois", ou perguntar o que está pendente. Use também sem ninguém pedir, sempre que encontrar durante o trabalho um bug colateral, uma dívida ou um pedaço de trabalho fora do escopo actual — registar em vez de perder é o objectivo deste sistema. E consulte antes de dar qualquer trabalho por concluído, porque fechar uma issue tem regra própria e nunca é automático.
---

# Linear como caderno de tasks do claudia-rh

## Porquê isto existe

O objectivo é **descarregar da cabeça**, não gerir projeto.

O claudia-rh é mantido por uma pessoa. O que se descobre a meio de uma sessão — um bug colateral, uma dívida, um "isto depois tem de mudar" — ou interrompe o trabalho em curso, ou fica na memória de alguém, ou desaparece. Este skill existe para haver um terceiro destino.

Daí as duas regras que moldam tudo o resto:

- **Criar é barato e não pede licença.** Qualquer fricção na anotação faz a pessoa voltar a guardar na cabeça, e aí o sistema não serve para nada.
- **Fechar é caro e pede sempre confirmação.** Este caderno é memória. Uma issue fechada sem estar feita não tem quem a apanhe — não há equipa, não há revisor. Um caderno que mente é pior do que caderno nenhum.

## O modelo, inteiro

| | |
|---|---|
| Task, bug ou feature | Issue no time `Claudia-rh`, prefixo `CLA` |
| Versão (`v0.3.0`, `v0.4.0`) | Project com o nome da versão |
| Classificação | Só os labels existentes: `Bug`, `Feature`, `Improvement` |
| Urgência | Priority nativa, e só quando for óbvia |

Issue que é para a próxima release entra no project da versão. O resto fica no backlog sem project.

Não invente labels novos, milestones, estimates nem sub-issues. Isso foi considerado e cortado de propósito: é estrutura que custa a manter e não ajuda a descarregar da cabeça.

## Quando criar

### A pedido

`anota isso`, `anota uma task`, `abre um bug para X`, `regista isso`.

Crie, devolva o ID, siga em frente. Não faça perguntas sobre coisas que dá para inferir da conversa — se o contexto diz que é um bug da UI, classifique como `Bug` e escreva a descrição a partir do que já foi dito. Perguntar "qual a prioridade?" e "qual o label?" a cada anotação é precisamente a fricção que mata o hábito.

Se faltar mesmo alguma coisa essencial e não inferível, crie a issue à mesma com o que tem e diga o que ficou por preencher. Uma issue incompleta registada vale mais do que uma issue perfeita que não chegou a existir.

### Sozinho, durante o trabalho

Encontrou algo fora do escopo do que está a fazer: **crie a issue, avise numa linha, e continue o trabalho em curso.** Não pare para pedir autorização — parar é o mesmo custo que o sistema veio eliminar.

O critério é se o item **sobrevive à sessão actual**: é trabalho real que alguém teria de fazer depois. Uma dúvida passageira não sobrevive. Algo que vai ser corrigido nos próximos minutos também não. Registar tudo enche o caderno de ruído, e um caderno ruidoso deixa de ser lido — o que dá no mesmo que não ter caderno.

### Como escrever a issue

Título curto e concreto, a dizer o que está errado ou o que falta. Não "melhorar o dashboard", mas "Dashboard não mostra estado quando a sessão falha no spawn".

Na descrição, o suficiente para a issue se explicar sozinha daqui a três meses: onde está (ficheiro, módulo), o que se observou, e porque importa. Se o contexto veio de uma sessão de trabalho, inclua o que estava a fazer quando apareceu — é o que permite reconstruir a situação.

## Como fechar

Fechar nunca é automático. São três passos, por esta ordem.

**1. Reunir evidência verificável.** Output de teste que passou, hash de commit ou merge em `dev`, comando executado com o resultado. Se não tem evidência, vá procurá-la — corra os testes, veja o `git log` — antes de assumir o que quer que seja.

**2. Perguntar, citando a evidência:**

> `CLA-12` parece pronta — `cargo test` 14/14 verde, merge `a1b2c3` em `dev`. Posso fechar?

**3. Só depois do sim:** mover para `Done` e deixar um comentário na issue com a evidência.

Não conta como evidência: "implementei", "deve estar a funcionar", "o código parece correcto". Isso é opinião sobre o trabalho, não prova de que funciona.

O comentário com a evidência não é burocracia. É o que responde ao "porquê" quando a issue fechada for reaberta meses depois — o mesmo valor de um `Fixes #12` no merge do GitHub.

---

# Mini-doc: a API do Linear

## MCP é o caminho normal

Já está autenticado nesta sessão. Praticamente tudo se faz com quatro tools:

| Tool | Para quê |
|---|---|
| `save_issue` | criar **e** actualizar issue (o mesmo tool faz as duas) |
| `list_issues` | procurar o que está pendente; filtra por `team`, `state`, `project`, `query` |
| `save_project` | criar/actualizar o project de uma versão |
| `save_comment` | registar a evidência ao fechar |

Úteis de vez em quando: `list_issue_statuses`, `list_projects`, `get_issue`, `list_issue_labels`.

Nas tools do Linear, passe texto directamente — newlines a sério na descrição em markdown, não `\n` literal.

## IDs deste workspace

Embutidos aqui para poupar um levantamento por sessão:

| | |
|---|---|
| Team `Claudia-rh` | `85ba56ec-dc3e-45b6-871a-fb8ee75c0a5c` |
| `Backlog` | `955b5e31-66e3-40f0-8cff-e53262738639` |
| `Todo` | `5568daad-14c5-481c-b196-fd9ca649ff4e` |
| `In Progress` | `ce964a2a-4713-4258-af6d-e475ef0a1bca` |
| `Done` | `d3ac5cac-23b6-48a0-b4a4-3fcdf4ff4bbc` |
| `Canceled` | `3e77c784-86a0-448d-8d67-6f0c5aacbfdb` |

Isto é cache e pode envelhecer. Se uma chamada falhar por ID desconhecido, reconfirme com `list_issue_statuses` e actualize esta tabela.

## GraphQL, só quando o MCP não alcançar

O MCP cobre issues, projects, labels, milestones, documentos e comentários. **Não cria workflow states nem templates** — nessas duas coisas só tem tools de leitura, e não há scope nem definição que acrescente as de escrita. É a única lacuna conhecida, e nenhuma das duas faz falta para anotar tasks.

Se algum dia for preciso:

- Endpoint: `https://api.linear.app/graphql`
- Header: `Authorization: <API_KEY>` — **cru, sem `Bearer`**
- Chave: *Settings → Account → Security & Access → Personal API keys*

```graphql
mutation {
  issueCreate(input: {
    title: "Título"
    description: "Markdown"
    teamId: "85ba56ec-dc3e-45b6-871a-fb8ee75c0a5c"
  }) { success issue { id identifier } }
}
```

`workflowStateCreate` existe provavelmente no schema mas **não está na doc pública** — trate como não verificado até alguma chamada confirmar.
