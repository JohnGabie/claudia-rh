# Lista de sites Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Entregar o catálogo padrão de sites de vagas (seed + botão "+"), a tela **Sites** (galeria e lista, busca, filtros, ordenação), o estado de login por site e a rigidez do MCP: a IA só abre sites da lista, liberados e logados.

**Architecture:** Um módulo Rust novo `src-tauri/src/sites/` com funções puras (casamento de domínio, validação e mesclagem do seed) e acesso ao SQLite (`contas_browser`, `dominios_recusados`, `avisos_sites`). O MCP consulta o módulo por um trait `SiteGate` injetado no `BrowserTools` (mesmo padrão do `GuardContext`). A tela vive em `src/components/sites/`, com a lógica de filtro e ordenação em `logica.ts`, pura e testada com `node --test`.

**Tech Stack:** Rust (Tauri v2, rusqlite 0.31 bundled, serde_json, rmcp), React + TypeScript, `node --test` para o front.

**Spec:** `docs/superpowers/specs/2026-10-14-lista-de-sites-design.md` (aprovada em 2026-10-14). Pesquisa do seed: `docs/pesquisa/2026-10-14-lista-de-sites-pesquisa.md`.

## Global Constraints

- Comentários de código, identificadores novos e mensagens de commit em **inglês**. Copy da UI em **pt-BR** (e `en` nos arquivos de i18n). Docs vivos em pt-BR.
- Cada commit termina com a linha `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- **Nunca** mudar o `identifier` de `src-tauri/tauri.conf.json`.
- **Nenhum passo de instalação novo:** o seed vai embutido no binário com `include_str!`.
- **Nenhum caminho absoluto de máquina** em código, testes, docs ou scripts.
- **Testes nunca tocam o disco real.** Os testes Rust usam `Connection::open_in_memory()` + `crate::db::apply(&conn)`. **`cargo test` (qualquer um) roda só no Sandbox** (skill `claudia-teste-sandbox`); no PC só `cargo check`, `cargo clippy --all-targets`, `cargo test --no-run`, `npx tsc --noEmit` e os testes de front (`node --test`), que não tocam o disco.
- Nada que apague arquivo entra neste plano. As remoções são de linhas do banco, só de sites `embutida = 0`.
- Só `https` nas URLs dos sites. O valor de cookie nunca é lido, guardado ou logado (esta branch não lê cookie).
- **`src-tauri/src/prompt_sistema_runtime.md` só é editado na Task 12**, depois de o usuário revisar o texto exato.
- Integrar (merge, push, PR) é decisão do usuário. Nada de push neste plano.
- Execução por agentes: uma worktree por agente, cada um com `CARGO_TARGET_DIR` fora do `target/` do repositório; revisão final por Opus (mudança de ferramenta do agente).

## Decisões que o plano toma (revise antes de executar)

- **D1, aviso de relogar (diverge da spec §4.4):** `pendencias.vaga_id` é `NOT NULL REFERENCES vagas(id)`, e "relogar em X" não tem vaga. Em vez de reconstruir a tabela `pendencias` do usuário (risco para dados reais), o plano cria a tabela `avisos_sites` e mostra o aviso **na tela Sites e no badge da sidebar**. A tela **Pendências** não muda. **Este trecho precisa da sua aprovação**, e a Task 8 não começa sem ela.
- **D2, vaga fora da lista:** a recusa é do `browser_open`; quem salva a vaga é o agente, com `register_vaga` + `update_vaga_status('pulada', "site fora da lista: <domínio>")`, instruído pela mensagem de recusa. O app **não** cria vaga sozinho (o `browser_open` só recebe uma URL, sem título nem empresa). A tabela `dominios_recusados` guarda só domínio, contagem, última vez e a última URL **sem query string**. Resolve o "em aberto" da spec §8.
- **D3, abrir o site para logar:** abre uma aba nova no browser da Claudia por CDP HTTP (`PUT /json/new?<url>`), depois de `request_start`. É o único ponto com leitura de código ainda por fazer (Task 9, Step 1).
- **D4, seed:** a Task 2 entrega um **seed inicial de 9 sites** com URLs confirmadas na pesquisa. A curadoria até ~100 é a Task 14 e **depende de você** (lista e risco dos termos de uso).
- **D5, fora do plano:** `DbGuardContext::site_allowed` continua retornando `true`. Ligá-lo à lista mudaria a guarda de envio e não está na spec.

## File Structure

| Arquivo | Responsabilidade |
|---|---|
| `src-tauri/src/sites/mod.rs` | Declara os submódulos e reexporta o que o resto do app usa |
| `src-tauri/src/sites/dominio.rs` | Puro: URL → host e caminho, casamento por fronteira de rótulo, caminho do ATS, chave a partir da URL |
| `src-tauri/src/sites/modelo.rs` | Constantes (`TIPOS`, `REGIOES`, `PAPEIS`, `REGRAS`), `SeedSite`, validação do seed |
| `src-tauri/src/sites/seed.rs` | Seed embutido, `parse_seed`, `merge` idempotente por `chave` |
| `src-tauri/src/sites/repo.rs` | `Site`, listar, permissão, login, adicionar, remover, resolver URL → site |
| `src-tauri/src/sites/gate.rs` | `decidir` (regras do `browser_open`), recusas, `DbSiteGate` |
| `src-tauri/src/sites/avisos.rs` | Tabela `avisos_sites`: criar, listar, contar, resolver |
| `src-tauri/sites-padrao.json` | O seed |
| `src-tauri/src/browser/sitegate.rs` | O trait `SiteGate` (o `browser/` não depende de `sites/`) |
| `src-tauri/src/mcp/tools/sites.rs` | A ferramenta `browser_contas` |
| `src-tauri/src/commands/sites.rs` | Comandos Tauri da tela |
| `src/lib/sites-api.ts` | A única porta do front para os comandos |
| `src/components/sites/` | `index.tsx`, `logica.ts`, `Barra.tsx`, `Galeria.tsx`, `Lista.tsx`, `FormAdicionar.tsx`, `Avatar.tsx` |
| `tests/front/sites-logica.test.ts` | Testes da lógica pura do front |

Modificados: `src-tauri/src/db/schema.sql`, `src-tauri/src/lib.rs`, `src-tauri/src/mcp/server.rs`, `src-tauri/src/mcp/mod.rs`, `src-tauri/src/mcp/tools/mod.rs`, `src-tauri/src/mcp/tools/browser.rs`, `src-tauri/src/browser/mod.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/prompts.rs` (teste), `src/App.tsx`, `src/components/Sidebar.tsx`, `src/i18n/pt.ts`, `src/i18n/en.ts`, `docs/padrao-design-claudia-rh.md`, `docs/arquitetura-sistema-candidaturas.md`.

## Review Focus

Entradas que a spec implica, nenhuma tarefa de feature exercita sozinha, e que mais provavelmente quebram para quem usa:

1. **URL enganosa no `browser_open`:** `https://linkedin.com@evil.com`, `https://evil.com\@linkedin.com`, `https://linkedin.com.evil.com`, `https://LINKEDIN.COM./`, `https://linkedin.com:443/`. Esperado: só o host real conta, e os três primeiros são recusados. (Task 1 e Task 5.)
2. **Seed colide com site do usuário:** o usuário adicionou `jobindex-dk` e uma versão nova do app traz a mesma `chave`. Esperado: nada do usuário é sobrescrito, nem `ia_pode_usar`, nem `estado`. (Task 3.)
3. **Permissão ligada com o site sem login:** o usuário (ou um bug da tela) liga "IA pode usar" num site `precisa_login`. Esperado: o **backend** recusa; a tela não é a única defesa. (Task 4.)
4. **Banco travado ou ilegível durante `browser_open`:** esperado: recusa clara ("não foi possível consultar a lista"), nunca `Allow` e nunca pânico. (Task 5.)
5. **Busca e ordenação com acento e vazio:** "Ørsted", "Zürich", busca por `jobindex.dk`, busca vazia, filtros que não casam nada. Esperado: acento não atrapalha, ordem estável, lista vazia sem erro. (Task 10.)

---

### Task 1: Casamento de domínio (puro)

**Files:**
- Create: `src-tauri/src/sites/mod.rs`
- Create: `src-tauri/src/sites/dominio.rs`
- Modify: `src-tauri/src/lib.rs` (declarar `mod sites;`)

**Interfaces:**
- Produces:
  - `pub fn host_and_path(url: &str) -> Option<(String, String)>`
  - `pub fn host_matches(host: &str, domain: &str) -> bool`
  - `pub fn path_under(path: &str, prefix: &str) -> bool`
  - `pub struct DomainRules<'a> { pub dominios: &'a [String], pub dominios_ats: &'a [String], pub caminho_ats: Option<&'a str> }`
  - `pub fn url_matches(rules: &DomainRules, url: &str) -> bool`
  - `pub fn chave_from_url(url: &str) -> Option<String>`
  - `pub fn valid_domain(d: &str) -> bool`

- [ ] **Step 1: Declarar o módulo**

Em `src-tauri/src/lib.rs`, ao lado dos outros `mod`, acrescente `mod sites;`. Crie `src-tauri/src/sites/mod.rs`:

```rust
//! The curated list of job sites: pure domain matching, the embedded seed, the stored sites and the
//! gate that decides what the agent may open.
pub mod dominio;
```

- [ ] **Step 2: Escrever os testes que falham**

Crie `src-tauri/src/sites/dominio.rs` só com o bloco de testes e as assinaturas vazias (`todo!()` não: use `unimplemented!()` só neste passo, para compilar):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rules<'a>(d: &'a [String], a: &'a [String], p: Option<&'a str>) -> DomainRules<'a> {
        DomainRules { dominios: d, dominios_ats: a, caminho_ats: p }
    }
    fn v(xs: &[&str]) -> Vec<String> { xs.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn the_real_host_is_the_one_after_the_last_at_sign() {
        assert_eq!(host_and_path("https://linkedin.com@evil.com/x").unwrap().0, "evil.com");
    }

    #[test]
    fn a_backslash_counts_as_a_slash_like_in_a_browser() {
        // Chrome reads this as host evil.com, path "/@linkedin.com".
        assert_eq!(host_and_path("https://evil.com\\@linkedin.com").unwrap().0, "evil.com");
    }

    #[test]
    fn host_is_lowercased_and_loses_port_and_trailing_dot() {
        assert_eq!(host_and_path("HTTPS://LinkedIn.COM.:443/Jobs?q=1#x").unwrap(), ("linkedin.com".to_string(), "/jobs".to_string()));
    }

    #[test]
    fn only_plain_http_and_https_with_a_clean_host_are_read() {
        for bad in ["", "ftp://a.com", "javascript:alert(1)", "https://", "https://[::1]/", "https://a b.com", "https://a.com/\n", "https://bücher.de/"] {
            assert!(host_and_path(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_domain_matches_itself_and_its_subdomains_only_at_a_label_boundary() {
        assert!(host_matches("linkedin.com", "linkedin.com"));
        assert!(host_matches("www.linkedin.com", "linkedin.com"));
        assert!(!host_matches("evil-linkedin.com", "linkedin.com"));
        assert!(!host_matches("linkedin.com.evil.com", "linkedin.com"));
        assert!(!host_matches("notlinkedin.com", "linkedin.com"));
        assert!(!host_matches("linkedin.com", ""));
    }

    #[test]
    fn a_path_prefix_matches_at_a_segment_boundary() {
        assert!(path_under("/canonical", "/canonical"));
        assert!(path_under("/canonical/jobs/1", "/canonical"));
        assert!(!path_under("/canonicalx", "/canonical"));
        assert!(!path_under("/other/canonical", "/canonical"));
        assert!(path_under("/anything", ""));
    }

    #[test]
    fn ats_domains_need_the_company_path_but_career_domains_do_not() {
        let dom = v(&["canonical.com"]);
        let ats = v(&["job-boards.greenhouse.io"]);
        let r = rules(&dom, &ats, Some("/canonical"));
        assert!(url_matches(&r, "https://canonical.com/careers"));
        assert!(url_matches(&r, "https://job-boards.greenhouse.io/canonical/jobs/1"));
        assert!(!url_matches(&r, "https://job-boards.greenhouse.io/anthropic"));
        assert!(!url_matches(&r, "https://greenhouse.io/canonical"));
    }

    #[test]
    fn tricky_urls_never_match_the_trusted_domain() {
        let dom = v(&["linkedin.com"]);
        let r = rules(&dom, &[], None);
        for url in ["https://linkedin.com@evil.com/", "https://evil.com\\@linkedin.com", "https://linkedin.com.evil.com/", "https://evil.com/linkedin.com"] {
            assert!(!url_matches(&r, url), "{url}");
        }
        for url in ["https://LINKEDIN.COM./", "https://linkedin.com:443/jobs", "http://www.linkedin.com/jobs"] {
            assert!(url_matches(&r, url), "{url}");
        }
    }

    #[test]
    fn the_key_of_a_user_added_site_comes_from_its_domain() {
        assert_eq!(chave_from_url("https://www.Jobindex.dk/jobsoegning").as_deref(), Some("jobindex-dk"));
        assert_eq!(chave_from_url("not a url"), None);
    }

    #[test]
    fn a_domain_for_the_seed_is_bare_lowercase_with_a_dot() {
        assert!(valid_domain("linkedin.com"));
        assert!(valid_domain("lego.wd103.myworkdayjobs.com"));
        for bad in ["", "localhost", "https://a.com", "a.com/x", "A.com", "a..com", ".a.com", "a.com."] {
            assert!(!valid_domain(bad), "{bad}");
        }
    }
}
```

- [ ] **Step 3: Rodar e ver falhar**

Run (no Sandbox): `cargo test --lib sites::dominio -- --nocapture` em `src-tauri/`
Expected: FAIL (a compilação falha por falta das funções).
No PC, só confira que o bloco de testes escrito compila depois do Step 4 com `cargo check --tests`.

- [ ] **Step 4: Implementar**

Acima do bloco `#[cfg(test)]`, em `src-tauri/src/sites/dominio.rs`:

```rust
//! Pure URL reading and domain matching. No I/O, so every rule is testable without a database.

/// Host and path of an http(s) URL as a browser would read them, both lowercased. `None` when it is
/// not a plain http(s) URL with an ASCII host (IP literals and IDNs are refused on purpose).
pub fn host_and_path(url: &str) -> Option<(String, String)> {
    let u = url.trim();
    if u.is_empty() || u.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let lower = u.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"))?;
    // In special schemes a browser treats "\" as "/": "https://evil.com\@good.com" is host evil.com.
    let rest = rest.replace('\\', "/");
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    // The host is what follows the LAST "@": everything before it is user info.
    let host_port = authority.rsplit('@').next()?;
    if host_port.starts_with('[') {
        return None;
    }
    let host = host_port.split(':').next()?.trim_end_matches('.');
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-') {
        return None;
    }
    let path = if tail.starts_with('/') { tail.split(['?', '#']).next().unwrap_or("/") } else { "/" };
    Some((host.to_string(), path.to_string()))
}

/// `host` is `domain` or a subdomain of it, matched only at a label boundary.
pub fn host_matches(host: &str, domain: &str) -> bool {
    let d = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    !d.is_empty() && (host == d || host.ends_with(&format!(".{d}")))
}

/// `path` is `prefix` or lives under it, at a segment boundary. An empty prefix matches everything.
pub fn path_under(path: &str, prefix: &str) -> bool {
    let p = prefix.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() {
        return true;
    }
    let p = if p.starts_with('/') { p } else { format!("/{p}") };
    path == p || path.starts_with(&format!("{p}/"))
}

pub struct DomainRules<'a> {
    pub dominios: &'a [String],
    pub dominios_ats: &'a [String],
    pub caminho_ats: Option<&'a str>,
}

/// A career-site domain matches any path; an ATS domain matches only under the company path.
pub fn url_matches(rules: &DomainRules, url: &str) -> bool {
    let Some((host, path)) = host_and_path(url) else { return false };
    if rules.dominios.iter().any(|d| host_matches(&host, d)) {
        return true;
    }
    rules.dominios_ats.iter().any(|d| host_matches(&host, d))
        && rules.caminho_ats.is_none_or(|p| path_under(&path, p))
}

/// Stable key of a user-added site: its host without "www.", dots turned into dashes.
pub fn chave_from_url(url: &str) -> Option<String> {
    let (host, _) = host_and_path(url)?;
    let host = host.strip_prefix("www.").unwrap_or(&host);
    Some(host.replace('.', "-"))
}

/// A bare, lowercase domain with at least one dot: the shape stored in the seed.
pub fn valid_domain(d: &str) -> bool {
    !d.is_empty()
        && d.contains('.')
        && !d.starts_with('.')
        && !d.ends_with('.')
        && !d.contains("..")
        && d.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}
```

- [ ] **Step 5: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::dominio`
Expected: PASS (10 testes). No PC: `cargo check --tests` sem erro.

- [ ] **Step 6: Prova de mutação**

Troque `rsplit('@')` por `split('@')` em `host_and_path`, rode o teste e confirme que **`the_real_host_is_the_one_after_the_last_at_sign` falha** (cole a linha `FAILED`). Troque o `replace('\\', "/")` por uma linha vazia e confirme que **`a_backslash_counts_as_a_slash_like_in_a_browser` falha**. Restaure as duas.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/sites/mod.rs src-tauri/src/sites/dominio.rs
git commit -m "feat(sites): pure domain matching for the site list"
```

---

### Task 2: Modelo, validação do seed e seed inicial

**Files:**
- Create: `src-tauri/src/sites/modelo.rs`
- Create: `src-tauri/src/sites/seed.rs`
- Create: `src-tauri/sites-padrao.json`
- Modify: `src-tauri/src/sites/mod.rs`

**Interfaces:**
- Consumes: `dominio::{host_and_path, host_matches, valid_domain}`
- Produces:
  - `pub const TIPOS: &[&str]`, `REGIOES`, `PAPEIS`, `REGRAS`
  - `pub struct SeedSite { chave, nome, url_login, dominios: Vec<String>, dominios_ats: Vec<String>, caminho_ats: Option<String>, tipo, regiao, pais: Option<String>, papel, regra_login }` (`Deserialize`, `Clone`, `Debug`, `PartialEq`)
  - `pub fn parse_seed(json: &str) -> Result<Vec<SeedSite>, String>`
  - `pub const SEED_JSON: &str` e `pub fn embedded() -> Result<Vec<SeedSite>, String>`

- [ ] **Step 1: Escrever os testes que falham**

Em `src-tauri/src/sites/seed.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn site(chave: &str) -> String {
        format!(r#"{{"chave":"{chave}","nome":"X","url_login":"https://www.x.com/","dominios":["x.com"],"tipo":"geral","regiao":"GLOBAL"}}"#)
    }
    fn file(sites: &[String]) -> String {
        format!(r#"{{"versao":1,"sites":[{}]}}"#, sites.join(","))
    }

    #[test]
    fn the_embedded_seed_is_valid() {
        let s = embedded().expect("the shipped seed must parse and validate");
        assert!(s.len() >= 9);
    }

    #[test]
    fn defaults_are_candidatura_and_manual() {
        let s = parse_seed(&file(&[site("x")])).unwrap();
        assert_eq!((s[0].papel.as_str(), s[0].regra_login.as_str()), ("candidatura", "manual"));
    }

    #[test]
    fn a_repeated_key_is_rejected() {
        assert!(parse_seed(&file(&[site("x"), site("x")])).unwrap_err().contains("repetida"));
    }

    #[test]
    fn invalid_fields_are_rejected_with_the_site_key_in_the_message() {
        let cases = [
            (r#""tipo":"geral""#, r#""tipo":"banana""#),
            (r#""regiao":"GLOBAL""#, r#""regiao":"MARS""#),
            (r#""url_login":"https://www.x.com/""#, r#""url_login":"http://www.x.com/""#),
            (r#""url_login":"https://www.x.com/""#, r#""url_login":"https://other.org/""#),
            (r#""dominios":["x.com"]"#, r#""dominios":[]"#),
            (r#""dominios":["x.com"]"#, r#""dominios":["https://x.com"]"#),
        ];
        for (from, to) in cases {
            let bad = file(&[site("x")]).replace(from, to);
            let e = parse_seed(&bad).unwrap_err();
            assert!(e.contains("x"), "{to}: {e}");
        }
    }

    #[test]
    fn an_ats_path_needs_an_ats_domain() {
        let bad = file(&[site("x")]).replace(r#""tipo":"geral""#, r#""tipo":"empresa","caminho_ats":"/x""#);
        assert!(parse_seed(&bad).unwrap_err().contains("caminho_ats"));
    }
}
```

- [ ] **Step 2: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib sites::seed`
Expected: FAIL (funções e arquivo JSON inexistentes).

- [ ] **Step 3: Implementar `modelo.rs`**

```rust
//! The vocabulary of the site list and the validation of the seed file.
use super::dominio::{host_and_path, host_matches, valid_domain};
use serde::Deserialize;
use std::collections::HashSet;

pub const TIPOS: &[&str] = &["geral", "regional", "tecnologia", "remoto", "freelance", "empresa", "nicho"];
pub const REGIOES: &[&str] = &["SA", "NA", "EU", "AS", "AF", "OC", "GLOBAL"];
pub const PAPEIS: &[&str] = &["candidatura", "descoberta"];
pub const REGRAS: &[&str] = &["manual", "cookie_sessao", "nenhuma"];

fn papel_padrao() -> String { "candidatura".into() }
fn regra_padrao() -> String { "manual".into() }

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SeedSite {
    pub chave: String,
    pub nome: String,
    pub url_login: String,
    pub dominios: Vec<String>,
    #[serde(default)]
    pub dominios_ats: Vec<String>,
    #[serde(default)]
    pub caminho_ats: Option<String>,
    pub tipo: String,
    pub regiao: String,
    #[serde(default)]
    pub pais: Option<String>,
    #[serde(default = "papel_padrao")]
    pub papel: String,
    #[serde(default = "regra_padrao")]
    pub regra_login: String,
}

#[derive(Deserialize)]
pub(super) struct SeedFile {
    #[allow(dead_code)]
    pub versao: u32,
    pub sites: Vec<SeedSite>,
}

fn key_ok(k: &str) -> bool {
    !k.is_empty()
        && k.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Checks one seed entry. The message always starts with the site key so a bad entry is easy to find.
pub fn validate_site(s: &SeedSite) -> Result<(), String> {
    let e = |m: &str| Err(format!("{}: {m}", s.chave));
    if !key_ok(&s.chave) {
        return e("chave inválida (minúsculas, números e hífen)");
    }
    if s.nome.trim().is_empty() {
        return e("nome vazio");
    }
    if !s.url_login.to_ascii_lowercase().starts_with("https://") {
        return e("url_login precisa ser https");
    }
    let Some((host, _)) = host_and_path(&s.url_login) else { return e("url_login ilegível") };
    if s.dominios.is_empty() {
        return e("dominios vazio");
    }
    if let Some(d) = s.dominios.iter().chain(&s.dominios_ats).find(|d| !valid_domain(d)) {
        return e(&format!("domínio inválido '{d}' (sem esquema, sem caminho, minúsculas)"));
    }
    if !s.dominios.iter().chain(&s.dominios_ats).any(|d| host_matches(&host, d)) {
        return e("url_login não pertence a nenhum dos domínios");
    }
    if let Some(p) = &s.caminho_ats {
        if s.dominios_ats.is_empty() {
            return e("caminho_ats exige dominios_ats");
        }
        if !p.starts_with('/') {
            return e("caminho_ats precisa começar com /");
        }
    }
    if !TIPOS.contains(&s.tipo.as_str()) { return e("tipo desconhecido"); }
    if !REGIOES.contains(&s.regiao.as_str()) { return e("regiao desconhecida"); }
    if !PAPEIS.contains(&s.papel.as_str()) { return e("papel desconhecido"); }
    if !REGRAS.contains(&s.regra_login.as_str()) { return e("regra_login desconhecida"); }
    if let Some(p) = &s.pais {
        if p.len() != 2 || !p.chars().all(|c| c.is_ascii_uppercase()) {
            return e("pais precisa ser código ISO de 2 letras maiúsculas");
        }
    }
    Ok(())
}

pub fn validate_all(sites: &[SeedSite]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for s in sites {
        if !seen.insert(s.chave.as_str()) {
            return Err(format!("{}: chave repetida", s.chave));
        }
        validate_site(s)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Implementar `seed.rs` (parse e leitura do embutido)**

Acima do bloco de testes:

```rust
//! The embedded seed and its merge into the stored list.
use super::modelo::{validate_all, SeedFile, SeedSite};

pub const SEED_JSON: &str = include_str!("../../sites-padrao.json");

pub fn parse_seed(json: &str) -> Result<Vec<SeedSite>, String> {
    let f: SeedFile = serde_json::from_str(json).map_err(|e| format!("seed ilegível: {e}"))?;
    validate_all(&f.sites)?;
    Ok(f.sites)
}

pub fn embedded() -> Result<Vec<SeedSite>, String> {
    parse_seed(SEED_JSON)
}
```

E em `src-tauri/src/sites/mod.rs`:

```rust
pub mod dominio;
pub mod modelo;
pub mod seed;
```

- [ ] **Step 5: Criar o seed inicial**

`src-tauri/sites-padrao.json` (9 sites, todos com URL aberta ou entregue pelo usuário na pesquisa; **o conteúdo continua não verificado até a revisão do usuário na Task 14**):

```json
{
  "versao": 1,
  "sites": [
    {"chave": "linkedin", "nome": "LinkedIn", "url_login": "https://www.linkedin.com/jobs/", "dominios": ["linkedin.com"], "tipo": "geral", "regiao": "GLOBAL", "papel": "candidatura"},
    {"chave": "indeed-br", "nome": "Indeed Brasil", "url_login": "https://br.indeed.com/", "dominios": ["indeed.com"], "tipo": "geral", "regiao": "GLOBAL", "papel": "descoberta"},
    {"chave": "vagas-com", "nome": "Vagas.com", "url_login": "https://www.vagas.com.br/login-candidatos", "dominios": ["vagas.com.br"], "tipo": "geral", "regiao": "SA", "pais": "BR"},
    {"chave": "jobindex", "nome": "Jobindex", "url_login": "https://www.jobindex.dk/bruger/log-ind", "dominios": ["jobindex.dk"], "tipo": "geral", "regiao": "EU", "pais": "DK"},
    {"chave": "thehub", "nome": "The Hub", "url_login": "https://thehub.io/", "dominios": ["thehub.io"], "tipo": "tecnologia", "regiao": "EU"},
    {"chave": "reed", "nome": "Reed", "url_login": "https://www.reed.co.uk/", "dominios": ["reed.co.uk"], "tipo": "geral", "regiao": "EU", "pais": "GB"},
    {"chave": "canonical", "nome": "Canonical", "url_login": "https://canonical.com/careers", "dominios": ["canonical.com"], "dominios_ats": ["job-boards.greenhouse.io"], "caminho_ats": "/canonical", "tipo": "empresa", "regiao": "GLOBAL", "regra_login": "nenhuma"},
    {"chave": "anthropic", "nome": "Anthropic", "url_login": "https://www.anthropic.com/careers", "dominios": ["anthropic.com"], "dominios_ats": ["job-boards.greenhouse.io"], "caminho_ats": "/anthropic", "tipo": "empresa", "regiao": "GLOBAL", "regra_login": "nenhuma"},
    {"chave": "lego", "nome": "LEGO Group", "url_login": "https://www.lego.com/en-dk/careers/how-to-apply", "dominios": ["lego.com"], "dominios_ats": ["lego.wd103.myworkdayjobs.com"], "tipo": "empresa", "regiao": "EU", "pais": "DK"}
  ]
}
```

Nota: a Canonical e a Anthropic compartilham `job-boards.greenhouse.io` com **caminho diferente**; o casamento por caminho (Task 1) é o que impede uma de abrir a outra.

- [ ] **Step 6: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::seed`
Expected: PASS (5 testes).

- [ ] **Step 7: Prova de mutação**

Em `validate_site`, apague o bloco `if !s.dominios.iter().chain(...).any(|d| host_matches(&host, d))` e confirme que o caso `url_login":"https://other.org/"` de **`invalid_fields_are_rejected_with_the_site_key_in_the_message`** falha. Restaure.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/sites src-tauri/sites-padrao.json
git commit -m "feat(sites): seed format, validation and a starter seed"
```

---

### Task 3: Schema e mesclagem idempotente do seed

**Files:**
- Modify: `src-tauri/src/db/schema.sql` (acrescentar ao final)
- Modify: `src-tauri/src/sites/seed.rs`

**Interfaces:**
- Consumes: `SeedSite`, `parse_seed`
- Produces: `pub fn merge(conn: &Connection, seed: &[SeedSite]) -> rusqlite::Result<()>` e `pub fn merge_embedded(conn: &Connection) -> Result<(), String>`

- [ ] **Step 1: Acrescentar as tabelas ao `schema.sql`**

Ao final de `src-tauri/src/db/schema.sql`:

```sql
-- Curated job sites (spec 2026-10-14-lista-de-sites). `dominios` and `dominios_ats` are JSON arrays of
-- bare domains. An ATS domain only matches under `caminho_ats` (several companies share one ATS host).
CREATE TABLE IF NOT EXISTS contas_browser (
    chave TEXT PRIMARY KEY,
    nome TEXT NOT NULL,
    url_login TEXT NOT NULL,
    dominios TEXT NOT NULL,
    dominios_ats TEXT NOT NULL DEFAULT '[]',
    caminho_ats TEXT,
    tipo TEXT NOT NULL CHECK (tipo IN ('geral','regional','tecnologia','remoto','freelance','empresa','nicho')),
    regiao TEXT NOT NULL CHECK (regiao IN ('SA','NA','EU','AS','AF','OC','GLOBAL')),
    pais TEXT,
    papel TEXT NOT NULL DEFAULT 'candidatura' CHECK (papel IN ('candidatura','descoberta')),
    regra_login TEXT NOT NULL DEFAULT 'manual' CHECK (regra_login IN ('manual','cookie_sessao','nenhuma')),
    ia_pode_usar INTEGER NOT NULL DEFAULT 0,
    embutida INTEGER NOT NULL DEFAULT 0,
    padrao_atual INTEGER NOT NULL DEFAULT 1,
    estado TEXT NOT NULL DEFAULT 'desconhecido' CHECK (estado IN ('logada','precisa_login','desconhecido')),
    verificada_em TEXT
);

-- Domains the agent tried to open that are not on the list. Only the host, a counter and the last URL
-- WITHOUT its query string (it may carry tokens). Nothing here promotes a site by itself.
CREATE TABLE IF NOT EXISTS dominios_recusados (
    dominio TEXT PRIMARY KEY,
    contagem INTEGER NOT NULL DEFAULT 0,
    ultima_vez TEXT NOT NULL,
    ultima_url TEXT NOT NULL
);

-- "Log in again" notices (decision D1: pendencias.vaga_id is NOT NULL and these have no vaga).
CREATE TABLE IF NOT EXISTS avisos_sites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chave TEXT NOT NULL REFERENCES contas_browser(chave) ON DELETE CASCADE,
    criado_em TEXT NOT NULL,
    resolvido INTEGER NOT NULL DEFAULT 0,
    resolvido_em TEXT
);
```

- [ ] **Step 2: Escrever os testes que falham**

Acrescente em `seed.rs`, dentro do `mod tests`:

```rust
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }
    fn one(chave: &str, nome: &str) -> SeedSite {
        parse_seed(&file(&[site(chave).replace(r#""nome":"X""#, &format!(r#""nome":"{nome}""#))])).unwrap().remove(0)
    }
    fn count(c: &Connection) -> i64 {
        c.query_row("SELECT COUNT(*) FROM contas_browser", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn merge_inserts_embedded_sites_closed_for_the_ai() {
        let c = db();
        merge(&c, &[one("x", "X")]).unwrap();
        let (ia, estado, emb): (i64, String, i64) = c
            .query_row("SELECT ia_pode_usar, estado, embutida FROM contas_browser WHERE chave='x'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!((ia, estado.as_str(), emb), (0, "desconhecido", 1));
    }

    #[test]
    fn merging_twice_gives_the_same_table() {
        let c = db();
        merge(&c, &[one("x", "X"), one("y", "Y")]).unwrap();
        merge(&c, &[one("x", "X"), one("y", "Y")]).unwrap();
        assert_eq!(count(&c), 2);
    }

    #[test]
    fn merge_updates_the_catalog_fields_but_never_the_users_choices() {
        let c = db();
        merge(&c, &[one("x", "Old")]).unwrap();
        c.execute("UPDATE contas_browser SET ia_pode_usar=1, estado='logada', regra_login='cookie_sessao', verificada_em='2026-01-01 00:00:00' WHERE chave='x'", []).unwrap();
        merge(&c, &[one("x", "New")]).unwrap();
        let (nome, ia, estado, regra): (String, i64, String, String) = c
            .query_row("SELECT nome, ia_pode_usar, estado, regra_login FROM contas_browser WHERE chave='x'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap();
        assert_eq!((nome.as_str(), ia, estado.as_str(), regra.as_str()), ("New", 1, "logada", "cookie_sessao"));
    }

    #[test]
    fn a_site_the_user_added_with_the_same_key_is_left_alone() {
        let c = db();
        c.execute("INSERT INTO contas_browser (chave,nome,url_login,dominios,tipo,regiao,embutida) VALUES ('x','Mine','https://mine.org/','[\"mine.org\"]','geral','GLOBAL',0)", []).unwrap();
        merge(&c, &[one("x", "FromSeed")]).unwrap();
        let (nome, url): (String, String) = c.query_row("SELECT nome, url_login FROM contas_browser WHERE chave='x'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((nome.as_str(), url.as_str()), ("Mine", "https://mine.org/"));
    }

    #[test]
    fn a_site_that_left_the_seed_is_kept_and_flagged_not_current() {
        let c = db();
        merge(&c, &[one("x", "X"), one("y", "Y")]).unwrap();
        c.execute("UPDATE contas_browser SET ia_pode_usar=1 WHERE chave='y'", []).unwrap();
        merge(&c, &[one("x", "X")]).unwrap();
        let (padrao, ia): (i64, i64) = c.query_row("SELECT padrao_atual, ia_pode_usar FROM contas_browser WHERE chave='y'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((padrao, ia), (0, 1));
    }
```

- [ ] **Step 3: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib sites::seed`
Expected: FAIL (`merge` não definida).

- [ ] **Step 4: Implementar a mesclagem**

Em `seed.rs`, acima dos testes:

```rust
use rusqlite::{params, Connection};
use std::collections::HashSet;

/// Merges the seed by `chave`, idempotently. New sites arrive closed (`ia_pode_usar = 0`). For a
/// seed site already stored it refreshes only the catalog fields, and only when the stored row is a
/// seed row (`embutida = 1`): a site the user added with the same key is never touched. It never
/// changes `ia_pode_usar`, `estado`, `regra_login` or `verificada_em`. A seed row that left the seed
/// is kept and flagged `padrao_atual = 0`.
pub fn merge(conn: &Connection, seed: &[SeedSite]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    for s in seed {
        let dominios = serde_json::to_string(&s.dominios).unwrap_or_else(|_| "[]".into());
        let ats = serde_json::to_string(&s.dominios_ats).unwrap_or_else(|_| "[]".into());
        tx.execute(
            "INSERT INTO contas_browser \
             (chave, nome, url_login, dominios, dominios_ats, caminho_ats, tipo, regiao, pais, papel, regra_login, embutida, padrao_atual) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,1) \
             ON CONFLICT(chave) DO UPDATE SET \
               nome=excluded.nome, url_login=excluded.url_login, dominios=excluded.dominios, \
               dominios_ats=excluded.dominios_ats, caminho_ats=excluded.caminho_ats, tipo=excluded.tipo, \
               regiao=excluded.regiao, pais=excluded.pais, papel=excluded.papel, padrao_atual=1 \
             WHERE contas_browser.embutida = 1",
            params![s.chave, s.nome, s.url_login, dominios, ats, s.caminho_ats, s.tipo, s.regiao, s.pais, s.papel, s.regra_login],
        )?;
    }
    let in_seed: HashSet<&str> = seed.iter().map(|s| s.chave.as_str()).collect();
    let stored: Vec<String> = {
        let mut st = tx.prepare("SELECT chave FROM contas_browser WHERE embutida = 1")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for chave in stored.iter().filter(|k| !in_seed.contains(k.as_str())) {
        tx.execute("UPDATE contas_browser SET padrao_atual = 0 WHERE chave = ?1", [chave])?;
    }
    tx.commit()
}

/// Reads the shipped seed and merges it. Called once when the app starts.
pub fn merge_embedded(conn: &Connection) -> Result<(), String> {
    let seed = embedded()?;
    merge(conn, &seed).map_err(|e| format!("erro ao mesclar o seed: {e}"))
}
```

- [ ] **Step 5: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::seed`
Expected: PASS (10 testes). No PC: `cargo check --tests`.

- [ ] **Step 6: Prova de mutação**

Tire o `WHERE contas_browser.embutida = 1` da mesclagem e confirme que **`a_site_the_user_added_with_the_same_key_is_left_alone`** falha. Acrescente `ia_pode_usar=0` ao `DO UPDATE SET` e confirme que **`merge_updates_the_catalog_fields_but_never_the_users_choices`** falha. Restaure as duas.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db/schema.sql src-tauri/src/sites/seed.rs
git commit -m "feat(sites): contas_browser schema and idempotent seed merge"
```

---

### Task 4: Repositório (listar, permissão, login, adicionar, remover, resolver)

**Files:**
- Create: `src-tauri/src/sites/repo.rs`
- Modify: `src-tauri/src/sites/mod.rs` (`pub mod repo;`)

**Interfaces:**
- Consumes: `dominio::{url_matches, DomainRules, chave_from_url, host_and_path}`, tabela `contas_browser`
- Produces:
  - `#[derive(Serialize, Clone, Debug)] pub struct Site { chave, nome, url_login, dominios: Vec<String>, dominios_ats: Vec<String>, caminho_ats: Option<String>, tipo, regiao, pais: Option<String>, papel, regra_login, ia_pode_usar: bool, embutida: bool, padrao_atual: bool, estado, verificada_em: Option<String> }`
  - `pub fn listar(conn) -> Result<Vec<Site>, String>`
  - `pub fn obter(conn, chave) -> Result<Option<Site>, String>`
  - `pub fn definir_ia_pode_usar(conn, chave, valor: bool) -> Result<(), String>`
  - `pub fn marcar_logada(conn, chave) -> Result<(), String>`
  - `pub fn marcar_precisa_login(conn, chave) -> Result<bool, String>` (`true` quando o estado mudou)
  - `pub fn adicionar(conn, nome, url, tipo, pais: Option<&str>) -> Result<String, String>` (devolve a `chave`)
  - `pub fn remover(conn, chave) -> Result<(), String>`
  - `pub fn resolver(conn, url) -> Result<Option<Site>, String>`

- [ ] **Step 1: Escrever os testes que falham**

Em `src-tauri/src/sites/repo.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::Connection;

    fn db_with(json_sites: &str) -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        merge(&c, &parse_seed(&format!(r#"{{"versao":1,"sites":[{json_sites}]}}"#)).unwrap()).unwrap();
        c
    }
    const LI: &str = r#"{"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}"#;
    const PUB: &str = r#"{"chave":"pub","nome":"Pub","url_login":"https://pub.com/careers","dominios":["pub.com"],"tipo":"empresa","regiao":"EU","regra_login":"nenhuma"}"#;

    #[test]
    fn the_ai_permission_cannot_be_turned_on_for_a_site_that_needs_login() {
        let c = db_with(LI);
        let e = definir_ia_pode_usar(&c, "linkedin", true).unwrap_err();
        assert!(e.contains("login"), "{e}");
        assert!(!obter(&c, "linkedin").unwrap().unwrap().ia_pode_usar);
    }

    #[test]
    fn the_ai_permission_turns_on_once_the_user_confirms_the_login() {
        let c = db_with(LI);
        marcar_logada(&c, "linkedin").unwrap();
        definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        let s = obter(&c, "linkedin").unwrap().unwrap();
        assert!(s.ia_pode_usar && s.estado == "logada" && s.verificada_em.is_some());
    }

    #[test]
    fn a_site_without_login_rule_can_be_turned_on_right_away() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        assert!(obter(&c, "pub").unwrap().unwrap().ia_pode_usar);
    }

    #[test]
    fn turning_the_permission_off_always_works() {
        let c = db_with(PUB);
        definir_ia_pode_usar(&c, "pub", true).unwrap();
        definir_ia_pode_usar(&c, "pub", false).unwrap();
        assert!(!obter(&c, "pub").unwrap().unwrap().ia_pode_usar);
    }

    #[test]
    fn marking_login_needed_reports_whether_anything_changed() {
        let c = db_with(LI);
        marcar_logada(&c, "linkedin").unwrap();
        assert!(marcar_precisa_login(&c, "linkedin").unwrap());
        assert!(!marcar_precisa_login(&c, "linkedin").unwrap());
    }

    #[test]
    fn an_unknown_key_is_an_error_not_a_silent_success() {
        let c = db_with(LI);
        assert!(marcar_logada(&c, "nope").is_err());
        assert!(definir_ia_pode_usar(&c, "nope", false).is_err());
    }

    #[test]
    fn adding_a_site_validates_and_starts_closed_and_manual() {
        let c = db_with(LI);
        let k = adicionar(&c, "Jobindex", "https://www.jobindex.dk/", "geral", Some("DK")).unwrap();
        assert_eq!(k, "jobindex-dk");
        let s = obter(&c, &k).unwrap().unwrap();
        assert!(!s.embutida && !s.ia_pode_usar && s.regra_login == "manual" && s.estado == "desconhecido");
    }

    #[test]
    fn adding_rejects_bad_input_and_domains_that_are_already_covered() {
        let c = db_with(LI);
        for (nome, url, tipo, pais) in [
            ("", "https://a.com/", "geral", None),
            ("A", "http://a.com/", "geral", None),
            ("A", "javascript:alert(1)", "geral", None),
            ("A", "https://a.com/", "banana", None),
            ("A", "https://a.com/", "geral", Some("brazil")),
            ("Again", "https://www.linkedin.com/feed", "geral", None),
        ] {
            assert!(adicionar(&c, nome, url, tipo, pais).is_err(), "{nome} {url} {tipo}");
        }
    }

    #[test]
    fn only_sites_the_user_added_can_be_removed() {
        let c = db_with(LI);
        assert!(remover(&c, "linkedin").is_err());
        let k = adicionar(&c, "Mine", "https://mine.org/", "geral", None).unwrap();
        remover(&c, &k).unwrap();
        assert!(obter(&c, &k).unwrap().is_none());
    }

    #[test]
    fn resolving_picks_the_most_specific_match_and_ignores_lookalikes() {
        let c = db_with(&format!("{LI},{PUB}"));
        assert_eq!(resolver(&c, "https://www.linkedin.com/jobs/1").unwrap().unwrap().chave, "linkedin");
        assert!(resolver(&c, "https://linkedin.com.evil.com/").unwrap().is_none());
        assert!(resolver(&c, "https://linkedin.com@evil.com/").unwrap().is_none());
        assert!(resolver(&c, "https://unknown.org/").unwrap().is_none());
    }
}
```

- [ ] **Step 2: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib sites::repo`
Expected: FAIL (módulo sem implementação).

- [ ] **Step 3: Implementar**

Acima do bloco de testes:

```rust
//! Reads and writes of the stored site list. Every rule the UI shows is enforced here too: the
//! screen is never the only defence.
use super::dominio::{chave_from_url, host_and_path, host_matches, url_matches, DomainRules};
use super::modelo::{PAPEIS, REGIOES, TIPOS};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Site {
    pub chave: String,
    pub nome: String,
    pub url_login: String,
    pub dominios: Vec<String>,
    pub dominios_ats: Vec<String>,
    pub caminho_ats: Option<String>,
    pub tipo: String,
    pub regiao: String,
    pub pais: Option<String>,
    pub papel: String,
    pub regra_login: String,
    pub ia_pode_usar: bool,
    pub embutida: bool,
    pub padrao_atual: bool,
    pub estado: String,
    pub verificada_em: Option<String>,
}

const COLS: &str = "chave, nome, url_login, dominios, dominios_ats, caminho_ats, tipo, regiao, pais, papel, \
                    regra_login, ia_pode_usar, embutida, padrao_atual, estado, verificada_em";

fn json_list(s: String) -> Vec<String> {
    serde_json::from_str(&s).unwrap_or_default()
}

fn row(r: &Row) -> rusqlite::Result<Site> {
    Ok(Site {
        chave: r.get(0)?,
        nome: r.get(1)?,
        url_login: r.get(2)?,
        dominios: json_list(r.get(3)?),
        dominios_ats: json_list(r.get(4)?),
        caminho_ats: r.get(5)?,
        tipo: r.get(6)?,
        regiao: r.get(7)?,
        pais: r.get(8)?,
        papel: r.get(9)?,
        regra_login: r.get(10)?,
        ia_pode_usar: r.get::<_, i64>(11)? != 0,
        embutida: r.get::<_, i64>(12)? != 0,
        padrao_atual: r.get::<_, i64>(13)? != 0,
        estado: r.get(14)?,
        verificada_em: r.get(15)?,
    })
}

pub fn listar(conn: &Connection) -> Result<Vec<Site>, String> {
    let mut st = conn
        .prepare(&format!("SELECT {COLS} FROM contas_browser ORDER BY nome COLLATE NOCASE"))
        .map_err(|e| e.to_string())?;
    let rows = st.query_map([], row).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

pub fn obter(conn: &Connection, chave: &str) -> Result<Option<Site>, String> {
    let mut st = conn.prepare(&format!("SELECT {COLS} FROM contas_browser WHERE chave = ?1")).map_err(|e| e.to_string())?;
    let mut rows = st.query_map([chave], row).map_err(|e| e.to_string())?;
    rows.next().transpose().map_err(|e| e.to_string())
}

fn exigir(conn: &Connection, chave: &str) -> Result<Site, String> {
    obter(conn, chave)?.ok_or_else(|| format!("site '{chave}' não existe"))
}

/// Turning the permission ON needs a site that is logged in (or that asks for no login). Turning it
/// OFF always works.
pub fn definir_ia_pode_usar(conn: &Connection, chave: &str, valor: bool) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if valor && s.regra_login != "nenhuma" && s.estado != "logada" {
        return Err("Confirme o login neste site (botão \"Já estou logado\") antes de liberar a IA.".into());
    }
    conn.execute("UPDATE contas_browser SET ia_pode_usar = ?2 WHERE chave = ?1", params![chave, valor as i64])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn marcar_logada(conn: &Connection, chave: &str) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if s.regra_login == "nenhuma" {
        return Err("Este site não exige login.".into());
    }
    conn.execute("UPDATE contas_browser SET estado = 'logada', verificada_em = datetime('now') WHERE chave = ?1", [chave])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// `true` when the state actually changed (so the caller creates the notice only once).
pub fn marcar_precisa_login(conn: &Connection, chave: &str) -> Result<bool, String> {
    exigir(conn, chave)?;
    let n = conn
        .execute("UPDATE contas_browser SET estado = 'precisa_login', verificada_em = datetime('now') WHERE chave = ?1 AND estado <> 'precisa_login'", [chave])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

pub fn adicionar(conn: &Connection, nome: &str, url: &str, tipo: &str, pais: Option<&str>) -> Result<String, String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("Informe o nome do site.".into());
    }
    if !url.trim().to_ascii_lowercase().starts_with("https://") {
        return Err("O endereço precisa começar com https://".into());
    }
    let Some((host, _)) = host_and_path(url) else { return Err("Endereço inválido.".into()) };
    if !TIPOS.contains(&tipo) {
        return Err("Tipo desconhecido.".into());
    }
    if let Some(p) = pais {
        if p.len() != 2 || !p.chars().all(|c| c.is_ascii_uppercase()) {
            return Err("O país precisa ser o código de 2 letras (ex.: DK).".into());
        }
    }
    if let Some(dup) = resolver(conn, url)? {
        return Err(format!("Este endereço já está coberto pelo site \"{}\".", dup.nome));
    }
    let chave = chave_from_url(url).ok_or("Endereço inválido.")?;
    if obter(conn, &chave)?.is_some() {
        return Err("Já existe um site com este endereço.".into());
    }
    let dominio = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let dominios = serde_json::to_string(&vec![dominio]).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO contas_browser (chave, nome, url_login, dominios, tipo, regiao, pais, papel, regra_login, embutida) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'GLOBAL', ?6, 'candidatura', 'manual', 0)",
        params![chave, nome, url.trim(), dominios, tipo, pais],
    )
    .map_err(|e| e.to_string())?;
    debug_assert!(REGIOES.contains(&"GLOBAL") && PAPEIS.contains(&"candidatura"));
    Ok(chave)
}

pub fn remover(conn: &Connection, chave: &str) -> Result<(), String> {
    let s = exigir(conn, chave)?;
    if s.embutida {
        return Err("Sites que vêm com o app não podem ser removidos; desligue a IA neste site.".into());
    }
    conn.execute("DELETE FROM contas_browser WHERE chave = ?1 AND embutida = 0", [chave]).map_err(|e| e.to_string())?;
    Ok(())
}

/// The site `url` belongs to; with several matches the one with the longest matching domain wins.
pub fn resolver(conn: &Connection, url: &str) -> Result<Option<Site>, String> {
    let Some((host, _)) = host_and_path(url) else { return Ok(None) };
    let mut best: Option<(usize, Site)> = None;
    for s in listar(conn)? {
        let rules = DomainRules { dominios: &s.dominios, dominios_ats: &s.dominios_ats, caminho_ats: s.caminho_ats.as_deref() };
        if !url_matches(&rules, url) {
            continue;
        }
        let weight = s.dominios.iter().chain(&s.dominios_ats).filter(|d| host_matches(&host, d)).map(|d| d.len()).max().unwrap_or(0);
        if best.as_ref().is_none_or(|(w, _)| weight > *w) {
            best = Some((weight, s));
        }
    }
    Ok(best.map(|(_, s)| s))
}
```

- [ ] **Step 4: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::repo`
Expected: PASS (10 testes). No PC: `cargo check --tests` e `cargo clippy --all-targets` sem avisos novos.

- [ ] **Step 5: Prova de mutação**

Tire a condição `if valor && s.regra_login != "nenhuma" && s.estado != "logada"` (deixe só o `UPDATE`) e confirme que **`the_ai_permission_cannot_be_turned_on_for_a_site_that_needs_login`** falha. Tire o `if s.embutida` de `remover` e confirme que **`only_sites_the_user_added_can_be_removed`** falha. Restaure.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/sites/repo.rs src-tauri/src/sites/mod.rs
git commit -m "feat(sites): stored site list with permission, login and add/remove rules"
```

---

### Task 5: Regras do `browser_open` (gate) e domínios recusados

**Files:**
- Create: `src-tauri/src/sites/gate.rs`
- Modify: `src-tauri/src/sites/mod.rs` (`pub mod gate;`)

**Interfaces:**
- Consumes: `repo::{resolver, marcar_precisa_login}`, `dominio::host_and_path`
- Produces:
  - `pub enum Decision { Allow(String), Refuse(String) }` (`Debug`, `PartialEq`)
  - `pub fn decidir(conn: &Connection, url: &str) -> Decision`
  - `pub const MSG_FORA_DA_LISTA: &str`
  - `pub fn registrar_recusa(conn: &Connection, url: &str)`

- [ ] **Step 1: Escrever os testes que falham**

Em `src-tauri/src/sites/gate.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::repo;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        let seed = r#"{"versao":1,"sites":[
            {"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"},
            {"chave":"pub","nome":"Pub","url_login":"https://pub.com/careers","dominios":["pub.com"],"tipo":"empresa","regiao":"EU","regra_login":"nenhuma"}]}"#;
        merge(&c, &parse_seed(seed).unwrap()).unwrap();
        c
    }
    fn refused(d: Decision) -> String {
        match d { Decision::Refuse(m) => m, Decision::Allow(k) => panic!("allowed {k}") }
    }

    #[test]
    fn a_site_off_the_list_is_refused_and_counted() {
        let c = db();
        let m = refused(decidir(&c, "https://stranger.org/jobs/1?token=secret"));
        assert!(m.contains("register_vaga") && m.contains("pulada"), "{m}");
        decidir(&c, "https://stranger.org/other");
        let (n, url): (i64, String) = c.query_row("SELECT contagem, ultima_url FROM dominios_recusados WHERE dominio='stranger.org'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(n, 2);
        assert!(!url.contains("token") && !url.contains('?'), "the query string must not be stored: {url}");
    }

    #[test]
    fn a_listed_site_stays_closed_until_the_user_opens_it() {
        let c = db();
        assert!(refused(decidir(&c, "https://pub.com/careers")).contains("não liberou"));
    }

    #[test]
    fn a_site_that_needs_login_is_refused_until_it_is_logged_in() {
        let c = db();
        repo::marcar_logada(&c, "linkedin").unwrap();
        repo::definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        assert_eq!(decidir(&c, "https://www.linkedin.com/jobs/1"), Decision::Allow("linkedin".into()));
        repo::marcar_precisa_login(&c, "linkedin").unwrap();
        assert!(refused(decidir(&c, "https://www.linkedin.com/jobs/1")).contains("login"));
    }

    #[test]
    fn a_site_without_login_rule_only_needs_the_permission() {
        let c = db();
        repo::definir_ia_pode_usar(&c, "pub", true).unwrap();
        assert_eq!(decidir(&c, "https://pub.com/careers/1"), Decision::Allow("pub".into()));
    }

    #[test]
    fn tricky_urls_do_not_borrow_the_permission_of_a_listed_site() {
        let c = db();
        repo::marcar_logada(&c, "linkedin").unwrap();
        repo::definir_ia_pode_usar(&c, "linkedin", true).unwrap();
        for url in ["https://linkedin.com@evil.com/", "https://evil.com\\@linkedin.com", "https://linkedin.com.evil.com/"] {
            assert!(matches!(decidir(&c, url), Decision::Refuse(_)), "{url}");
        }
    }

    #[test]
    fn an_unreadable_database_refuses_and_never_panics() {
        let c = Connection::open_in_memory().unwrap(); // no tables at all
        assert!(matches!(decidir(&c, "https://www.linkedin.com/"), Decision::Refuse(m) if m.contains("não foi possível consultar")));
    }
}
```

- [ ] **Step 2: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib sites::gate`
Expected: FAIL (módulo vazio).

- [ ] **Step 3: Implementar**

Acima dos testes:

```rust
//! What the agent may open. Every failure to read the list is a refusal (fail closed).
use super::dominio::host_and_path;
use super::repo;
use rusqlite::{params, Connection};

#[derive(Debug, PartialEq)]
pub enum Decision {
    /// The agent may open the URL; carries the site key.
    Allow(String),
    /// The message is shown to the agent as the tool result.
    Refuse(String),
}

pub const MSG_FORA_DA_LISTA: &str = "RECUSADO: este site não está na lista de sites da Claudia, então você não pode abri-lo. \
Salve a vaga com register_vaga, marque com update_vaga_status como 'pulada' com o motivo \"site fora da lista: <domínio>\" \
e siga para a próxima. Não tente outro endereço do mesmo site. Chame browser_contas para ver onde você pode abrir páginas.";

/// Counts a refused domain. Stores only the host and the URL without its query string or fragment.
pub fn registrar_recusa(conn: &Connection, url: &str) {
    let Some((host, path)) = host_and_path(url) else { return };
    let limpa = format!("https://{host}{path}");
    let _ = conn.execute(
        "INSERT INTO dominios_recusados (dominio, contagem, ultima_vez, ultima_url) VALUES (?1, 1, datetime('now'), ?2) \
         ON CONFLICT(dominio) DO UPDATE SET contagem = contagem + 1, ultima_vez = datetime('now'), ultima_url = excluded.ultima_url",
        params![host, limpa],
    );
}

pub fn decidir(conn: &Connection, url: &str) -> Decision {
    let site = match repo::resolver(conn, url) {
        Ok(s) => s,
        Err(e) => return Decision::Refuse(format!("RECUSADO: não foi possível consultar a lista de sites ({e}). Tente de novo.")),
    };
    let Some(s) = site else {
        registrar_recusa(conn, url);
        return Decision::Refuse(MSG_FORA_DA_LISTA.to_string());
    };
    if !s.ia_pode_usar {
        return Decision::Refuse(format!("RECUSADO: o usuário não liberou o site \"{}\" para a IA. Pule esta fonte e siga para a próxima.", s.nome));
    }
    if s.regra_login != "nenhuma" && s.estado != "logada" {
        return Decision::Refuse(format!("RECUSADO: o site \"{}\" não está com login confirmado. Pule esta fonte; o usuário precisa entrar nele de novo.", s.nome));
    }
    Decision::Allow(s.chave)
}
```

- [ ] **Step 4: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::gate`
Expected: PASS (6 testes).

- [ ] **Step 5: Prova de mutação**

Em `registrar_recusa`, troque `limpa` por `url` no `params!` e confirme que **`a_site_off_the_list_is_refused_and_counted`** falha (a query string vazou). Troque `Decision::Refuse(...)` do ramo `Err(e)` por `Decision::Allow(String::new())` e confirme que **`an_unreadable_database_refuses_and_never_panics`** falha. Restaure.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/sites/gate.rs src-tauri/src/sites/mod.rs
git commit -m "feat(sites): browser_open gate rules and refused-domain counting"
```

---

### Task 6: Ligar o gate ao `browser_open` e o login_wall ao estado

**Files:**
- Create: `src-tauri/src/browser/sitegate.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod sitegate;`)
- Modify: `src-tauri/src/sites/gate.rs` (acrescentar `DbSiteGate`)
- Modify: `src-tauri/src/mcp/tools/browser.rs` (campo, `with_site_gate`, `open`, gancho de `login_wall`)
- Modify: `src-tauri/src/mcp/server.rs` (injetar o gate em `run`)

**Interfaces:**
- Consumes: `sites::gate::decidir`, `sites::repo::{resolver, marcar_precisa_login}`
- Produces:
  - `pub trait SiteGate: Send + Sync { fn check_open(&self, url: &str) -> Result<String, String>; fn login_wall(&self, url: &str); }`
  - `BrowserTools::with_site_gate(self, Arc<dyn SiteGate>) -> Self`
  - `pub struct DbSiteGate { pub data_dir: PathBuf }`

- [ ] **Step 1: Criar o trait**

`src-tauri/src/browser/sitegate.rs`:

```rust
//! What `browser_open` asks the site list. A trait so `browser/` and `mcp/tools/browser.rs` do not
//! depend on the database module, and tests can plug a fake.
pub trait SiteGate: Send + Sync {
    /// `Ok(site key)` when the agent may open `url`; `Err(message)` is returned to the agent.
    fn check_open(&self, url: &str) -> Result<String, String>;
    /// A read of the page at `url` ended in `estado: login_wall`.
    fn login_wall(&self, url: &str);
}
```

Em `src-tauri/src/browser/mod.rs` acrescente `pub mod sitegate;`.

- [ ] **Step 2: Escrever os testes que falham (gate real, sem o browser)**

Acrescente em `src-tauri/src/sites/gate.rs`, dentro do `mod tests`:

```rust
    #[test]
    fn the_db_gate_turns_a_login_wall_into_a_state_change_and_one_notice() {
        let dir = crate::mcp::tools::test_support::temp_dir("site-gate");
        crate::mcp::tools::test_support::seed_db(&dir);
        {
            let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
            crate::sites::seed::merge(&conn, &crate::sites::seed::parse_seed(
                r#"{"versao":1,"sites":[{"chave":"linkedin","nome":"LinkedIn","url_login":"https://www.linkedin.com/jobs/","dominios":["linkedin.com"],"tipo":"geral","regiao":"GLOBAL"}]}"#).unwrap()).unwrap();
            repo::marcar_logada(&conn, "linkedin").unwrap();
        }
        let gate = DbSiteGate { data_dir: dir.clone() };
        gate.login_wall("https://www.linkedin.com/authwall");
        gate.login_wall("https://www.linkedin.com/authwall");
        gate.login_wall("https://stranger.org/login"); // off the list: ignored
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        assert_eq!(repo::obter(&conn, "linkedin").unwrap().unwrap().estado, "precisa_login");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM avisos_sites WHERE chave='linkedin'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "the notice is created once, not at every read");
    }
```

`test_support::temp_dir(tag)` cria uma pasta única sob a pasta temporária do sistema e `seed_db(&dir)` cria o banco nela (verificado em `src-tauri/src/mcp/tools/mod.rs`); `seed_db` **não** devolve a conexão, por isso o teste a abre com `Connection::open(dir.join("claudia_rh.db"))`. O teste usa só essa pasta temporária, nunca um caminho real, e não apaga nada.

A tabela `avisos_sites` é usada por este teste, e o módulo `avisos` é criado na Task 8. Para esta task, `DbSiteGate::login_wall` grava o aviso com um `INSERT` direto (Step 4); a Task 8 troca por `avisos::criar`.

- [ ] **Step 3: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib sites::gate::tests::the_db_gate`
Expected: FAIL (`DbSiteGate` inexistente).

- [ ] **Step 4: Implementar `DbSiteGate`**

Em `src-tauri/src/sites/gate.rs`, acima dos testes:

```rust
use crate::browser::sitegate::SiteGate;
use std::path::PathBuf;
use std::time::Duration;

/// The gate backed by the main database file. Opens its own connection per call: the MCP server
/// runs in a separate process from the app.
pub struct DbSiteGate {
    pub data_dir: PathBuf,
}

impl DbSiteGate {
    fn conn(&self) -> Result<Connection, String> {
        let c = Connection::open(self.data_dir.join("claudia_rh.db")).map_err(|e| e.to_string())?;
        c.busy_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())?;
        Ok(c)
    }
}

impl SiteGate for DbSiteGate {
    fn check_open(&self, url: &str) -> Result<String, String> {
        let conn = self.conn().map_err(|e| format!("RECUSADO: não foi possível consultar a lista de sites ({e}). Tente de novo."))?;
        match decidir(&conn, url) {
            Decision::Allow(k) => Ok(k),
            Decision::Refuse(m) => Err(m),
        }
    }

    fn login_wall(&self, url: &str) {
        let Ok(conn) = self.conn() else { return };
        let Ok(Some(site)) = repo::resolver(&conn, url) else { return };
        if site.regra_login == "nenhuma" {
            return; // a public site that happens to show a sign-in box is not "logged out"
        }
        if repo::marcar_precisa_login(&conn, &site.chave).unwrap_or(false) {
            let _ = conn.execute("INSERT INTO avisos_sites (chave, criado_em) VALUES (?1, datetime('now'))", [&site.chave]);
        }
    }
}
```

- [ ] **Step 5: Ligar ao `BrowserTools`**

Em `src-tauri/src/mcp/tools/browser.rs`:

1. Na `struct BrowserTools` (linha ~483), acrescente o campo `site_gate: Option<Arc<dyn crate::browser::sitegate::SiteGate>>,` e, em `BrowserTools::new` (linha ~519), inicialize com `site_gate: None,`.
2. Ao lado de `with_documents`, acrescente:

```rust
    /// Plugs the site list into `browser_open`. Production (`mcp::server::run`) always sets it; tests
    /// that open local fixtures leave it unset on purpose.
    pub fn with_site_gate(mut self, gate: Arc<dyn crate::browser::sitegate::SiteGate>) -> Self {
        self.site_gate = Some(gate);
        self
    }

    /// Tells the site list when a page ended in a login wall.
    async fn note_state(&self, c: &CdpClient, state_line: &str) {
        if !state_line.trim_end().ends_with("login_wall") {
            return;
        }
        if let Some(g) = &self.site_gate {
            g.login_wall(&Self::page_url(c).await);
        }
    }
```

3. Em `open` (linha ~950), logo depois do bloco `if let Some(why) = url_refusal(...) { ... }`, acrescente:

```rust
        if let Some(g) = &self.site_gate {
            if let Err(why) = g.check_open(url) {
                return self.finish("browser_open", started, Err(why), None).await;
            }
        }
```

4. Ainda em `open`, no trecho que monta `message` com `state_line`, troque por:

```rust
            if let Ok(ax) = c.call("Accessibility.getFullAXTree", json!({})).await {
                let line = Self::state_line(&c, &ax).await;
                self.note_state(&c, &line).await;
                message.push('\n');
                message.push_str(&line);
            }
```

5. Rode `grep -n "state_line(" src-tauri/src/mcp/tools/browser.rs`. Para cada outro ponto em que `state_line` alimenta o resultado de `browser_read` ou `browser_text`, faça a mesma troca (`let line = ...; self.note_state(&c, &line).await;`). Registre no commit quais pontos foram cobertos.

- [ ] **Step 6: Injetar no servidor**

Em `src-tauri/src/mcp/server.rs`, em `run` (linha ~597), troque a criação por:

```rust
        let site_gate = Arc::new(crate::sites::gate::DbSiteGate { data_dir: cfg.data_dir.clone() });
        let browser = Arc::new(BrowserTools::new(state_file, guard, sink).with_documents(docs).with_site_gate(site_gate));
```

- [ ] **Step 7: Teste de integração do `open` com um gate falso**

Em `src-tauri/src/mcp/tools/browser.rs`, no `mod tests`, perto do teste `a_release_build_can_never_allow_local_hosts`, acrescente (usa o construtor de teste da linha ~1945, que já monta um `BrowserTools` sem browser):

```rust
    struct Refusing;
    impl crate::browser::sitegate::SiteGate for Refusing {
        fn check_open(&self, _url: &str) -> Result<String, String> { Err("RECUSADO: teste".into()) }
        fn login_wall(&self, _url: &str) {}
    }

    #[tokio::test]
    async fn browser_open_stops_at_the_site_gate_before_touching_the_browser() {
        let dir = crate::mcp::tools::test_support::temp_dir("site-gate-open");
        let t = BrowserTools::new(dir.join("s.json"), Arc::new(DbGuardContext::new(dir.clone())), Arc::new(NoSink))
            .with_site_gate(Arc::new(Refusing));
        let r = t.open("https://example.com/").await;
        assert_eq!(r.unwrap_err(), "RECUSADO: teste");
    }
```

- [ ] **Step 8: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::gate && cargo test --lib mcp::tools::browser::tests::browser_open_stops`
Expected: PASS. No PC: `cargo check --tests` e `cargo clippy --all-targets`.

- [ ] **Step 9: Prova de mutação**

Remova o bloco `if let Some(g) = &self.site_gate { ... check_open ... }` de `open` e confirme que **`browser_open_stops_at_the_site_gate_before_touching_the_browser`** falha (cole a linha `FAILED`). Em `DbSiteGate::login_wall`, remova o `if site.regra_login == "nenhuma" { return; }` e escreva um teste rápido que mostre um site `nenhuma` virando `precisa_login`; confirme que falha com a regra removida e passa com ela (deixe o teste no código: `a_public_site_never_becomes_login_needed`). Restaure.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/browser src-tauri/src/sites/gate.rs src-tauri/src/mcp
git commit -m "feat(mcp): browser_open only opens sites on the list; login walls update the site state"
```

---

### Task 7: A ferramenta `browser_contas`

**Files:**
- Create: `src-tauri/src/mcp/tools/sites.rs`
- Modify: `src-tauri/src/mcp/tools/mod.rs`, `src-tauri/src/mcp/mod.rs`, `src-tauri/src/mcp/server.rs`

**Interfaces:**
- Consumes: `sites::repo::listar`
- Produces: `pub fn browser_contas(data_dir: &Path) -> Result<String, String>`

- [ ] **Step 1: Escrever o teste que falha**

Em `src-tauri/src/mcp/tools/sites.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};
    use crate::sites::{repo, seed};

    #[test]
    fn it_lists_only_the_sites_the_user_opened_and_says_how_many_are_closed() {
        let dir = temp_dir("browser-contas");
        seed_db(&dir);
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        seed::merge(&conn, &seed::parse_seed(
            r#"{"versao":1,"sites":[
            {"chave":"a","nome":"Alpha","url_login":"https://a.com/","dominios":["a.com"],"tipo":"geral","regiao":"EU","regra_login":"nenhuma"},
            {"chave":"b","nome":"Beta","url_login":"https://b.com/","dominios":["b.com"],"tipo":"geral","regiao":"EU"}]}"#).unwrap()).unwrap();
        repo::definir_ia_pode_usar(&conn, "a", true).unwrap();
        let out = browser_contas(&dir).unwrap();
        assert!(out.contains("Alpha") && out.contains("https://a.com/"), "{out}");
        assert!(!out.contains("Beta"), "{out}");
        assert!(out.contains("1 site"), "it must say how many are closed: {out}");
    }

    #[test]
    fn it_says_so_when_nothing_is_open() {
        let dir = temp_dir("browser-contas-empty");
        seed_db(&dir);
        assert!(browser_contas(&dir).unwrap().contains("nenhum site liberado"));
    }
}
```

- [ ] **Step 2: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib mcp::tools::sites`
Expected: FAIL.

- [ ] **Step 3: Implementar**

Acima dos testes, em `sites.rs`:

```rust
//! `browser_contas`: tells the agent where it may open pages. Read-only.
use std::path::Path;

use super::open_db;
use crate::sites::repo;

pub fn browser_contas(data_dir: &Path) -> Result<String, String> {
    let conn = open_db(data_dir)?;
    let all = repo::listar(&conn)?;
    let open: Vec<_> = all.iter().filter(|s| s.ia_pode_usar).collect();
    let closed = all.len() - open.len();
    if open.is_empty() {
        return Ok(format!("Nenhum site liberado pelo usuário ({closed} na lista, todos fechados). Não abra páginas com browser_open; avise o usuário numa pendência se isso travar o trabalho."));
    }
    let mut out = format!("{} site(s) liberado(s) para você abrir com browser_open:\n", open.len());
    for s in open {
        let login = match (s.regra_login.as_str(), s.estado.as_str()) {
            ("nenhuma", _) => "sem login".to_string(),
            (_, "logada") => "login confirmado".to_string(),
            (_, e) => format!("login: {e}"),
        };
        let papel = if s.papel == "descoberta" { ", só descoberta (as vagas levam a outro site)" } else { "" };
        out.push_str(&format!("- {} [{} · {}] {login}{papel} → {}\n", s.nome, s.tipo, s.regiao, s.url_login));
    }
    out.push_str(&format!("Outros {closed} site(s) da lista estão fechados; qualquer endereço fora da lista é recusado."));
    Ok(out)
}
```

Em `src-tauri/src/mcp/tools/mod.rs`, acrescente `mod sites;` e `pub use sites::browser_contas;`.
Em `src-tauri/src/mcp/mod.rs`, ao lado da linha `"get_memory_summary" => ...` (~159), acrescente:

```rust
        "browser_contas" => tools::browser_contas(&cfg.data_dir),
```

Em `src-tauri/src/mcp/server.rs`, ao lado de `get_memory_summary`, acrescente:

```rust
    #[tool(
        description = "Lista os sites em que você pode abrir páginas com browser_open: nome, tipo, região, se há login confirmado e o endereço de entrada. Qualquer endereço fora desta lista é recusado. Chame no começo da rodada para planejar a procura."
    )]
    async fn browser_contas(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("browser_contas", serde_json::json!({}))
    }
```

- [ ] **Step 4: Atualizar o teste de registro**

Abra `src-tauri/src/mcp/mod.rs` perto da linha 342 (testes de `dispatch`) e `src-tauri/src/commands/prompts.rs:458`. Rode `cargo test --lib mcp::` no Sandbox: se algum teste enumera os nomes das tools, acrescente `browser_contas`. **Não** acrescente `browser_contas` ao teste do prompt agora (Task 12).

- [ ] **Step 5: Rodar e ver passar**

Run (Sandbox): `cargo test --lib mcp::`
Expected: PASS.

- [ ] **Step 6: Prova de mutação**

Troque `filter(|s| s.ia_pode_usar)` por `filter(|_| true)` e confirme que **`it_lists_only_the_sites_the_user_opened_and_says_how_many_are_closed`** falha. Restaure.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/mcp
git commit -m "feat(mcp): browser_contas lists where the agent may open pages"
```

---

### Task 8: Avisos de "relogar" (D1; só depois da aprovação do desvio)

> **Bloqueada** até o usuário aprovar a decisão D1 (aviso em tabela própria, e não em `pendencias`). Ao aprovar, registre a data na spec §4.4.

**Files:**
- Create: `src-tauri/src/sites/avisos.rs`
- Modify: `src-tauri/src/sites/mod.rs`, `src-tauri/src/sites/gate.rs`

**Interfaces:**
- Produces: `criar(conn, chave) -> Result<(), String>`, `contar(conn) -> Result<i64, String>`, `listar_abertos(conn) -> Result<Vec<Aviso>, String>`, `resolver_do_site(conn, chave) -> Result<(), String>`; `#[derive(Serialize)] pub struct Aviso { id: i64, chave: String, nome: String, criado_em: String }`

- [ ] **Step 1: Escrever os testes que falham**

Em `src-tauri/src/sites/avisos.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::seed::{merge, parse_seed};
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        merge(&c, &parse_seed(r#"{"versao":1,"sites":[{"chave":"a","nome":"Alpha","url_login":"https://a.com/","dominios":["a.com"],"tipo":"geral","regiao":"EU"}]}"#).unwrap()).unwrap();
        c
    }

    #[test]
    fn one_open_notice_per_site_even_if_created_twice() {
        let c = db();
        criar(&c, "a").unwrap();
        criar(&c, "a").unwrap();
        assert_eq!(contar(&c).unwrap(), 1);
        assert_eq!(listar_abertos(&c).unwrap()[0].nome, "Alpha");
    }

    #[test]
    fn confirming_the_login_resolves_the_notice() {
        let c = db();
        criar(&c, "a").unwrap();
        resolver_do_site(&c, "a").unwrap();
        assert_eq!(contar(&c).unwrap(), 0);
    }

    #[test]
    fn removing_the_site_removes_its_notices() {
        let c = db();
        c.execute("PRAGMA foreign_keys=ON", []).unwrap();
        c.execute("UPDATE contas_browser SET embutida = 0 WHERE chave='a'", []).unwrap();
        criar(&c, "a").unwrap();
        crate::sites::repo::remover(&c, "a").unwrap();
        assert_eq!(contar(&c).unwrap(), 0);
    }
}
```

- [ ] **Step 2: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib sites::avisos`
Expected: FAIL.

- [ ] **Step 3: Implementar**

```rust
//! "Log in again" notices, shown in the Sites screen and in the sidebar badge.
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Aviso {
    pub id: i64,
    pub chave: String,
    pub nome: String,
    pub criado_em: String,
}

/// Idempotent: a site has at most one open notice.
pub fn criar(conn: &Connection, chave: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO avisos_sites (chave, criado_em) SELECT ?1, datetime('now') \
         WHERE NOT EXISTS (SELECT 1 FROM avisos_sites WHERE chave = ?1 AND resolvido = 0)",
        [chave],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn contar(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM avisos_sites WHERE resolvido = 0", [], |r| r.get(0)).map_err(|e| e.to_string())
}

pub fn listar_abertos(conn: &Connection) -> Result<Vec<Aviso>, String> {
    let mut st = conn
        .prepare("SELECT a.id, a.chave, c.nome, a.criado_em FROM avisos_sites a JOIN contas_browser c ON c.chave = a.chave WHERE a.resolvido = 0 ORDER BY a.criado_em")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map([], |r| Ok(Aviso { id: r.get(0)?, chave: r.get(1)?, nome: r.get(2)?, criado_em: r.get(3)? }))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

pub fn resolver_do_site(conn: &Connection, chave: &str) -> Result<(), String> {
    conn.execute("UPDATE avisos_sites SET resolvido = 1, resolvido_em = datetime('now') WHERE chave = ?1 AND resolvido = 0", [chave])
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

Em `src-tauri/src/sites/mod.rs` acrescente `pub mod avisos;`.

- [ ] **Step 4: Usar `avisos` nos dois lugares certos**

1. Em `DbSiteGate::login_wall` (`gate.rs`), troque o `INSERT` direto por `let _ = super::avisos::criar(&conn, &site.chave);`.
2. Em `repo::marcar_logada`, depois do `UPDATE`, acrescente `let _ = super::avisos::resolver_do_site(conn, chave);`.
3. Em `repo::remover`, antes do `DELETE`, garanta o cascade: o `PRAGMA foreign_keys=ON` já é ligado em `db::apply`; o teste acima cobre.

- [ ] **Step 5: Rodar e ver passar**

Run (Sandbox): `cargo test --lib sites::`
Expected: PASS (todos os testes de `sites`).

- [ ] **Step 6: Prova de mutação**

Remova o `WHERE NOT EXISTS` de `criar` e confirme que **`one_open_notice_per_site_even_if_created_twice`** falha. Restaure.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/sites
git commit -m "feat(sites): log-in-again notices in their own table"
```

---

### Task 9: Comandos Tauri, mesclagem na abertura e "Abrir" o site

**Files:**
- Create: `src-tauri/src/commands/sites.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/browser/service.rs`

**Interfaces:**
- Consumes: `sites::{repo, avisos, seed}`, `DbState`
- Produces (comandos): `listar_sites() -> Vec<Site>`, `definir_ia_pode_usar(chave: String, valor: bool)`, `marcar_site_logado(chave: String)`, `adicionar_site(nome: String, url: String, tipo: String, pais: Option<String>) -> String`, `remover_site(chave: String)`, `listar_avisos_sites() -> Vec<Aviso>`, `contar_avisos_sites() -> i64`, `abrir_site(app, chave: String)`

- [ ] **Step 1: Ler como abrir uma aba no browser da Claudia (D3)**

Leia `src-tauri/src/browser/manager.rs` (`probe`, `wait_ready`, `BrowserState` e `read_state`) e `src-tauri/src/browser/service.rs` (`request_start`, `state_file`). Confirme duas coisas e anote no commit: (a) `BrowserState` guarda a porta de debug; (b) o helper HTTP que `probe` usa (raw `TcpStream` ou outro). Se (a) ou (b) não existirem, **pare e pergunte ao usuário**: não improvise.

- [ ] **Step 2: Escrever os testes que falham (comandos como funções puras)**

Em `src-tauri/src/commands/sites.rs`, escreva cada comando como uma função fina sobre `&Connection` (testável) mais o wrapper `#[tauri::command]`. Teste:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        crate::sites::seed::merge_embedded(&c).unwrap();
        c
    }

    #[test]
    fn the_embedded_seed_shows_up_in_the_list_closed() {
        let c = db();
        let all = do_listar(&c).unwrap();
        assert!(all.len() >= 9 && all.iter().all(|s| !s.ia_pode_usar));
    }

    #[test]
    fn confirming_the_login_clears_the_notice_and_unlocks_the_permission() {
        let c = db();
        crate::sites::repo::marcar_precisa_login(&c, "linkedin").unwrap();
        crate::sites::avisos::criar(&c, "linkedin").unwrap();
        assert!(do_definir_ia(&c, "linkedin", true).is_err());
        do_marcar_logado(&c, "linkedin").unwrap();
        assert_eq!(crate::sites::avisos::contar(&c).unwrap(), 0);
        do_definir_ia(&c, "linkedin", true).unwrap();
    }
}
```

- [ ] **Step 3: Rodar e ver falhar**

Run (Sandbox): `cargo test --lib commands::sites`
Expected: FAIL.

- [ ] **Step 4: Implementar os comandos**

```rust
//! Tauri commands of the Sites screen. Each command is a thin wrapper over a function on `&Connection`
//! so the rules stay testable without a window.
use crate::sites::avisos::{self, Aviso};
use crate::sites::repo::{self, Site};
use crate::DbState;
use rusqlite::Connection;
use tauri::{AppHandle, State};

pub(crate) fn do_listar(c: &Connection) -> Result<Vec<Site>, String> { repo::listar(c) }
pub(crate) fn do_definir_ia(c: &Connection, chave: &str, valor: bool) -> Result<(), String> { repo::definir_ia_pode_usar(c, chave, valor) }
pub(crate) fn do_marcar_logado(c: &Connection, chave: &str) -> Result<(), String> { repo::marcar_logada(c, chave) }

#[tauri::command]
pub fn listar_sites(state: State<'_, DbState>) -> Result<Vec<Site>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    do_listar(&conn)
}

#[tauri::command]
pub fn definir_ia_pode_usar(state: State<'_, DbState>, chave: String, valor: bool) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    do_definir_ia(&conn, &chave, valor)
}

#[tauri::command]
pub fn marcar_site_logado(state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    do_marcar_logado(&conn, &chave)
}

#[tauri::command]
pub fn adicionar_site(state: State<'_, DbState>, nome: String, url: String, tipo: String, pais: Option<String>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repo::adicionar(&conn, &nome, &url, &tipo, pais.as_deref())
}

#[tauri::command]
pub fn remover_site(state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    repo::remover(&conn, &chave)
}

#[tauri::command]
pub fn listar_avisos_sites(state: State<'_, DbState>) -> Result<Vec<Aviso>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    avisos::listar_abertos(&conn)
}

#[tauri::command]
pub fn contar_avisos_sites(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    avisos::contar(&conn)
}

/// "Open" button: starts the Claudia browser if needed and opens the site's entry page in a new tab,
/// so the user can log in by hand. The app never sees a password.
#[tauri::command]
pub async fn abrir_site(app: AppHandle, state: State<'_, DbState>, chave: String) -> Result<(), String> {
    let url = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        repo::obter(&conn, &chave)?.ok_or_else(|| format!("site '{chave}' não existe"))?.url_login
    };
    crate::browser::service::open_tab_when_ready(&app, &url).await
}
```

- [ ] **Step 5: Implementar `open_tab_when_ready` em `browser/service.rs`**

Com base no que o Step 1 confirmou, acrescente a `service.rs`:

```rust
/// Starts the browser if it is not running, waits for its debug port and opens `url` in a new tab
/// (`PUT /json/new?<url>`) and brings that tab to the front. Used by the Sites screen.
pub async fn open_tab_when_ready(app: &AppHandle, url: &str) -> Result<(), String> {
    request_start(app);
    let local = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let state_path = super::state_file(&local);
    for _ in 0..60 {
        if let Some(st) = super::manager::read_state(&state_path) {
            if super::manager::probe(st.port).await.is_some() {
                return super::manager::open_tab(st.port, url).await;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Err("o browser da Claudia não ficou pronto a tempo".into())
}
```

E em `manager.rs`, ao lado de `probe`, acrescente `open_tab(port, url)` **usando o mesmo helper HTTP que `probe` usa** (Step 1), enviando `PUT /json/new?<url com % codificado para espaço, #, ?>` para `127.0.0.1:{port}` e aceitando só resposta 200. Escreva um teste com um servidor TCP local falso (o mesmo estilo dos testes de `service.rs`, que já usam `tokio::io::AsyncReadExt/AsyncWriteExt`) que confirme que o pedido enviado começa por `PUT /json/new?https://` e que um 4xx vira `Err`. Se `BrowserState` não tiver o campo `port` com esse nome, use o nome real que o Step 1 encontrou.

- [ ] **Step 6: Registrar e mesclar na abertura**

Em `src-tauri/src/commands/mod.rs`: `pub mod sites;`. Em `src-tauri/src/lib.rs`, na lista do `invoke_handler`, depois de `browser::service::browser_abrir,` acrescente:

```rust
            commands::sites::listar_sites,
            commands::sites::definir_ia_pode_usar,
            commands::sites::marcar_site_logado,
            commands::sites::adicionar_site,
            commands::sites::remover_site,
            commands::sites::listar_avisos_sites,
            commands::sites::contar_avisos_sites,
            commands::sites::abrir_site,
```

E logo depois de `app.manage(DbState(Arc::clone(&conn_arc)));` (linha ~210):

```rust
            if let Ok(conn) = conn_arc.lock() {
                if let Err(e) = crate::sites::seed::merge_embedded(&conn) {
                    eprintln!("[sites] {e}");
                }
            }
```

- [ ] **Step 7: Rodar e ver passar**

Run (Sandbox): `cargo test --lib commands::sites` e `cargo test --lib browser::manager`
Expected: PASS. No PC: `cargo check` e `cargo clippy --all-targets`.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src
git commit -m "feat(sites): Tauri commands, seed merge at startup and opening a site in the Claudia browser"
```

---

### Task 10: Lógica pura do front (busca, filtros, ordenação)

**Files:**
- Create: `src/components/sites/logica.ts`
- Test: `tests/front/sites-logica.test.ts`

**Interfaces:**
- Produces:
  - `export interface Site { chave: string; nome: string; url_login: string; dominios: string[]; tipo: string; regiao: string; pais: string | null; papel: "candidatura" | "descoberta"; regra_login: string; ia_pode_usar: boolean; embutida: boolean; padrao_atual: boolean; estado: "logada" | "precisa_login" | "desconhecido"; verificada_em: string | null }`
  - `export interface Filtros { busca: string; tipo: string; regiao: string; pais: string }` (valor `""` = todos)
  - `export type Campo = "nome" | "tipo" | "regiao" | "pais" | "estado"`
  - `export interface Ordem { campo: Campo; desc: boolean }`
  - `export function filtrar(sites: Site[], f: Filtros): Site[]`
  - `export function ordenar(sites: Site[], o: Ordem): Site[]`
  - `export function valoresDe(sites: Site[], campo: "tipo" | "regiao" | "pais"): string[]`
  - `export function diasDesde(verificadaEm: string | null, agora: Date): number | null`

- [ ] **Step 1: Escrever os testes que falham**

`tests/front/sites-logica.test.ts`:

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { diasDesde, filtrar, ordenar, valoresDe, type Site } from "../../src/components/sites/logica.ts";

const base: Omit<Site, "chave" | "nome"> = {
  url_login: "https://x.com/", dominios: ["x.com"], tipo: "geral", regiao: "GLOBAL", pais: null,
  papel: "candidatura", regra_login: "manual", ia_pode_usar: false, embutida: true, padrao_atual: true,
  estado: "desconhecido", verificada_em: null,
};
const s = (chave: string, nome: string, extra: Partial<Site> = {}): Site => ({ ...base, chave, nome, ...extra });
const sites = [
  s("orsted", "Ørsted", { dominios: ["orsted.com"], tipo: "empresa", regiao: "EU", pais: "DK" }),
  s("zurich", "Zürich Jobs", { dominios: ["zurich.ch"], regiao: "EU", pais: "CH", estado: "logada" }),
  s("jobindex", "Jobindex", { dominios: ["jobindex.dk"], regiao: "EU", pais: "DK", estado: "precisa_login" }),
  s("vagas", "Vagas.com", { dominios: ["vagas.com.br"], regiao: "SA", pais: "BR" }),
  s("linkedin", "LinkedIn", { dominios: ["linkedin.com"] }),
];
const nenhum = { busca: "", tipo: "", regiao: "", pais: "" };

test("empty filters keep everything", () => assert.equal(filtrar(sites, nenhum).length, 5));

test("search ignores case and accents and also looks at the domain", () => {
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "zurich" }).map((x) => x.chave), ["zurich"]);
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "ØRSTED" }).map((x) => x.chave), ["orsted"]);
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "jobindex.dk" }).map((x) => x.chave), ["jobindex"]);
});

test("search, type, region and country combine", () => {
  assert.deepEqual(filtrar(sites, { busca: "", tipo: "", regiao: "EU", pais: "DK" }).map((x) => x.chave).sort(), ["jobindex", "orsted"]);
  assert.deepEqual(filtrar(sites, { busca: "o", tipo: "empresa", regiao: "EU", pais: "DK" }).map((x) => x.chave), ["orsted"]);
});

test("a filter that matches nothing gives an empty list, not an error", () => {
  assert.deepEqual(filtrar(sites, { ...nenhum, busca: "nada-disso" }), []);
  assert.deepEqual(filtrar([], nenhum), []);
});

test("sorting by name ignores accents and is stable on ties", () => {
  assert.deepEqual(ordenar(sites, { campo: "nome", desc: false }).map((x) => x.chave), ["jobindex", "linkedin", "orsted", "vagas", "zurich"]);
  assert.deepEqual(ordenar(sites, { campo: "nome", desc: true }).map((x) => x.chave), ["zurich", "vagas", "orsted", "linkedin", "jobindex"]);
  const iguais = [s("b", "Mesmo"), s("a", "Mesmo")];
  assert.deepEqual(ordenar(iguais, { campo: "nome", desc: false }).map((x) => x.chave), ["a", "b"]);
});

test("sorting by login state puts logged-in first, then unknown, then needs-login", () => {
  assert.deepEqual(ordenar(sites, { campo: "estado", desc: false }).map((x) => x.estado),
    ["logada", "desconhecido", "desconhecido", "desconhecido", "precisa_login"]);
});

test("sorting does not change the input", () => {
  const copia = [...sites];
  ordenar(sites, { campo: "pais", desc: false });
  assert.deepEqual(sites, copia);
});

test("the filter options are the distinct values present, sorted, without empties", () => {
  assert.deepEqual(valoresDe(sites, "regiao"), ["EU", "GLOBAL", "SA"]);
  assert.deepEqual(valoresDe(sites, "pais"), ["BR", "CH", "DK"]);
});

test("days since the confirmation read the SQLite UTC timestamp", () => {
  const agora = new Date("2026-10-14T12:00:00Z");
  assert.equal(diasDesde("2026-10-11 12:00:00", agora), 3);
  assert.equal(diasDesde("2026-10-14 11:00:00", agora), 0);
  assert.equal(diasDesde(null, agora), null);
  assert.equal(diasDesde("lixo", agora), null);
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `node --test tests/front/sites-logica.test.ts`
Expected: FAIL (módulo inexistente).

- [ ] **Step 3: Implementar**

`src/components/sites/logica.ts`:

```ts
// Pure search, filter and sort of the site list. No React, no Tauri: tested with node --test.

export interface Site {
  chave: string;
  nome: string;
  url_login: string;
  dominios: string[];
  tipo: string;
  regiao: string;
  pais: string | null;
  papel: "candidatura" | "descoberta";
  regra_login: string;
  ia_pode_usar: boolean;
  embutida: boolean;
  padrao_atual: boolean;
  estado: "logada" | "precisa_login" | "desconhecido";
  verificada_em: string | null;
}

export interface Filtros { busca: string; tipo: string; regiao: string; pais: string }
export type Campo = "nome" | "tipo" | "regiao" | "pais" | "estado";
export interface Ordem { campo: Campo; desc: boolean }

const sem = (t: string) => t.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

export function filtrar(sites: Site[], f: Filtros): Site[] {
  const q = sem(f.busca.trim());
  return sites.filter((s) => {
    if (f.tipo && s.tipo !== f.tipo) return false;
    if (f.regiao && s.regiao !== f.regiao) return false;
    if (f.pais && s.pais !== f.pais) return false;
    if (!q) return true;
    return sem(s.nome).includes(q) || s.dominios.some((d) => sem(d).includes(q));
  });
}

const RANK_ESTADO: Record<Site["estado"], number> = { logada: 0, desconhecido: 1, precisa_login: 2 };
const porNome = (a: Site, b: Site) => a.nome.localeCompare(b.nome, "pt", { sensitivity: "base" }) || a.chave.localeCompare(b.chave);

function chave(s: Site, campo: Campo): string | number {
  if (campo === "estado") return RANK_ESTADO[s.estado];
  if (campo === "nome") return s.nome;
  return s[campo] ?? "￿"; // sites without a country go last
}

export function ordenar(sites: Site[], o: Ordem): Site[] {
  const dir = o.desc ? -1 : 1;
  return [...sites].sort((a, b) => {
    const ka = chave(a, o.campo);
    const kb = chave(b, o.campo);
    const c = typeof ka === "number" && typeof kb === "number"
      ? ka - kb
      : String(ka).localeCompare(String(kb), "pt", { sensitivity: "base" });
    return c !== 0 ? c * dir : porNome(a, b);
  });
}

export function valoresDe(sites: Site[], campo: "tipo" | "regiao" | "pais"): string[] {
  return [...new Set(sites.map((s) => s[campo]).filter((v): v is string => !!v))].sort();
}

/** Whole days since a SQLite `datetime('now')` timestamp (UTC), or null when absent or unreadable. */
export function diasDesde(verificadaEm: string | null, agora: Date): number | null {
  if (!verificadaEm) return null;
  const t = Date.parse(verificadaEm.replace(" ", "T") + "Z");
  if (Number.isNaN(t)) return null;
  return Math.max(0, Math.floor((agora.getTime() - t) / 86_400_000));
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `node --test tests/front/sites-logica.test.ts && npx tsc --noEmit`
Expected: PASS (9 testes) e `tsc` sem erro.

- [ ] **Step 5: Prova de mutação**

Em `sem`, remova o `.normalize(...).replace(...)` e confirme que **o teste "search ignores case and accents..."** falha. Em `ordenar`, troque `c * dir` por `c` e confirme que **o teste de ordem decrescente** falha. Restaure.

- [ ] **Step 6: Commit**

```bash
git add src/components/sites/logica.ts tests/front/sites-logica.test.ts
git commit -m "feat(sites): pure search, filter and sort for the site list"
```

---

### Task 11: A tela Sites (galeria, lista, barra, adicionar), sidebar e i18n

**Files:**
- Create: `src/lib/sites-api.ts`, `src/components/sites/index.tsx`, `Barra.tsx`, `Galeria.tsx`, `Lista.tsx`, `FormAdicionar.tsx`, `Avatar.tsx`
- Modify: `src/components/Sidebar.tsx`, `src/App.tsx`, `src/i18n/pt.ts`, `src/i18n/en.ts`

**Interfaces:**
- Consumes: os comandos da Task 9 e `logica.ts` da Task 10.
- Produces: `export function Sites()`, e `View` ganha `"sites"`.

- [ ] **Step 1: A porta do front para os comandos**

`src/lib/sites-api.ts`:

```ts
// The only place the front talks to the Sites commands.
import { invoke } from "@tauri-apps/api/core";
import type { Site } from "../components/sites/logica";

export interface AvisoSite { id: number; chave: string; nome: string; criado_em: string }

export const sitesApi = {
  listar: () => invoke<Site[]>("listar_sites"),
  definirIa: (chave: string, valor: boolean) => invoke<void>("definir_ia_pode_usar", { chave, valor }),
  marcarLogado: (chave: string) => invoke<void>("marcar_site_logado", { chave }),
  adicionar: (nome: string, url: string, tipo: string, pais: string | null) => invoke<string>("adicionar_site", { nome, url, tipo, pais }),
  remover: (chave: string) => invoke<void>("remover_site", { chave }),
  abrir: (chave: string) => invoke<void>("abrir_site", { chave }),
  avisos: () => invoke<AvisoSite[]>("listar_avisos_sites"),
  contarAvisos: () => invoke<number>("contar_avisos_sites"),
};
```

- [ ] **Step 2: i18n**

Leia a forma de `t.documents` em `src/i18n/pt.ts` (linha ~484) e de `t.nav` (linha ~4). Em `pt.ts` acrescente `sites` em `nav` e um bloco `sites` (e o equivalente em `en.ts`, mesmo conjunto de chaves, textos em inglês):

```ts
  // dentro de nav:
  sites: "Sites",

  // bloco novo, irmão de documents:
  sites: {
    title: "Sites de vagas",
    searchPlaceholder: "Buscar por nome ou domínio",
    type: "Tipo", region: "Região", country: "País", all: "Todos",
    sortBy: "Ordenar por", sortName: "Nome (A–Z)", sortType: "Tipo", sortRegion: "Região", sortCountry: "País", sortState: "Estado de login",
    gallery: "Galeria", list: "Lista",
    add: "Adicionar site", counter: (n: number, total: number) => `${n} de ${total} sites`,
    clearFilters: "Limpar filtros", noMatch: "Nenhum site combina com esses filtros.",
    loading: "Carregando…", error: "Não foi possível carregar os sites", retry: "Tentar de novo",
    open: "Abrir", loggedIn: "Já estou logado", remove: "Remover", removeConfirm: "Remover este site da lista?",
    aiCanUse: "IA pode usar",
    state: { logada: "Logada", precisa_login: "Precisa de login", desconhecido: "Sem confirmação" },
    confirmedDaysAgo: (d: number) => (d === 0 ? "confirmada hoje" : `confirmada há ${d} ${d === 1 ? "dia" : "dias"}`),
    discovery: "descoberta",
    types: { geral: "Geral", regional: "Regional", tecnologia: "Tecnologia", remoto: "Remoto", freelance: "Freelance", empresa: "Empresa", nicho: "Nicho" },
    regions: { SA: "América do Sul", NA: "América do Norte", EU: "Europa", AS: "Ásia", AF: "África", OC: "Oceania", GLOBAL: "Global" },
    form: { name: "Nome do site", url: "Endereço (https://…)", type: "Tipo", country: "País (opcional, ex.: DK)", save: "Salvar", cancel: "Cancelar" },
    relogin: (nome: string) => `Entre de novo em ${nome}`,
  },
```

- [ ] **Step 3: Componentes**

Crie os componentes em `src/components/sites/`, seguindo `docs/padrao-design-claudia-rh.md` §13 (a aba Currículos é o modelo: tokens `--bg-surface`, `--border`, `--accent`, raio 8px, grade `repeat(auto-fill, minmax(190px, 1fr))`):

- `Avatar.tsx`: círculo de 32px em `--bg-sunken` com a **letra inicial** do nome (sem favicon, sem rede).
- `Barra.tsx`: campo de busca, `select` de Tipo/Região/País (valores de `valoresDe`), `select` de ordenação, alternador Galeria/Lista (botões com `aria-pressed`), botão "Adicionar site" (fundo `--accent`, texto branco), contador. Props: `filtros`, `onFiltros`, `ordem`, `onOrdem`, `modo`, `onModo`, `sites`, `visiveis`, `onAdicionar`.
- `Galeria.tsx` e `Lista.tsx`: recebem `sites: Site[]`, `agora: Date` e as ações `onAbrir`, `onLogado`, `onIa`, `onRemover`. Cada item mostra avatar, nome, domínio (`dominios[0]`), pills de tipo e país (11px, `--bg-sunken`), pill "descoberta" quando `papel === "descoberta"`, badge de estado (`logada` com `--success`, `desconhecido` com `--warning`, `precisa_login` em **cinza** `--text-tertiary`), `confirmadaHa` quando `diasDesde` não é nulo, interruptor "IA pode usar", botão **Abrir**, botão "Já estou logado" só quando `regra_login === "manual"` e `estado !== "logada"`, e **Remover** só quando `!embutida`. Na lista, o cabeçalho das colunas chama `onOrdem` (clique ordena, segundo clique inverte). Site com `padrao_atual === false` leva a legenda "não é mais padrão".
- `FormAdicionar.tsx`: painel acima da grade com nome, URL, tipo e país opcional; chama `onSalvar` e mostra o erro devolvido pelo backend (a validação é do backend, a tela só repassa a mensagem).
- `index.tsx`: carrega com `sitesApi.listar()`, guarda `filtros`, `ordem` e `modo` (o `modo` em `localStorage` com `try/catch`, chave `sites-modo`), usa `filtrar`/`ordenar`, escuta o evento `db-atualizada` (como `ouvirDocumentos` em `src/components/documentos/ouvir.ts`) para recarregar, mostra os estados carregando/erro (borda `--danger` + "Tentar de novo")/vazio com "Limpar filtros", e uma faixa no topo com `sitesApi.avisos()` ("Entre de novo em X") quando houver aviso.

Cada componente tem **uma** responsabilidade e não passa de ~150 linhas.

- [ ] **Step 4: Sidebar e App**

Em `src/components/Sidebar.tsx`: acrescente `"sites"` ao tipo `View`, importe `Globe` de `lucide-react` e acrescente, logo depois de `documentos`, `{ id: "sites", label: t.nav.sites, Icon: Globe }`. Para o badge de avisos, acrescente a prop `sitesCount?: number` e mostre-o com o mesmo estilo do badge de `pendenciasCount` (leia o JSX de `pendencias` mais abaixo no arquivo e copie o padrão).

Em `src/App.tsx`: `const [sitesCount, setSitesCount] = useState(0);`, uma `refreshSitesCount` com `invoke<number>("contar_avisos_sites")`, chamada nos mesmos pontos de `refreshPendenciasCount` e no `listen("db-atualizada", ...)`. Passe `sitesCount` ao `Sidebar` e monte, depois do bloco de Currículos:

```tsx
              {/* Sites */}
              <div style={{ display: view === "sites" ? "flex" : "none", flexDirection: "column", height: "100%", overflow: "hidden" }}>
                <Sites />
              </div>
```

com `import { Sites } from "./components/sites";`.

- [ ] **Step 5: Verificar**

Run: `npx tsc --noEmit && npm run build`
Expected: sem erro de tipo e o build do Vite termina com código 0. Confira o código de saída, sem filtrar a saída.

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "feat(sites): Sites screen with gallery and list views, search, filters and sort"
```

---

### Task 12: Prompt de runtime (texto aprovado antes de aplicar)

> O usuário autorizou editar o prompt para citar `browser_contas` e a regra da lista. **O texto exato abaixo precisa ser revisado por ele antes do Step 2.**

**Files:**
- Modify: `src-tauri/src/prompt_sistema_runtime.md` (seção "O browser")
- Modify: `src-tauri/src/commands/prompts.rs` (teste na linha ~458)

- [ ] **Step 1: Mostrar o texto ao usuário e esperar o OK**

Texto proposto, a entrar como **novo item** da lista da seção "O browser", logo depois do item de `browser_open`:

```markdown
- Você só pode abrir sites que estão na lista da Claudia e que o usuário liberou. Chame `browser_contas` no começo da rodada para ver quais são. Se `browser_open` recusar um endereço (site fora da lista, não liberado ou sem login confirmado), **não tente outro caminho para o mesmo site**: se for uma vaga que você já achou, salve com `register_vaga`, marque com `update_vaga_status` como `pulada` com o motivo "site fora da lista: <domínio>" e siga para a próxima fonte.
```

E, no primeiro parágrafo da seção, acrescentar `browser_contas` à lista de ferramentas citadas. **Pare aqui até o usuário responder.**

- [ ] **Step 2: Escrever o teste que falha**

Em `prompts.rs`, no teste `the_shipped_runtime_prompt_no_longer_sends_the_agent_to_the_extension`, acrescente `"browser_contas"` ao vetor de ferramentas da linha ~458. Run (Sandbox): `cargo test --lib commands::prompts`. Expected: FAIL ("the prompt must teach browser_contas").

- [ ] **Step 3: Aplicar a edição aprovada**

Edite `prompt_sistema_runtime.md` com **exatamente** o texto aprovado no Step 1.

- [ ] **Step 4: Rodar e ver passar**

Run (Sandbox): `cargo test --lib commands::prompts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/prompt_sistema_runtime.md src-tauri/src/commands/prompts.rs
git commit -m "feat(prompt): teach the agent the site list and browser_contas"
```

---

### Task 13: Documentação, smoke de uso real e revisão

**Files:**
- Modify: `docs/padrao-design-claudia-rh.md` (§3 navegação + nova §15 "Sites"), `docs/arquitetura-sistema-candidaturas.md`, `CLAUDE.md` (a lista de ferramentas do MCP e a linha "O que este repo já é")
- Modify: `docs/superpowers/specs/2026-10-14-lista-de-sites-design.md` (registrar D1 aprovada, D2 e a tabela de aviso)

- [ ] **Step 1: Documentar**

Em `docs/padrao-design-claudia-rh.md`: acrescente o item **Sites** (ícone `Globe`) depois de Currículos na §3 (renumerando) e uma §15 com a tela (barra de ferramentas, galeria `minmax(190px, 1fr)`, lista com linha de 48px, cores de estado com `precisa_login` em cinza, formulário "+", estados vazio/erro). Em `docs/arquitetura-sistema-candidaturas.md`: descreva a rigidez do `browser_open`, a tabela `contas_browser`, `dominios_recusados`, `avisos_sites` e a regra "o seed nunca sobrescreve escolhas do usuário". Em `CLAUDE.md`: acrescente `browser_contas` à lista de ferramentas e uma linha sobre `src-tauri/src/sites/`.

- [ ] **Step 2: Rodadas de teste (no Sandbox para o que apaga ou roda `cargo test`)**

Run (Sandbox, skill `claudia-teste-sandbox`): `cargo test --lib` completo. Expected: PASS; confira o código de saída.
Run (PC): `cargo clippy --all-targets`, `npx tsc --noEmit`, `npm run build` e `node --test`. Expected: todos com código 0.

- [ ] **Step 3: Smoke de uso real (skill `claudia-teste-e2e`, páginas locais, sem site real)**

Abra o app, vá à tela **Sites** e confira, nos dois idiomas, com prints: galeria e lista; busca por "jobindex" e por um nome com acento; filtros Tipo, Região e País combinados; as 5 ordenações; adicionar um site (https), ver o erro de um `http://` e de um domínio repetido; remover o site adicionado e confirmar que um embutido não tem "Remover"; ligar "IA pode usar" num site `manual` sem login e ver a recusa; "Já estou logado" e depois ligar; **Abrir** (a aba abre no browser da Claudia). Com uma sessão de agente contra a página local: `browser_contas` lista só o liberado; `browser_open` de um endereço fora da lista é recusado com a mensagem de salvar a vaga; um `login_wall` na página local de teste vira o aviso na tela e o badge da sidebar. Registre o que foi visto e o que não foi.

- [ ] **Step 4: Revisão de código por Opus**

Peça a revisão (skill `requesting-code-review`, modelo Opus, obrigatória por ser ferramenta do agente e segurança). Escopo: `sites/dominio.rs`, `sites/gate.rs`, `sites/repo.rs`, o `with_site_gate` e o gancho de `login_wall` em `browser.rs`, `browser_contas`. Corrija ou registre cada achado.

- [ ] **Step 5: Commit**

```bash
git add docs CLAUDE.md
git commit -m "docs(sites): navigation, architecture and design for the site list"
```

---

### Task 14: Curadoria do seed completo (depende do usuário)

> Esta task **não é executável por agente sozinho**: precisa da lista final e da decisão sobre o risco dos termos de uso.

**Files:**
- Modify: `src-tauri/sites-padrao.json`

- [ ] **Step 1: Receber do usuário** a lista final (~100) a partir de `docs/pesquisa/2026-10-14-lista-de-sites-pesquisa.md` e a decisão sobre os sites que **proíbem automação por escrito** (entram como `descoberta`, ficam fora, ou entram assim mesmo).
- [ ] **Step 2: Gerar as entradas** no formato da Task 2, uma por site, com `dominios`, `dominios_ats` e `caminho_ats` para as empresas (ATS compartilhado exige o caminho da empresa), `regra_login: "nenhuma"` para páginas de carreiras públicas e `papel: "descoberta"` para agregadores.
- [ ] **Step 3: Rodar** `cargo test --lib sites::seed` (Sandbox). Expected: PASS (o teste `the_embedded_seed_is_valid` valida chaves únicas, tipos, https e a consistência do `url_login`).
- [ ] **Step 4: Commit**

```bash
git add src-tauri/sites-padrao.json
git commit -m "feat(sites): full default site list"
```

---

## Self-Review

**Cobertura da spec:**
- §4.1 modelo → Task 3 (schema), Task 4 (`Site`). `regiao`, `papel`, `nicho` → Tasks 2 e 3. P12 (domínios de carreiras, ATS e caminho) → Tasks 1, 2, 3.
- §4.2 seed e mesclagem → Tasks 2 e 3. Validação do seed → Task 2.
- §4.3 telas (galeria, lista, busca, filtros, ordenação, "+", estados) → Tasks 10 e 11. Sidebar "Sites" (P1), país (P2), cinza (P3), contador e limpar filtros (P8) → Task 11.
- §4.4 login (`manual`, "Já estou logado", sem expiração, `login_wall` → `precisa_login`, aviso) → Tasks 4, 6, 8. **Aviso diverge da spec (D1) e está marcado como bloqueado.** Detecção por cookie (LinkedIn) está **fora** (spec §7).
- §4.5 rigidez → Tasks 5, 6, 7. Contagem de domínios recusados → Task 5. Navegação por clique só registrada no Activity log: **não coberta por tarefa**; é o limite conhecido da spec e já vai no log existente pelo `finish`, sem mudança.
- §4.6 segurança (fronteira de rótulo, só https, mutação) → Tasks 1, 2, 5 e as provas de mutação.
- §6 testes (unitários, persistência, smoke) → cada task + Task 13. Persistência entre reinícios: o SQLite em disco é o mesmo caminho do restante do app; o smoke da Task 13 cobre abrir, fechar e reabrir o app.
- Prompt de runtime → Task 12.

**Placeholders:** os passos de código trazem o código. Os trechos que dependem de ler o código antes (Task 6 Step 5.5, Task 9 Steps 1 e 5) dizem **o que ler, o que decidir e quando parar**; o `PUT /json/new` da Task 9 Step 5 está **não verificado** até o Step 1.

**Consistência de tipos:** `Site` (Rust) e `Site` (TS) têm os mesmos campos; `SiteGate::check_open -> Result<String,String>` é usado igual em Task 5, 6 e 7; `decidir` devolve `Decision`; `marcar_precisa_login` devolve `bool` (Task 4) e é usado assim em `DbSiteGate::login_wall` (Task 6).

**Review Focus:** os cinco itens estão cobertos: (1) URL enganosa → Task 1 e 5; (2) colisão do seed → Task 3; (3) permissão sem login → Task 4; (4) banco ilegível → Task 5; (5) acento e vazio → Task 10.

## Pendências que o plano não resolve

- **Risco dos termos de uso** (quase todos os sites proíbem automação): decisão do usuário, necessária para a Task 14.
- **Notícias de peso** sobre as empresas, URLs de login não verificadas e África e Oriente Médio: ficam na lista de lacunas da pesquisa.
- **D5:** ligar `DbGuardContext::site_allowed` à lista fica de fora; mudaria a guarda de envio.
