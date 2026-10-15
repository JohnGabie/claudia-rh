# Fluxo guiado da tela Sites: plano de implementação

> **Para agentes:** SUB-SKILL OBRIGATÓRIA: `superpowers:subagent-driven-development` (recomendada) ou `superpowers:executing-plans`. Os passos usam caixas `- [ ]`. **Ciclo executor ↔ revisor do `CLAUDE.md` (§2) vale para cada tarefa: uma por vez, executor não é encerrado antes do "Looks good to me" do revisor.**

**Objetivo:** trocar o comportamento do cartão de Sites (só a faixa de baixo liga a IA) e entregar a página do site com passos guiados, painel "Sobre o site" com Conta e Uso pela IA, zona de risco recolhível, tipos com nomes novos e a visão em lista coerente.

**Arquitetura:** o backend ganha 1 coluna dupla (`aberturas`, `ultima_abertura`), 3 comandos (`estatisticas_site`, `definir_site_sem_login`, `marcar_site_nao_logado`) e a contagem de abertura no bloqueio do `browser_open`. O front ganha lógica pura testável (`pronto`, situação da faixa, passo 1), cartão com corpo que abre a página, página de duas colunas com `Passos` e `Sobre`, e rótulos novos. A regra de segurança (sair de "não pede login" sem estar logado fecha a IA) vive no backend, não na tela.

**Stack:** Rust (rusqlite, Tauri v2), React + TypeScript (Vite), `node --test` para a lógica pura do front, Windows Sandbox para `cargo test`.

**Spec:** `docs/superpowers/specs/2026-10-15-sites-fluxo-guiado-design.md` (aprovada pelo usuário em 2026-10-15). Referência visual aprovada: `docs/superpowers/specs/assets/2026-10-15-sites-fluxo-guiado-mock.html` e `.css` (medidas, cores e estrutura; os números de "Uso pela IA" nele são inventados). Spec-base: `2026-10-14-lista-de-sites-design.md`.

## Global Constraints

- Branch de trabalho: `feat/lista-de-sites-impl` (esta worktree). Sem push, sem merge, sem PR. `main` nunca.
- Comentários de código, mensagens de commit e identificadores novos: **inglês**. Copy da UI: **pt-BR e en** (`src/i18n/pt.ts`, `src/i18n/en.ts`). Docs vivos: pt-BR.
- Todo commit termina com as linhas:
  `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` e `Claude-Session: https://claude.ai/code/session_0191VgKyPTkcCfdPFh4A8byq`.
- **`cargo test` (qualquer um) roda só no Windows Sandbox** (skill `claudia-teste-sandbox`, `sandbox-lab/`). No PC: só `cargo check`, `cargo clippy --all-targets`, `cargo test --no-run`, `npx tsc --noEmit`, `npm run build` e `node --test` de lógica pura que não toca o disco. Cada agente usa o próprio `CARGO_TARGET_DIR` fora do repositório (`$TEMP/...`).
- Nenhum teste usa caminho real nem apaga arquivo. Os testes novos usam `Connection::open_in_memory()`.
- Nada de caminho absoluto de máquina em código, testes, scripts ou docs.
- **Não editar** `src-tauri/src/prompt_sistema_runtime.md` nem o `identifier` do `tauri.conf.json`.
- `src-tauri/Cargo.toml` aparece modificado na worktree antes deste plano: **não tocar nem commitar**.
- Tipos no banco não mudam de chave (`geral`, `empresa`...). Só o rótulo muda (spec P3).
- Subagentes que mexam com arquivos recebem estas regras no prompt. Nenhuma tarefa aqui apaga arquivo; se uma parecer exigir, parar e perguntar.
- Prova de mutação em toda regra nova: quebrar de propósito, mostrar a linha `FAILED` do teste nomeado, restaurar.
- Cada tarefa termina com o veredito escrito do revisor com evidência (comandos, código de saída, prints no caso de front).

## Review Focus

Entradas e condições que a spec implica e nenhuma tarefa isolada cobre; o revisor as exercita:

1. **Sair de "não pede login" sem estar logado fecha a IA**, também com `estado = precisa_login`, e **manter** quando `estado = logada` (T3).
2. **Contagem de aberturas só de aberturas aceitas**: site fechado, site `precisa_login`, URL http ou fora da lista não somam (T1).
3. **Vaga que casa com outro site ou com ATS compartilhado** não conta no site errado; vaga sem site da lista não conta em nenhum (T2).
4. **Falha ao carregar as estatísticas não quebra a página**: mostra "Uso pela IA" indisponível e o resto funciona (T6).
5. **Clicar em "Abrir ↗" ou no domínio não abre a página**; clicar no corpo abre; Enter/Espaço na faixa liga; foco visível; largura estreita sem texto cortado (T5).
6. **Site `cookie_sessao` (LinkedIn)** não mostra o botão "Este site não pede login" e o backend recusa o comando (T3, T6).
7. **Migração idempotente** num banco que já tem a tabela `contas_browser` com dados (T1).

## Mapa de arquivos

| Arquivo | Tarefa | Responsabilidade |
|---|---|---|
| `src-tauri/src/db/mod.rs` | T1 | `ALTER TABLE` das duas colunas |
| `src-tauri/src/sites/gate.rs` | T1 | `registrar_abertura` na decisão `Allow` |
| `src-tauri/src/sites/repo.rs` | T2, T3 | `resolver_em` (puro), `definir_sem_login`, `marcar_nao_logado` |
| `src-tauri/src/sites/stats.rs` (novo), `sites/mod.rs` | T2 | `SiteStats` e `estatisticas` |
| `src-tauri/src/commands/sites.rs`, `src-tauri/src/lib.rs` | T2, T3 | 3 comandos e registro no `invoke_handler` |
| `src/components/sites/estado.ts`, `logica.ts` | T4 | `pronto`, `semLogin`, `situacaoFaixa`, `passo1`, cor `semlogin` |
| `src/i18n/pt.ts`, `src/i18n/en.ts` | T4 a T6 | rótulos de tipo e textos novos |
| `src/lib/sites-api.ts`, `src/lib/sites.mock.ts`, `src/lib/dev-mock.ts` | T4 | comandos novos e mock de dev |
| `src/components/sites/Cartao.tsx`, `Faixa.tsx`, `estilos.ts`, `Galeria.tsx` | T5 | cartão retangular, faixa única |
| `src/components/sites/Detalhe.tsx`, `Passos.tsx` (novo), `Sobre.tsx` (novo), `ZonaRisco.tsx` (novo); `Acoes.tsx` sai | T6 | página do site |
| `src/components/sites/Lista.tsx` | T7 | linha abre a página, só o status liga |
| `docs/padrao-design-claudia-rh.md` §15, `docs/arquitetura-sistema-candidaturas.md` §3.8 | T8 | docs vivos |
| `tests/front/sites-logica.test.ts`, `tests/front/sites-passos.test.ts` (novo) | T4 | testes de lógica pura |

Ordem: T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8. Uma por vez.

---

### Task 1: contar aberturas aceitas (coluna + bloqueio)

**Files:**
- Modify: `src-tauri/src/db/mod.rs` (lista de `ALTER TABLE`, junto das linhas existentes)
- Modify: `src-tauri/src/sites/gate.rs` (`decidir` e testes)

**Interfaces:**
- Consumes: `decidir(conn, url) -> Decision`, `repo::resolver`.
- Produces: colunas `contas_browser.aberturas INTEGER NOT NULL DEFAULT 0` e `ultima_abertura TEXT`; `pub fn registrar_abertura(conn: &Connection, chave: &str)` em `sites/gate.rs`.

- [ ] **Step 1: testes que falham** (em `gate.rs`, módulo `tests`, usando os helpers `db()` e as constantes que o módulo já tem)

```rust
fn aberturas(c: &Connection, chave: &str) -> (i64, Option<String>) {
    c.query_row("SELECT aberturas, ultima_abertura FROM contas_browser WHERE chave = ?1", [chave], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
}

#[test]
fn an_allowed_open_is_counted_with_its_date() {
    let c = db();
    // "pub" is a public site (regra_login = nenhuma): releasing the AI is enough.
    repo::definir_ia_pode_usar(&c, "pub", true).unwrap();
    assert!(matches!(decidir(&c, "https://pub.com/careers"), Decision::Allow(_)));
    assert!(matches!(decidir(&c, "https://pub.com/careers/x"), Decision::Allow(_)));
    let (n, ultima) = aberturas(&c, "pub");
    assert_eq!(n, 2);
    assert!(ultima.is_some());
}

#[test]
fn refused_opens_are_never_counted() {
    let c = db();
    // closed for the AI
    assert!(matches!(decidir(&c, "https://pub.com/careers"), Decision::Refuse(_)));
    // off the list and not https
    assert!(matches!(decidir(&c, "https://fora.example/x"), Decision::Refuse(_)));
    assert!(matches!(decidir(&c, "http://pub.com/careers"), Decision::Refuse(_)));
    // needs login and is not logged in
    repo::marcar_precisa_login(&c, "linkedin").unwrap();
    assert!(matches!(decidir(&c, "https://www.linkedin.com/jobs/"), Decision::Refuse(_)));
    assert_eq!(aberturas(&c, "pub").0, 0);
    assert_eq!(aberturas(&c, "linkedin").0, 0);
}
```

Se `db()` do módulo não carrega `LI` e `PUB` juntos, ajustar o helper de teste para fazer `merge` das duas constantes (já existem em `repo.rs`; copiar as duas linhas JSON para `gate.rs` se necessário).

- [ ] **Step 2: rodar no Sandbox e ver falhar** (`cargo test --lib sites::gate` pela skill `claudia-teste-sandbox`). Esperado: erro de compilação ou `no such column: aberturas`.

- [ ] **Step 3: implementar**

Em `db/mod.rs`, ao lado dos outros `ALTER TABLE`:

```rust
("ALTER TABLE contas_browser ADD COLUMN aberturas INTEGER NOT NULL DEFAULT 0;", "contas_browser.aberturas"),
("ALTER TABLE contas_browser ADD COLUMN ultima_abertura TEXT;", "contas_browser.ultima_abertura"),
```

Em `gate.rs`:

```rust
/// Counts an open the gate accepted. A failure to count never refuses the open.
pub fn registrar_abertura(conn: &Connection, chave: &str) {
    let _ = conn.execute(
        "UPDATE contas_browser SET aberturas = aberturas + 1, ultima_abertura = datetime('now') WHERE chave = ?1",
        [chave],
    );
}
```

e na última linha de `decidir`, trocar `Decision::Allow(s.chave)` por:

```rust
    registrar_abertura(conn, &s.chave);
    Decision::Allow(s.chave)
```

- [ ] **Step 4: rodar no Sandbox e ver passar**, incluindo `cargo test --lib db::` (o teste `apply_is_idempotent` existente cobre aplicar duas vezes). Acrescentar um teste de migração: criar a tabela `contas_browser` **sem** as colunas novas, inserir uma linha, chamar `db::apply`, conferir que a linha sobrevive com `aberturas = 0`.

- [ ] **Step 5: prova de mutação.** (a) Comentar `registrar_abertura(conn, &s.chave);` e mostrar `FAILED` em `an_allowed_open_is_counted_with_its_date`. (b) Mover a chamada para antes do teste `ia_pode_usar` e mostrar `FAILED` em `refused_opens_are_never_counted`. Restaurar.

- [ ] **Step 6: `cargo check` e `cargo clippy --all-targets` no PC; commit** `feat(sites): count the opens the gate accepts`.

---

### Task 2: estatísticas por site

**Files:**
- Modify: `src-tauri/src/sites/repo.rs` (extrair `resolver_em`)
- Create: `src-tauri/src/sites/stats.rs`; Modify: `src-tauri/src/sites/mod.rs` (`pub mod stats;`)
- Modify: `src-tauri/src/commands/sites.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `repo::listar`, `repo::obter`, `url_matches`, `host_matches`, tabelas `avisos_sites`, `vagas`, `candidaturas`, colunas da T1.
- Produces:

```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SiteStats {
    pub relogins: i64,                 // notices ever created for the site (open or resolved)
    pub aberturas: i64,
    pub ultima_abertura: Option<String>,
    pub vagas: i64,                    // vagas whose URL resolves to this site
    pub candidaturas: i64,             // candidaturas of those vagas
    pub ultima_candidatura: Option<String>,
}
pub fn estatisticas(conn: &Connection, chave: &str) -> Result<SiteStats, String>
// repo.rs
pub fn resolver_em<'a>(sites: &'a [Site], url: &str) -> Option<&'a Site>
// Tauri command
#[tauri::command] pub fn estatisticas_site(state: State<'_, DbState>, chave: String) -> Result<SiteStats, String>
```

- [ ] **Step 1: refatorar sem mudar comportamento.** Mover o corpo de `repo::resolver` para `resolver_em(&[Site], &str) -> Option<&Site>` (mesma regra: casamento por domínio, o de maior domínio vence) e fazer `resolver` chamar `resolver_em(&listar(conn)?, url).cloned()`. Rodar os testes existentes de `repo` e `gate` no Sandbox: tudo verde antes de seguir.

- [ ] **Step 2: testes que falham** (em `stats.rs`, `mod tests`, banco em memória com `crate::db::apply` e `merge` de `LI` e de um site ATS como os do `repo.rs`; inserir linhas direto em `vagas` e `candidaturas`)

```rust
fn vaga(c: &Connection, url: &str) -> i64 {
    c.execute("INSERT INTO vagas (titulo, empresa, plataforma, url, descoberta_em) VALUES ('t','e','p',?1,datetime('now'))", [url]).unwrap();
    c.last_insert_rowid()
}
fn candidatura(c: &Connection, vaga_id: i64, quando: &str) {
    c.execute("INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES (?1, ?2, '', 'x')", params![vaga_id, quando]).unwrap();
}

#[test]
fn counts_only_the_vagas_and_candidaturas_that_resolve_to_the_site() {
    let c = db_with_linkedin_and_ats();
    let a = vaga(&c, "https://www.linkedin.com/jobs/view/1");
    let _b = vaga(&c, "https://www.linkedin.com/jobs/view/2");
    let outra = vaga(&c, "https://elsewhere.example/job/9");
    candidatura(&c, a, "2026-10-01 10:00:00");
    candidatura(&c, outra, "2026-10-02 10:00:00");
    let s = estatisticas(&c, "linkedin").unwrap();
    assert_eq!((s.vagas, s.candidaturas), (2, 1));
    assert_eq!(s.ultima_candidatura.as_deref(), Some("2026-10-01 10:00:00"));
}

#[test]
fn a_vaga_on_a_shared_ats_host_counts_only_for_the_site_that_owns_the_path() {
    // two companies on the same ATS host, told apart by caminho_ats
    let c = db_with_two_ats_companies();
    vaga(&c, "https://boards.example-ats.com/acme/jobs/1");
    assert_eq!(estatisticas(&c, "acme").unwrap().vagas, 1);
    assert_eq!(estatisticas(&c, "globex").unwrap().vagas, 0);
}

#[test]
fn relogins_count_every_notice_open_or_resolved() {
    let c = db_with_linkedin_and_ats();
    crate::sites::avisos::criar(&c, "linkedin").unwrap();
    crate::sites::avisos::resolver_do_site(&c, "linkedin").unwrap();
    crate::sites::avisos::criar(&c, "linkedin").unwrap();
    assert_eq!(estatisticas(&c, "linkedin").unwrap().relogins, 2);
}

#[test]
fn a_site_that_does_not_exist_is_an_error_and_an_unused_site_is_all_zero() {
    let c = db_with_linkedin_and_ats();
    assert!(estatisticas(&c, "nope").is_err());
    let s = estatisticas(&c, "linkedin").unwrap();
    assert_eq!((s.relogins, s.aberturas, s.vagas, s.candidaturas), (0, 0, 0, 0));
    assert!(s.ultima_abertura.is_none() && s.ultima_candidatura.is_none());
}
```

Os helpers `db_with_linkedin_and_ats` e `db_with_two_ats_companies` montam o seed com `merge(&c, &parse_seed(...))`; usar o mesmo formato JSON das constantes de `repo.rs` (dois sites com `dominios_ats: ["boards.example-ats.com"]` e `caminho_ats` `/acme` e `/globex`).

- [ ] **Step 3: rodar no Sandbox e ver falhar.**

- [ ] **Step 4: implementar `estatisticas`.** Carregar `listar` **uma vez**; ler `aberturas`/`ultima_abertura` com `SELECT ... FROM contas_browser WHERE chave = ?1` (erro se não existe); `relogins` = `SELECT COUNT(*) FROM avisos_sites WHERE chave = ?1`; para vagas, `SELECT url FROM vagas` e contar as que `resolver_em(&sites, &url)` devolve com a mesma `chave`; para candidaturas, `SELECT v.url, c.enviada_em FROM candidaturas c JOIN vagas v ON v.id = c.vaga_id`, contando as que casam e guardando o maior `enviada_em`. Registrar o comando `estatisticas_site` em `commands/sites.rs` (wrapper fino sobre `stats::estatisticas`) e em `lib.rs` junto dos outros `commands::sites::*`.

- [ ] **Step 5: rodar no Sandbox e ver passar** (`cargo test --lib sites::`).

- [ ] **Step 6: prova de mutação.** (a) Trocar a comparação de chave por `true` e mostrar `FAILED` em `counts_only_the_vagas_...`. (b) Em `resolver_em`, ignorar `caminho_ats` e mostrar `FAILED` em `a_vaga_on_a_shared_ats_host_...`. (c) Trocar `COUNT(*)` por `COUNT(*) WHERE resolvido = 0` e mostrar `FAILED` em `relogins_count_every_notice_...`. Restaurar.

- [ ] **Step 7: `cargo check`, `cargo clippy --all-targets`; commit** `feat(sites): per-site stats (relogins, opens, vagas, applications)`.

---

### Task 3: "não pede login" e "não estou mais logado"

**Files:**
- Modify: `src-tauri/src/sites/repo.rs` (funções e testes)
- Modify: `src-tauri/src/commands/sites.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Produces:

```rust
/// valor = true: the site asks for no login. valor = false: back to manual login; if the user is not
/// logged in, the AI permission is closed in the same transaction.
pub fn definir_sem_login(conn: &Connection, chave: &str, valor: bool) -> Result<(), String>
/// "I am no longer logged in": estado = precisa_login and the AI permission closed.
pub fn marcar_nao_logado(conn: &Connection, chave: &str) -> Result<(), String>
// commands (state: State<'_, DbState>)
definir_site_sem_login(chave: String, valor: bool) -> Result<(), String>
marcar_site_nao_logado(chave: String) -> Result<(), String>
```

- [ ] **Step 1: testes que falham** (em `repo.rs`, módulo `tests`, usando `db_with`, `LI` e `PUB`; para um site `manual` que o usuário põe em `nenhuma`, usar `LI`)

```rust
fn ia(c: &Connection, k: &str) -> bool { obter(c, k).unwrap().unwrap().ia_pode_usar }

#[test]
fn turning_no_login_on_lets_the_ai_be_released_without_a_login() {
    let c = db_with(LI); // LI has the default manual rule
    definir_sem_login(&c, "linkedin", true).unwrap();
    assert_eq!(obter(&c, "linkedin").unwrap().unwrap().regra_login, "nenhuma");
    definir_ia_pode_usar(&c, "linkedin", true).unwrap();
}

#[test]
fn going_back_to_manual_without_being_logged_in_closes_the_ai() {
    let c = db_with(PUB); // regra_login = nenhuma, estado = desconhecido
    definir_ia_pode_usar(&c, "pub", true).unwrap();
    definir_sem_login(&c, "pub", false).unwrap();
    assert_eq!(obter(&c, "pub").unwrap().unwrap().regra_login, "manual");
    assert!(!ia(&c, "pub"));
}

#[test]
fn going_back_to_manual_while_needing_login_also_closes_the_ai() {
    let c = db_with(PUB);
    definir_ia_pode_usar(&c, "pub", true).unwrap();
    marcar_precisa_login(&c, "pub").unwrap();
    definir_sem_login(&c, "pub", false).unwrap();
    assert!(!ia(&c, "pub"));
}

#[test]
fn going_back_to_manual_while_logged_in_keeps_the_ai() {
    let c = db_with(PUB);
    definir_ia_pode_usar(&c, "pub", true).unwrap();
    c.execute("UPDATE contas_browser SET estado='logada' WHERE chave='pub'", []).unwrap();
    definir_sem_login(&c, "pub", false).unwrap();
    assert!(ia(&c, "pub"));
}

#[test]
fn a_cookie_session_site_refuses_the_toggle_and_stays_untouched() {
    let c = db_with(LI);
    c.execute("UPDATE contas_browser SET regra_login='cookie_sessao' WHERE chave='linkedin'", []).unwrap();
    assert!(definir_sem_login(&c, "linkedin", true).is_err());
    assert_eq!(obter(&c, "linkedin").unwrap().unwrap().regra_login, "cookie_sessao");
    assert!(definir_sem_login(&c, "nope", true).is_err());
}

#[test]
fn turning_no_login_on_resolves_the_open_relogin_notice() {
    let c = db_with(LI);
    crate::sites::avisos::criar(&c, "linkedin").unwrap();
    definir_sem_login(&c, "linkedin", true).unwrap();
    assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 0);
}

#[test]
fn not_logged_in_anymore_closes_the_ai_and_marks_the_site() {
    let c = db_with(LI);
    marcar_logada(&c, "linkedin").unwrap();
    definir_ia_pode_usar(&c, "linkedin", true).unwrap();
    marcar_nao_logado(&c, "linkedin").unwrap();
    let s = obter(&c, "linkedin").unwrap().unwrap();
    assert_eq!(s.estado, "precisa_login");
    assert!(!s.ia_pode_usar);
    assert!(marcar_nao_logado(&c, "pub-que-nao-existe").is_err());
}
```

- [ ] **Step 2: rodar no Sandbox e ver falhar.**

- [ ] **Step 3: implementar.**

```rust
pub fn definir_sem_login(conn: &Connection, chave: &str, valor: bool) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if s.regra_login == "cookie_sessao" {
        return Err("Este site é verificado pelo cookie da sessão; a opção \"não pede login\" não vale aqui.".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    if valor {
        tx.execute("UPDATE contas_browser SET regra_login = 'nenhuma' WHERE chave = ?1", [chave]).map_err(|e| e.to_string())?;
        super::avisos::resolver_do_site(&tx, chave)?;
    } else {
        // Back to manual login: without a confirmed login the AI permission closes in the same step.
        tx.execute(
            "UPDATE contas_browser SET regra_login = 'manual', \
             ia_pode_usar = CASE WHEN estado = 'logada' THEN ia_pode_usar ELSE 0 END WHERE chave = ?1",
            [chave],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn marcar_nao_logado(conn: &Connection, chave: &str) -> Result<(), String> {
    exigir(conn, chave)?;
    conn.execute("UPDATE contas_browser SET estado = 'precisa_login', ia_pode_usar = 0 WHERE chave = ?1", [chave])
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

(Os caracteres acentuados nas mensagens são UTF-8 normal: o arquivo já os tem; após salvar, o revisor confere que não há mojibake, como no incidente do commit `bde702b`.) Em `commands/sites.rs`: `definir_site_sem_login` (ao ligar, `app.emit("aviso-site-resolvido", chave)` como em `marcar_site_logado`) e `marcar_site_nao_logado`; registrar os dois em `lib.rs`. Nenhum desses dois comandos cria aviso: foi o próprio usuário que disse.

- [ ] **Step 4: rodar no Sandbox e ver passar** (`cargo test --lib sites::`), e conferir que `commands::sites` ainda compila.

- [ ] **Step 5: prova de mutação.** (a) Trocar o `CASE` por `ia_pode_usar` simples e mostrar `FAILED` em `going_back_to_manual_without_being_logged_in_closes_the_ai` **e** em `..._while_needing_login_also_closes_the_ai`. (b) Trocar `estado = 'logada'` por `estado <> 'logada'` e mostrar `FAILED` em `..._while_logged_in_keeps_the_ai`. (c) Remover o `if s.regra_login == "cookie_sessao"` e mostrar `FAILED` em `a_cookie_session_site_refuses...`. Restaurar.

- [ ] **Step 6: `cargo check`, `cargo clippy --all-targets`; commit** `feat(sites): no-login toggle and "not logged in" command with the close-the-AI rule`.

---

### Task 4: lógica pura, rótulos, API e mock de dev

**Files:**
- Modify: `src/components/sites/estado.ts`, `src/components/sites/logica.ts` (só se necessário)
- Modify: `src/i18n/pt.ts`, `src/i18n/en.ts`, `src/lib/sites-api.ts`, `src/lib/sites.mock.ts`, `src/lib/dev-mock.ts`
- Modify: `tests/front/sites-logica.test.ts`; Create: `tests/front/sites-passos.test.ts`

**Interfaces:**
- Produces (em `estado.ts`):

```ts
export type CorEstado = "logada" | "sem" | "precisa" | "semlogin";   // "semlogin" = green dot, "does not ask for login"
export const pronto = (s: Site): boolean => !bloqueado(s);            // may the AI be released?
export const semLogin = (s: Site): boolean => s.regra_login === "nenhuma";
export type SituacaoFaixa = "falta" | "fechada" | "ligada";
export const situacaoFaixa = (s: Site): SituacaoFaixa => !pronto(s) ? "falta" : s.ia_pode_usar ? "ligada" : "fechada";
export type Passo1 = "pendente" | "logado" | "semLogin";
export const passo1 = (s: Site): Passo1 => semLogin(s) ? "semLogin" : s.estado === "logada" ? "logado" : "pendente";
/** The "does not ask for login" button is offered only where it is safe (spec P5). */
export const podeAlternarSemLogin = (s: Site): boolean => s.regra_login !== "cookie_sessao";
/** "Already logged in" appears only after the site was opened from the page in this visit. */
export const mostraJaEntrei = (jaAbriu: boolean, s: Site): boolean => jaAbriu && passo1(s) === "pendente";
```

- Produces (em `sites-api.ts`):

```ts
export interface SiteStats { relogins: number; aberturas: number; ultima_abertura: string | null; vagas: number; candidaturas: number; ultima_candidatura: string | null }
// sitesApi additions
estatisticas: (chave: string) => call<SiteStats>("estatisticas_site", { chave }),
definirSemLogin: (chave: string, valor: boolean) => call<void>("definir_site_sem_login", { chave, valor }),
marcarNaoLogado: (chave: string) => call<void>("marcar_site_nao_logado", { chave }),
```

  e `ESCRITAS` ganha `"definir_site_sem_login"` e `"marcar_site_nao_logado"`.

- [ ] **Step 1: testes que falham** (`tests/front/sites-passos.test.ts`, no estilo do `sites-logica.test.ts`: `node:test`, importa de `../../src/components/sites/estado.ts`; construir `Site` com um helper `site({...})`)

Casos: `pronto` é verdadeiro para `nenhuma` (qualquer estado), para `manual`+`logada`, falso para `manual`+`precisa_login` e `manual`+`desconhecido`; `situacaoFaixa` devolve `falta`/`fechada`/`ligada` nas três situações; `passo1` devolve `semLogin`, `logado`, `pendente`; `podeAlternarSemLogin` é falso só para `cookie_sessao`; `mostraJaEntrei(false, pendente)` é falso, `mostraJaEntrei(true, pendente)` é verdadeiro, `mostraJaEntrei(true, logado)` é falso; `estadoVisual` de um site `nenhuma` devolve `cor: "semlogin"` e o texto `t.sites.noLogin`. Um teste do dicionário: para cada uma das 7 chaves de tipo, `pt.sites.types[k]` e `en.sites.types[k]` existem e não são vazios, e os valores de `pt` são exatamente os da tabela abaixo.

| chave | pt | en |
|---|---|---|
| geral | Portal de vagas | Job board |
| regional | Portal regional | Regional job board |
| tecnologia | Vagas de tecnologia | Tech jobs |
| remoto | Vagas remotas | Remote jobs |
| freelance | Plataforma freelance | Freelance platform |
| empresa | Site da empresa | Company site |
| nicho | Portal especializado | Niche job board |

- [ ] **Step 2: `node --test tests/front/` e ver falhar** (exportações ausentes).

- [ ] **Step 3: implementar** `estado.ts` conforme as interfaces; em `estadoVisual`, o ramo `s.regra_login !== "manual"` passa a devolver `cor: "semlogin"`. Trocar os rótulos de tipo em `pt.ts` e `en.ts`. Acrescentar às duas línguas as chaves abaixo (o executor escreve o texto en equivalente; o pt é este):

```
step1Title: "Entrar no site"
step1Hint: "Abra o site no browser da Claudia, entre ou crie a sua conta, e volte aqui."
step1Done: "Você confirmou que entrou."
step1NoLogin: "Este site não pede login."
afterOpenHint: "O botão \"Já entrei\" aparece depois de abrir o site."
noLoginBtn: "Este site não pede login"
asksLoginBtn: "Este site pede login"
notLoggedBtn: "Não estou mais logado"
doneBadge: "Feito"
step2Title: "Configurar o perfil no site"
step2Hint: "A Claudia usa a aba para preencher a sua experiência no site. Opcional, e não bloqueia a liberação."
step2Btn: "Configurar perfil"
soonBadge: "Em breve"
step3Title: "Liberar para a IA"
step3Hint: "A IA só abre este site depois que você liberar."
step3Locked: "Disponível depois do passo 1."
lockedBadge: "Bloqueado"
needStep1: "Faça o passo 1 para liberar"
enterArrow: "Entrar →"
sectionRegistry: "Cadastro"
sectionAccount: "Conta"
sectionUsage: "Uso pela IA"
confirmedOn: "Confirmado em"
notEntered: "Ainda não entrou"
relogins: "Relogins"
timesOne: "vez"   timesMany: "vezes"
loginRow: "Login"   loginNotAsked: "Não pede"
opens: "Aberturas"   lastOn: (d: string) => `última ${d}`
vagas: "Vagas analisadas"
applications: "Candidaturas"
rate: "Taxa"   rateOf: (p: number) => `${p}% das analisadas`
neverOpened: "A IA ainda não abriu este site."
statsUnavailable: "Não foi possível carregar o uso deste site."
riskZone: "Zona de risco"
removeHint: "Só sites que você adicionou podem ser removidos. Este veio com o app."
removeBtn: "Remover site"
```

  Em `sites-api.ts`, as três funções e o tipo `SiteStats`. Em `sites.mock.ts` (e a rota em `dev-mock.ts`, na lista de comandos de Sites): responder `estatisticas_site` com números fixos por chave e aplicar em memória as regras de `definir_site_sem_login` e `marcar_site_nao_logado` idênticas às do backend (incluindo recusar `cookie_sessao` e fechar a IA ao voltar para manual sem estar logado).

- [ ] **Step 4: `node --test tests/front/`, `npx tsc --noEmit`, `npm run build`: tudo com código de saída 0.**

- [ ] **Step 5: prova de mutação.** (a) `pronto` retornando sempre `true`: `FAILED` no teste nomeado de `pronto`. (b) `podeAlternarSemLogin` retornando sempre `true`: `FAILED`. (c) Apagar uma chave de tipo do `en`: `FAILED` no teste do dicionário. Restaurar.

- [ ] **Step 6: commit** `feat(sites): pure step logic, new type labels and API bindings for the guided flow`.

---

### Task 5: cartão retangular e faixa única

**Files:**
- Modify: `src/components/sites/Cartao.tsx`, `Faixa.tsx`, `estilos.ts`, `Galeria.tsx` (se a assinatura mudar)
- Modify: `src/components/sites/Pill.tsx` (só se a cor `semlogin` exigir)

**Interfaces:**
- Consumes: `situacaoFaixa`, `pronto`, `estadoVisual` (T4); `AcoesSite.onIa`, `onAbrir`, `onDetalhe`.
- Produces: `Faixa` vira um botão de verdade:

```tsx
export const Faixa: React.FC<{ situacao: SituacaoFaixa; nome: string; onClick: () => void; className?: string }>
```

  `falta` renderiza `<button class="st-faixa falta">` com o texto `t.sites.needStep1` e `t.sites.enterArrow` à direita, **sem** `role="switch"`, e o clique chama `onClick` (que na galeria abre a página). `fechada` e `ligada` renderizam `role="switch"` com `aria-checked` e `aria-label={`${nome}: ${texto}`}`, e o clique chama `onClick` (que alterna a IA).

- [ ] **Step 1: escrever o componente seguindo o mock** (`docs/superpowers/specs/assets/2026-10-15-sites-fluxo-guiado-mock.css`, regras de `.grade`, `.card`, `.capa`, `.info`, `.estado`, `.faixa`; valem os **últimos** valores de cada propriedade, pois o arquivo do mock usa cascata): grade `repeat(auto-fill, 300px)` e gap de 16 px; capa de 52 px com o nome à esquerda, a letra inicial grande como marca d'água e "Abrir ↗" à direita; bloco branco com padding `16px 14px 18px` e gap de 10 px contendo domínio (link), selos de país (nome por extenso) e tipo, e o estado de login **em linha própria**; faixa de 42 px.

  **HTML válido e acessível:** o mock aninhava `<a>`/`<button>` dentro de um `<button>`; no app **não**. O cartão é um `<article class="st-card">`; o nome é um `<button class="nome">` com `::after { content:""; position:absolute; inset:0 }` (link esticado) que abre a página; "Abrir ↗" e o domínio têm `position: relative; z-index: 1` e chamam `onAbrir` com `stopPropagation`; a faixa é irmã do bloco e fica acima do link esticado (`position: relative; z-index: 1`). O cartão **não** usa mais `Interruptor`.

- [ ] **Step 2: ligar o clique.** Faixa `falta` → `onDetalhe(site)`; `fechada`/`ligada` → `onIa(site, !site.ia_pode_usar)`. O erro do backend continua aparecendo no cartão (`erro`).

- [ ] **Step 3: CSS.** Portar para `estilos.ts` mantendo os tokens do app (`var(--...)`); a cor `semlogin` do `.st-estado` é `var(--success)`. Cuidar das larguras estreitas: nada de `white-space: nowrap` que corte o texto da faixa "falta" até 300 px (se cortar, reduzir o `gap`, não o texto).

- [ ] **Step 4: `npx tsc --noEmit`, `npm run build`, `node --test tests/front/`: código de saída 0.**

- [ ] **Step 5 (revisor, front): abrir o app** (`/claudia-teste-e2e`, worktree desta branch) e conferir, com prints, nos dois idiomas: galeria com sites logado, sem login, não logado e fechado/liberado; clicar no corpo abre a página; clicar em "Abrir ↗" e no domínio **não** abre a página; clicar na faixa de um site pronto alterna a IA e **não** abre a página; clicar na faixa de um site não pronto abre a página; Enter e Espaço na faixa; foco visível; larguras estreitas; o bloco de erro do backend. Comparar com o mock aberto lado a lado.

- [ ] **Step 6: commit** `feat(sites): rectangular card, whole-card opens the page, only the strip toggles the AI`.

---

### Task 6: página do site com passos, Sobre e zona de risco

**Files:**
- Create: `src/components/sites/Passos.tsx`, `Sobre.tsx`, `ZonaRisco.tsx`
- Modify: `src/components/sites/Detalhe.tsx`, `estilos.ts`, `tipos.ts`, `index.tsx` (novas ações)
- Delete: `src/components/sites/Acoes.tsx` (**só depois** de `Detalhe` não importá-lo mais; `git rm`, um único arquivo de código-fonte, nada de caminho variável)

**Interfaces:**
- Consumes: T4 (`passo1`, `pronto`, `podeAlternarSemLogin`, `mostraJaEntrei`, `situacaoFaixa`, `sitesApi.estatisticas`, textos), `Faixa` (T5).
- Produces: `AcoesSite` ganha

```ts
onSemLogin: (s: Site, valor: boolean) => void;   // sitesApi.definirSemLogin
onNaoLogado: (s: Site) => void;                  // sitesApi.marcarNaoLogado
```

  e `index.tsx` as liga com o mesmo `agir(chave, ...)` das demais ações.

- [ ] **Step 1: `Passos.tsx`.** Painel "O que a IA pode fazer aqui" com três cartões, sempre na ordem 1, 2, 3, conforme a spec §2.4 e o mock (`.passo`, `.passo.atual`, `.passo.feito`, `.passo.travado`, `.num`):
  - **Passo 1:** `pendente` → "Abrir o site ↗" (primário, chama `onAbrir` e marca `jaAbriu = true` no estado local do `Detalhe`), "Já entrei" só quando `mostraJaEntrei(jaAbriu, site)` (senão o texto `afterOpenHint`), e o botão comum `noLoginBtn` quando `podeAlternarSemLogin(site)`. `logado` → selo "Feito", "Abrir o site ↗" e `notLoggedBtn`. `semLogin` → selo "Feito", texto `step1NoLogin` e `asksLoginBtn` (só quando `podeAlternarSemLogin`).
  - **Passo 2:** desabilitado, selo `soonBadge`.
  - **Passo 3:** se `pronto(site)`, mostra a `Faixa` (a mesma do cartão, com o mesmo clique); senão o texto `step3Locked` e o selo `lockedBadge`.
- [ ] **Step 2: `Sobre.tsx`** com três blocos separados por divisórias com título (`.divisoria`): **Cadastro** (endereço como link que chama `onAbrir`, tipo, região, país, papel, origem, domínios), **Conta** e **Uso pela IA**. Conta: site `semLogin` mostra só `loginRow: loginNotAsked`; senão "Confirmado em" (data local formatada com `toLocaleDateString(locale)` a partir de `verificada_em`, ou `notEntered`) e "Relogins" (`relogins` + `timesOne`/`timesMany`). Uso: `aberturas` (+ `lastOn`), `vagas`, `candidaturas` (+ `lastOn` se houver), e "Taxa" **somente** quando `vagas > 0`; `aberturas === 0 && vagas === 0 && candidaturas === 0` mostra `neverOpened`. As estatísticas vêm de `sitesApi.estatisticas(site.chave)`, carregadas ao abrir a página e recarregadas quando `SITES_MUDARAM` dispara; **erro de carregamento** mostra `statsUnavailable` no bloco "Uso pela IA" e o resto da página segue funcionando. "Logado desde" do mock vira "Confirmado em" (a data é a de `verificada_em`).
- [ ] **Step 3: `ZonaRisco.tsx`**: `<details>` recolhido por padrão, título em `--danger`, linha "Remover da lista" com o botão `removeBtn`, **desabilitado** quando `site.embutida` (com `removeHint`), e o fluxo de confirmação que `Acoes.tsx` já tinha para os sites do usuário (`removeConfirm`, Cancelar). A abertura da seção sobrevive a recarregar os dados (estado em `Detalhe`, não no filho).
- [ ] **Step 4: `Detalhe.tsx`** compõe: topo ("← Sites", nome, selos local e tipo, estado à direita), duas colunas de **mesma altura** (`align-items: stretch`; o cartão da esquerda preenche a altura da direita), e a zona de risco abaixo. Em largura menor que 820 px, uma coluna. Remover `Acoes.tsx`.
- [ ] **Step 5: `npx tsc --noEmit`, `npm run build`, `node --test tests/front/`: código de saída 0.**
- [ ] **Step 6 (revisor, front): abrir o app** e, com prints nos dois idiomas, exercitar: site não logado (Abrir → aparece "Já entrei" → clicar → passo 1 vira "Feito" e o passo 3 libera; liberar liga a IA); "Não estou mais logado" fecha a IA e volta o passo 1; **"Este site não pede login" depois "Este site pede login" sem estar logado deixa a IA fechada** (confirmar também no banco com o `invoke` de `listar_sites`); LinkedIn (`cookie_sessao`) sem o botão; site embutido com "Remover site" desabilitado e site adicionado por você removível com confirmação; colunas com a mesma altura; seção "Uso pela IA" com dados, zerada e com erro (forçar o erro no mock); teclado; larguras estreitas.
- [ ] **Step 7: commit** `feat(sites): guided site page with steps, account and usage panels, collapsible risk zone`.

---

### Task 7: visão em lista (só começa depois do desenho aprovado)

**Pré-condição (o orquestrador cumpre antes de despachar):** mostrar ao usuário um mock em HTML da linha da lista no mesmo padrão e registrar no ledger a aprovação escrita dele. Sem isso, **esta tarefa não começa** (spec §8). O mock da linha vai para `docs/superpowers/specs/assets/` e a spec ganha a data.

**Files:** Modify `src/components/sites/Lista.tsx`, `estilos.ts`.

**Interfaces:** consumes `situacaoFaixa`, `pronto`, `estadoVisual` (cor `semlogin`).

- [ ] **Step 1:** a linha inteira abre a página (mesmo padrão de link esticado da T5); só o texto de status à direita liga a IA, e vira um botão com `role="switch"` quando o site está pronto (`Liberada` / `Fechada`), ou um botão que abre a página quando não está (`Faça o passo 1`); "Abrir ↗" e o domínio não abrem a página. Sem `Interruptor` na linha.
- [ ] **Step 2:** tipos com os rótulos novos; risco lateral com a cor `semlogin`.
- [ ] **Step 3:** `npx tsc --noEmit`, `npm run build`, `node --test tests/front/`: código de saída 0.
- [ ] **Step 4 (revisor, front):** abrir o app em modo lista, nos dois idiomas, com prints; mesmos comportamentos de clique e teclado da T5.
- [ ] **Step 5: commit** `feat(sites): list rows open the page, only the status text toggles the AI`.

---

### Task 8: docs vivos e smoke de uso real

**Files:** Modify `docs/padrao-design-claudia-rh.md` (§15), `docs/arquitetura-sistema-candidaturas.md` (§3.8), `docs/superpowers/specs/2026-10-15-sites-fluxo-guiado-design.md` (seção "Estado da implementação", com o que foi e o que ficou "não verificado").

- [ ] **Step 1:** atualizar §15 (cartão retangular de 300 px, faixa única, página com passos, zona de risco, rótulos de tipo) e §3.8 (colunas novas, três comandos, a regra de fechar a IA, contagem de aberturas aceitas).
- [ ] **Step 2: smoke de uso real** (`/claudia-teste-e2e`, sem site real, sem enviar candidatura): abrir o app, ir a Sites, exercitar o fluxo completo (galeria → página → passos → liberar → voltar; modo lista; nos dois idiomas; estados vazio, erro e com dados), chamar por `invoke` `estatisticas_site`, `definir_site_sem_login` e `marcar_site_nao_logado` e conferir o banco. Anexar prints e saídas na descrição do PR **se** o usuário pedir PR.
- [ ] **Step 3:** rodar de novo as três rodadas: `cargo test --lib` no Sandbox, `npx tsc --noEmit`, `npm run build`, `node --test tests/front/`, todos com código de saída conferido. Registrar no Linear (skill `linear-claudia`) os itens fora de escopo achados no caminho.
- [ ] **Step 4: commit** `docs(sites): document the guided flow and record the smoke result`. **Não** fazer merge, push nem PR: é decisão do usuário.

---

## Autorrevisão

- **Cobertura da spec:** §2.1 a §2.3 → T5; §2.4 e §2.5 → T3, T6; §2.6 → T6; §2.7 e §4.3 → T2, T6; §2.8 e §4.4 → T4; P2 → T1; P4 → T7; P5 → T3, T4, T6; §4.2 (`marcar_site_nao_logado`) → T3. Sem lacuna conhecida.
- **Placeholders:** os dois pontos que dependem do que o executor encontrar no código (helpers `db_with_linkedin_and_ats` e `db_with_two_ats_companies`; o ajuste do helper `db()` do `gate.rs`) estão descritos com o formato exato do seed a usar.
- **Consistência de tipos:** `SituacaoFaixa`, `Passo1`, `SiteStats`, `definir_sem_login`, `marcar_nao_logado`, `resolver_em` aparecem com os mesmos nomes e assinaturas nas tarefas que os produzem e consomem.
- **Pontos em aberto herdados da spec §8:** confiabilidade de "Vagas analisadas" e "Candidaturas" por domínio é **verificada** na T2 (testes com ATS compartilhado). Se o revisor da T2 mostrar que o casamento por URL erra, o orquestrador **para** e pergunta ao usuário se esses números e a Taxa ficam ocultos (T6 já esconde a Taxa quando `vagas = 0`).
