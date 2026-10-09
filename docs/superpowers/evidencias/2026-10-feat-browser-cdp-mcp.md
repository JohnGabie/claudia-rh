# Evidência da branch `feat/browser-cdp-mcp`

> **Estado em 2026-10-08, HEAD `45172f9` (antes das correções da revisão final, Tarefa 9); a seção 8.1 foi preenchida com a rodada do Sandbox sobre o HEAD `22c2854`.** Este arquivo é a prova da branch dentro do repositório (a pasta `.superpowers/`, onde ficam os ledgers, não é versionada). Ele separa três coisas: o que **já está provado** (seções 1 a 6), o que foi **provado só no laboratório do Sandbox** (seção 7) e o que está **pendente** (seção 8). O que não foi rodado está escrito como "pendente" ou "não verificado", sem número inventado.

Fontes: os ledgers `.superpowers/sdd/2026-10-02-browser-cdp-motor/progress.md` e `.superpowers/sdd/2026-10-04-anexar-curriculo/progress.md` (fora do git; só caminhos relativos aqui, nenhum dado pessoal copiado), `docs/pesquisa/2026-10-02-validacao-execucao.md`, `docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md`, `.claude/skills/claudia-teste-sandbox/references/limites.md` (itens 13 a 17) e o `git log dev..HEAD`.

Sobre os hashes: a branch foi reescrita em 2026-10-05, então os hashes dos ledgers e dos relatórios dos agentes não existem mais. Os hashes abaixo são os **atuais**, achados pelo assunto do commit (`git log --format='%h %s' dev..HEAD`). Quando um commit atual reúne vários do ledger, a linha diz isso.

Regra de leitura: "mutação" = quebrar o código de propósito, ver o teste **nomeado** falhar (linha `FAILED`) e restaurar. "Relatada" = o agente ou o orquestrador escreveu que fez; a linha `FAILED` colada só existe onde o ledger a cola.

## 1. Resumo

| Ciclo | Implementado | Revisão Opus | Rodada completa verde depois da última mudança | Pendente |
|---|---|---|---|---|
| Motor do browser (CLA-31) | sim | final de 2026-10-02 deu FAIL; correções feitas; **revisão nova pendente (Tarefa 9)** | último verde completo: 2026-10-02 (326 unitários, 42 live) | seção 8 |
| Leitura robusta do MCP | sim | misturada na do motor | idem | mutações nunca coladas (seção 3) |
| Anexar currículo, Parte 1 | sim | sim (3 Important corrigidos) | 2026-10-04 (433 unitários, 65 live) | seção 8 |
| Anexar currículo, Parte 2 | sim | sim (5 Important corrigidos nas frentes K, L, L2 e M) | 2026-10-05 (499 unitários; live 67 de 68 na Onda 2, 70 de 70 na Onda 3 na segunda tentativa) | revisão das correções K, L, L2, M nunca feita |
| Fechamento (auditoria, guarda de envio, eventos do browser, diálogos, CLA-40) | sim | **pendente** | **pendente** | seção 8 |

Nada do que está na coluna "pendente" foi dado como verificado. O único verde recente e completo de `cargo test --lib` é o do laboratório (seção 7), com a ressalva de lá.

## 2. Ciclo do motor do browser

Ledger: `.superpowers/sdd/2026-10-02-browser-cdp-motor/progress.md`. Plano: `docs/superpowers/plans/2026-10-02-browser-cdp-motor.md`.

| Tarefa | Commits atuais | Testes e mutações (do ledger) | Revisão |
|---|---|---|---|
| 0.1 a 0.6, fase 0 (versão e SHA-256 fixados, tema laranja, Chrome for Testing contra Chrome comum) | `7325c8c`, `7cfc99f`, `046e7ed`, `d8db514`, `da9bc15` | hash conferido à parte (`Get-FileHash`); licença do binário **não estabelecida** (CLA-34) | Opus: CONDITIONAL PASS; usuário escolheu manter o Chrome for Testing |
| 1.1 módulo e caminhos | `714cfe2`, `47e4491` | 130 testes; pin com mutante corrigido; clippy 3 avisos (baseline) | PASS |
| 1.2 instalação | `ed137eb`, `3947a85`, `46ebb9b`, `9a9cd56`, `63a2c21` | reescrita pelo orquestrador: 24 testes, mutações M1 a M9, cada uma morta por teste nomeado | FAIL na 1a rodada (sem timeout, corrida entre instâncias); corrigido |
| 1.3 cliente CDP | `79d695c`, `be0cf7f`, `75a2611`, `9b135f8` | mutações C1 a C5 e C7 mortas; **C6 (corrida na reconferência de `is_closed`), dreno de falha do escritor e vazamento no drop sem cobertura** | FAIL na 1a rodada; corrigido |
| 1.4 refs da árvore de acessibilidade | `a4a3504` | RED 6; 5 mutações mortas; 175 testes | revisão de fase |
| 1.5 guarda de envio | `2d44ef9`, `de46afd`, `3dfdb8f`, `2396579`, `86ac280` | 8 mutações com `FAILED` colado; sonda do orquestrador: 25 de 25 nomes adversariais pegos, 0 de 14 falsos positivos | Opus de segurança: FAIL; corrigido |
| 1.6 tema e idiomas | `4150aeb`, `066b322` | 3 mutações mortas; teste live de `navigator.languages` (pular a chamada falha) | revisão de fase |
| 1.7 gestor (lançar, reusar, sondar) | `3bee160`, `516711e`, `d20085a`, `c48939a` | 9 mutações G1 a G9; fornecedor de lançamento e kill injetáveis | revisão de fase: PASS com condições; seguimento feito (`c48939a`) |
| 2.1 harness live | `c898c02` | 5 testes live; a asserção do escopo de diálogo agora exclui o que está fora do modal | sem revisão própria |
| 2.2 e 2.3 tools MCP | `1ee2c96`, `e8a0240`, `b438570` | 29 live, 15 de 16 mutações mortas; depois da revisão: 272 unitários, 37 live, smoke do MCP ok | Opus de segurança: FAIL; corrigido (alvos calculados pelo browser, mundo isolado, falha fecha) |
| 2.4 ciclo de vida (serviço, vigia, desligamento) | `4d17de3`, `2d5faa3` | 11 mutações no gestor, 8 no serviço; 295 unitários | **revisão da Fase 2 pulada** (Opus discordou); a revisão final da Tarefa 9 deve cobrir |
| 3.1 a 3.5 Activity log (banco, prints, retenção, navegação, tela) | `1dafabf`, `74eab01` | 15 de 17 mutações mortas (2 equivalentes); rastreador 6; gravação 9; 324 unitários, 42 live | Opus de segurança na redação |
| 4.1 aviso de preparo | `8d897dd` | tela | sem revisão própria |
| 4.2 limpeza própria na desinstalação | cortada | decisão do usuário (D5, 2026-10-07): fica só a caixa nativa do instalador | n/a |
| 5.1 cortar a extensão (`--no-chrome`) | `beaeb33`, `89f5242`, `1e44fdd` | proposta em `dbaa7c5`, aplicada em `beaeb33` | revisão Opus dos achados em `1e44fdd` |
| 6.1, 6.2 e 6.4 validações | `4d47baf`, `a5ed059` | `docs/pesquisa/2026-10-02-validacao-execucao.md` (rodadas A, B, C **uma vez**; Activity log: 22 registros, 20 imagens, 0 órfãos) | final de 2026-10-02: FAIL (Fase 5 e corte da 4.2); resolvido por D5 e por `beaeb33` |
| Depois do corte | `4ff26b3` (instância única), `027996d` (browser só quando a IA é chamada), `919d02b` (ícone), `3605a07` e `c10e1f2` (Activity log sempre ligado, tela própria) | testes na suíte | revisão pendente (Tarefa 9) |

Não verificado no ciclo do motor (do ledger e da validação): LinkedIn real e envio (6.3), persistência do login entre duas aberturas, Chrome do app com sandbox, desligamento do Chrome ao fechar o app pela janela (só testado por unidade), desinstalação real, rodadas 6.2 A, B e C uma segunda vez, e os cenários 4 a 7 de 6.4 (retenção com linhas de 40 dias, dois processos MCP, `dev/` só leitura, viewer).

## 3. Leitura robusta no MCP

Spec: `docs/superpowers/specs/2026-10-03-mcp-leitura-robusta-design.md` (sem plano nem ledger próprio; a prova está no ledger do motor).

| Item | Commits atuais | Prova |
|---|---|---|
| spec | `ccc187b` | n/a |
| 3.1 viewport mínimo 1200 por 800 | `e6b3a0a`, `f0f4f33`, `0cb8580` | testes de `viewport_live.rs` (5) |
| 3.2 `browser_text`, 3.3 linha `estado:`, 3.4 detector de loop, 3.5 `browser_scroll` | `381d21a`, `e32ae46` | testes unitários e live; `233629a` fixa as linhas `estado:` e de conteúdo não confiável no teste do prompt, "mutation-proofed" **sem linha `FAILED` colada** |
| 3.7 correções da revisão | `392602f`, `83b8341`, `10c1a07`, `5cbf9c9`, `e4cdbaf`, `395ad4f`, `70036a2` | testes live novos para viewport via tool, `click_at` depois de redimensionar, texto forjado e a roda |

Mutações desta parte **nunca registradas com `FAILED`**: viewport mínimo, `browser_text` sem valor digitado, `login_wall`, limiar do detector de loop, `browser_scroll` em contêiner. Pendentes (Tarefa 4, Step 3).

## 4. Anexar currículo, Parte 1 (upload)

Ledger: `.superpowers/sdd/2026-10-04-anexar-curriculo/progress.md`. Spec: `docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md`.

| Tarefa | Commit atual | Testes novos e mutação (do ledger) |
|---|---|---|
| 1 tabelas e armazém | `8ea8d59` | 412 unitários; mutação `DO NOTHING` falha `one_preferred_per_type_and_variant_the_last_mark_wins` |
| 2 resolver id em arquivo só dentro da pasta, só se inalterado | `dba09f3` | 417; guardas desligadas falham `a_name_that_escapes_the_folder_is_refused`; conferência do sha desligada falha `edited_file_is_refused` |
| 3 importar PDF e DOCX do usuário | `9358f90` | 424; cabeçalho desligado falha `a_fake_pdf_is_refused`; limite de tamanho desligado falha `a_file_over_ten_megabytes_is_refused` |
| 4 regra de escolha | `a7c9365` | 428; ordem trocada falha `the_variant_preference_wins_over_everything`; filtro de origem desligado falha `without_preferences_the_newest_generated_for_that_variant`; um teste do plano foi reforçado porque a mutação sobrevivia |
| 5 `browser_upload` | `3093bd7` | live 63; guarda de envio desligada falha `upload_refuses_a_submit_button_and_never_clicks_it`; vaga ativa desligada falha `upload_without_an_active_vaga_is_refused`; interceptação não liberada falha `after_an_upload_the_file_chooser_is_no_longer_intercepted` (teste novo, a mutação sobrevivia) |
| 6 ligar à app e ao MCP | `ac55b99` | 431 unitários, live 63, viewport 5, smoke do MCP "TUDO OK"; vaga ignorada falha `picks_by_the_vagas_variant_and_resolves_safely` |
| 7 prompt (aprovado pelo usuário) | `4432173` | prompt sem `browser_upload` falha `the_shipped_runtime_prompt_no_longer_sends_the_agent_to_the_extension` |
| 8 rodada de agente e smoke | `91dcb2b` (correções da revisão final) | rodada Haiku pelo MCP real com observador Opus (o "achado crítico" do Haiku era erro do script dele); smoke de leitura no app real; **importar e marcar preferido no app real NÃO feitos** (escreveria no banco real); **site real NÃO feito** |
| revisão final (Opus) | `91dcb2b` | 3 Important corrigidos: nome do arquivo legível para o site (`the_site_receives_the_documents_name_with_the_same_bytes`), alarme falso em campo nativo sem eco (`a_plain_file_input_without_an_echo_is_not_reported_as_rejected`, `a_file_field_accept_rule_is_read_like_the_browser_does`), ref inválida explicada (`an_html_id_instead_of_a_ref_gets_a_message_that_explains_refs`); 433 unitários, live 65, viewport 5 |

Falhas isoladas anotadas pelo ledger: uma em `browser_live` ("browser unreachable") e uma em `viewport_live` (`a_wide_but_short_window_grows_in_height_only`, 1 de 6), ambas coincidindo com o disco cheio da época. Viraram CLA-36 (viewport) e, na Parte 2, CLA-38.

## 5. Anexar currículo, Parte 2 (biblioteca, PDF, aba, migração)

Hashes atuais, achados pelo assunto. As mutações são as relatadas pelos agentes em 2026-10-04 e 2026-10-05, que **nunca foram para o ledger**; a coluna é o teste que falhou.

| Frente | Commits atuais | Mutações (teste que falhou) |
|---|---|---|
| A, PDF pelo WebView2 | `6a708b5`, `41dbbbb` | `a_failed_pdf_registers_nothing_and_leaves_no_files`, `an_invalid_pdf_is_refused` |
| B1, variante da vaga | `6d28a89` | `without_a_vaga_variant_the_principal_is_used`, `an_unknown_variant_is_refused_and_nothing_is_written`, `marking_a_principal_unmarks_the_others` |
| B2, comandos da biblioteca | `f16f6a3` | `deleting_never_touches_a_file_outside_the_folder`, `the_preview_is_read_only_from_inside_the_folder`, `a_zip_renamed_docx_is_refused`, `an_existing_destination_is_never_overwritten`, `schema_creates_all_tables`, `deleting_removes_its_files_and_its_preferred_marks_only` |
| C, aba Currículos | `1c5067d` | `ehPreferido`, `filtrar`, sandbox do `FolhaA4` (`node --test`, falhou) |
| D, migração dos arquivos antigos | `dc295d3` | `running_twice_migrates_once`, `old_cvs_and_letters_become_documents_and_the_originals_stay`, `a_half_done_migration_is_retried`, `a_pending_letter_from_the_agent_becomes_a_document` |
| E, tools MCP de documentos e variantes | `fc8d36c`, `34f8762`, `e9f2ac5`, `6897e41`, `4671efc` | 8 mutações, entre elas `update_variante_is_refused_in_the_execution_session`, `reads_never_return_a_path`, `no_document_tool_returns_a_path`, `marking_preferred_is_a_profile_session_write`, `the_execution_session_cannot_write_variants_or_preferred_documents` |
| F, resultado do upload | `872873f` | `a_failed_upload_is_logged_with_its_target`, `an_upload_result_ends_with_the_state_line`; do orquestrador: `the_upload_message_and_the_log_use_the_same_size_unit` |
| G, anexos | `23b5eb9` | `history_names_the_document_used`, `the_candidatura_remembers_the_cv_that_was_attached`, `the_current_vaga_lists_its_latest_attachments`, `an_attachment_is_recorded_for_the_vaga`, `the_upload_line_in_the_activity_log_names_the_document_and_its_id` |
| H, telas (Dashboard, Histórico, Pendências, Activity log, Welcome) | `8b09177`, `7501803` | 4 mutações em `tests/front/onde-aparece.test.ts` (falharam) |
| I, prompt | `3751566`, `0c840ae` | teste do prompt sem `set_vaga_variante`; categorias `sem_curriculo` e `sem_carta` (um primeiro rascunho usava `sem_documento`, contra a decisão P6 do usuário, corrigido com teste que o proíbe) |
| J, anexo só no sucesso | `b96bfa7` | `successful_upload_calls_attached_with_vaga_type_and_document_id`, `failed_upload_does_not_call_attached` |
| K, migração concorrente | `551779f` | do orquestrador: `a_migrated_document_the_user_deletes_does_not_come_back`, `two_concurrent_migrations_register_each_file_once_and_keep_the_files` (3 de 3) |
| L, filtro de perfil e prompt P4 | `be0491b`, `28e0418` | `get_search_variants` sem `cv_gerado_path`; regra P4 no prompt do Perfil |
| L2, recarga da aba | `0be9a2c`, `b7b504e` | `unmounting before listen resolves still releases the late subscription`, `a burst of events reloads once` (falharam) |
| M, `update_variante` | `9eaf3ec`, `6b043ae` | `update_variante_changes_only_the_requested_field`, `update_variante_preserves_unknown_keys_everywhere`, `the_screen_single_variant_save_and_weights_keep_the_rest`, `the_screen_whole_file_save_keeps_what_the_struct_does_not_model` |

Reruns do orquestrador (ledger): Onda 1 (`cargo test --lib` 458, `browser_live` 65, `viewport_live` 5, smoke "TUDO OK", `tsc` 0); Onda 2 (484 unitários; `browser_live` 67 passou e 1 falhou, `the_wheel_does_not_change_a_focused_number_field`, só na suíte inteira: CLA-38); Onda 3 (491 unitários; live 68 de 70 na primeira vez, com 2 testes de guarda falhando ao **lançar** o Chrome e 2 Chromes de teste órfãos: CLA-39; 70 de 70 e viewport 5 de 5 na repetição); correção K e L (499 unitários).

Achados da revisão Opus da Parte 2 que só existiam na conversa:
- **Minors (7):** janela de impressão do PDF com JavaScript ligado (risco aceito, G6 de 2026-10-08); `PrintToPdf` pode gravar depois do timeout; anexo gravado mesmo com `accept` recusando o formato; Histórico não distingue "documento apagado" de "nunca anexado"; atalho das Pendências não abre a sub-aba Cartas para `sem_carta`; arquivo antigo cujo PDF falha é tentado de novo a cada `save_cover_letter` (espera de 45 s); resposta de `update_variante` com nome de arquivo (`.bak-1`).
- **Declined to judge (6):** `register_candidatura` ainda recebe `pasta_arquivos`; `guardar_variante_unica` sem backup; `apagar_documento` sem `forcar`; texto de `get_documento` pode ir para o `pty-tail.log`; PDF dentro do app real (ainda pendente, seção 8); spec 2.3 não implementada (correto, saiu da branch).
- **Minors da Parte 1 adiados:** releitura do tamanho e `create_new` na importação; qualquer ZIP renomeado `.docx` é aceito; janela entre a conferência do hash e a leitura preguiçosa do Chrome; conexões de `commands/documentos.rs` sem `PRAGMA foreign_keys` (o SQLite embutido já liga por padrão; o teste `open_conn_turns_foreign_keys_on` fica como guarda); teste de esquema sem `documentos`/`preferidos`; estrela do `ParaAnexar` local; comandos de documento síncronos (hash de 10 MB na thread principal); resultado sem a linha `estado:` e sem a variante usada; upload com falha depois da ref sem alvo no log.

As correções K, L, L2 e M vieram depois da revisão da Parte 2 e **nunca tiveram revisão independente nova** (Tarefa 9).

## 6. Fechamento da branch (2026-10-06 em diante)

| Item | Commits | Estado |
|---|---|---|
| Auditoria de deleção | `316a088` (`docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md`) | 7 `BLOQUEIA` (B1 a B7) achados; decisões do usuário em `78e8c4b` e `05c1254` |
| Regras duras: nenhum caminho real em teste, tudo que apaga só no Sandbox, nunca o Chrome pessoal | `0ce8cfb`, `9805ffe`, `05c1254` | em `CLAUDE.md` |
| Correções B1 a B6 (travas das skills de teste) e P1/P3 (versão do browser, pasta de dados do MCP) | `595bada`, `db06c88`, `9418510`, `0cdd31a`, `f359672`, `10931d8`, `86cc713`, merges `4245c6b` e `04de093` | integradas; **a revisão Opus final que confirma "nenhum `BLOQUEIA` sobrou" não está registrada aqui: pendente**. B7 (kill injetado em `prepare`): **feito**, confirmado no código de `browser/service.rs` (`prepare` recebe o kill por parâmetro; a produção passa `manager::kill_tree` e os testes passam `|_| {}`); commit não localizado pelo assunto |
| D6 guarda de envio, opção C (CLA-37) | `0dfe876`, `1f0119d`, `0021fbf`, `cebb58c`, `14e90a6` | implementada; testes live **pendentes no Sandbox** |
| D7 eventos do próprio browser no Activity log (11 eventos) | `7e9ab91` | implementado; teste live e tela **pendentes** |
| D8 diálogos nativos (política P2) e queda de aba | `bfa5aa6`, `0102e81` | implementado; o commit `0102e81` diz "pending Sandbox": testes live **pendentes** |
| D9 JavaScript na janela do PDF | nenhum | **decisão G6: deixar como está** (risco aceito: a janela não tem acesso ao app) |
| D10 CLA-40 (carta sem nome) | `8f5c5e2`, `848810f`, `76f6992`, `7e5ec14` | implementado; CLA-35 **ignorado nesta branch por decisão do usuário** |
| D11 protótipo Playwright fora da branch | `309dc56` | feito; o histórico antigo continua com os caminhos pessoais |
| Laboratório do Sandbox dentro do repositório e ambientes declarativos | merges `e242f28`, `486007b`, `f009c91`, `2097b3a`, `2836146`, `966e6c0`, `5963ea1`, `33e0627` e as correções até `a8f9f24` | seção 7 |
| Tarefa 1 (CLA-39, CLA-36, CLA-38: harness live) | `c048439`, `b71317d`, `52f90f6` | Step 5 rodado no Sandbox em 2026-10-08 (seção 8.1): CLA-38 e CLA-39 ok; **CLA-36 falha no guest** (janela limitada a 800×600); **o perfil de teste não é apagado** (achado novo) |

## 7. Provado no laboratório do Sandbox (rodada de 2026-10-08)

Fonte: `.claude/skills/claudia-teste-sandbox/references/limites.md`, itens 13 a 17. Tudo abaixo rodou **dentro do Windows Sandbox**, no disco `Z:` do guest (VHD do próprio guest), nunca no PC. A rodada usou os scripts do laboratório e o projeto montado no guest; o commit exato do projeto em teste **não está registrado** em `limites.md` (a reverificação das correções do laboratório foi em `ab21b0e`), então os números de `cargo test` abaixo valem para "o projeto no momento da rodada", não para o HEAD `a8f9f24`.

| O que | Resultado | Item |
|---|---|---|
| Disco `Z:` no guest | formatado pelo `diskpart` sobre o arquivo do VHD, NTFS, `LABZ`, 21,47 GB; 3 subidas seguidas (`testes`, `app-winapp`, `app-winapp -DadosReais`) com `disco-z.json` `ok: true`, 6,1 a 6,4 s; escrita e leitura em `Z:` conferidas; o Node do guest resolve `Z:\claudia-teste-sandbox` sem trocar de letra | 13 |
| Layout offline do MSVC | construído dentro do guest (669 arquivos, cerca de 2,3 GiB, 68,7 s) e importado no PC com `cache-up.ps1 -ImportarMsvc`; segunda importação recusada (`already in the cache`) | 14 |
| Instalar o MSVC do layout no ambiente `testes` | funcionou, `cl.exe` 14.44.35207; **3 h 08 min** (11.296,7 s); o gargalo é o Windows SDK (cerca de 66 pacotes `.msi`, 2 min cada), não o compilador | 14 |
| `npm ci` e `npm run build` no guest | exit 0 em 7,7 s e 7,5 s | 17 |
| **`cargo test --lib` no guest** | **exit 0 em 138,4 s (compilação do zero 2 min 10 s): 578 passed, 0 failed, 1 ignored** | 17 |
| **6 arquivos de teste de Sandbox em `Z:`** | `comum.test.mjs` 21 de 21, `novo.test.mjs` 2 de 2, `mock.test.mjs` 1 de 1, `copia.test.mjs` 5 de 5, `publicar.test.mjs` 7 de 7 (todos exit 0, 0,3 a 1 s) e `travas.sandbox.ps1` com 26 ok e 0 `FAILED` na reverificação (na primeira rodada 25 ok e 1 `FAILED`, `credential: a second copy replaces the first`, causa e correção no item 17) | 17 |
| Todas as pastas dos testes ficaram em `Z:\claudia-teste-sandbox\`; `Z:\tmp` vazio no fim; nenhuma pasta de teste em `C:` do guest | verificado | 17 |
| Firewall do guest | `guest-firewall.ps1` como System, exit 0; `firewall.txt` com `INTERNET: BLOQUEADA` e `storage.googleapis.com: BLOQUEADO` antes de qualquer `guest-run`; a aceitação do arquivo passou a depender do id desta subida (`BOOTID`) | 15, 17 |
| Montagem dos dados reais somente leitura (`-DadosReais`) | `ReadOnly=true`; criar arquivo no mount foi negado no guest | 15, 17 |
| **Cópia de dados reais (cenário `copia`) com a pasta de origem intacta** | `cenario-no-guest.mjs --cenario copia --autorizado` exit 0 contra o `diagnostics\latest` real; 158 arquivos na cópia e 160 na origem, a diferença é exatamente `claudia_rh.db-wal` e `.db-shm`; 0 links, `cenario.json` presente, `disparo.json` com `ativo: false`. **Origem intacta:** a listagem da pasta de dados no PC (caminho relativo, tamanho, data UTC e atributos, sem abrir arquivo) deu 253 itens antes, durante e depois do `test-down`, 0 diferenças | 17 |
| Cenário parcial recusado | `guest-run.ps1 -Cenario` sobre pasta sem `cenario.json` gravou o erro, não copiou dados e a pasta parcial ficou intacta; toda recusa do `guest-run` agora sai com 1 | 17 |
| Busca de token | `conferir-token.mjs` sobre as saídas do laboratório e do guest: exit 0, 55 arquivos, `achados: []` | 17 |
| `conferir-repo.mjs` | exit 0 (401 arquivos, 0 proibidos, 0 caminhos, 0 tokens) em `ab21b0e` | 17 |
| `test-down` | exit 0, `wsb list` vazio, nenhuma cópia da credencial no PC | 17 |

**Não verificado nessa rodada (e ainda não verificado hoje):**
- `claude -p` no guest respondendo de verdade: o guest alcançou o servidor e foi reconhecido, mas a conta estava no limite de sessão (`You've hit your session limit`), então a **resposta não foi verificada**;
- o risco da renovação do token (A4): o usuário trocou de conta no PC depois da subida e o `claude -p` não rodou no PC, então a comparação não prova nada;
- a subida real com o `BOOTID` (id gerado no guest, `firewall.txt` novo com a primeira linha, `guest-run` aceitando e recusando um `firewall.txt` de outro Sandbox): só testes puros e estáticos;
- qual commit do projeto o `cargo test --lib` de 578 testes cobriu;
- Chrome de teste dentro do guest: o `compose.json` declara o Chrome em `shared/tools/chrome/`, mas nenhuma rodada de teste live (`browser_live`, `viewport_live`) foi feita no guest (feita depois, em 2026-10-08: seção 8.1).

Próximo ciclo do laboratório (decidido, não feito): o MSVC leva 3 h no ambiente `testes` por causa do SDK; a saída é um MSVC/SDK enxuto pré-instalado no cache, e um ambiente `testes-node` (só Node) para o que não precisa compilar.

## 8. Pendente: rodadas que ainda vão rodar (Tarefas 4, 5 e 6 do plano)

Todas rodam **no Sandbox**, pela regra de 2026-10-07 (nenhum teste que apaga roda no PC). No PC ficam só compilar, clippy, `tsc`, build e os testes de front sem disco. Cada linha abaixo será preenchida com comando, código de saída e números. Enquanto estiver "pendente", **não há número**.

### 8.1 Tarefa 4: rodadas completas sobre o HEAD

| Rodada | Resultado |
|---|---|
| `npx tsc --noEmit` | pendente (roda no PC, pelo orquestrador) |
| testes de front (`tests/front/*.test.ts`, lista explícita) | pendente (roda no PC, pelo orquestrador) |
| `npm run build` e conferência de que nenhum mock vai para o bundle | pendente (roda no PC, pelo orquestrador); no guest, `npm run build` exit 0 (abaixo) |
| `cargo test --lib` no HEAD `22c2854` | **exit 0, 580 passed, 0 failed, 0 ignored**, 20,8 s (Sandbox) |
| `cargo clippy --all-targets` contra o baseline de 3 avisos | exit 0; os 3 do baseline + **2 novos** (abaixo) |
| suíte live `browser_live` e `viewport_live`, três vezes | depois das correções (abaixo, HEAD `70d16c5`): **exit 0 duas vezes seguidas, 93/93 e 6/6**. Na rodada original: **exit 101 nas três**: `browser_live` 90/92, 90/92 e 89/92; `viewport_live` 2/5 nas três (rodado à parte). Detalhe abaixo |
| 3 testes de ícone (janela visível), uma vez | **exit 0, 3 passed** |
| smoke do MCP (`tests/mcp-browser-smoke.mjs`) | **exit 0, última linha `TUDO OK`** (32 verificações) |
| mutações da leitura robusta (viewport, texto sem valor digitado, `login_wall`, loop, scroll em contêiner) | **feitas**, com as das Tarefas 1 e 3 e das correções: 13 de 13 falham no teste nomeado e voltam a passar restauradas (abaixo) |
| testes live novos de D6, D7 e D8 (guarda de formulário, eventos do browser, diálogos, queda de aba) | depois das correções: **todos passam** nas 2 rodadas finais. Na rodada original: **2 falham nas três rodadas** (`a_click_at_inside_a_shadow_root_finds_the_submit_button_and_its_form`, `a_confirm_during_a_send_the_guard_approved_is_accepted_and_only_then`); os demais passam |

#### Rodada de 2026-10-08 no Sandbox (HEAD `22c2854`, ambiente `testes`)

Tudo rodou **dentro do Windows Sandbox**, nada no PC. Ambiente `testes` com as ferramentas do PC montadas só leitura em `C:\hosttools` (node v24.17.0, cargo 1.95.0, `cl.exe` 14.44.35207). Projeto copiado de `C:\labproject` para `C:\labdata\project` pelo `guest-build.ps1`; `CARGO_HOME` e `CARGO_TARGET_DIR` em `C:\labdata`; `TEMP`/`TMP` = `Z:\tmp`; `CLAUDIA_TEST_CHROME=C:\lab\tools\chrome\chrome-win64\chrome.exe` (Chrome for Testing 154.0.8037.92 do laboratório). Cada comando rodou por `wsb exec -r ExistingLogin` num script da rodada (`.sandbox-work\guest-rw\t4\`, fora do git), com saída e exit em arquivo.

- **Compilação (`guest-build.ps1 -Precisa cargo`):** `npm.cmd ci --no-audit --no-fund` exit 0 em 18,4 s; `npm.cmd run build && cd src-tauri && cargo test --lib --no-run` exit 0 em 243,7 s. A primeira tentativa, com `wsb exec ... -File guest-build.ps1 -Install '...'`, não rodou nada: o `-File` não respeita aspas simples e o `-Install` chegou como `'npm.cmd` (exit 1 em 0 s). Com `-EncodedCommand` funcionou.
- **`cargo test --lib`:** exit 0 em 20,8 s, `580 passed; 0 failed; 0 ignored` (a Correção C da Tarefa 3 tirou o único ignorado).
- **`cargo clippy --all-targets`:** exit 0 em 127,4 s. Do baseline (`clippy-baseline.txt` do ledger do motor, só `--lib`): `new` e `write_both` nunca usados (`diagnostics/mod.rs`), `LASTINPUTINFO` (`idle_watcher.rs`). **Novos:** `redundant guard` em `src/browser/pagestate.rs:117` (código de 2026-10-03, desta branch, no alvo lib) e `items after a test module` em `src/prompt.rs:16` (código anterior à branch; só aparece no alvo de teste, que o baseline não cobria). Não corrigidos (nenhum código foi editado nesta rodada): **registrar no Linear**.
- **Suíte live, três vezes** (`cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon`): exit 101 nas três (295 s, 226 s, 281 s). O cargo para no primeiro binário que falha, então o `viewport_live` não rodou nesse comando; rodou à parte, três vezes (`cargo test --test viewport_live -- --ignored --test-threads=2`, exit 101, 16 a 18 s).
  - `browser_live`: rodada 1 `90 passed; 2 failed`, rodada 2 `90 passed; 2 failed`, rodada 3 `89 passed; 3 failed`. Falham nas três: `a_click_at_inside_a_shadow_root_finds_the_submit_button_and_its_form` (`unwrap_err()` em `Ok`: `"Clicou em (600, 469.5)."`, o `browser_click_at` sobre o shadow root não foi recusado) e `a_confirm_during_a_send_the_guard_approved_is_accepted_and_only_then` (com `confirm_submit` válido, o app respondeu `Cancelar` ao `confirm` "Enviar?"). Só na rodada 3: `a_tool_call_restores_the_minimum_viewport_and_says_so` (a resposta não trouxe `viewport ajustado:`).
  - **CLA-38** (`the_wheel_does_not_change_a_focused_number_field`): ok nas três. **Tarefa 3** (`a_file_the_field_refuses_is_not_recorded_as_attached`): ok nas três; `failed_upload_does_not_call_attached` e `scroll_with_a_ref_moves_that_elements_own_container_and_says_when_it_is_at_the_limit` também.
  - **CLA-36** (`a_wide_but_short_window_grows_in_height_only`): **falha nas três**, junto com `a_shrunk_window_is_corrected_to_the_minimum` e `an_already_big_window_is_left_exactly_as_it_is` (2 de 5 passam). Mensagens: `ViewportReport { before: (762, 428), after: (767, 439), adjusted: true, below_minimum: true }` e `the height reaches the minimum: (767, 439, 800, 600, "maximized")`. No guest a janela do Chrome de teste não passa de **800×600**, nem maximizada, então o mínimo de 1200×800 é inalcançável ali. Parece limite da tela do Sandbox (sessão aberta por `wsb connect` com janela oculta), **não verificado**; na rodada 3 o `a_tool_call_restores_the_minimum_viewport_and_says_so` caiu pela mesma razão. CLA-36 continua **sem prova verde**.
  - **CLA-39 / Chrome órfão:** depois de cada rodada (e de ícones, smoke e mutações), `Get-CimInstance Win32_Process` no guest: **0** `chrome.exe` com `claudia-live-` ou `claudia-smoke-` na linha de comando (11 `chrome.exe` no total, todos do Chrome que o `bootstrap.ps1` do laboratório abre).
  - **Achado novo: o perfil de teste nunca é apagado.** Pastas `claudia-live-*` deixadas em `Z:\tmp`: 89 depois da rodada 1, 178 depois da 2, 266 depois da 3 (cerca de uma por teste que usa `launch()`), 298 no fim; cada uma com o perfil Chrome inteiro (163 a 183 arquivos, cerca de 3,8 MB). Diagnóstico com `--nocapture` num teste só (`a_file_the_field_refuses_is_not_recorded_as_attached`): nenhuma linha `refusing to delete` nem `not killing pid`, e a pasta continuou. Ou seja: as travas aceitaram o caminho e o kill rodou, mas o `remove_dir_all` falhou em silêncio (`let _ =` em `tests/support/mod.rs:63`). Hipótese **não verificada**: o Chrome ainda segura arquivos logo depois do `taskkill /F`. No PC isso encheria o `%TEMP%`. A smoke (`claudia-smoke-*`) apagou a própria pasta. **Registrar no Linear.**
- **Ícones** (`cargo test --test browser_live -- --ignored icon --test-threads=1`): exit 0 em 10,9 s, `3 passed`.
- **Smoke MCP:** `cargo build` exit 0 em 92,6 s; `CLAUDIA_ALLOW_LOCAL_URLS=1 node tests/mcp-browser-smoke.mjs C:\labdata\cargo-target\debug\claudia-rh.exe` exit 0 em 7 s, 32 linhas `OK`, última linha `TUDO OK`.
- **Mutações** (cada uma só na cópia do guest; aplicada por regex com exatamente 1 ocorrência; restaurada copiando o arquivo de `C:\labproject`, SHA-256 igual ao do PC; o teste nomeado rodado de novo):

| Mutação | Teste nomeado | Linha `FAILED` |
|---|---|---|
| `support::is_removable_profile_dir`: `p.parent() == Some(temp_root)` → `true` | `support::tests::anything_else_is_refused` | `test support::tests::anything_else_is_refused ... FAILED` ("two levels down") |
| sem `name.len() > PROFILE_PREFIX.len()` | o mesmo | `test support::tests::anything_else_is_refused ... FAILED` ("prefix only") |
| `chrome_path_problem` sem `eq_ignore_ascii_case("chrome.exe")` | `only_an_absolute_chrome_exe_is_accepted_as_the_test_chrome` | `test support::tests::only_an_absolute_chrome_exe_is_accepted_as_the_test_chrome ... FAILED` ("another program") |
| `is_our_chrome_cmdline` sem `--remote-debugging-port=` | `a_pid_is_killed_only_when_its_command_line_is_our_chrome` | `test support::tests::a_pid_is_killed_only_when_its_command_line_is_our_chrome ... FAILED` ("another port") |
| `browser.rs`: `self.docs.attached(...)` sem `accept_allows` (live) | `a_file_the_field_refuses_is_not_recorded_as_attached` | `test a_file_the_field_refuses_is_not_recorded_as_attached ... FAILED` ("a refused format is not recorded in anexos") |
| `update_variante` de volta com `search_variants.yaml.bak-1` | `update_variante_writes_with_a_backup_and_keeps_one_principal` | `test mcp::tools::variantes::tests::update_variante_writes_with_a_backup_and_keeps_one_principal ... FAILED` |
| `settle_done`: `(changed && !missed)` → `changed` | `a_requested_size_that_the_window_does_not_have_yet_is_waited_for_longer` | `test browser::viewport::tests::a_requested_size_that_the_window_does_not_have_yet_is_waited_for_longer ... FAILED` |
| `scroll_moved` sem o termo `content_h` | `a_scroll_counts_as_started_once_the_document_moved_or_changed_height` | `test mcp::tools::browser::tests::a_scroll_counts_as_started_once_the_document_moved_or_changed_height ... FAILED` ("a page that grew (infinite scroll) counts") |
| viewport mínimo: `if vw >= MIN_W && vh >= MIN_H` → `if vw >= 1 && vh >= 1` (live) | `viewport_live.rs` | `test a_minimized_window_is_restored_and_sized ... FAILED` e `test resizing_never_fakes_the_device ... FAILED`, além dos 3 que já falham no guest (1 de 5 passa; sem a mutação, 2 de 5) |
| `browser_text`: `is_field` sempre falso | `what_was_typed_into_a_field_never_comes_out_not_even_a_password` | `test browser::text::tests::what_was_typed_into_a_field_never_comes_out_not_even_a_password ... FAILED` |
| `login_wall`: `if login {` → `if false && login {` | testes de `pagestate` | `test browser::pagestate::tests::a_login_url_is_a_login_wall_but_only_at_a_path_boundary ... FAILED`, `a_dialog_that_asks_to_sign_in_is_a_login_wall ... FAILED` (e mais 2) |
| `loopguard.rs`: `THRESHOLD = 4` → `40` | teste de loop | `test browser::loopguard::tests::the_fourth_equal_call_gets_the_hint_and_the_first_three_do_not ... FAILED` (e mais 3) |
| `browser_scroll` em contêiner: a referência resolvida é descartada (rola a página) (live) | `scroll_with_a_ref_moves_that_elements_own_container_and_says_when_it_is_at_the_limit` | `test scroll_with_a_ref_moves_that_elements_own_container_and_says_when_it_is_at_the_limit ... FAILED` |

  Restauradas, todas voltam a passar (exit 0; o `viewport_live` volta a 2 de 5, o estado do guest sem mutação). Armadilha vista: o `Copy-Item` preserva a data antiga do arquivo, e o cargo reaproveitou o binário com a mutação (as 13 "restauradas" falharam na primeira tentativa, com o SHA-256 já igual ao do PC). Depois de pôr a data atual nos arquivos restaurados (só a data, conteúdo conferido), recompilou e passou.
- **Fim:** `test-down.ps1` exit 0, `wsb list` vazio, nenhuma cópia da credencial no PC. `git status` no PC igual ao do início (nenhum código editado).

#### Correções das falhas da rodada acima (2026-10-08, noite; commits `5516988`, `160e48b`, `0483517`, `70d16c5`)

Tudo rodou **dentro do Windows Sandbox** (ambiente `testes`, projeto = worktree do agente, copiado para `C:\labdata\project`), com scripts da rodada fora do git (`.sandbox-work\t5a7\` da worktree, lidos no guest em `C:\labproject`; saídas em `C:\labwork\t5a7\out`). No PC só `cargo clippy --all-targets` com `CARGO_TARGET_DIR` próprio. Mutações só na cópia do guest, uma de cada vez, restauradas copiando de `C:\labproject` (SHA-256 igual) e com a data posta em "agora" (a armadilha da rodada anterior).

| Falha | Causa (com evidência) | Correção | Teste e mutação (linha `FAILED`) |
|---|---|---|---|
| Pastas `claudia-live-*` nunca apagadas (298 em `Z:\tmp`) | O `remove_dir_all` rodava logo depois do `taskkill /F` e achava arquivo ainda aberto: com a mensagem ligada, `deleting ...claudia-live-browser-... failed on the first attempt (O arquivo já está sendo usado por outro processo. (os error 32))`; a 2ª tentativa, 267 ms depois, apagou. Além disso, um processo utilitário do Chrome (`--utility-sub-type=chrome.mojom.ProcessorMetrics`, com o perfil do teste na linha de comando) sobreviveu ao `taskkill /T` por 13 a 30 s. | `tests/support/mod.rs`: depois do `taskkill /T`, varre todo `chrome.exe` cuja linha de comando está **dentro** da pasta do perfil do teste (função pura `is_chrome_on_profile`; só para pasta aprovada por `is_removable_profile_dir`) e espera até 10 s; o apagar tenta de novo por até 20 s, conferindo a trava pura antes de **cada** tentativa (`is_removable_profile_dir` sem mudança); falha final = **o teste falha** com o motivo (só imprime se a thread já está em pânico). `kill_tree` passou a mostrar erro do `taskkill`. | Novo live `the_profile_folder_is_deleted_when_the_live_session_ends` (pasta some e nenhum Chrome do perfil fica). Antes (m1+m6: uma tentativa, erro ignorado, sem varredura): 3 de 3 `test the_profile_folder_is_deleted_when_the_live_session_ends ... FAILED` (`os error 32`, pasta com 159 a 183 arquivos). Cada mecanismo sozinho já resolve (m1 só, m6 só: passa). Pura: m5 (sem o separador depois do nome do perfil) → `test support::tests::a_helper_process_counts_only_when_it_runs_on_this_harness_profile ... FAILED`. |
| `a_click_at_inside_a_shadow_root_finds_the_submit_button_and_its_form` | **O teste mirava o lugar errado**, não a guarda: o ponto era o centro do `#shadow-host`, um `<div>` de largura inteira, e o botão de 160 px fica à esquerda; o clique `(600, 469.5)` caiu em espaço vazio, que não envia nada, e a guarda deixou passar com razão. | Só o teste: mira o centro do botão dentro do shadow root e confere (`shadowRoot.elementFromPoint`) que o ponto está sobre o botão e que o centro do host **não** está. A guarda (`INFO_FN`, ascensão pela árvore composta) não mudou. | Passa com a guarda como estava. m2 (`up` só por `parentElement`, sem passar pelo host) → `test a_click_at_inside_a_shadow_root_finds_the_submit_button_and_its_form ... FAILED`. |
| `a_confirm_during_a_send_the_guard_approved_is_accepted_and_only_then` | As fixtures abrem como `file:///...`; `origin_of` devolvia `None` para URL sem host, e aprovação com origem desconhecida nunca casa: o `confirm` "Enviar?" recebia Cancelar. | `page_events.rs`: página `file:` tem como chave o endereço inteiro do documento (`file://` + caminho, sem query nem fragmento, minúsculo), **mais estrito** que origem. A aprovação continua de uso único, mesma chave e 2 s depois da tool. | Novos unitários `a_local_file_page_is_keyed_by_the_whole_document_address` e `an_approval_on_a_local_file_covers_that_file_only_once`. m3 (sem o caso `file`) → os 2 unitários `... FAILED` e `test a_confirm_during_a_send_the_guard_approved_is_accepted_and_only_then ... FAILED`. |
| Viewport limitado a 800×600 (CLA-36 e cia.) | **Não é a sessão do `wsb connect`**: o Chrome de teste é headless, e a tela virtual do headless é 800×600 (o novo teste leu `screen` = `800x600`); maximizar dava 800×600. Corrigida a tela, apareceram mais duas causas: (a) a janela muda na hora, mas a página só se reajusta 250 a 500 ms depois (diagnóstico: janela 700×400 com página 927×908 por 250 ms; 1600×500 com página 682×248), e o `ensure_min_viewport` dava a medição por estável com a página antiga (pedia 1600×952, media 348 px de altura e maximizava); (b) na 2ª suíte seguida, o pedido de tamanho feito logo após abrir o Chrome foi ignorado e a janela ficou no tamanho inicial (`945x1060`, `normal`) nos 5 s de espera do setup. | `manager.rs`: só o Chrome headless de teste recebe `--screen-info={0,0 1920x1080}` (o visível usa o monitor real; teste unitário garante). `viewport.rs`: `page_changed` conta só a mudança do viewport da página, e um tamanho pedido só está alcançado quando a janela o tem **e** a página mudou. Testes: `set_window` reenvia o pedido a cada 1 s por até 10 s e espera o viewport acompanhar a janela (`viewport_follows_window`, pura). | Novos: live `the_test_browser_has_a_screen_where_the_minimum_fits`, unitários `the_headless_test_browser_gets_a_screen_above_the_minimum_and_the_visible_one_does_not`, `new_window_bounds_over_the_old_page_are_not_a_change`, `support::tests::a_page_viewport_belongs_to_its_window_only_when_the_frame_is_plausible`. m4 (sem `--screen-info`) → `test browser::manager::tests::the_headless_test_browser_gets_a_screen_above_the_minimum_and_the_visible_one_does_not ... FAILED` e `the screen of the test browser is 800x600`; m8 (mudança da janela conta como mudança) → `test browser::viewport::tests::new_window_bounds_over_the_old_page_are_not_a_change ... FAILED` e `test a_wide_but_short_window_grows_in_height_only ... FAILED`; m7 (não espera o viewport) → o unitário puro `... FAILED` e 4 de 6 do `viewport_live` `FAILED`. |
| Clippy `redundant guard` em `pagestate.rs:117` | — | padrão `Some('/' \| '?' \| ...)` | clippy sem o aviso |

- **Suíte final no guest (HEAD `70d16c5`), duas vezes seguidas, mesmo Sandbox:** `cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon` → rodada 1 exit 0 (429 s): `browser_live` **93 passed, 0 failed**, `viewport_live` **6 passed, 0 failed**; rodada 2 exit 0 (408 s): os mesmos 93/93 e 6/6. Depois de cada rodada: **0** pastas `claudia-live-*`/`claudia-smoke-*` em `Z:\tmp` e no `%TEMP%` do guest, **0** `chrome.exe` com perfil de teste na linha de comando (11 no total, todos do `bootstrap.ps1`). Uma tentativa anterior (HEAD `0483517`, antes do reenvio no `set_window`) teve a rodada 1 verde e a rodada 2 com `a_minimized_window_is_restored_and_sized` e `a_shrunk_window_is_corrected_to_the_minimum` falhando **no setup** (janela ficou `945x1060 normal`): daí o commit `70d16c5`.
- **`cargo test --lib` no guest:** exit 0, **584 passed, 0 failed, 0 ignored**.
- **`cargo clippy --all-targets` no guest:** exit 0; restam os 3 do baseline (`new`, `write_both`, `LASTINPUTINFO`) e `items after a test module` em `src/prompt.rs:16` (anterior à branch, fora do escopo); o `redundant guard` sumiu.
- **Não verificado nesta correção:** os 3 testes de ícone (janela visível) e o smoke do MCP não rodaram de novo; o `ensure_min_viewport` com o Chrome visível (o do usuário) só por raciocínio: a mudança deixa a espera mais estrita (só conta a página relaida), não muda a decisão.
- **Fim:** `test-down` exit 0, `wsb list` vazio, sem `.credentials.json` no PC.

### 8.2 Tarefa 5: smoke de uso real (bloqueada)

Bloqueada até a branch dos dados falsos completos (`feat/dados-falsos-teste`) existir. Decisão do usuário: o smoke roda no Sandbox, com dados falsos completos; os dados reais não entram. Limite conhecido: o CDP não abre dentro do Sandbox, a observação é por acessibilidade (UIA), e o que não for verificável assim fica "não verificado".

| Item | Resultado |
|---|---|
| migração da pasta antiga: um documento por arquivo, originais intactos, reabrir não duplica | pendente |
| aba Currículos nos dois idiomas (grade, cartas, página A4, Voltar, tags, renomear, preferido geral e por variante, select do Perfil) | pendente |
| gerar currículo e carta, PDF começando com `%PDF-` e **nenhuma janela extra** (PDF pelo WebView2 dentro do app real) | pendente |
| importar e excluir (só o arquivo do documento sai da pasta) | pendente |
| Chrome do app com sandbox (sem `--no-sandbox` na linha de comando) | pendente |
| persistência do perfil entre duas aberturas e desligamento do Chrome ao fechar o app | pendente |

### 8.3 Tarefa 6: rodadas de agente e validações do motor

| Item | Resultado |
|---|---|
| rodada de agente da Parte 2 (variantes, documentos, carta, upload com variante, escrita recusada na execução, e aceita no Perfil), observada pelo Opus | pendente |
| upload em `the-internet.herokuapp.com/upload`, sem enviar candidatura (D3) | pendente |
| `activity-log-validate.mjs` depois de uma sessão | pendente |
| validações 6.2 do motor, segunda execução (rodadas A, B estendida e C) | pendente |
| validações 6.4 do motor, cenários 4 a 7 (retenção, dois processos MCP, `dev/` só leitura, viewer) | pendente |
| candidatura real no LinkedIn (D4), com o usuário presente e dizendo "pode enviar" | pendente |

### 8.4 Investigação de `tailor-application` (Tarefa 7, Step 1, só leitura)

Feita em 2026-10-08. A skill **não foi encontrada**: não existe em `~/.claude/skills/` (só `mobile-design`, `sec-audit`, `synced`, `ux-design`, `vue-*`), nem nos plugins instalados (busca por nome e por conteúdo), nem no repositório (só aparece como texto em `src-tauri/src/prompt_sistema_runtime.md`, linha 157, e em docs). O prompt ainda manda o agente "seguir integralmente" uma skill que **não existe nesta máquina**; e a arquitetura (§11) a trata como lógica de conteúdo existente. Consequências, não verificadas: o agente em produção pode não ter a skill (depende de ela estar instalada na conta de quem usa), e, se tiver, **onde ela grava os arquivos é desconhecido**, então não se sabe se o currículo adaptado por ela entra na biblioteca (`<dados>/documentos/`). O caminho que entra na biblioteca com certeza é `save_cover_letter` (carta) e a geração da aba Currículos. O prompt é do usuário (não editar sem pedido); a decisão de manter, trocar ou remover a menção volta a ele. Linear: a registrar (Tarefa 10; o MCP do Linear não foi usado nesta tarefa).

### 8.5 Outros itens sem destino fechado nesta evidência

Revisão final Opus da branch (Tarefa 9), issues do Linear (Tarefa 10, ver `linear-pendente.md`) e a entrega ao usuário (Tarefa 11): pendentes. As decisões D12 e D13 do usuário estão no plano.
