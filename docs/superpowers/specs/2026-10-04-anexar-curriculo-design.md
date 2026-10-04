# Spec: anexar currículo (e outros documentos) pelo agente

> Spec de design. Não é o plano de implementação. Complementa `2026-10-02-browser-cdp-motor-design.md` e `2026-10-03-mcp-leitura-robusta-design.md`.

**Data:** 2026-10-04
**Branch:** `feat/browser-cdp-mcp` (continuação)

## 1. Problema (verificado no código em 2026-10-04)

1. **O agente não tem como anexar arquivo.** Nenhuma das 9 tools `browser_*` envia arquivo; não há `setFileInputFiles` nem tratamento de seletor de arquivo no código. Todo formulário com "Anexar currículo" trava a candidatura.
2. **O currículo que a Claudia gera não é anexável.** `gerar_curriculo` (`commands/curriculos.rs`) escreve **HTML** em `<dados>/curriculos/cv_<template>_<data>.html`. Sites de vagas pedem PDF ou DOCX. A tela já avisa "Exportar CV — Em breve — requer template".
3. **O usuário não pode escolher outro currículo.** Não existe como importar um currículo próprio, nem escolher qual vale para qual vaga. O esquema de `search_variants.yaml` tem `cv_gerado_path` por variante, mas nada o preenche nem o usa.
4. **Risco de segurança se a tool receber um caminho:** texto malicioso numa página poderia induzir o agente a subir qualquer arquivo do computador do usuário (chaves, documentos). O caminho nunca pode vir do agente.
5. `apagar_curriculo(path)` apaga **qualquer** caminho recebido (sem checar que está em `curriculos/`). Não faz parte desta spec, mas o novo registro deve corrigir isso.

## 2. Objetivo

O agente anexa o currículo certo, e só um currículo que o **usuário** tem na biblioteca, sem conseguir subir arquivo arbitrário. O usuário vê, escolhe e importa os currículos no app. Nenhum passo de instalação novo.

## 3. Biblioteca de currículos (modelo)

Tabela `documentos` no banco principal (migration, não `CREATE TABLE` solto):

| Campo | Significado |
|---|---|
| `id` | inteiro estável; é o que o agente cita |
| `tipo` | `curriculo` (agora) ou `carta` (reservado, ver seção 9) |
| `nome` | rótulo exibido ("CV Backend PT", "Meu CV 2026") |
| `origem` | `gerado` (pela Claudia) ou `importado` (do usuário) |
| `arquivo` | caminho **dentro** de `<dados>/curriculos/`, nunca fora |
| `formato` | `pdf` ou `docx` (único que o agente pode anexar) |
| `idioma` | `pt` ou `en` (opcional) |
| `sha256`, `tamanho` | integridade; conferidos antes de cada anexo |
| `criado_em` | |

Regras de escolha (valem no código, não no prompt), em ordem:
1. **`politica_cv = fixo`**: o usuário marcou um currículo como "usar sempre este": é ele, e a Claudia não gera outro para a vaga.
2. **Vaga com currículo adaptado gerado para ela** (política `adaptado`, o padrão): o gerado para aquela vaga.
3. **Currículo da variante** da vaga (a variante ativa que casa com a vaga; campo `cv_id` na variante, no lugar do `cv_gerado_path` sem uso).
4. **Currículo padrão** do usuário.
5. Nenhum: a tool recusa e o prompt manda abrir uma pendência "sem currículo para anexar" (nunca inventar nem pular o campo).

O usuário sempre pode trocar: a escolha dele vence a geração automática.

## 4. Formato: PDF pelo nosso próprio Chrome

- O gerador atual continua produzindo HTML; a **exportação para PDF** usa o Chrome do app (`Page.printToPDF`, em uma aba própria e invisível ao agente, A4, fundo e CSS de impressão). Sem biblioteca nova e sem instalação: respeita a regra do projeto.
- `gerar_curriculo` passa a gravar o HTML **e** o PDF, e só o PDF entra na biblioteca como anexável. O HTML segue como pré-visualização.
- Importar: o usuário escolhe um `.pdf` ou `.docx` (seletor de arquivo do sistema). O app **copia** para `<dados>/curriculos/` (não referencia o original), valida a extensão **e** o tipo real do arquivo (cabeçalho `%PDF-` ou o ZIP de um docx), limita o tamanho (10 MB) e registra o `sha256`.

## 5. Tool `browser_upload`

- Parâmetros: `ref` (o campo de arquivo ou o botão que o abre, vindo de `browser_read`), `documento` opcional (`id` da biblioteca). **Nunca um caminho.**
- Sem `documento`, o app escolhe pela regra da seção 3. Com `documento`, só vale se for da biblioteca e do tipo certo.
- Mecânica (sem JS na página): se a `ref` é um `<input type=file>`, `DOM.setFileInputFiles` direto. Se é um botão que abre o seletor (LinkedIn), liga `Page.setInterceptFileChooserDialog`, clica e responde ao seletor com o arquivo. Resultado diz o nome do arquivo anexado e se a página mostrou o nome dele depois.
- Reconferir `sha256` e existência antes de anexar. Arquivo alterado ou sumido: recusa com mensagem clara.
- Passa pelo viewport mínimo, pelo `estado:` e pelo detector de loop como as outras tools.
- **Não é envio** (não passa pela guarda de envio), mas só funciona se houver **uma única vaga em `candidatando`** (a mesma regra da guarda): sem vaga ativa conhecida, recusa.
- Activity log: grava a chamada com `nome`, `formato`, `tamanho` e `sha256` do arquivo, **nunca o conteúdo**; print antes e depois como as demais.
- Falhas explícitas: campo não é de arquivo, tipo não aceito pelo site ("só PDF"), arquivo grande demais para o site. O agente recebe o motivo e abre pendência em vez de repetir.

## 6. Prompt (`prompt_sistema_runtime.md`)

Edição só para ensinar `browser_upload`: usar `browser_read` para achar o campo de anexo, chamar `browser_upload` sem escolher arquivo (a Claudia escolhe), conferir que o nome apareceu, e abrir pendência quando não houver currículo. Pede aprovação explícita do usuário antes de editar (regra do projeto).

## 7. Impacto no frontend

| Tela | Mudança |
|---|---|
| **Perfil → Currículos** (`CurriculosView.tsx`) | vira a **biblioteca**: lista gerados e importados com nome, origem, formato, idioma, tamanho, data. Ações: **Importar o meu** (PDF/DOCX), **Definir como padrão**, **Renomear**, **Abrir**, **Excluir** (com confirmação; não apaga o que está em uso por uma variante sem avisar). "Gerar" passa a produzir PDF, com aviso de que o HTML é só pré-visualização. Seção nova **Como a Claudia escolhe**: `adaptado` ou `fixo` + qual é o fixo. |
| **Perfil → Variantes** (`sections.tsx` / `ResumoView`) | em cada variante, seletor "Currículo desta variante" (lista da biblioteca; vazio = usa o padrão). |
| **Dashboard / vaga em andamento** (`Vagas.tsx`, painel "Now applying") | mostra o currículo anexado à vaga atual (ou "sem currículo"). |
| **Histórico** | cada candidatura mostra o currículo usado (coluna `documento_id` em `candidaturas`; hoje só há `pasta_arquivos`). |
| **Pendências** (`Pendencias.tsx`) | nova categoria "Sem currículo para anexar", com atalho para a biblioteca. |
| **Activity log** (`ActivityLogScreen.tsx`) | linhas de `browser_upload` mostram o nome e o formato do arquivo (nunca o conteúdo). |
| **Welcome** | passo "Perfil": avisa que é preciso ter ao menos um currículo (gerar ou importar) antes da primeira candidatura. |
| i18n | todos os textos novos em `pt.ts` e `en.ts`. |

Backend de suporte: comandos `importar_curriculo`, `listar_documentos`, `definir_documento_padrao`, `renomear_documento`, `apagar_documento` (apaga só dentro de `curriculos/`, por `id`, não por caminho), `definir_politica_cv`; `listar_curriculos`/`apagar_curriculo` atuais são substituídos.

## 8. Segurança

- O agente só cita `id`; o caminho vem do banco e é sempre `canonicalize`d e conferido dentro de `<dados>/curriculos/` (sem `..`, links simbólicos ou caminhos absolutos de fora).
- Só `pdf`/`docx`, tamanho limitado, tipo real conferido.
- Dado pessoal: o currículo contém telefone e endereço. Nunca vai para o log nem para o diagnóstico; o `sha256` e o nome do arquivo bastam. O pacote de diagnóstico continua sem currículos.
- Texto de página nunca escolhe o arquivo.

## 9. Fora do escopo

- **Carta de apresentação:** o mesmo mecanismo serve (`tipo = carta`, e `cover_letter.rs` já gera cartas), mas só entra num segundo passo depois que o currículo funcionar de ponta a ponta.
- Editar o conteúdo do currículo importado; converter DOCX em PDF; vários arquivos por campo; anexar fora de uma vaga em `candidatando`.
- Descobrir onde a skill `tailor-application` grava os arquivos adaptados por vaga: **não verificado**. A seção 3, item 2, depende disso e precisa de uma tarefa de investigação no plano (se a skill gerar fora da biblioteca, o agente precisará de um passo que a registre).

## 10. Testes

- **Unidade:** regra de escolha em cada ordem (fixo > adaptado > variante > padrão > nenhum); caminho fora da pasta, `..`, link simbólico e `id` inexistente recusados; tipo falso (extensão `.pdf` com conteúdo de outro tipo) recusado; `sha256` diferente recusa; apagar só dentro da pasta.
- **Live (Chrome de teste, fixture local `upload-form.html`):** campo `<input type=file>` visível; campo escondido atrás de botão (o padrão do LinkedIn); página que só aceita PDF; o nome do arquivo aparece depois; recusa sem vaga ativa; o agente não consegue anexar um arquivo fora da biblioteca nem por `ref` nem por `documento` forjado.
- **Mutação obrigatória** para cada regra de segurança e de escolha.
- **Frontend (claudia-teste):** importar, definir padrão, escolher por variante, "usar sempre este", excluir em uso; prints. Nenhuma ação em site de vagas.
