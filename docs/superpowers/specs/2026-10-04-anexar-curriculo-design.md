# Spec: biblioteca de documentos (currículos e cover letters) e anexo pelo agente

> Spec de design. Não é o plano de implementação. Complementa `2026-10-02-browser-cdp-motor-design.md` e `2026-10-03-mcp-leitura-robusta-design.md`. Versão 2 (2026-10-04): a proposta original (biblioteca, regra de escolha, PDF, `browser_upload`, segurança) com as 9 adições pedidas pelo usuário, listadas na seção 2.
>
> **Status: RASCUNHO, aguardando aprovação do usuário. Nenhum código é escrito antes dela** (regra do `CLAUDE.md`). A seção 2 é o que o usuário pediu; o resto é a proposta técnica que ele ainda vai aprovar ou ajustar.

**Branch:** `feat/browser-cdp-mcp` (continuação)

## 1. Problema (verificado no código em 2026-10-04)

1. **O agente não tem como anexar arquivo.** Nenhuma das 9 tools `browser_*` envia arquivo; não há `setFileInputFiles` nem tratamento de seletor de arquivo. Todo formulário com "Anexar currículo" trava.
2. **O currículo gerado é HTML**, em `<dados>/curriculos/cv_<template>_<data>.html` (`commands/curriculos.rs`); sites pedem PDF ou DOCX.
3. **Não há como importar um currículo próprio nem escolher qual vale para qual vaga.** `search_variants.yaml` tem `cv_gerado_path` por variante, sem uso.
4. **Currículos e cover letters vivem escondidos dentro do Perfil**, em listas, sem pré-visualização.
5. **O MCP não tem nada de documentos nem de escrita de variantes** (só `get_search_variants`/`get_candidate_profile` de leitura e `update_profile`).
6. **Segurança:** se uma tool recebesse um caminho, texto malicioso numa página poderia induzir o agente a subir qualquer arquivo do usuário. `apagar_curriculo(path)` e `apagar_cover_letter(path)` apagam **qualquer** caminho recebido.

## 2. Decisões do usuário (2026-10-04)

- Currículos e cover letters passam a ser **uma aba própria na sidebar**, com duas sub-abas: **Currículos** e **Cover letters**.
- Cada sub-aba é uma **grade**; ao clicar num item abre-se **uma página nova** com a visualização e um botão **Voltar**.
- A visualização é **HTML com borda imitando folha A4** (leve), não PDF.
- **Currículos personalizados:** o usuário pode adicionar o seu (importar).
- **Preferencial:** o usuário marca um documento como preferido; pode depender da **variante**.
- **Tags:** variante, tipo de template (ATS etc.), cor usada (mais idioma e livres).
- **MCP** novo para variantes, cover letters e documentos, e `browser_upload` para anexar.

## 2.1 Respostas do usuário às pendências da revisão (2026-10-04)

1. **Variante da vaga: o agente decide.** As variantes são perfis com modificações ou exclusões de experiência para áreas diferentes (desenvolvimento, DevOps, IA...), cada uma podendo gerar o seu currículo. Para cada vaga, o agente (pelo MCP) decide qual variante faz sentido e a grava na vaga. Se o usuário marcou uma variante como **principal**, ela é o padrão quando o agente não tiver motivo para outra. Hoje `vagas.variante_id` existe mas ninguém o preenche: passa a ser preenchido pelo agente.
2. **PDF por biblioteca, sem usar o browser.** O usuário não quer o Chrome envolvido na geração. Ver seção 5 (custo: o template do PDF é separado do HTML da tela).
3. **Currículo importado: sem extração de texto agora.** Importar e anexar continua no escopo; extrair texto de PDF/DOCX para o agente ler fica para depois (mais código, mais bug). O agente responde formulários pelo `candidate_base.yaml`.
4. **Vagas que se acumulam em `candidatando`:** bug conhecido, em aberto, resolvido em outra frente (registrar no Linear). Não entra nesta spec.
5. **Regra dura: o Chrome nunca pode falhar por causa de um upload.** Ver seção 8.

## 3. Modelo de dados (migration, não `CREATE TABLE` solto)

Tabela `documentos`:

| Campo | Significado |
|---|---|
| `id` | inteiro estável; é o que o agente cita |
| `tipo` | `curriculo` ou `carta` |
| `nome` | rótulo editável |
| `origem` | `gerado` (pela Claudia) ou `importado` (do usuário) |
| `arquivo` | caminho do PDF/DOCX anexável, **sempre dentro de** `<dados>/documentos/` |
| `arquivo_html` | HTML de pré-visualização (só `gerado`; vazio no importado) |
| `formato` | `pdf` ou `docx` |
| `idioma` | `pt`, `en` ou vazio |
| `tags` | JSON `[{"tipo":"variante"\|"template"\|"cor"\|"idioma"\|"livre","valor":"..."}]` |
| `sha256`, `tamanho` | integridade, conferidos antes de cada anexo |
| `vaga_id` | opcional: carta ou currículo feito para uma vaga |
| `variante_id` | variante para a qual o documento foi gerado (campo, não tag: a escolha não depende de tag editável) |
| `criado_em` | |

Tabela `preferidos` (`tipo`, `variante_id` NULL = global, `documento_id`), chave única `(tipo, variante_id)`. Um preferido por variante e um global, por tipo.

`search_variants.yaml` ganha `principal: true` em no máximo uma variante (marcada pelo usuário). `vagas.variante_id` passa a ser gravado pelo agente (`register_vaga` e `update_vaga_status` aceitam `variante_id`, validado contra as variantes existentes).

Migração dos arquivos antigos: os `curriculos/*.html` e as cartas existentes viram documentos `gerado`; o PDF é gerado pela biblioteca na migração. Nada é apagado.

## 4. Regra de escolha (em código, não no prompt)

A variante da vaga é a que o agente gravou em `vagas.variante_id`; sem ela, a variante **principal**; sem principal, nenhuma (passos 1 e 3 são pulados). O resultado do `browser_upload` sempre diz qual variante e qual regra foram usadas.

Para uma vaga e um `tipo`, nesta ordem:
1. o **preferido da variante** da vaga;
2. o **preferido global**;
3. o documento **gerado mais recente com `variante_id` dessa variante**;
4. nenhum: a tool recusa e o prompt manda abrir a pendência "sem currículo/carta para anexar".

A marca do usuário sempre vence a geração automática. (Não há política "fixo/adaptado": o preferido é essa escolha.)

## 5. PDF e visualização

- A visualização é o **HTML** com moldura A4 (210×297 mm proporcional, sombra e margens), dentro de um contêiner sem scripts (`<iframe sandbox>` com `srcdoc`). Miniaturas da grade: o mesmo HTML escalado, carregado só quando o card aparece.
- O **PDF** existe só para anexar e para o usuário abrir fora. **Decisão do usuário: gerado por biblioteca, sem o browser.** Não há biblioteca Rust madura que converta HTML+CSS em PDF sem um motor de browser; a proposta é gerar o PDF direto dos dados com **Typst** (crate Rust embutida, sem instalação). Custo assumido: cada template existe em HTML (tela) e em Typst (PDF), e os dois precisam ficar parecidos; um teste compara os textos extraídos dos dois. **A confirmar na aprovação.**
- Documento **importado**: sem HTML. A página mostra um cartão (nome, formato, tamanho, data, tags) e "Abrir" no visualizador do sistema; se o WebView exibir PDF em `<iframe>` de forma segura, mostrar o PDF, senão cartão. (A verificar na implementação.)
- Importar: o usuário escolhe `.pdf` ou `.docx` pelo seletor do sistema; o app **copia** para `<dados>/documentos/`, confere extensão **e** tipo real (`%PDF-` ou ZIP de docx), limita a 10 MB e grava o `sha256`.

## 6. Contrato backend ↔ frontend (comandos Tauri)

`Documento` (JSON): `{ id, tipo, nome, origem, formato, idioma, tags:[{tipo,valor}], tamanho, criado_em, vaga_id, preferido_global:boolean, preferido_variantes:string[], tem_preview:boolean }`

| Comando | Faz |
|---|---|
| `listar_documentos(tipo)` | lista do tipo (`"curriculo"`/`"carta"`) |
| `obter_documento(id)` | `Documento` + `preview_html: string \| null` |
| `gerar_curriculo(template_id, cor_primaria, idioma, variante_id?)` | gera HTML + PDF, tags automáticas (variante, template, cor, idioma), devolve `Documento` |
| `gerar_cover_letter(...)` (assinatura atual + `variante_id?`) | idem para carta |
| `importar_documento(tipo, caminho_origem, nome?)` | copia e valida, devolve `Documento` |
| `renomear_documento(id, nome)` | |
| `atualizar_tags_documento(id, tags)` | edita as tags (variante/livres; as automáticas também) |
| `definir_preferido(id, variante_id?)` / `remover_preferido(tipo, variante_id?)` | marca/desmarca |
| `abrir_documento(id)` | abre o arquivo no sistema |
| `apagar_documento(id)` | só dentro de `documentos/`, por id; avisa se for preferido |
| `listar_variantes()` | `{id, nome_exibicao, ativa}[]` para os seletores |

Os comandos antigos `listar_curriculos`, `apagar_curriculo`, `abrir_curriculo`, `listar_cover_letters`, `abrir_cover_letter`, `apagar_cover_letter` são **removidos** (e `gerar_*` substituídos pelos novos).

## 7. MCP (`claudia`)

Leitura vale para os dois tipos de sessão; **escrita só na sessão de Perfil** (a arquitetura manda que a sessão de execução não edite o perfil).

| Tool | Sessão | Faz |
|---|---|---|
| `list_variantes` | ambas | variantes (id, nome, peso, ativa, foco) |
| `get_variante(id)` | ambas | uma variante completa |
| `update_variante(id, campos)` | Perfil | valida e grava, com backup como `update_profile`; nunca afirma fato fora de `candidate_base.yaml` |
| `list_documentos(tipo)` | ambas | metadados e tags, sem caminho |
| `get_documento(id)` | ambas | metadados + texto se `gerado`; para `importado` não há texto (sem extração por enquanto) e o agente responde formulários pelo `candidate_base.yaml` |
| `save_cover_letter(texto, vaga_id?, idioma, variante_id?)` | ambas | o agente escreve a carta de uma vaga; o app a renderiza (HTML A4 + PDF), guarda e etiqueta |
| `set_documento_preferido(id, variante_id?)` | Perfil | marca o preferido |
| `browser_upload(ref, tipo?, documento?)` | execução | anexa (seção 8) |
| `set_vaga_variante(vaga_id, variante_id)` | execução | grava a variante escolhida para a vaga (ou via `register_vaga`) |

Nenhuma tool devolve ou aceita caminho de arquivo.

## 8. Tool `browser_upload`

- Parâmetros: `ref` (campo de arquivo ou botão que o abre, de `browser_read`), `tipo` (`curriculo` padrão ou `carta`), `documento` opcional (`id`). **Nunca um caminho.**
- Sem `documento`, o app escolhe pela regra da seção 4. Com `documento`, só vale se existir, for do `tipo` certo.
- Mecânica sem JS na página: `<input type=file>` direto com `DOM.setFileInputFiles`; botão que abre o seletor (LinkedIn): `Page.setInterceptFileChooserDialog`, clica e responde ao seletor com o arquivo. O resultado diz o nome anexado e se a página o mostrou depois.
- Antes de anexar: `canonicalize` e conferir que está dentro de `<dados>/documentos/`, reconferir `sha256`. Alterado ou sumido: recusa.
- Só funciona com **uma única vaga em `candidatando`** (a regra da guarda). Não é envio e não passa pela guarda de envio. Passa por viewport, `estado:` e detector de loop.
- Activity log: nome, formato, tamanho e `sha256`, **nunca o conteúdo**.
- Falhas explícitas (campo não é de arquivo, site só aceita PDF, arquivo grande demais) viram pendência, não repetição.
- **O Chrome nunca falha por causa de um upload (regra dura do usuário).** Toda falha do upload é um erro da tool, nunca do browser:
  - timeout em cada passo (no máximo alguns segundos), sem espera infinita;
  - a interceptação do seletor de arquivo (`Page.setInterceptFileChooserDialog`) é sempre desligada ao final, com sucesso ou erro, para não deixar a aba presa;
  - um seletor de arquivo nativo que abriu sem ser interceptado é fechado (Escape) e reportado, nunca fica aberto bloqueando a aba;
  - nada no upload fecha, recarrega ou relança o Chrome; a aba continua usável;
  - teste live: depois de cada cenário de falha, `browser_read` e `browser_click` na mesma aba ainda funcionam.

## 9. Prompt

`prompt_sistema_runtime.md` precisa de uma edição para ensinar `browser_upload`, `list_documentos`/`save_cover_letter` e as variantes. **Pede aprovação explícita do usuário antes de editar** (regra do projeto). O agente nunca escolhe arquivo.

## 10. Impacto no frontend

| Tela | Mudança |
|---|---|
| **Sidebar** (`Sidebar.tsx`, `App.tsx`, `View`) | novo item **Currículos** (view `documentos`) entre Perfil e Histórico. |
| **Aba Currículos** (novo `components/documentos/`) | duas sub-abas **Currículos** e **Cover letters**. Cada uma: grade de cartões (miniatura A4, nome, tags, estrela de preferido, origem), filtro por tag (variante/template/cor/idioma), botões **Gerar** (template, cor, idioma, variante; assistente existente) e **Adicionar o meu** (importar). |
| **Página do documento** | abre ao clicar num cartão, com **← Voltar** no topo. Folha A4 de pré-visualização (somente leitura), metadados, **tags editáveis**, **marcar como preferido** (global e/ou por variante), renomear, abrir, excluir (confirmação; avisa se for preferido). |
| **Perfil** (`perfil/index.tsx`, `ResumoView`, `CurriculosView`, `CoverLettersView`) | saem os atalhos e as telas internas de currículos/cartas; no lugar, um link para a nova aba. As **variantes** ganham o seletor "Currículo preferido desta variante" e "Carta preferida". |
| **Dashboard / vaga atual** | mostra o currículo/carta anexado. |
| **Histórico** | cada candidatura mostra o documento usado (`documento_id` em `candidaturas`). |
| **Pendências** | categoria nova "Sem currículo/carta para anexar", com atalho para a aba. |
| **Activity log** | linhas de `browser_upload` mostram nome e formato do arquivo. |
| **Welcome** | avisa que é preciso ter ao menos um currículo (gerar ou importar). |
| **i18n** | todos os textos novos em `pt.ts` e `en.ts`. |

## 11. Segurança

- O agente só cita `id`. O caminho vem do banco e é sempre `canonicalize`d dentro de `<dados>/documentos/` (sem `..`, links simbólicos ou caminho absoluto de fora).
- Só `pdf`/`docx`, tamanho limitado, tipo real conferido.
- O currículo tem telefone e endereço: nunca vai para o log nem para o diagnóstico; nome e `sha256` bastam. O pacote de diagnóstico continua sem documentos.
- A pré-visualização roda em `<iframe sandbox>` sem scripts; HTML de documento é escapado na geração.
- Texto de página nunca escolhe o arquivo.

## 12. Ordem de implementação

1. **A, backend da biblioteca:** migration, comandos da seção 6, importação segura, PDF por `printToPDF`, migração dos arquivos antigos, remoção dos comandos antigos.
2. **C, frontend:** sidebar, aba, grade, página do documento, tags, preferido, importar, gerar, Perfil limpo, seletores de variante. Em paralelo com A, sobre o contrato da seção 6.
3. **B, MCP:** tools da seção 7 e `browser_upload` (depende da tabela de A).
4. Integração, suíte completa, `claudia-teste` no app, revisão Opus. **Edição do prompt só com aprovação.**

## 13. Fora do escopo

- **Extrair texto de currículo importado** (PDF/DOCX) para o agente ler: depois, por decisão do usuário.
- **Vagas acumuladas em `candidatando`**: bug em aberto, outra frente. Enquanto não for resolvido, o upload (como o envio) recusa quando houver mais de uma.

- Editar o conteúdo do currículo importado; converter DOCX em PDF; vários arquivos por campo; anexar fora de uma vaga em `candidatando`.
- Gerar currículo adaptado por vaga: **não verificado** onde a skill `tailor-application` grava os arquivos. Se ela gerar fora da biblioteca, o agente precisará registrá-los (tarefa de investigação no plano).

## 14. Testes

- **Unidade:** regra de escolha em cada ordem; caminho fora da pasta, `..`, link simbólico e `id` inexistente recusados; tipo falso (extensão `.pdf` com outro conteúdo) recusado; `sha256` diferente recusa; apagar só dentro da pasta e por id; um preferido por (tipo, variante); tags.
- **Live (Chrome de teste, fixture local `upload-form.html`):** campo visível; campo escondido atrás de botão (padrão do LinkedIn); site que só aceita PDF; nome aparece depois; recusa sem vaga ativa; arquivo fora da biblioteca nunca anexável.
- **Mutação obrigatória** para cada regra de segurança e de escolha.
- **Frontend (`claudia-teste`):** abrir aba, grade, abrir página, Voltar, marcar preferido por variante, tags, importar, gerar, excluir preferido. Prints. Nenhuma ação em site de vagas.
