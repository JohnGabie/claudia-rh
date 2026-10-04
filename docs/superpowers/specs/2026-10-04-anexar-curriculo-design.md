# Spec: aba Currículos (currículos e cover letters), MCP de variantes e documentos, e anexo pelo agente

> Spec de design. Não é plano de implementação. **Rascunho para aprovação do usuário: nenhum código é escrito até ele aprovar.** Versão 3 (2026-10-04): contém o que o usuário pediu, e separa na seção 4 o que foi proposta do assistente e ainda não foi decidida.

**Branch:** `feat/browser-cdp-mcp` (continuação)

## 1. Por que (verificado no código em 2026-10-04)

- O agente **não consegue anexar o currículo** em formulários: nenhuma das 9 tools `browser_*` envia arquivo.
- O currículo que a Claudia gera hoje é **HTML**, guardado em `<dados>/curriculos/` (`commands/curriculos.rs`). As cover letters ficam em `commands/cover_letter.rs`. Os dois vivem como listas dentro da aba Perfil.
- Não existe no MCP nada de currículos, cover letters nem de escrita de variantes. Só existem `get_candidate_profile`, `get_search_variants` (leitura) e `update_profile`.
- Não há como o usuário trazer o seu próprio currículo nem escolher qual usar.

## 2. O que o usuário pediu (2026-10-04, literal agrupado)

### 2.1 Navegação e telas
1. **Currículos vira uma aba na sidebar.** Dentro dela ficam duas abas: **Currículos** e **Cover letters**.
2. **Visualização em grade**, tanto para currículos quanto para cover letters.
3. **Ao clicar num item, abre uma página nova** onde o documento é visualizado, **com um botão de voltar**.
4. **A visualização não precisa ser PDF: pode ser HTML**, só com **bordas imitando uma página A4** (é mais leve).

### 2.2 Documentos
5. **Adicionar currículos personalizados** (o usuário traz o seu).
6. **Marca de preferencial:** o usuário marca um documento como preferido, e isso **pode depender da variante**.
7. **Tags:** variantes, tipo (por exemplo ATS) e cor usada.

### 2.3 MCP
8. **MCP das variantes de perfil.**
9. **MCP da cover letter.**

(O pedido listou "mcp das variantes" duas vezes; está tratado como um só.)

## 3. O que cada pedido significa (leitura direta, sem acrescentar)

### 3.1 Aba Currículos
- Novo item na sidebar, com a view própria. Sub-abas **Currículos** e **Cover letters**.
- Cada sub-aba é uma **grade de cartões**. Cada cartão mostra o documento em miniatura, o nome, as tags e a marca de preferido.
- Clicar num cartão abre a **página do documento**: a folha A4 (HTML com moldura de página, somente leitura), os dados do documento, as tags e a ação de marcar preferido. Botão **Voltar** no topo.
- Os atalhos e as telas de currículos e cover letters que hoje estão dentro do Perfil saem de lá e passam a ser esta aba.

### 3.2 Currículos personalizados
- Botão para o usuário **adicionar o seu currículo**, que passa a ser um item da grade como os gerados, com origem "importado".

### 3.3 Preferido, possivelmente por variante
- O usuário marca um documento como **preferido**. A marca pode ser **geral** ou **de uma variante** (por exemplo, "para a variante Backend, usar este").
- Vale para currículos e para cover letters.

### 3.4 Tags
- Cada documento tem tags dos tipos: **variante**, **tipo de template** (ATS e os outros) e **cor usada**. O usuário vê as tags no cartão e na página.

### 3.5 MCP
- **Variantes:** o agente consegue listar e ler as variantes de perfil (e, na sessão de Perfil, alterá-las).
- **Cover letter:** o agente consegue listar e ler cover letters e salvar uma que ele escreveu.
- Por consequência do que foi pedido (preferido, personalizados, tags), o MCP também expõe os **documentos** (listar, ler, marcar preferido).

## 4. Propostas do assistente, AINDA NÃO DECIDIDAS (aprovar, mudar ou cortar)

Nada abaixo foi pedido pelo usuário. Cada item tem uma recomendação.

| # | Proposta | Recomendação |
|---|---|---|
| P1 | **`browser_upload`**: tool para o agente anexar o currículo no formulário (este é o problema que originou tudo) | Fazer. Sem ela o resto não resolve a candidatura. |
| P2 | O agente **nunca recebe nem passa caminho de arquivo**; só `id` de documento da biblioteca, e o app confere que o arquivo está dentro de `<dados>/documentos/` e não foi alterado (`sha256`) | Fazer. Evita que texto malicioso numa página mande subir qualquer arquivo do usuário. |
| P3 | **Regra de escolha** do que anexar: preferido da variante, depois preferido geral, depois o gerado mais recente da variante, senão pendência "sem currículo para anexar" | Fazer. É o que dá sentido à marca de preferido. |
| P4 | **PDF** gerado do HTML pelo Chrome do próprio app (`Page.printToPDF`), usado só para anexar e para abrir fora; sem biblioteca nova | Fazer: site de vagas não aceita HTML. |
| P5 | Importar só `.pdf` e `.docx`, até 10 MB, conferindo o tipo real do arquivo; o app **copia** o arquivo para a sua pasta | Fazer. |
| P6 | Documento importado não tem HTML: a página mostra um cartão com os dados e "Abrir" | Aceitar, ou mostrar o PDF no WebView se for seguro (a verificar). |
| P7 | **Escrita só na sessão de Perfil** (`update_variante`, `set_documento_preferido`); a sessão de execução só lê | Fazer. Segue a arquitetura: a execução não edita o perfil. |
| P8 | Importar um currículo **não** o torna preferido sozinho; o usuário marca | Fazer. |
| P9 | Migração: os currículos e cartas existentes viram documentos, sem apagar nada | Fazer. |
| P10 | Corrigir `apagar_curriculo`/`apagar_cover_letter`, que apagam **qualquer caminho** recebido: passam a apagar por id, só dentro da pasta | Fazer. É um bug de segurança que já existe. |
| P11 | Mostrar o documento usado em cada candidatura (Dashboard e Histórico) e uma categoria de Pendência "sem currículo para anexar" | Fazer depois da aba. |
| P12 | **Edição do `prompt_sistema_runtime.md`** para ensinar as tools novas ao agente | Precisa de aprovação explícita antes. |
| P13 | Anexar também a **cover letter** no formulário (a mesma tool com `tipo=carta`) | Fazer junto com P1: custo baixo. |

## 5. Como ficaria o MCP (se P1 a P3 e P7 forem aprovadas)

| Tool | Sessão | Faz |
|---|---|---|
| `list_variantes` / `get_variante(id)` | ambas | lê as variantes |
| `update_variante(id, campos)` | só Perfil | valida e grava, com backup como `update_profile` |
| `list_documentos(tipo)` / `get_documento(id)` | ambas | metadados e tags; `get_documento` devolve também o texto (para responder formulários) |
| `save_cover_letter(texto, vaga_id?, idioma, variante_id?)` | ambas | guarda a carta escrita pelo agente como documento |
| `set_documento_preferido(id, variante_id?)` | só Perfil | marca o preferido |
| `browser_upload(ref, tipo?, documento?)` | execução | anexa o documento; sem `documento`, o app escolhe pela regra P3 |

Nenhuma tool devolve ou aceita caminho de arquivo.

## 6. Dados (resumo, para aprovar o desenho; detalhe fica no plano)

- Tabela `documentos`: `id`, `tipo` (`curriculo`/`carta`), `nome`, `origem` (`gerado`/`importado`), arquivo anexável (PDF ou DOCX), HTML de pré-visualização (só gerado), `idioma`, `tags`, `sha256`, `tamanho`, `vaga_id` opcional, `criado_em`.
- Tabela `preferidos`: (`tipo`, `variante` ou geral, `documento`), um por par.
- As tags do tipo variante, template e cor são preenchidas na geração; o usuário pode editar e criar tags livres.

## 7. Impacto no frontend (lista do que muda)

| Onde | Mudança |
|---|---|
| Sidebar e navegação (`Sidebar.tsx`, `App.tsx`) | novo item **Currículos** |
| Nova aba (`components/documentos/`) | sub-abas Currículos e Cover letters; grade; filtro por tag; **Gerar** e **Adicionar o meu** |
| Página do documento | folha A4, Voltar, tags editáveis, marcar preferido (geral e por variante), renomear, abrir, excluir |
| Perfil | saem os atalhos e telas de currículos/cartas; as variantes ganham o seletor de preferido |
| Dashboard, Histórico, Pendências, Activity log, Welcome | itens de P11 (depois da aba) |
| i18n | tudo em `pt.ts` e `en.ts` |

## 8. Testes (resumo)

- Unidade para cada regra de escolha e de segurança (caminho fora da pasta, `..`, tipo falso, `sha256`, um preferido por par), com **mutação** para cada uma.
- Live com Chrome de teste e página local: campo de arquivo visível, campo escondido atrás de botão (padrão do LinkedIn), site que só aceita PDF, recusa sem vaga ativa.
- Frontend com a skill `claudia-teste`: abrir a aba, a grade, a página, o Voltar, marcar preferido por variante, tags, importar. Sem nenhuma ação em site de vagas.

## 9. Ordem proposta (só depois da aprovação desta spec)

1. Backend da biblioteca (tabelas, comandos, importação, PDF, migração).
2. Frontend da aba (em paralelo, sobre o contrato de comandos que o plano definir).
3. MCP e `browser_upload`.
4. Integração, suíte completa, validação no app e revisão Opus.

## 10. Em aberto

- Onde a skill `tailor-application` grava os currículos adaptados por vaga: **não verificado**.
- Mostrar PDF importado no WebView ou só o cartão (P6).
- Cover letter: o preferido por variante também vale para ela (assumido que sim).
