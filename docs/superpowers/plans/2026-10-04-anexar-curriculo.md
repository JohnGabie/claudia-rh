# Anexar currículo — Plano de implementação (Parte 1: upload; Parte 2: o resto)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** **`browser_upload`**: o agente consegue anexar o currículo (e a cover letter) certo num formulário de candidatura, só de documentos da biblioteca do usuário, sem nunca derrubar o Chrome. Depois: PDF dos currículos gerados, a aba Currículos completa e o MCP de documentos e variantes.

**Architecture:** o foco é a tool `browser_upload`. Um módulo Rust novo `documentos` dá a ela o que anexar (tabelas `documentos` e `preferidos`, importação segura, regra de escolha, resolução segura do arquivo); na Parte 1 o arquivo anexável é o currículo que o usuário importa. O PDF dos currículos gerados (pelo `PrintToPdf` do WebView2 do app) é da Parte 2. A tool `browser_upload` vive em `mcp/tools/browser.rs`, recebe uma `ref` e no máximo um `id` de documento (nunca um caminho), e anexa por CDP (`DOM.setFileInputFiles`, com `Page.setInterceptFileChooserDialog` para o botão que abre o seletor).

**Tech Stack:** Rust (Tauri 2.11, rusqlite, tokio, sha2, zip, CDP próprio em `browser/cdp.rs`), `webview2-com` 0.38 + `windows-core` 0.61 (já no grafo via `wry`), React/TS no front.

**Spec:** `docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md` (aprovada pelo usuário em 2026-10-04).

## Como este plano é executado (pedido do usuário)

Duas partes, cada uma com **execução e teste**:

- **Parte 1 — `browser_upload`** (Tarefas 1 a 8): o mínimo para o agente anexar um currículo de verdade: a biblioteca, a importação do currículo do usuário, a regra de escolha e a tool. Sem geração de PDF: o anexo da Parte 1 é o PDF/DOCX importado. Termina num **portão**: rodadas de teste completas + aprovação do usuário.
- **Parte 2 — O resto** (Tarefas 9 a 16): PDF dos currículos gerados (WebView2), aba Currículos com grade e página A4, tags, preferido por variante, MCP de documentos/variantes/cover letter, migração. **Só começa depois do portão da Parte 1.** Detalhada na revisão de 2026-10-04 (depois do portão): execução em ondas de agentes paralelos, contrato entre frentes, PROPOSTAS NÃO DECIDIDAS (P1–P7) que o usuário decide antes de cada onda. Ver a seção "PARTE 2".

## Global Constraints

- **Nenhum passo de instalação novo** (CLAUDE.md): nada de Python, Node, binário externo; só crates Rust.
- **O agente nunca passa nem recebe caminho de arquivo**: só `ref` e `id` de documento.
- **O Chrome nunca falha por causa de um upload** (regra dura do usuário): toda falha é erro da tool; a aba continua usável.
- **Só PDF e DOCX, até 10 MB**, tipo real conferido (`%PDF-` ou ZIP `PK\x03\x04`).
- Arquivos anexáveis só dentro de `<dados>/documentos/` (`<dados>` = `app_data_dir`, onde fica `claudia_rh.db`).
- Upload só com **exatamente uma** vaga em `candidatando` (mesma regra da guarda; o acúmulo de vagas é o bug CLA-35, fora deste plano).
- Currículo nunca vai para log ou diagnóstico: só nome, formato, tamanho e `sha256`.
- `prompt_sistema_runtime.md` só é editado com **autorização explícita** do usuário (Tarefa 7 fica bloqueada até ela).
- Comentários e commits em inglês; UI e docs em pt-BR; i18n pt e en.
- Testes: `CARGO_TARGET_DIR` fora do `target/` do repo; exit code real; **mutação** para cada regra nova (colar a linha `FAILED`).
- Nunca testar em site real sem pedido; nunca enviar candidatura.

## Review Focus (Parte 1; a da Parte 2 está no fim do plano)

1. **Arquivo do documento apagado ou editado fora do app** depois de registrado → o upload recusa com mensagem clara ("o arquivo mudou/sumiu, gere ou importe de novo"), nunca anexa um arquivo diferente. (Tarefa 2, testes `missing_file_is_refused`, `edited_file_is_refused`.)
2. **Usuário importa o mesmo PDF duas vezes ou um arquivo com nome estranho** (`..\x.pdf`, acentos, espaços, 200 caracteres) → vira um nome seguro dentro da pasta, sem sobrescrever outro. (Tarefa 3, `import_sanitizes_names_and_never_overwrites`.)
3. **Site que aceita só PDF e o preferido é DOCX** → a página recusa ou o input ignora; a tool tem de dizer que a página não mostrou o arquivo, não "anexou". (Tarefa 5, `upload_reports_when_the_page_did_not_take_the_file`.)
4. **Botão de anexo que não abre seletor** (é outro botão, ou a página trava) → timeout curto, interceptação desligada, aba usável. (Tarefa 5, `a_chooser_that_never_opens_times_out_and_the_tab_still_works`.)
5. **Nenhum documento disponível** (biblioteca vazia, ou sem preferido e sem variante) → recusa explicando o que falta ("marque um currículo como preferido"), sem anexar nada. (Tarefa 4, `nothing_to_pick_returns_none`; Tarefa 5, `upload_with_an_empty_library_is_refused_with_a_hint`.)

---

# PARTE 1 — `browser_upload`

## Estrutura de arquivos da Parte 1

| Arquivo | Responsabilidade |
|---|---|
| `src-tauri/src/documentos/mod.rs` (novo) | modelo `Documento`, pasta, CRUD no banco |
| `src-tauri/src/documentos/seguranca.rs` (novo) | `resolve_attachable`: caminho dentro da pasta, existe, `sha256` confere |
| `src-tauri/src/documentos/importar.rs` (novo) | importar arquivo do usuário com validação |
| `src-tauri/src/documentos/escolha.rs` (novo) | regra de escolha (preferido da variante → geral → gerado da variante) |
| `src-tauri/src/documentos/fonte.rs` (novo) | `DbDocumentSource`: o que `browser_upload` consulta |
| `src-tauri/src/db/schema.sql` | tabelas `documentos` e `preferidos` |
| `src-tauri/src/commands/documentos.rs` (novo) | comandos Tauri da Parte 1 |
| `src-tauri/src/mcp/tools/browser.rs` | `browser_upload` + trait `DocumentSource` |
| `src-tauri/src/mcp/server.rs` | registra `browser_upload` |
| `src/components/perfil/CurriculosView.tsx` | bloco mínimo "Para anexar": importar e marcar preferido |
| `src-tauri/tests/fixtures/upload-form.html` (novo) | página de teste com os três tipos de campo |
| `src-tauri/tests/browser_live.rs` | testes live do upload |

---

### Tarefa 1: Biblioteca no banco (`documentos`, `preferidos`)

**Files:**
- Modify: `src-tauri/src/db/schema.sql`
- Create: `src-tauri/src/documentos/mod.rs`
- Modify: `src-tauri/src/lib.rs` (`mod documentos;`)

**Interfaces:**
- Produces:
  - `pub fn docs_dir(data_dir: &Path) -> PathBuf` → `data_dir.join("documentos")`
  - `pub struct Documento { pub id: i64, pub tipo: String, pub nome: String, pub origem: String, pub arquivo: String, pub arquivo_html: Option<String>, pub formato: String, pub idioma: Option<String>, pub tags: String, pub sha256: String, pub tamanho: i64, pub vaga_id: Option<i64>, pub variante_id: Option<String>, pub criado_em: String }` (`serde::Serialize`)
  - `pub struct NovoDocumento<'a> { pub tipo: &'a str, pub nome: &'a str, pub origem: &'a str, pub arquivo: &'a str, pub arquivo_html: Option<&'a str>, pub formato: &'a str, pub idioma: Option<&'a str>, pub sha256: &'a str, pub tamanho: i64, pub variante_id: Option<&'a str> }`
  - `pub fn inserir(conn: &Connection, d: &NovoDocumento) -> rusqlite::Result<i64>`
  - `pub fn obter(conn: &Connection, id: i64) -> rusqlite::Result<Option<Documento>>`
  - `pub fn listar(conn: &Connection, tipo: &str) -> rusqlite::Result<Vec<Documento>>` (mais novo primeiro)
  - `pub fn definir_preferido(conn: &Connection, id: i64, variante_id: Option<&str>) -> Result<(), String>` (o tipo vem do próprio documento)
  - `pub fn preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<i64>`

- [ ] **Step 1: Schema.** Acrescentar ao fim de `schema.sql`:

```sql
-- Documents the agent may attach (CVs, cover letters). `arquivo` is a bare file name inside
-- <app data>/documentos/: never a path.
CREATE TABLE IF NOT EXISTS documentos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo TEXT NOT NULL CHECK (tipo IN ('curriculo', 'carta')),
    nome TEXT NOT NULL,
    origem TEXT NOT NULL CHECK (origem IN ('gerado', 'importado')),
    arquivo TEXT NOT NULL,
    arquivo_html TEXT,
    formato TEXT NOT NULL CHECK (formato IN ('pdf', 'docx')),
    idioma TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    sha256 TEXT NOT NULL,
    tamanho INTEGER NOT NULL,
    vaga_id INTEGER REFERENCES vagas(id),
    variante_id TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One preferred document per (tipo, variante). variante_id '' = the general preference.
CREATE TABLE IF NOT EXISTS preferidos (
    tipo TEXT NOT NULL,
    variante_id TEXT NOT NULL DEFAULT '',
    documento_id INTEGER NOT NULL REFERENCES documentos(id) ON DELETE CASCADE,
    PRIMARY KEY (tipo, variante_id)
);
```

- [ ] **Step 2: Testes que falham** (em `documentos/mod.rs`, `#[cfg(test)] mod tests`, banco em memória com `crate::db::apply`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    pub(crate) fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }

    pub(crate) fn novo<'a>(tipo: &'a str, arquivo: &'a str, variante: Option<&'a str>) -> NovoDocumento<'a> {
        NovoDocumento { tipo, nome: "CV", origem: "gerado", arquivo, arquivo_html: None, formato: "pdf", idioma: Some("pt"), sha256: "00", tamanho: 1, variante_id: variante }
    }

    #[test]
    fn insert_get_and_list_newest_first() {
        let c = mem();
        let a = inserir(&c, &novo("curriculo", "a.pdf", None)).unwrap();
        let b = inserir(&c, &novo("curriculo", "b.pdf", None)).unwrap();
        inserir(&c, &novo("carta", "c.pdf", None)).unwrap();
        assert_eq!(obter(&c, a).unwrap().unwrap().arquivo, "a.pdf");
        let l = listar(&c, "curriculo").unwrap();
        assert_eq!(l.iter().map(|d| d.id).collect::<Vec<_>>(), vec![b, a], "only CVs, newest first");
        assert!(obter(&c, 999).unwrap().is_none());
    }

    #[test]
    fn one_preferred_per_type_and_variant_the_last_mark_wins() {
        let c = mem();
        let a = inserir(&c, &novo("curriculo", "a.pdf", None)).unwrap();
        let b = inserir(&c, &novo("curriculo", "b.pdf", None)).unwrap();
        definir_preferido(&c, a, None).unwrap();
        definir_preferido(&c, b, None).unwrap();
        assert_eq!(preferido(&c, "curriculo", None), Some(b));
        definir_preferido(&c, a, Some("backend")).unwrap();
        assert_eq!(preferido(&c, "curriculo", Some("backend")), Some(a));
        assert_eq!(preferido(&c, "curriculo", None), Some(b), "the general mark is separate from the variant's");
        assert_eq!(preferido(&c, "carta", None), None);
    }

    #[test]
    fn marking_an_unknown_document_is_an_error() {
        let c = mem();
        assert!(definir_preferido(&c, 42, None).is_err());
    }
}
```

- [ ] **Step 3: Rodar e ver falhar.** `cargo test --lib documentos` → FAIL (não compila: funções inexistentes).

- [ ] **Step 4: Implementar** `documentos/mod.rs`:

```rust
//! The library of documents the agent may attach. Files live in <app data>/documentos/; the database
//! holds only bare file names, so a row can never point outside that folder.
pub mod escolha;
pub mod fonte;
pub mod importar;
pub mod seguranca;

use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub fn docs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("documentos")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Documento {
    pub id: i64,
    pub tipo: String,
    pub nome: String,
    pub origem: String,
    pub arquivo: String,
    pub arquivo_html: Option<String>,
    pub formato: String,
    pub idioma: Option<String>,
    pub tags: String,
    pub sha256: String,
    pub tamanho: i64,
    pub vaga_id: Option<i64>,
    pub variante_id: Option<String>,
    pub criado_em: String,
}

pub struct NovoDocumento<'a> {
    pub tipo: &'a str,
    pub nome: &'a str,
    pub origem: &'a str,
    pub arquivo: &'a str,
    pub arquivo_html: Option<&'a str>,
    pub formato: &'a str,
    pub idioma: Option<&'a str>,
    pub sha256: &'a str,
    pub tamanho: i64,
    pub variante_id: Option<&'a str>,
}

const COLS: &str = "id, tipo, nome, origem, arquivo, arquivo_html, formato, idioma, tags, sha256, tamanho, vaga_id, variante_id, criado_em";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Documento> {
    Ok(Documento {
        id: r.get(0)?,
        tipo: r.get(1)?,
        nome: r.get(2)?,
        origem: r.get(3)?,
        arquivo: r.get(4)?,
        arquivo_html: r.get(5)?,
        formato: r.get(6)?,
        idioma: r.get(7)?,
        tags: r.get(8)?,
        sha256: r.get(9)?,
        tamanho: r.get(10)?,
        vaga_id: r.get(11)?,
        variante_id: r.get(12)?,
        criado_em: r.get(13)?,
    })
}

pub fn inserir(conn: &Connection, d: &NovoDocumento) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO documentos (tipo, nome, origem, arquivo, arquivo_html, formato, idioma, sha256, tamanho, variante_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![d.tipo, d.nome, d.origem, d.arquivo, d.arquivo_html, d.formato, d.idioma, d.sha256, d.tamanho, d.variante_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn obter(conn: &Connection, id: i64) -> rusqlite::Result<Option<Documento>> {
    conn.query_row(&format!("SELECT {COLS} FROM documentos WHERE id = ?1"), [id], row).optional()
}

pub fn listar(conn: &Connection, tipo: &str) -> rusqlite::Result<Vec<Documento>> {
    let mut st = conn.prepare(&format!("SELECT {COLS} FROM documentos WHERE tipo = ?1 ORDER BY id DESC"))?;
    let rows = st.query_map([tipo], row)?;
    rows.collect()
}

pub fn definir_preferido(conn: &Connection, id: i64, variante_id: Option<&str>) -> Result<(), String> {
    let doc = obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("documento {id} não existe"))?;
    conn.execute(
        "INSERT INTO preferidos (tipo, variante_id, documento_id) VALUES (?1, ?2, ?3)
         ON CONFLICT (tipo, variante_id) DO UPDATE SET documento_id = excluded.documento_id",
        params![doc.tipo, variante_id.unwrap_or(""), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<i64> {
    conn.query_row(
        "SELECT documento_id FROM preferidos WHERE tipo = ?1 AND variante_id = ?2",
        params![tipo, variante_id.unwrap_or("")],
        |r| r.get(0),
    )
    .ok()
}
```

Criar `escolha.rs`, `fonte.rs`, `importar.rs`, `seguranca.rs` vazios (só um comentário de uma linha) para compilar; as próximas tarefas os preenchem.

- [ ] **Step 5: Rodar e ver passar.** `cargo test --lib documentos` → PASS.

- [ ] **Step 6: Mutação.** Trocar `ON CONFLICT ... DO UPDATE` por `DO NOTHING` → `one_preferred_per_type_and_variant_the_last_mark_wins` tem de FALHAR (colar a linha). Restaurar.

- [ ] **Step 7: Commit.** `git commit -m "feat(documentos): library tables and store (one preferred per type and variant)"`

---

### Tarefa 2: Resolver o arquivo com segurança

**Files:**
- Modify: `src-tauri/src/documentos/seguranca.rs`

**Interfaces:**
- Consumes: `documentos::{obter, docs_dir, Documento}`
- Produces:
  - `pub struct Anexavel { pub id: i64, pub nome: String, pub formato: String, pub tamanho: u64, pub sha256: String, pub path: PathBuf }`
  - `pub fn sha256_file(path: &Path) -> std::io::Result<String>` (hex minúsculo)
  - `pub fn resolve_attachable(conn: &Connection, data_dir: &Path, id: i64) -> Result<Anexavel, String>`

- [ ] **Step 1: Testes que falham.**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{inserir, NovoDocumento};

    fn setup(name: &str) -> (rusqlite::Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-docs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir)).unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        (c, dir)
    }

    fn add(c: &rusqlite::Connection, dir: &Path, arquivo: &str, bytes: &[u8]) -> i64 {
        let p = crate::documentos::docs_dir(dir).join(arquivo);
        if !arquivo.contains("..") && !arquivo.contains('/') && !arquivo.contains('\\') {
            std::fs::write(&p, bytes).unwrap();
        }
        let sha = if p.exists() { sha256_file(&p).unwrap() } else { "00".into() };
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "importado", arquivo, arquivo_html: None, formato: "pdf", idioma: None, sha256: &sha, tamanho: bytes.len() as i64, variante_id: None }).unwrap()
    }

    #[test]
    fn a_registered_file_resolves_inside_the_folder() {
        let (c, dir) = setup("ok");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 hello");
        let a = resolve_attachable(&c, &dir, id).unwrap();
        assert!(a.path.ends_with("cv.pdf"));
        assert_eq!(a.tamanho, 14);
    }

    #[test]
    fn a_name_that_escapes_the_folder_is_refused() {
        let (c, dir) = setup("escape");
        std::fs::write(dir.join("segredo.pdf"), b"%PDF-1.7 x").unwrap();
        for bad in ["..\\segredo.pdf", "../segredo.pdf", "C:\\Windows\\win.ini", "sub/cv.pdf"] {
            let id = add(&c, &dir, bad, b"");
            let e = resolve_attachable(&c, &dir, id).unwrap_err();
            assert!(e.contains("fora da pasta"), "{bad}: {e}");
        }
    }

    #[test]
    fn missing_file_is_refused() {
        let (c, dir) = setup("missing");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 x");
        std::fs::remove_file(crate::documentos::docs_dir(&dir).join("cv.pdf")).unwrap();
        assert!(resolve_attachable(&c, &dir, id).unwrap_err().contains("sumiu"));
    }

    #[test]
    fn edited_file_is_refused() {
        let (c, dir) = setup("edited");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 original");
        std::fs::write(crate::documentos::docs_dir(&dir).join("cv.pdf"), b"%PDF-1.7 changed").unwrap();
        assert!(resolve_attachable(&c, &dir, id).unwrap_err().contains("mudou"));
    }

    #[test]
    fn unknown_id_is_refused() {
        let (c, dir) = setup("unknown");
        assert!(resolve_attachable(&c, &dir, 77).unwrap_err().contains("não existe"));
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.** `cargo test --lib documentos::seguranca` → FAIL.

- [ ] **Step 3: Implementar.**

```rust
//! The only way from a document id to a file on disk. The agent never gives a path; this checks that
//! the stored name stays inside <app data>/documentos/, that the file is there, and that it is the
//! same file the user registered (sha256).
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Anexavel {
    pub id: i64,
    pub nome: String,
    pub formato: String,
    pub tamanho: u64,
    pub sha256: String,
    pub path: PathBuf,
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn is_bare_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".." && !name.starts_with("..")
}

pub fn resolve_attachable(conn: &Connection, data_dir: &Path, id: i64) -> Result<Anexavel, String> {
    let doc = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    if !is_bare_name(&doc.arquivo) {
        return Err(format!("o documento {id} aponta para fora da pasta de documentos: recusado"));
    }
    let base = std::fs::canonicalize(super::docs_dir(data_dir)).map_err(|_| "a pasta de documentos não existe".to_string())?;
    let path = base.join(&doc.arquivo);
    let real = std::fs::canonicalize(&path).map_err(|_| format!("o arquivo de \"{}\" sumiu: gere ou importe de novo", doc.nome))?;
    if !real.starts_with(&base) {
        return Err(format!("o documento {id} aponta para fora da pasta de documentos: recusado"));
    }
    let meta = std::fs::metadata(&real).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(format!("o documento {id} aponta para fora da pasta de documentos: recusado"));
    }
    let sha = sha256_file(&real).map_err(|e| e.to_string())?;
    if sha != doc.sha256 {
        return Err(format!("o arquivo de \"{}\" mudou depois de registrado: gere ou importe de novo", doc.nome));
    }
    Ok(Anexavel { id, nome: doc.nome, formato: doc.formato, tamanho: meta.len(), sha256: sha, path: real })
}
```

- [ ] **Step 4: Rodar e ver passar.** `cargo test --lib documentos::seguranca` → PASS.

- [ ] **Step 5: Mutações** (uma por vez, colar o FAILED, restaurar):
  - remover o `if !is_bare_name(...)` **e** o `if !real.starts_with(&base)` → `a_name_that_escapes_the_folder_is_refused` falha;
  - remover a comparação de `sha` → `edited_file_is_refused` falha.

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): resolve a document id to a file only inside the folder and only if unchanged"`

---

### Tarefa 3: Importar o currículo do usuário

**Files:**
- Modify: `src-tauri/src/documentos/importar.rs`

**Interfaces:**
- Consumes: `docs_dir`, `inserir`, `obter`, `NovoDocumento`, `seguranca::sha256_file`
- Produces: `pub const MAX_BYTES: u64 = 10 * 1024 * 1024;` e `pub fn importar(conn: &Connection, data_dir: &Path, tipo: &str, origem: &Path, nome: Option<&str>) -> Result<Documento, String>`

- [ ] **Step 1: Testes que falham.**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> (rusqlite::Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-imp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        (c, dir)
    }

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn a_real_pdf_is_copied_registered_and_resolvable() {
        let (c, dir) = setup("pdf");
        let src = file(&dir, "Meu CV.pdf", b"%PDF-1.7 conteudo");
        let d = importar(&c, &dir, "curriculo", &src, None).unwrap();
        assert_eq!((d.origem.as_str(), d.formato.as_str(), d.nome.as_str()), ("importado", "pdf", "Meu CV"));
        assert!(src.exists(), "the user's original is never moved");
        crate::documentos::seguranca::resolve_attachable(&c, &dir, d.id).unwrap();
    }

    #[test]
    fn a_docx_is_accepted_by_its_zip_header() {
        let (c, dir) = setup("docx");
        let src = file(&dir, "cv.docx", b"PK\x03\x04 resto");
        assert_eq!(importar(&c, &dir, "curriculo", &src, Some("Word")).unwrap().formato, "docx");
    }

    #[test]
    fn a_fake_pdf_is_refused() {
        let (c, dir) = setup("fake");
        let src = file(&dir, "cv.pdf", b"MZ not a pdf");
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("não é um PDF"));
    }

    #[test]
    fn other_extensions_are_refused() {
        let (c, dir) = setup("ext");
        let src = file(&dir, "cv.exe", b"%PDF-1.7");
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("PDF ou DOCX"));
    }

    #[test]
    fn a_file_over_ten_megabytes_is_refused() {
        let (c, dir) = setup("big");
        let mut big = b"%PDF-1.7".to_vec();
        big.resize((MAX_BYTES + 1) as usize, b'x');
        let src = file(&dir, "cv.pdf", &big);
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("10 MB"));
    }

    #[test]
    fn import_sanitizes_names_and_never_overwrites() {
        let (c, dir) = setup("names");
        let src = file(&dir, "cv.pdf", b"%PDF-1.7 a");
        let a = importar(&c, &dir, "curriculo", &src, Some("..\\..\\x")).unwrap();
        let b = importar(&c, &dir, "curriculo", &src, Some("..\\..\\x")).unwrap();
        assert_ne!(a.arquivo, b.arquivo, "the same import twice gets two files");
        for d in [&a, &b] {
            assert!(!d.arquivo.contains(['\\', '/', ':']) && !d.arquivo.starts_with(".."), "{}", d.arquivo);
            crate::documentos::seguranca::resolve_attachable(&c, &dir, d.id).unwrap();
        }
    }

    #[test]
    fn a_wrong_type_is_refused() {
        let (c, dir) = setup("tipo");
        let src = file(&dir, "cv.pdf", b"%PDF-1.7");
        assert!(importar(&c, &dir, "foto", &src, None).is_err());
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.**

- [ ] **Step 3: Implementar.**

```rust
//! Bringing the user's own CV into the library: copied (never moved or referenced), checked by
//! extension AND content, at most 10 MB, stored under a generated bare name.
use super::{docs_dir, inserir, obter, Documento, NovoDocumento};
use rusqlite::Connection;
use std::path::Path;

pub const MAX_BYTES: u64 = 10 * 1024 * 1024;

pub fn importar(conn: &Connection, data_dir: &Path, tipo: &str, origem: &Path, nome: Option<&str>) -> Result<Documento, String> {
    if tipo != "curriculo" && tipo != "carta" {
        return Err(format!("tipo inválido: {tipo}"));
    }
    let ext = origem.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let formato = match ext.as_str() {
        "pdf" => "pdf",
        "docx" => "docx",
        _ => return Err("só é possível importar PDF ou DOCX".into()),
    };
    let meta = std::fs::metadata(origem).map_err(|_| "não foi possível ler o arquivo escolhido".to_string())?;
    if meta.len() > MAX_BYTES {
        return Err("o arquivo passa de 10 MB".into());
    }
    let bytes = std::fs::read(origem).map_err(|_| "não foi possível ler o arquivo escolhido".to_string())?;
    let ok = match formato {
        "pdf" => bytes.starts_with(b"%PDF-"),
        _ => bytes.starts_with(b"PK\x03\x04"),
    };
    if !ok {
        return Err(if formato == "pdf" { "o arquivo não é um PDF de verdade".into() } else { "o arquivo não é um DOCX de verdade".into() });
    }
    let dir = docs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let arquivo = format!("importado-{nanos}.{formato}");
    let dest = dir.join(&arquivo);
    if dest.exists() {
        return Err("conflito de nome ao importar; tente de novo".into());
    }
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    let sha = super::seguranca::sha256_file(&dest).map_err(|e| e.to_string())?;
    let stem = origem.file_stem().and_then(|s| s.to_str()).unwrap_or("Currículo");
    let nome = nome.unwrap_or(stem).trim();
    let nome = if nome.is_empty() { "Currículo" } else { nome };
    let id = inserir(conn, &NovoDocumento { tipo, nome, origem: "importado", arquivo: &arquivo, arquivo_html: None, formato, idioma: None, sha256: &sha, tamanho: bytes.len() as i64, variante_id: None })
        .map_err(|e| {
            let _ = std::fs::remove_file(&dest);
            e.to_string()
        })?;
    obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| "falha ao registrar o documento".into())
}
```

O `nome` exibido pode ter qualquer texto; o **arquivo** sempre é `importado-<nanos>.<formato>`.

- [ ] **Step 4: Rodar e ver passar.**

- [ ] **Step 5: Mutações:** remover a checagem de cabeçalho → `a_fake_pdf_is_refused` falha; remover o limite → `a_file_over_ten_megabytes_is_refused` falha.

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): import the user's own PDF/DOCX safely (copy, real type, 10 MB, generated name)"`

---

### Tarefa 4: Regra de escolha

**Files:**
- Modify: `src-tauri/src/documentos/escolha.rs`

**Interfaces:**
- Consumes: `preferido`, tabela `documentos`
- Produces:
  - `pub struct Escolha { pub id: i64, pub regra: &'static str }` — `regra` ∈ `"preferido da variante"`, `"preferido geral"`, `"gerado mais recente da variante"`
  - `pub fn escolher(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<Escolha>`

- [ ] **Step 1: Testes que falham.**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{definir_preferido, inserir, NovoDocumento};

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }
    fn doc(c: &rusqlite::Connection, origem: &str, variante: Option<&str>) -> i64 {
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem, arquivo: "x.pdf", arquivo_html: None, formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: variante }).unwrap()
    }

    #[test]
    fn the_variant_preference_wins_over_everything() {
        let c = mem();
        let geral = doc(&c, "importado", None);
        let var = doc(&c, "importado", None);
        doc(&c, "gerado", Some("backend"));
        definir_preferido(&c, geral, None).unwrap();
        definir_preferido(&c, var, Some("backend")).unwrap();
        let e = escolher(&c, "curriculo", Some("backend")).unwrap();
        assert_eq!((e.id, e.regra), (var, "preferido da variante"));
    }

    #[test]
    fn without_a_variant_preference_the_general_one() {
        let c = mem();
        let geral = doc(&c, "importado", None);
        doc(&c, "gerado", Some("backend"));
        definir_preferido(&c, geral, None).unwrap();
        assert_eq!(escolher(&c, "curriculo", Some("backend")).unwrap().regra, "preferido geral");
        assert_eq!(escolher(&c, "curriculo", None).unwrap().id, geral);
    }

    #[test]
    fn without_preferences_the_newest_generated_for_that_variant() {
        let c = mem();
        doc(&c, "gerado", Some("backend"));
        let novo = doc(&c, "gerado", Some("backend"));
        doc(&c, "gerado", Some("devops"));
        doc(&c, "importado", None);
        let e = escolher(&c, "curriculo", Some("backend")).unwrap();
        assert_eq!((e.id, e.regra), (novo, "gerado mais recente da variante"));
    }

    #[test]
    fn nothing_to_pick_returns_none() {
        let c = mem();
        doc(&c, "importado", None);
        doc(&c, "gerado", Some("devops"));
        assert!(escolher(&c, "curriculo", Some("backend")).is_none(), "no preference, nothing generated for backend");
        assert!(escolher(&c, "curriculo", None).is_none(), "no variant and no general preference");
        assert!(escolher(&c, "carta", None).is_none());
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.**

- [ ] **Step 3: Implementar.**

```rust
//! Which document to attach (spec section 4). The user's marks always win over what was generated.
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq)]
pub struct Escolha {
    pub id: i64,
    pub regra: &'static str,
}

pub fn escolher(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<Escolha> {
    if let Some(v) = variante_id {
        if let Some(id) = super::preferido(conn, tipo, Some(v)) {
            return Some(Escolha { id, regra: "preferido da variante" });
        }
    }
    if let Some(id) = super::preferido(conn, tipo, None) {
        return Some(Escolha { id, regra: "preferido geral" });
    }
    let v = variante_id?;
    conn.query_row(
        "SELECT id FROM documentos WHERE tipo = ?1 AND origem = 'gerado' AND variante_id = ?2 ORDER BY id DESC LIMIT 1",
        params![tipo, v],
        |r| r.get(0),
    )
    .ok()
    .map(|id| Escolha { id, regra: "gerado mais recente da variante" })
}
```

- [ ] **Step 4: Rodar e ver passar.**

- [ ] **Step 5: Mutações:** trocar a ordem dos dois primeiros blocos → `the_variant_preference_wins_over_everything` falha; tirar `AND origem = 'gerado'` → `nothing_to_pick_returns_none` ou `without_preferences_...` falha (colar qual).

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): selection rule - variant preference, general preference, newest generated for the variant"`

---

### Tarefa 5: Tool `browser_upload` (com a regra "o Chrome nunca falha")

**Files:**
- Modify: `src-tauri/src/mcp/tools/browser.rs`
- Create: `src-tauri/tests/fixtures/upload-form.html`
- Modify: `src-tauri/tests/browser_live.rs`

**Interfaces:**
- Produces (em `mcp/tools/browser.rs`):
  - `pub struct Attachment { pub id: i64, pub nome: String, pub formato: String, pub tamanho: u64, pub sha256: String, pub path: std::path::PathBuf, pub regra: String }`
  - `pub trait DocumentSource: Send + Sync { fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String>; }`
  - `pub struct NoDocuments;` (implementa recusando: "a biblioteca de documentos não está disponível")
  - `BrowserTools::with_documents(self, docs: Arc<dyn DocumentSource>) -> Self`
  - `pub async fn upload(&self, reference: &str, tipo: Option<&str>, documento: Option<i64>) -> Result<String, String>`

- [ ] **Step 1: Fixture** `src-tauri/tests/fixtures/upload-form.html`:

```html
<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><title>Upload</title></head>
<body>
  <h1>Candidatura de teste</h1>
  <label for="cv">Currículo</label>
  <input id="cv" type="file" onchange="document.getElementById('nome-cv').textContent = this.files[0] ? this.files[0].name : ''">
  <span id="nome-cv"></span>

  <!-- LinkedIn pattern: the real input is hidden, a button opens the chooser -->
  <input id="escondido" type="file" style="display:none" onchange="document.getElementById('nome-esc').textContent = this.files[0] ? this.files[0].name : ''">
  <button id="anexar" type="button" onclick="document.getElementById('escondido').click()">Carregar currículo</button>
  <span id="nome-esc"></span>

  <!-- only PDF: the page itself rejects anything else -->
  <input id="so-pdf" type="file" accept=".pdf" aria-label="Só PDF" onchange="var f=this.files[0]; document.getElementById('nome-pdf').textContent = (f && f.name.toLowerCase().endsWith('.pdf')) ? f.name : 'rejeitado'">
  <span id="nome-pdf"></span>

  <button id="morto" type="button">Botão que não abre nada</button>
  <button id="enviar" type="button" onclick="window.__submitted = true">Enviar candidatura</button>
  <script>window.__submitted = false;</script>
</body></html>
```

- [ ] **Step 2: Testes live que falham** (em `tests/browser_live.rs`; use os helpers existentes `launch`, `tools`, `Ctx::candidatando`, `fixture_file`, `agent_client`, `eval`, `ref_of`):

```rust
struct FakeDocs {
    path: std::path::PathBuf,
    formato: &'static str,
}

impl claudia_rh_lib::mcp::tools::browser::DocumentSource for FakeDocs {
    fn for_upload(&self, _vaga: i64, _tipo: &str, _doc: Option<i64>) -> Result<claudia_rh_lib::mcp::tools::browser::Attachment, String> {
        Ok(claudia_rh_lib::mcp::tools::browser::Attachment {
            id: 1,
            nome: "CV Teste".into(),
            formato: self.formato.into(),
            tamanho: std::fs::metadata(&self.path).unwrap().len(),
            sha256: "abc".into(),
            path: self.path.clone(),
            regra: "preferido geral".into(),
        })
    }
}

fn fake_doc(ext: &'static str) -> Arc<FakeDocs> {
    let p = std::env::temp_dir().join(format!("claudia-upload-{}.{ext}", std::process::id()));
    std::fs::write(&p, if ext == "pdf" { &b"%PDF-1.7 teste"[..] } else { &b"PK\x03\x04 teste"[..] }).unwrap();
    Arc::new(FakeDocs { path: p, formato: ext })
}

async fn text_of(l: &Live, id: &str) -> String {
    let a = agent_client(l).await;
    eval(&a, &format!("document.getElementById('{id}').textContent")).await.as_str().unwrap_or("").to_string()
}

#[tokio::test]
#[ignore]
async fn upload_into_a_visible_file_input_and_the_page_sees_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap();
    assert!(out.contains("CV Teste") && out.contains("preferido geral"), "{out}");
    assert!(text_of(&l, "nome-cv").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_through_a_button_that_opens_the_chooser() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Carregar currículo"), None, None).await.unwrap();
    assert!(text_of(&l, "nome-esc").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_refuses_a_submit_button_and_never_clicks_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Enviar candidatura"), None, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn upload_without_an_active_vaga_is_refused() {
    let l = launch().await;
    let none = Arc::new(Ctx { status: "candidatando", active: None, claimed: Mutex::new(HashSet::new()) });
    let t = tools(&l, none).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap_err();
    assert!(e.contains("candidatando"), "{e}");
}

#[tokio::test]
#[ignore]
async fn upload_reports_when_the_page_did_not_take_the_file() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("docx"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Só PDF"), None, None).await.unwrap();
    assert!(out.contains("a página não mostrou o nome do arquivo"), "{out}");
}

#[tokio::test]
#[ignore]
async fn a_chooser_that_never_opens_times_out_and_the_tab_still_works() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Botão que não abre nada"), None, None).await.unwrap_err();
    assert!(e.contains("seletor de arquivo"), "{e}");
    // The tab is still usable, and the chooser is no longer intercepted: a later real chooser works.
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Carregar currículo"), None, None).await.unwrap();
    assert!(text_of(&l, "nome-esc").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_with_an_empty_library_is_refused_with_a_hint() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)); // NoDocuments
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap_err();
    assert!(e.contains("documentos"), "{e}");
    t.read(None).await.unwrap(); // tab still fine
}
```

Se o papel AX do `<input type=file>` não for `button` no Chrome for Testing 154, ajustar `ref_of` para o papel real que `browser_read` mostrar (ler a saída de `browser_read` da fixture primeiro e anotar no teste). Se `Ctx`/`tools` não forem públicos o suficiente, ajustar apenas a visibilidade dentro do arquivo de teste.

- [ ] **Step 3: Rodar e ver falhar** (não compila: `with_documents`/`upload` não existem). Comando live do `CLAUDE.md` com `--test browser_live upload`.

- [ ] **Step 4: Implementar em `mcp/tools/browser.rs`.** Acrescentar perto de `ActionSink`:

```rust
/// What browser_upload attaches: always resolved by the app from the library, never a path from the agent.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: i64,
    pub nome: String,
    pub formato: String,
    pub tamanho: u64,
    pub sha256: String,
    pub path: std::path::PathBuf,
    pub regra: String,
}

pub trait DocumentSource: Send + Sync {
    fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String>;
}

pub struct NoDocuments;
impl DocumentSource for NoDocuments {
    fn for_upload(&self, _: i64, _: &str, _: Option<i64>) -> Result<Attachment, String> {
        Err("a biblioteca de documentos não está disponível: gere ou importe um currículo no app".into())
    }
}

const UPLOAD_STEP: Duration = Duration::from_secs(5);
```

No struct `BrowserTools` acrescentar `docs: Arc<dyn DocumentSource>,` iniciado em `new` com `Arc::new(NoDocuments)`, e:

```rust
pub fn with_documents(mut self, docs: Arc<dyn DocumentSource>) -> Self {
    self.docs = docs;
    self
}
```

O método (junto das outras tools):

```rust
pub async fn upload(&self, reference: &str, tipo: Option<&str>, documento: Option<i64>) -> Result<String, String> {
    let started = Instant::now();
    self.agent_active.store(true, Ordering::SeqCst);
    let (c, _, _) = match self.tab().await {
        Ok(t) => t,
        Err(e) => return self.finish("browser_upload", started, Err(e), None).await,
    };
    let r = async {
        let tipo = tipo.unwrap_or("curriculo");
        if tipo != "curriculo" && tipo != "carta" {
            return Err(format!("tipo inválido: {tipo} (use curriculo ou carta)"));
        }
        let vaga = self.guard.active_vaga().ok_or_else(|| {
            "RECUSADO: anexar exige exatamente uma vaga em 'candidatando' (a vaga sendo candidatada).".to_string()
        })?;
        let att = self.docs.for_upload(vaga, tipo, documento)?;
        let n = self.resolve_ref(&c, reference).await?;
        let file = att.path.to_string_lossy().to_string();
        let (shot, x, y) = self.locate(&c, &n, true).await?;
        if Self::is_file_input(&c, n.backend_node_id).await {
            tokio::time::timeout(UPLOAD_STEP, c.call("DOM.setFileInputFiles", json!({ "files": [file], "backendNodeId": n.backend_node_id })))
                .await
                .map_err(|_| "o campo de arquivo não respondeu a tempo".to_string())??;
        } else {
            // A button that opens the chooser: it must not be a submit (the upload never submits).
            let mut probe = Probe::default();
            probe.add_name(&n.name);
            Self::probe_node(&c, n.backend_node_id, &mut probe).await;
            if let Some(hit) = Self::node_at_point(&c, x, y).await {
                if hit != n.backend_node_id {
                    Self::probe_node(&c, hit, &mut probe).await;
                }
            }
            let url = Self::page_url(&c).await;
            if self.run_guard(&probe, &url, None).is_err() {
                return Err("RECUSADO: esse elemento envia a candidatura; browser_upload só anexa arquivos.".into());
            }
            Self::upload_via_chooser(&c, x, y, &file).await?;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        let file_name = att.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let shown = Self::eval(&c, &format!("document.body ? document.body.innerText.includes({}) : false", serde_json::to_string(&file_name).unwrap_or_default()))
            .await
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let kb = (att.tamanho + 1023) / 1024;
        let mut message = format!("Anexou \"{}\" ({}, {} KB) pela regra: {}.", att.nome, att.formato, kb, att.regra);
        if !shown {
            message.push_str(" Atenção: a página não mostrou o nome do arquivo; confira com browser_read ou browser_text antes de seguir (o site pode ter recusado o formato).");
        }
        Ok(Outcome {
            message,
            target: Some(n.name),
            role: Some(n.role),
            // Never the content: name, format, size and a short hash only.
            text: Some(format!("{} | {} | {} B | sha256 {}", att.nome, att.formato, att.tamanho, &att.sha256[..att.sha256.len().min(12)])),
            shot_before: shot,
            ..Default::default()
        })
    }
    .await;
    self.finish("browser_upload", started, r, Some(&c)).await
}

async fn is_file_input(c: &CdpClient, backend: i64) -> bool {
    let Ok(d) = c.call("DOM.describeNode", json!({ "backendNodeId": backend })).await else { return false };
    let node = &d["node"];
    if node["nodeName"].as_str() != Some("INPUT") {
        return false;
    }
    let attrs = node["attributes"].as_array().cloned().unwrap_or_default();
    attrs.chunks(2).any(|kv| kv.first().and_then(|k| k.as_str()) == Some("type") && kv.get(1).and_then(|v| v.as_str()).map(|v| v.eq_ignore_ascii_case("file")) == Some(true))
}

/// Clicks a button that opens the file chooser, intercepting the chooser so no native dialog ever
/// appears. The interception is ALWAYS turned off again, success or not: the tab must stay usable.
async fn upload_via_chooser(c: &CdpClient, x: f64, y: f64, file: &str) -> Result<(), String> {
    let mut events = c.subscribe();
    let _ = c.call("Page.enable", json!({})).await;
    c.call("Page.setInterceptFileChooserDialog", json!({ "enabled": true })).await?;
    let result = async {
        Self::mouse_click(c, x, y).await?;
        let backend = tokio::time::timeout(UPLOAD_STEP, async {
            loop {
                match events.recv().await {
                    Ok(ev) if ev["method"] == "Page.fileChooserOpened" => {
                        return ev["params"]["backendNodeId"].as_i64();
                    }
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return None,
                }
            }
        })
        .await
        .map_err(|_| "o elemento não abriu o seletor de arquivo (tempo esgotado)".to_string())?
        .ok_or_else(|| "o seletor de arquivo abriu sem um campo de arquivo associado".to_string())?;
        tokio::time::timeout(UPLOAD_STEP, c.call("DOM.setFileInputFiles", json!({ "files": [file], "backendNodeId": backend })))
            .await
            .map_err(|_| "o campo de arquivo não respondeu a tempo".to_string())??;
        Ok(())
    }
    .await;
    let _ = c.call("Page.setInterceptFileChooserDialog", json!({ "enabled": false })).await;
    result
}
```

Confirmar no código os nomes/assinaturas reais de `Probe`, `probe_node`, `node_at_point`, `run_guard`, `mouse_click`, `locate` e `CdpClient::subscribe` (são os usados em `click`, linhas ~954–986); se algum diferir, usar o nome real **sem mudar a lógica**. O formato dos eventos em `subscribe()` deve ser conferido em `browser/cdp.rs` (campo `method`/`params`); se for outro, ajustar só a leitura do evento.

- [ ] **Step 5: Rodar e ver passar** os 7 testes live de upload e a suíte live inteira (`--skip icon`).

- [ ] **Step 6: Mutações** (uma por vez, colar o FAILED, restaurar):
  - remover a chamada `Page.setInterceptFileChooserDialog` com `enabled: false` → `a_chooser_that_never_opens_times_out_and_the_tab_still_works` tem de falhar (a segunda tentativa trava ou não recebe o evento; se não falhar, o teste não protege a regra: reforçar até falhar);
  - remover o bloco do guard (`run_guard`) → `upload_refuses_a_submit_button_and_never_clicks_it` falha;
  - remover o `active_vaga().ok_or_else` → `upload_without_an_active_vaga_is_refused` falha.

- [ ] **Step 7: Commit.** `git commit -m "feat(mcp): browser_upload attaches a library document by ref; chooser interception always released; never submits"`

---

### Tarefa 6: Ligar a biblioteca ao app e ao MCP

**Files:**
- Modify: `src-tauri/src/documentos/fonte.rs`
- Create: `src-tauri/src/commands/documentos.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (registrar comandos; remover `debug_pdf_prova`)
- Modify: `src-tauri/src/mcp/server.rs` (`browser_upload` + `with_documents`)
- Modify: `src/components/perfil/CurriculosView.tsx`, `src/i18n/pt.ts`, `src/i18n/en.ts`

**Interfaces:**
- Consumes: `escolha::escolher`, `seguranca::resolve_attachable`, `importar::importar`, `DocumentSource`
- Produces:
  - `pub struct DbDocumentSource { pub data_dir: PathBuf }` implementando `DocumentSource`
  - comandos Tauri: `listar_documentos(tipo: String) -> Vec<Documento>`, `importar_documento(tipo: String, caminho_origem: String, nome: Option<String>) -> Documento`, `definir_preferido(id: i64, variante_id: Option<String>)`

- [ ] **Step 1: Teste que falha para `DbDocumentSource`** (em `fonte.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::browser::DocumentSource;

    fn setup() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-fonte-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        crate::db::apply(&c).unwrap();
        c.execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status, variante_id) VALUES (7, 't', 'e', 'p', 'u', datetime('now'), 'candidatando', 'backend')", []).unwrap();
        dir
    }

    #[test]
    fn picks_by_the_vagas_variant_and_resolves_safely() {
        let dir = setup();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("meu.pdf");
        std::fs::write(&src, b"%PDF-1.7 x").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, Some("Meu")).unwrap();
        crate::documentos::definir_preferido(&c, d.id, Some("backend")).unwrap();
        let a = DbDocumentSource { data_dir: dir.clone() }.for_upload(7, "curriculo", None).unwrap();
        assert_eq!((a.id, a.regra.as_str()), (d.id, "preferido da variante"));
    }

    #[test]
    fn an_explicit_document_must_be_of_the_right_type() {
        let dir = setup();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("carta.pdf");
        std::fs::write(&src, b"%PDF-1.7 x").unwrap();
        let carta = crate::documentos::importar::importar(&c, &dir, "carta", &src, None).unwrap();
        let e = DbDocumentSource { data_dir: dir }.for_upload(7, "curriculo", Some(carta.id)).unwrap_err();
        assert!(e.contains("não é um curriculo"), "{e}");
    }

    #[test]
    fn nothing_to_attach_explains_what_to_do() {
        let dir = setup();
        let e = DbDocumentSource { data_dir: dir }.for_upload(7, "curriculo", None).unwrap_err();
        assert!(e.contains("preferido"), "{e}");
    }
}
```

(Se o `INSERT INTO vagas` precisar de outras colunas `NOT NULL`, ver `schema.sql` e preencher.)

- [ ] **Step 2: Implementar `fonte.rs`.**

```rust
//! What browser_upload asks: "which file for this vaga?". Answered from the library only.
use crate::mcp::tools::browser::{Attachment, DocumentSource};
use std::path::PathBuf;

pub struct DbDocumentSource {
    pub data_dir: PathBuf,
}

impl DocumentSource for DbDocumentSource {
    fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String> {
        let conn = crate::mcp::tools::open_db(&self.data_dir)?;
        let (id, regra) = match documento {
            Some(id) => {
                let d = super::obter(&conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
                if d.tipo != tipo {
                    return Err(format!("o documento {id} não é um {tipo}"));
                }
                (id, "escolhido pelo agente".to_string())
            }
            None => {
                let variante: Option<String> = conn
                    .query_row("SELECT variante_id FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0))
                    .ok()
                    .flatten();
                let e = super::escolha::escolher(&conn, tipo, variante.as_deref()).ok_or_else(|| {
                    format!("não há {tipo} para anexar: marque um como preferido no app (Perfil → Currículos) ou gere um para a variante da vaga")
                })?;
                (e.id, e.regra.to_string())
            }
        };
        let a = super::seguranca::resolve_attachable(&conn, &self.data_dir, id)?;
        Ok(Attachment { id: a.id, nome: a.nome, formato: a.formato, tamanho: a.tamanho, sha256: a.sha256, path: a.path, regra })
    }
}
```

Se `open_db` for `pub(crate)` em outro módulo, usar o caminho real.

- [ ] **Step 3: Rodar** `cargo test --lib documentos::fonte` → PASS. Mutação: ignorar `vaga.variante_id` (passar `None`) → `picks_by_the_vagas_variant_and_resolves_safely` falha.

- [ ] **Step 4: Comandos Tauri** em `commands/documentos.rs`:

```rust
use crate::documentos::{self, Documento};
use tauri::{AppHandle, Manager};

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn conn(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    crate::mcp::tools::open_db(&data_dir(app)?)
}

#[tauri::command]
pub fn listar_documentos(app: AppHandle, tipo: String) -> Result<Vec<Documento>, String> {
    documentos::listar(&conn(&app)?, &tipo).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn importar_documento(app: AppHandle, tipo: String, caminho_origem: String, nome: Option<String>) -> Result<Documento, String> {
    documentos::importar::importar(&conn(&app)?, &data_dir(&app)?, &tipo, std::path::Path::new(&caminho_origem), nome.as_deref())
}

#[tauri::command]
pub fn definir_preferido(app: AppHandle, id: i64, variante_id: Option<String>) -> Result<(), String> {
    documentos::definir_preferido(&conn(&app)?, id, variante_id.as_deref())
}
```

Registrar os três no `invoke_handler` de `lib.rs`.

- [ ] **Step 5: MCP.** Em `mcp/server.rs`:

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserUploadArgs {
    /// Ref de browser_read do campo de arquivo ou do botão que abre o seletor de arquivo
    reference: String,
    /// "curriculo" (padrão) ou "carta"
    tipo: Option<String>,
    /// Opcional: id de um documento da biblioteca. Sem ele, a Claudia escolhe (preferido da variante da vaga, depois o preferido geral, depois o gerado da variante). Nunca um caminho de arquivo.
    documento: Option<i64>,
}
```

e a tool:

```rust
#[tool(
    description = "Anexa o currículo (ou a carta) da biblioteca da Claudia ao campo de arquivo indicado pela ref de browser_read, ou ao botão que abre o seletor de arquivo. A Claudia escolhe o arquivo; você nunca passa um caminho. Só funciona com a vaga ativa em 'candidatando'. Nunca envia a candidatura."
)]
async fn browser_upload(&self, Parameters(a): Parameters<BrowserUploadArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
    self.browser_reply("browser_upload", self.browser.upload(&a.reference, a.tipo.as_deref(), a.documento).await)
}
```

Onde o `BrowserTools` é criado (≈ linha 462): `BrowserTools::new(state_file, guard, sink).with_documents(Arc::new(crate::documentos::fonte::DbDocumentSource { data_dir: cfg.data_dir.clone() }))`. A tool só fica disponível para a sessão de execução: seguir o mesmo filtro por `SessionKind` que as outras `browser_*` já usam (conferir em `server.rs`).

- [ ] **Step 6: Bloco mínimo no front** (`CurriculosView.tsx`), abaixo da lista atual: título "Para anexar", lista de `listar_documentos("curriculo")` com nome, origem, formato e botão estrela "Usar como preferido" (`definir_preferido({ id })`), e botão "Adicionar o meu" que usa `open` de `@tauri-apps/plugin-dialog` com filtros `pdf`/`docx` e chama `importar_documento({ tipo: "curriculo", caminhoOrigem, nome: null })`. Mostrar o erro do backend quando houver. Textos em `pt.ts`/`en.ts` (`profile.attach.title`, `profile.attach.importMine`, `profile.attach.makePreferred`, `profile.attach.preferred`, `profile.attach.empty`). Este bloco é provisório: a Parte 2 o substitui pela aba Currículos.

- [ ] **Step 7: Verificar.** `cargo test --lib` (exit 0), `npx tsc --noEmit` (exit 0), suíte live inteira (`--skip icon`, exit 0), smoke MCP (`tests/mcp-browser-smoke.mjs`) TUDO OK.

- [ ] **Step 8: Commit.** `git commit -m "feat: wire the document library to the app and the MCP (browser_upload, import, preferred)"`

---

### Tarefa 7: Ensinar o agente (BLOQUEADA até autorização explícita do usuário)

**Files:** `src-tauri/src/prompt_sistema_runtime.md`, `src-tauri/src/commands/prompts.rs` (teste do prompt)

- [ ] **Step 1: Pedir autorização ao usuário**, mostrando o texto exato a acrescentar. Sem um "pode editar o prompt", **pular esta tarefa** e registrar que o agente ainda não conhece a tool.
- [ ] **Step 2: Texto proposto** (na seção das ferramentas do browser, depois do bullet de `browser_scroll`):

```markdown
- `browser_upload` anexa o currículo (ou, com `tipo: "carta"`, a carta) ao campo de arquivo ou ao botão que abre o seletor, pela ref de `browser_read`. **Não escolha arquivo nem passe caminho:** a Claudia escolhe pela variante da vaga e pelas marcas do usuário, e diz qual usou. Depois, confira com `browser_read` ou `browser_text` que o nome do arquivo apareceu. Se a tool disser que não há currículo para anexar, registre uma pendência e não siga com a candidatura.
```

- [ ] **Step 3: Teste do prompt** em `commands/prompts.rs`: acrescentar `"browser_upload"` à lista do teste `the_shipped_runtime_prompt_no_longer_sends_the_agent_to_the_extension` (que exige cada tool no prompt). Rodar → PASS. Mutação: tirar a linha do prompt → o teste falha.
- [ ] **Step 4: Commit.** `git commit -m "docs(prompt): teach browser_upload (approved by the user)"`

---

### Tarefa 8: Rodada de testes da Parte 1 e portão

Seguir o `CLAUDE.md` (Processo de desenvolvimento, seções 3 a 6).

- [ ] **Step 1: Unitários e regressão:** `cargo test --lib`, `npx tsc --noEmit`, `npm run build` — exit 0.
- [ ] **Step 2: Integração:** suíte live inteira (`--skip icon`) e smoke MCP — exit 0 / TUDO OK.
- [ ] **Step 3: Rodada de agente** (Haiku faz, Opus observa): o agente, pelo MCP do binário, abre `upload-form.html` (Chrome de teste, pasta temporária, `CLAUDIA_ALLOW_LOCAL_URLS`), lê, anexa nos três campos, tenta anexar no "Enviar candidatura" e no "Botão que não abre nada". O Opus cruza transcript, Activity log (linhas `browser_upload` com nome/formato/tamanho, **sem conteúdo**) e prints.
- [ ] **Step 4: Smoke de uso real no app** (`/claudia-teste`): Perfil → Currículos → bloco "Para anexar": importar um PDF próprio, marcar preferido; conferir no banco (`banco.mjs "SELECT id, nome, origem, formato FROM documentos"`) e na pasta `documentos/`. Prints.
- [ ] **Step 5: Upload num site real — só se o usuário autorizar a rodada** (ex.: LinkedIn Easy Apply até o passo do currículo, **sem enviar**). Sem autorização, registrar como "não testado em site real".
- [ ] **Step 6: Review Opus** do diff da Parte 1 (segurança do caminho, robustez do upload, guarda). Corrigir ou registrar os achados.
- [ ] **Step 7: Portão.** Relatório ao usuário: o que foi feito, evidências (exit codes, FAILED das mutações, prints), o que não foi verificado. **A Parte 2 só começa com a aprovação dele.**

---

# PARTE 2 — O resto (depois do portão da Parte 1)

> Revisão de 2026-10-04, escrita depois do portão da Parte 1 (commits `e3d942e..e9337a3`), contra o código atual. Substitui o esboço anterior das Tarefas 9 a 16. Os trechos de Rust da Onda 0 (`biblioteca.rs`) e das Tarefas 9 (`pdf.rs`) e 9b (`registrar.rs`) foram **compilados e testados** pelo planejador numa worktree descartável (`cargo test --lib documentos` → 30 passed; `pdf.rs` compila e o futuro de `html_to_pdf` é `Send`). O resto foi conferido contra os nomes reais do código, mas não compilado: se o compilador discordar de um nome, siga a mensagem dele **sem mudar a lógica**.

## O que a Parte 1 ensinou (e muda a Parte 2)

- `<input type=file>` aparece em `browser_read` como `button` com o nome do rótulo; as refs são `eN`.
- Toda conexão de `commands/documentos.rs` e de `fonte.rs` vem de `crate::mcp::tools::open_db`, **sem** `PRAGMA foreign_keys`: o `ON DELETE CASCADE` de `preferidos` não roda. Corrigido na Onda 0 (`biblioteca::open_conn`).
- `fonte.rs` já entrega ao Chrome uma cópia com nome legível em `documentos/envio/<id>/`. A migração e o apagar precisam pular/limpar essa subpasta.
- Comandos Tauri **síncronos rodam na thread principal** (hash de 10 MB trava a UI). Os comandos de documentos passam a `async` com `spawn_blocking`.
- O processo do MCP (`claudia-rh --mcp-serve`) **não tem WebView2 nem `AppHandle`**: não consegue gerar PDF sozinho. Isso afeta `save_cover_letter` (ver P3).
- `generate_handler!` aceita `#[cfg(debug_assertions)]` por item (conferido em `tauri-macros 2.6.3`).
- O callback do `PrintToPdfCompletedHandler` recebe `(windows_core::Result<()>, bool)`, não `BOOL`. Só `webview2-com 0.38` e `windows-core 0.61` são necessários (o crate `windows` não).
- `search_variants.yaml` é lido para `SearchVariants` e regravado inteiro: campo novo precisa estar no struct, senão some no primeiro save.
- `SearchVariant` tem `cv_gerado_path` (um caminho): nenhuma tool MCP pode devolvê-lo (spec 7, "nenhuma tool devolve caminho").

## Decisões que o plano NÃO toma (PROPOSTAS NÃO DECIDIDAS)

A spec não cobre estes pontos. Cada um tem recomendação; **nada aqui é implementado sem o "sim" do usuário**. Onde o plano depende de uma delas, o passo está marcado `[depende de Pn]` e é pulado se a resposta for "não".

| # | Ponto | Recomendação | Bloqueia |
|---|---|---|---|
| **P1** | `gerar_cover_letter` hoje não devolve nada: gera em segundo plano e emite `cover-letter-stream` / `cover-letter-done` / `cover-letter-error`. A spec diz "idem, devolve `Documento`". | Manter o streaming e o retorno vazio; o evento `cover-letter-done` passa a levar o `Documento` do contrato (em vez de `CoverLetterInfo`). | Onda 1 (A e C) |
| **P2** | Onde o usuário marca a variante **principal** (spec 2.1 diz que ele marca; a seção 10 não diz onde). | Um botão "Principal" no cartão da variante, no Perfil. O backend garante no máximo uma (feito na Tarefa 10 de qualquer forma). | Onda 1 (C, um passo) |
| **P3** | `save_cover_letter` precisa de PDF, e o processo do MCP não tem WebView2. | O MCP grava o HTML e um `.json` com os metadados em `documentos/pendentes/`, avisa o app (`"documentos"` na porta de notificação), o app gera o PDF e registra; a tool espera até 20 s pelo id e responde com ele (ou "em preparo" se o app não responder). | Onda 2 (D e E) |
| **P4** | `update_variante`: "nunca afirma fato fora do `candidate_base.yaml`" — qual é a checagem concreta. | Só nos campos que a chamada altera: cada item de `foco_competencias` precisa existir (sem diferenciar maiúsculas) em `competencias` ou nas `tecnologias` de experiências/projetos; cada item de `foco_experiencia` precisa ser a `empresa` ou o `cargo` de uma experiência. **Não verificado** o formato real de `foco_experiencia` nos dados do usuário. | Onda 2 (E) |
| **P5** | Como saber qual documento foi anexado (spec 10: `documento_id` em `candidaturas`; Dashboard mostra o anexado). Na hora do upload ainda não existe candidatura. | Tabela nova `anexos (vaga_id, tipo, documento_id, anexado_em)`, gravada pelo app a cada `browser_upload` bem-sucedido; `register_candidatura` copia o currículo mais recente da vaga para `candidaturas.documento_id`; o Dashboard lê `anexos` da vaga em `candidatando`. | Onda 3 (G e H) |
| **P6** | Categoria da pendência "sem currículo/carta para anexar". | `sem_documento`. O agente só a usa depois da edição do prompt (Tarefa 14b, bloqueada). | Onda 3 (H) e 14b |
| **P7** | Gerar currículo "para uma variante" muda o **conteúdo** (ex.: só as experiências de `foco_experiencia`)? A spec só fala de `variante_id` e da tag. | Nesta parte, não: a variante fica registrada (campo e tag) e o conteúdo é o mesmo de hoje. Conteúdo por variante vira spec própria. | Onda 1 (A) |

Decisão **derivada da spec, não nova**: `arquivo_html` passa a ser um **nome simples dentro de `documentos/`**, como `arquivo` (seção 3: arquivos "sempre dentro de `<dados>/documentos/`"; seção 11: caminho sempre conferido dentro da pasta). Os currículos novos deixam de ir para `curriculos/`; os antigos são copiados pela migração (nada é apagado).

## Execução em ondas

Regras (CLAUDE.md, "Quem faz cada etapa"): uma preocupação por agente, uma worktree por agente, **arquivos diferentes por frente**, contrato escrito. O orquestrador (Opus) integra depois que todas as frentes da onda terminam, roda de novo as rodadas e só então abre a onda seguinte. Cada agente usa o próprio `CARGO_TARGET_DIR` fora do repo (ex.: `C:/Users/joaog/.cache/claudia-p2-<frente>`) e confere a base da worktree antes de começar (`git log -1` igual ao commit que o orquestrador indicou).

| Onda | Frente | Tarefas | Modelo | Por quê | Arquivos que **só ela** toca |
|---|---|---|---|---|---|
| **0** (sequencial, antes de abrir worktrees) | Contrato em código | 8b | **Sonnet** | vários arquivos (lib.rs, módulos, stubs) | `src-tauri/src/documentos/mod.rs`; `documentos/biblioteca.rs` (novo); `documentos/{pdf,registrar,variantes,migrar,texto}.rs` (novos, vazios/stub); `src-tauri/src/commands/documentos.rs`; `src-tauri/src/lib.rs` |
| **1** | A — PDF | 9, 9b | **Sonnet** | COM/unsafe, async, portão de qualidade | `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `documentos/pdf.rs`, `documentos/registrar.rs`, `commands/curriculos.rs`, `commands/cover_letter.rs`, `src-tauri/src/lib.rs` (só o comando de prova, que entra e sai) |
| **1** | B1 — variante da vaga | 10 | **Sonnet** | lógica nova em 6 arquivos | `documentos/variantes.rs`, `documentos/fonte.rs`, `commands/perfil.rs`, `mcp/tools/vagas.rs`, `mcp/tools/mod.rs`, `mcp/mod.rs`, `mcp/server.rs` |
| **1** | B2 — comandos da biblioteca | 11 | **Sonnet** | segurança do apagar, vários comandos | `documentos/biblioteca.rs`, `documentos/importar.rs`, `documentos/seguranca.rs` (só visibilidade), `commands/documentos.rs`, `src-tauri/src/db/mod.rs` |
| **1** | C — aba Currículos | 13 | **Sonnet** | UI nova, muitos arquivos | `src/**` exceto `src/components/{dashboard,Vagas.tsx,Pendencias.tsx,ActivityLogScreen.tsx,Welcome.tsx}`; `tests/front/` (novo) |
| **2** | D — migração | 12 | **Sonnet** | async, idempotência, nada apagado | `documentos/migrar.rs` |
| **2** | E — MCP | 14 | **Sonnet** | 7 tools, sessão, backup | `mcp/tools/documentos.rs` (novo), `mcp/tools/variantes.rs` (novo), `mcp/tools/mod.rs`, `mcp/tools/profile.rs`, `mcp/mod.rs`, `mcp/server.rs`, `documentos/texto.rs`, `commands/cover_letter.rs` [P3], `src-tauri/tests/mcp-browser-smoke.mjs` |
| **2** | F — acabamento do upload | 14c | **Haiku** | uma tool, regras pequenas e delimitadas | `mcp/tools/browser.rs`, `src-tauri/tests/browser_live.rs` |
| **3** | G — onde aparece (backend) | 15a [P5] | **Sonnet** | schema + tool + trait | `db/schema.sql`, `db/mod.rs`, `db/queries.rs`, `mcp/tools/vagas.rs`, `mcp/tools/browser.rs`, `documentos/fonte.rs` |
| **3** | H — onde aparece (frontend) | 15b | **Sonnet** | 5 telas | `src/App.tsx` (só a prop `onNavigateToDocumentos` das Pendências), `src/components/dashboard/index.tsx`, `src/components/dashboard/types.ts`, `src/components/Vagas.tsx`, `src/components/Pendencias.tsx`, `src/components/ActivityLogScreen.tsx`, `src/components/Welcome.tsx`, `src/i18n/pt.ts`, `src/i18n/en.ts` |
| **3** | I — prompt (**BLOQUEADA**) | 14b | **Haiku** | texto + um teste | `src-tauri/src/prompt_sistema_runtime.md`, `src-tauri/src/commands/prompts.rs` |
| **4** | Orquestrador | 16 | **Opus** (+ Haiku na rodada de agente) | rodadas, revisão, smoke | nenhum de produção |

Ajuste em relação à proposta de partida: **B foi dividida em B1 (Tarefa 10) e B2 (Tarefa 11)**. As duas não compartilham arquivo nenhum (B2 lê as variantes por `commands::perfil::ler_search_variants`, que já existe), então rodam em paralelo; juntas seriam 12 arquivos e duas preocupações num agente só. E entrou uma **Onda 0**: sem ela, A, B1, B2 e D disputariam `lib.rs` e `documentos/mod.rs` (registro de comandos e declaração de módulos). A Onda 0 escreve o contrato **em código** (tipos, stubs registrados, módulos declarados), e cada frente só preenche corpos nos arquivos dela.

### Contrato entre as frentes (copiado da spec, seção 6, com os nomes reais)

**JSON `Documento`** (Rust `documentos::biblioteca::DocumentoView`, TS `Documento` em `src/types.ts`):

```
{ id: number, tipo: "curriculo" | "carta", nome: string, origem: "gerado" | "importado",
  formato: "pdf" | "docx", idioma: string | null, tags: { tipo: "variante" | "template" | "cor" | "idioma" | "livre", valor: string }[],
  tamanho: number, criado_em: string, vaga_id: number | null,
  preferido_global: boolean, preferido_variantes: string[], tem_preview: boolean }
```

`DocumentoDetalhe` = `Documento` + `preview_html: string | null` (Rust: `#[serde(flatten)]`). `VarianteResumo` = `{ id: string, nome_exibicao: string, ativa: boolean }`. Nenhum desses tipos tem `arquivo`, `arquivo_html`, `sha256` ou caminho.

**Comandos Tauri** (nome Rust; chaves do `invoke` em camelCase, como o Tauri converte):

| Comando | Args do `invoke` | Retorno | Frente |
|---|---|---|---|
| `listar_documentos` | `{ tipo }` | `Documento[]` (mais novo primeiro) | 0 (corpo) / B2 (async) |
| `obter_documento` | `{ id }` | `DocumentoDetalhe` | B2 |
| `gerar_curriculo` | `{ templateId, corPrimaria, idioma, varianteId }` | `Documento` | A |
| `gerar_cover_letter` | `{ empresa, cargo, descricaoVaga, notaExtra, idioma, corPrimaria, varianteId }` | `void`; eventos `cover-letter-stream` (string), `cover-letter-done` (**`Documento`**, P1), `cover-letter-error` (string) | A |
| `importar_documento` | `{ tipo, caminhoOrigem, nome }` | `Documento` | 0 / B2 |
| `renomear_documento` | `{ id, nome }` | `void` | B2 |
| `atualizar_tags_documento` | `{ id, tags }` | `void` | B2 |
| `definir_preferido` | `{ id, varianteId }` | `void` | já existe |
| `remover_preferido` | `{ tipo, varianteId }` | `void` | B2 |
| `abrir_documento` | `{ id }` | `void` | B2 |
| `apagar_documento` | `{ id }` | `void` (o aviso de "é preferido" é da UI, pelos campos `preferido_*` do `Documento`) | B2 |
| `listar_variantes` | `{}` | `VarianteResumo[]` | B2 |

`renomear`/`atualizar_tags`/`remover_preferido`/`apagar` devolvem `void` porque a spec não diz o retorno; a UI recarrega com `obter_documento`/`listar_documentos`.

**`SearchVariant` ganha `principal: boolean`** (Rust `#[serde(default)]`, TS em `src/types.ts`). No máximo uma `true`; `guardar_variante_unica` com `principal: true` desmarca as outras (B1).

**Funções Rust compartilhadas** (assinaturas fixas; quem produz → quem consome):

| Função | Produz | Consome |
|---|---|---|
| `biblioteca::open_conn(data_dir: &Path) -> Result<Connection, String>` | 0 | A, B2, D, E |
| `biblioteca::{ver, listar_view}(conn, ...) -> Result<DocumentoView \| Vec<_>, String>` | 0 | A, B2, E |
| `biblioteca::gravar_tags(conn, id, &[Tag]) -> Result<(), String>` | 0 | A, B2 |
| `biblioteca::detalhe(conn, data_dir, id) -> Result<DocumentoDetalhe, String>` | B2 | E |
| `registrar::registrar_gerado(data_dir, NovoGerado, print) -> Result<i64, String>` (async, `print: FnOnce(PathBuf, PathBuf) -> impl Future<Output = Result<(), String>>`) | A | D |
| `pdf::html_to_pdf(app: &AppHandle, html: &Path, pdf: &Path) -> Result<(), String>` (async) | A | D |
| `variantes::{ler, validar, principal, manter_uma_principal}` | B1 | E |
| `migrar::ao_iniciar(app: AppHandle)` (async; stub na Onda 0) | D | lib.rs (Onda 0) |
| `texto::html_para_texto(&str) -> String` | E | E |

**Arquivos em `documentos/`**: `<base>.pdf` e `<base>.html` lado a lado (gerados), `importado-<nanos>.<ext>` (importados), `migrado-<nome antigo>.{html,pdf}` (migração), `envio/<id>/` (cópias para o Chrome, Parte 1), `pendentes/` [P3]. Todo nome gravado no banco é **simples** (sem `/`, `\`, `:`, sem começar com `.`).

## Portões de decisão

- Antes da Onda 1: respostas a **P1, P2, P7**.
- Antes da Onda 2: **P3, P4**; e o **portão técnico do PDF** (Tarefa 9, Step 6) aprovado pelo usuário.
- Antes da Onda 3: **P5, P6**; e autorização explícita para a Tarefa 14b (prompt).

## Global Constraints da Parte 2 (somam-se às da Parte 1)

- Nenhuma tool MCP aceita ou devolve caminho, nome de arquivo ou `cv_gerado_path`.
- Escrita de variante e de preferido pelo MCP só na sessão de Perfil (`SessionKind::Interactive`).
- PDF só pelo WebView2 do app, numa janela invisível, sem capability; **nenhuma janela pode aparecer**.
- Nada nas pastas antigas (`curriculos/`, `cover_letters/`) é apagado ou movido.
- A pré-visualização roda em `<iframe sandbox="">` (sem `allow-scripts`, sem `allow-same-origin`).
- Testes e provas nunca escrevem nos dados reais (`%APPDATA%\io.github.johngabie.claudia-rh`): pasta temporária ou o scratchpad. O app em dev usa os dados reais: a prova do PDF lê um HTML e grava o PDF no scratchpad.

---

### Tarefa 8b (Onda 0): contrato em código

**Files:**
- Create: `src-tauri/src/documentos/biblioteca.rs`
- Create (só uma linha de comentário, ou stub): `src-tauri/src/documentos/{pdf,registrar,variantes,migrar,texto}.rs`
- Modify: `src-tauri/src/documentos/mod.rs`, `src-tauri/src/commands/documentos.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `Tag`, `TAG_TIPOS`, `DocumentoView`, `DocumentoDetalhe`, `VarianteResumo`, `open_conn`, `normalizar_tags`, `gravar_tags`, `ver`, `listar_view` (código abaixo); `migrar::ao_iniciar` (stub); todos os comandos da tabela do contrato registrados em `lib.rs` (stubs onde a frente ainda vai preencher).
- Consumes: `documentos::{obter, listar, Documento}`, `mcp::tools::open_db`.

- [ ] **Step 1: Módulos.** Em `documentos/mod.rs`, trocar o bloco de `pub mod` por:

```rust
pub mod biblioteca;
pub mod escolha;
pub mod fonte;
pub mod importar;
pub mod migrar;
pub mod pdf;
pub mod registrar;
pub mod seguranca;
pub mod texto;
pub mod variantes;
```

Criar `pdf.rs`, `registrar.rs`, `variantes.rs`, `texto.rs` com uma linha (`//! Filled by Part 2, wave 1/2 (see the plan).`). `migrar.rs`:

```rust
//! Old generated CVs and cover letters join the library (Part 2, Task 12). Stub until wave 2.
pub async fn ao_iniciar(_app: tauri::AppHandle) {}
```

- [ ] **Step 2: Testes que falham** (em `biblioteca.rs`, `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{definir_preferido, inserir, NovoDocumento};

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }

    fn doc(c: &Connection, html: Option<&str>) -> i64 {
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "gerado", arquivo: "segredo-123.pdf", arquivo_html: html, formato: "pdf", idioma: Some("pt"), sha256: "abcdef", tamanho: 10, variante_id: Some("backend") }).unwrap()
    }

    #[test]
    fn the_view_reads_the_marks_and_carries_no_file_name() {
        let c = mem();
        let id = doc(&c, Some("segredo-123.html"));
        definir_preferido(&c, id, None).unwrap();
        definir_preferido(&c, id, Some("devops")).unwrap();
        let v = ver(&c, id).unwrap();
        assert!(v.preferido_global);
        assert_eq!(v.preferido_variantes, vec!["devops".to_string()]);
        assert!(v.tem_preview);
        let json = serde_json::to_string(&v).unwrap();
        for leak in ["segredo-123", "arquivo", "sha256", "abcdef"] {
            assert!(!json.contains(leak), "{leak} leaked: {json}");
        }
        assert!(!ver(&c, doc(&c, None)).unwrap().tem_preview, "no HTML, no preview");
    }

    #[test]
    fn tags_are_checked_trimmed_and_deduplicated() {
        let c = mem();
        let id = doc(&c, None);
        let t = |tipo: &str, valor: &str| Tag { tipo: tipo.into(), valor: valor.into() };
        gravar_tags(&c, id, &[t("variante", "backend"), t("variante", " backend "), t("livre", " sênior ")]).unwrap();
        assert_eq!(ver(&c, id).unwrap().tags, vec![t("variante", "backend"), t("livre", "sênior")]);
        assert!(gravar_tags(&c, id, &[t("cor_favorita", "x")]).is_err(), "unknown kind");
        assert!(gravar_tags(&c, id, &[t("livre", "   ")]).is_err(), "empty value");
        assert!(gravar_tags(&c, 999, &[]).is_err(), "unknown document");
        let var: Option<String> = c.query_row("SELECT variante_id FROM documentos WHERE id = ?1", [id], |r| r.get(0)).unwrap();
        assert_eq!(var.as_deref(), Some("backend"), "tags never change the variant field");
    }

    #[test]
    fn open_conn_turns_foreign_keys_on() {
        let dir = std::env::temp_dir().join(format!("claudia-bib-fk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let c = open_conn(&dir).unwrap();
        let on: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(on, 1);
    }
}
```

E em `commands/documentos.rs` (fim do arquivo):

```rust
#[cfg(test)]
mod tests {
    /// The old commands took a path from the UI and deleted or opened it (spec 1.6). They must stay
    /// out of the handler for good.
    #[test]
    fn the_old_path_commands_are_gone() {
        let lib = include_str!("../lib.rs");
        for old in ["listar_curriculos", "abrir_curriculo", "apagar_curriculo", "listar_cover_letters", "abrir_cover_letter", "apagar_cover_letter"] {
            assert!(!lib.contains(old), "{old} took a path from the UI and must stay unregistered");
        }
    }
}
```

- [ ] **Step 3: Rodar e ver falhar.** `cd src-tauri && CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-onda0 cargo test --lib documentos::biblioteca commands::documentos` → FAIL (não compila; depois, `the_old_path_commands_are_gone` falha).

- [ ] **Step 4: Implementar `biblioteca.rs`** (compilado e testado pelo planejador):

```rust
//! The library as the app's screens and the agent's tools see it (spec section 6): no file names, no
//! paths, the user's marks and tags already decoded.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The kinds of tag a document can carry (spec section 2).
pub const TAG_TIPOS: &[&str] = &["variante", "template", "cor", "idioma", "livre"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub tipo: String,
    pub valor: String,
}

/// `Documento` of the spec's contract. Never carries `arquivo`, `arquivo_html`, `sha256` or a path.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentoView {
    pub id: i64,
    pub tipo: String,
    pub nome: String,
    pub origem: String,
    pub formato: String,
    pub idioma: Option<String>,
    pub tags: Vec<Tag>,
    pub tamanho: i64,
    pub criado_em: String,
    pub vaga_id: Option<i64>,
    pub preferido_global: bool,
    pub preferido_variantes: Vec<String>,
    pub tem_preview: bool,
}

/// `obter_documento`: the document plus its preview HTML (generated documents only).
#[derive(Debug, Clone, Serialize)]
pub struct DocumentoDetalhe {
    #[serde(flatten)]
    pub documento: DocumentoView,
    pub preview_html: Option<String>,
}

/// `listar_variantes`: what the variant pickers need.
#[derive(Debug, Clone, Serialize)]
pub struct VarianteResumo {
    pub id: String,
    pub nome_exibicao: String,
    pub ativa: bool,
}

/// Every connection the document code opens: foreign keys ON, so deleting a document also drops its
/// preferred marks (preferidos.documento_id ... ON DELETE CASCADE).
pub fn open_conn(data_dir: &Path) -> Result<Connection, String> {
    let c = crate::mcp::tools::open_db(data_dir)?;
    c.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;
    Ok(c)
}

/// Trims, refuses unknown kinds and empty values, drops duplicates (first one wins).
pub fn normalizar_tags(tags: &[Tag]) -> Result<Vec<Tag>, String> {
    let mut out: Vec<Tag> = Vec::new();
    for t in tags {
        let tipo = t.tipo.trim();
        let valor = t.valor.trim();
        if !TAG_TIPOS.contains(&tipo) {
            return Err(format!("tipo de tag inválido: {tipo} (use {})", TAG_TIPOS.join(", ")));
        }
        if valor.is_empty() {
            return Err("uma tag precisa de um valor".into());
        }
        let tag = Tag { tipo: tipo.to_string(), valor: valor.to_string() };
        if !out.contains(&tag) {
            out.push(tag);
        }
    }
    Ok(out)
}

/// Replaces the document's tags. Tags are labels only: they never change `variante_id`.
pub fn gravar_tags(conn: &Connection, id: i64, tags: &[Tag]) -> Result<(), String> {
    let tags = normalizar_tags(tags)?;
    let json = serde_json::to_string(&tags).map_err(|e| e.to_string())?;
    let n = conn.execute("UPDATE documentos SET tags = ?1 WHERE id = ?2", params![json, id]).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("o documento {id} não existe"));
    }
    Ok(())
}

fn view_de(conn: &Connection, d: super::Documento) -> Result<DocumentoView, String> {
    let mut st = conn
        .prepare("SELECT variante_id FROM preferidos WHERE documento_id = ?1 ORDER BY variante_id")
        .map_err(|e| e.to_string())?;
    let marks: Vec<String> = st
        .query_map([d.id], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    Ok(DocumentoView {
        id: d.id,
        tipo: d.tipo,
        nome: d.nome,
        origem: d.origem,
        formato: d.formato,
        idioma: d.idioma.filter(|s| !s.is_empty()),
        tags: serde_json::from_str(&d.tags).unwrap_or_default(),
        tamanho: d.tamanho,
        criado_em: d.criado_em,
        vaga_id: d.vaga_id,
        preferido_global: marks.iter().any(|m| m.is_empty()),
        preferido_variantes: marks.into_iter().filter(|m| !m.is_empty()).collect(),
        tem_preview: d.arquivo_html.as_deref().is_some_and(|h| !h.is_empty()),
    })
}

pub fn ver(conn: &Connection, id: i64) -> Result<DocumentoView, String> {
    let d = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    view_de(conn, d)
}

pub fn listar_view(conn: &Connection, tipo: &str) -> Result<Vec<DocumentoView>, String> {
    super::listar(conn, tipo).map_err(|e| e.to_string())?.into_iter().map(|d| view_de(conn, d)).collect()
}
```

- [ ] **Step 5: Comandos (stubs com a assinatura final)** — `commands/documentos.rs` inteiro passa a ser:

```rust
//! Tauri commands of the document library (spec section 6). Wave 0 fixes the signatures; wave 1 (B2)
//! fills the bodies marked NOT_YET.
use crate::documentos::biblioteca::{self, DocumentoDetalhe, DocumentoView, Tag, VarianteResumo};
use crate::documentos;
use tauri::{AppHandle, Manager};

const NOT_YET: &str = "não implementado ainda (Parte 2, onda 1)";

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_documentos(app: AppHandle, tipo: String) -> Result<Vec<DocumentoView>, String> {
    biblioteca::listar_view(&biblioteca::open_conn(&data_dir(&app)?)?, &tipo)
}

#[tauri::command]
pub fn obter_documento(_app: AppHandle, _id: i64) -> Result<DocumentoDetalhe, String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn importar_documento(app: AppHandle, tipo: String, caminho_origem: String, nome: Option<String>) -> Result<DocumentoView, String> {
    let dir = data_dir(&app)?;
    let conn = biblioteca::open_conn(&dir)?;
    let d = documentos::importar::importar(&conn, &dir, &tipo, std::path::Path::new(&caminho_origem), nome.as_deref())?;
    biblioteca::ver(&conn, d.id)
}

#[tauri::command]
pub fn renomear_documento(_app: AppHandle, _id: i64, _nome: String) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn atualizar_tags_documento(_app: AppHandle, _id: i64, _tags: Vec<Tag>) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn definir_preferido(app: AppHandle, id: i64, variante_id: Option<String>) -> Result<(), String> {
    documentos::definir_preferido(&biblioteca::open_conn(&data_dir(&app)?)?, id, variante_id.as_deref())
}

#[tauri::command]
pub fn remover_preferido(_app: AppHandle, _tipo: String, _variante_id: Option<String>) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn abrir_documento(_app: AppHandle, _id: i64) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn apagar_documento(_app: AppHandle, _id: i64) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn listar_variantes(_app: AppHandle) -> Result<Vec<VarianteResumo>, String> {
    Err(NOT_YET.into())
}
```

(+ o `mod tests` do Step 2.)

- [ ] **Step 6: `lib.rs`.**
  - Remover `listar_cover_letters, abrir_cover_letter, apagar_cover_letter` da linha `use commands::cover_letter::{...}` (fica só `gerar_cover_letter`) e `listar_curriculos, abrir_curriculo, apagar_curriculo` da linha `use commands::curriculos::{...}` (fica só `gerar_curriculo`); remover as seis entradas do `generate_handler!`. As funções continuam nos arquivos (avisos de código morto) até a frente A apagá-las.
  - Registrar, junto dos três `commands::documentos::*` que já existem: `obter_documento`, `renomear_documento`, `atualizar_tags_documento`, `remover_preferido`, `abrir_documento`, `apagar_documento`, `listar_variantes`.
  - No `setup`, logo depois de `commands::prompts::ensure_all_prompts(&data_dir);`:

```rust
            // Old generated CVs/letters join the document library in the background (never deleted).
            tauri::async_runtime::spawn(documentos::migrar::ao_iniciar(app.handle().clone()));
```

  - `[depende de P3]` em `start_mcp_notify_listener`, um braço novo no `match line.trim()`:

```rust
                    // The MCP wrote a cover letter to documentos/pendentes/: the app makes its PDF.
                    "documentos" => {
                        tauri::async_runtime::spawn(crate::documentos::migrar::ao_iniciar(app.clone()));
                    }
```

- [ ] **Step 7: Rodar e ver passar.** `cargo test --lib` → exit 0 (contagem anterior + 4). `npx tsc --noEmit` → exit 0 (o front ainda não mudou).

- [ ] **Step 8: Mutações** (uma por vez, colar a linha `FAILED`, restaurar):
  - tirar o `PRAGMA foreign_keys=ON` de `open_conn` → `open_conn_turns_foreign_keys_on` FAILED;
  - tirar o `if !TAG_TIPOS.contains` → `tags_are_checked_trimmed_and_deduplicated` FAILED;
  - pôr `apagar_curriculo,` de volta no `generate_handler!` → `the_old_path_commands_are_gone` FAILED.

- [ ] **Step 9: Commit.** `git commit -m "feat(documentos): part 2 contract in code - library view types, tags, FK-on connection, command stubs; old path commands unregistered"`

> Efeito colateral conhecido: entre este commit e a integração da Onda 1, o Perfil → Currículos/Cover letters do app em dev chama comandos que não existem mais. Não rodar o app desta branch para uso real nesse intervalo.

---

### Tarefa 9 (Onda 1, frente A): PDF dos currículos gerados pelo WebView2 do app

Começa por uma prova do caminho. **Se o PDF sair ruim (texto cortado, cores erradas, página errada), parar e voltar ao usuário** antes do Step 7.

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/documentos/pdf.rs` (stub da Onda 0), `src-tauri/src/lib.rs` (só o comando de prova, que entra no Step 3 e sai no Step 7), `src-tauri/src/commands/curriculos.rs` (teste ignorado que gera o HTML de amostra)
- `src-tauri/capabilities/` não muda: a janela invisível não ganha capability (sem IPC).

**Interfaces:**
- Produces: `pub async fn html_to_pdf(app: &tauri::AppHandle, html: &std::path::Path, pdf: &std::path::Path) -> Result<(), String>` em `crate::documentos::pdf` (futuro `Send`, conferido).

- [ ] **Step 1: Dependências.** Em `src-tauri/Cargo.toml`, na seção existente `[target.'cfg(windows)'.dependencies]`, depois de `windows-sys`:

```toml
webview2-com = "0.38"
windows-core = "0.61"
```

Rodar `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-a cargo tree -i webview2-com --depth 1`. Expected: **uma** versão (`v0.38.2`), com `tauri`, `wry` e `claudia-rh` como dependentes. O `Cargo.lock` muda em 2 linhas.

- [ ] **Step 2: `documentos/pdf.rs`** (compilado pelo planejador contra `webview2-com 0.38.2`, `windows-core 0.61.2`, `tauri 2.11.2`):

```rust
//! HTML -> PDF with the app's OWN WebView2 (the Edge engine Tauri already runs on), in a hidden window.
//! No browser of ours, no new runtime: the same HTML the user previews becomes the PDF.
use std::path::Path;
use std::time::Duration;

const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
const PRINT_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn html_to_pdf(app: &tauri::AppHandle, html: &Path, pdf: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::print(app, html, pdf).await
    }
    #[cfg(not(windows))]
    {
        let _ = (app, html, pdf);
        Err("gerar PDF só está disponível no Windows".into())
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tauri::{webview::PageLoadEvent, WebviewUrl, WebviewWindowBuilder};
    use tokio::sync::oneshot;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment6, ICoreWebView2_7};
    use webview2_com::PrintToPdfCompletedHandler;
    use windows_core::{Interface, HSTRING};

    type Slot<T> = Arc<Mutex<Option<oneshot::Sender<T>>>>;

    fn send<T>(slot: &Slot<T>, v: T) {
        if let Some(tx) = slot.lock().ok().and_then(|mut g| g.take()) {
            let _ = tx.send(v);
        }
    }

    pub async fn print(app: &tauri::AppHandle, html: &Path, pdf: &Path) -> Result<(), String> {
        let url = tauri::Url::from_file_path(html).map_err(|_| "caminho do HTML inválido".to_string())?;
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let label = format!("pdf-{nanos}");
        let (loaded_tx, loaded_rx) = oneshot::channel::<()>();
        let loaded: Slot<()> = Arc::new(Mutex::new(Some(loaded_tx)));
        let loaded_cb = loaded.clone();
        // Hidden, A4-ish size, no capability for this label: the page has no IPC to the app.
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
            .visible(false)
            .inner_size(794.0, 1123.0)
            .on_page_load(move |_w, p| {
                if p.event() == PageLoadEvent::Finished {
                    send(&loaded_cb, ());
                }
            })
            .build()
            .map_err(|e| format!("não foi possível abrir a janela de impressão: {e}"))?;

        let result = async {
            tokio::time::timeout(LOAD_TIMEOUT, loaded_rx)
                .await
                .map_err(|_| "o HTML não carregou a tempo".to_string())?
                .map_err(|_| "a janela de impressão fechou".to_string())?;
            let (tx, rx) = oneshot::channel::<Result<(), String>>();
            let slot: Slot<Result<(), String>> = Arc::new(Mutex::new(Some(tx)));
            let out = HSTRING::from(pdf.as_os_str());
            let slot_wv = slot.clone();
            win.with_webview(move |wv| {
                let slot_cb = slot_wv.clone();
                let started: windows_core::Result<()> = unsafe {
                    (|| {
                        let core = wv.controller().CoreWebView2()?;
                        let core7: ICoreWebView2_7 = core.cast()?;
                        let env6: ICoreWebView2Environment6 = wv.environment().cast()?;
                        let s = env6.CreatePrintSettings()?;
                        s.SetShouldPrintBackgrounds(true)?;
                        s.SetShouldPrintHeaderAndFooter(false)?;
                        s.SetPageWidth(8.27)?; // A4, inches
                        s.SetPageHeight(11.69)?;
                        s.SetMarginTop(0.0)?;
                        s.SetMarginBottom(0.0)?;
                        s.SetMarginLeft(0.0)?;
                        s.SetMarginRight(0.0)?;
                        let handler = PrintToPdfCompletedHandler::create(Box::new(move |hr: windows_core::Result<()>, ok: bool| {
                            send(&slot_cb, match (hr, ok) {
                                (Ok(()), true) => Ok(()),
                                (Ok(()), false) => Err("o WebView2 não gerou o PDF".to_string()),
                                (Err(e), _) => Err(format!("erro do WebView2 ao gerar o PDF: {e}")),
                            });
                            Ok(())
                        }));
                        core7.PrintToPdf(&out, &s, &handler)
                    })()
                };
                if let Err(e) = started {
                    send(&slot_wv, Err(format!("não foi possível iniciar a impressão: {e}")));
                }
            })
            .map_err(|e| e.to_string())?;
            tokio::time::timeout(PRINT_TIMEOUT, rx)
                .await
                .map_err(|_| "a geração do PDF demorou demais".to_string())?
                .map_err(|_| "a impressão foi interrompida".to_string())?
        }
        .await;
        let _ = win.destroy();
        result
    }
}
```

Os templates têm `@page { size: A4; margin: 18mm 20mm; }` (carta: `22mm 26mm`). **Não verificado** se o WebView2 respeita a margem do CSS quando as margens do `PrintSettings` são 0: o portão do Step 6 decide. Se a página sair sem margem, trocar as quatro `SetMargin*` pelos valores do template em polegadas (18 mm = 0,709; 20 mm = 0,787) e repetir a prova. **Não verificado** também se `WebviewUrl::External` aceita `file://` em todas as instalações; se a janela não carregar, a alternativa é `NavigateToString` com o HTML (limite de 2 MB, folgado para um CV) — voltar ao orquestrador antes de trocar.

- [ ] **Step 3: Comando de prova (temporário, sai no Step 7).** Em `lib.rs`, perto dos outros comandos:

```rust
/// Part 2 proof only: prints an HTML file to a PDF with the app's WebView2. Debug builds only.
#[cfg(debug_assertions)]
#[tauri::command]
async fn debug_pdf_prova(app: tauri::AppHandle, html: String, pdf: String) -> Result<(), String> {
    crate::documentos::pdf::html_to_pdf(&app, std::path::Path::new(&html), std::path::Path::new(&pdf)).await
}
```

e no `generate_handler!`: `#[cfg(debug_assertions)] debug_pdf_prova,` (o macro aceita atributo por item).

E um teste ignorado em `commands/curriculos.rs` (no `mod tests` existente, que já tem `perfil_simples()`), para ter um HTML realista **sem tocar nos dados do usuário**:

```rust
    /// Writes a sample CV (fake profile) for the PDF proof. Run by hand:
    /// CLAUDIA_PDF_PROVA_DIR=<scratchpad> cargo test --lib write_sample_cv_for_the_pdf_proof -- --ignored
    #[test]
    #[ignore]
    fn write_sample_cv_for_the_pdf_proof() {
        let dir = std::path::PathBuf::from(std::env::var("CLAUDIA_PDF_PROVA_DIR").expect("set CLAUDIA_PDF_PROVA_DIR"));
        std::fs::create_dir_all(&dir).unwrap();
        for (id, html) in [
            ("classic-ats", template_classic_ats(&perfil_simples(), "#D97757", pick_lang("pt"))),
            ("hybrid-skills", template_hybrid_skills(&perfil_simples(), "#2E6DB4", pick_lang("en"))),
            ("dev-compact", template_dev_compact(&perfil_simples(), "#3A7D44", pick_lang("pt"))),
        ] {
            std::fs::write(dir.join(format!("amostra-{id}.html")), html).unwrap();
        }
    }
```

- [ ] **Step 4: Compilar.** `cd src-tauri && CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-a cargo build` → exit 0.

- [ ] **Step 5: Prova no app rodando** (skill `claudia-teste`; **pedir ao usuário antes** se o app dele estiver aberto sem CDP — `subir.mjs` responde `perguntar`):
  1. `CLAUDIA_PDF_PROVA_DIR=<scratchpad>\pdf-prova cargo test --lib write_sample_cv_for_the_pdf_proof -- --ignored` → três `amostra-*.html`.
  2. Para cada um: `node .claude/skills/claudia-teste/scripts/invocar.mjs debug_pdf_prova '{"html":"<scratchpad>\\pdf-prova\\amostra-classic-ats.html","pdf":"<scratchpad>\\pdf-prova\\amostra-classic-ats.pdf"}' --escrever` (a escrita é só no scratchpad).
  3. Conferir: o arquivo começa com `%PDF-`; nenhuma janela apareceu (`ver.mjs tela` antes e depois, e um print da área de trabalho); abrir o PDF e tirar print da página 1 (e 2, se houver) ao lado do HTML aberto no Edge.
  4. Opcional, só leitura: se existir `%APPDATA%\io.github.johngabie.claudia-rh\curriculos\cv_*.html`, repetir com ele como `html` e o PDF no scratchpad (o HTML do usuário não é alterado).

- [ ] **Step 6: Portão técnico.** Mostrar ao usuário os prints lado a lado (margens, cores, quebras de página, acentos). Se a qualidade não servir, **parar** e decidir com ele antes do Step 7. Anotar no ledger a resposta.

- [ ] **Step 7: Tirar o comando de prova.** Remover `debug_pdf_prova` e a entrada `#[cfg(debug_assertions)] debug_pdf_prova,` de `lib.rs` (o teste ignorado de amostra fica: serve para repetir a prova). `git diff <base da onda> -- src-tauri/src/lib.rs` → vazio.

- [ ] **Step 8: Commit.** `git commit -m "feat(pdf): HTML to PDF with the app's own WebView2 in a hidden window (proof approved by the user)"`

---

### Tarefa 9b (Onda 1, frente A): `gerar_curriculo` e `gerar_cover_letter` registram o documento

**Files:**
- Modify: `src-tauri/src/documentos/registrar.rs` (stub da Onda 0), `src-tauri/src/commands/curriculos.rs`, `src-tauri/src/commands/cover_letter.rs`

**Interfaces:**
- Consumes: `biblioteca::{open_conn, gravar_tags, ver, Tag, DocumentoView}`, `documentos::{docs_dir, inserir, NovoDocumento}`, `pdf::html_to_pdf`
- Produces:
  - `pub struct NovoGerado { pub tipo: String, pub nome: String, pub base: String, pub html: String, pub idioma: Option<String>, pub variante_id: Option<String>, pub vaga_id: Option<i64>, pub tags: Vec<Tag> }`
  - `pub async fn registrar_gerado<F, Fut>(data_dir: &Path, novo: NovoGerado, print: F) -> Result<i64, String> where F: FnOnce(PathBuf, PathBuf) -> Fut, Fut: Future<Output = Result<(), String>>`
  - `gerar_curriculo(app, template_id: String, cor_primaria: Option<String>, idioma: Option<String>, variante_id: Option<String>) -> Result<DocumentoView, String>` (async)
  - `gerar_cover_letter(app, empresa, cargo, descricao_vaga: Option<String>, nota_extra: Option<String>, idioma: Option<String>, cor_primaria: Option<String>, variante_id: Option<String>)` → `cover-letter-done` com `DocumentoView` **[depende de P1]**

- [ ] **Step 1: Testes que falham** em `registrar.rs` (compilados e passando no rascunho do planejador):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-reg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        dir
    }

    fn novo(base: &str) -> NovoGerado {
        NovoGerado {
            tipo: "curriculo".into(),
            nome: "Classic ATS (pt)".into(),
            base: base.into(),
            html: "<html><body>CV</body></html>".into(),
            idioma: Some("pt".into()),
            variante_id: Some("backend".into()),
            vaga_id: None,
            tags: vec![Tag { tipo: "template".into(), valor: "classic-ats".into() }],
        }
    }

    fn count(dir: &Path) -> i64 {
        rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap().query_row("SELECT COUNT(*) FROM documentos", [], |r| r.get(0)).unwrap()
    }

    #[tokio::test]
    async fn a_generated_document_gets_its_html_and_pdf_side_by_side() {
        let dir = setup("ok");
        let id = registrar_gerado(&dir, novo("cv_a"), |_h, p| async move { std::fs::write(p, b"%PDF-1.7 fake").map_err(|e| e.to_string()) }).await.unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let d = crate::documentos::obter(&c, id).unwrap().unwrap();
        assert_eq!((d.arquivo.as_str(), d.arquivo_html.as_deref(), d.origem.as_str()), ("cv_a.pdf", Some("cv_a.html"), "gerado"));
        assert_eq!(d.variante_id.as_deref(), Some("backend"));
        assert_eq!(crate::documentos::biblioteca::ver(&c, id).unwrap().tags.len(), 1);
        crate::documentos::seguranca::resolve_attachable(&c, &dir, id).expect("the PDF is attachable as registered");
    }

    #[tokio::test]
    async fn a_failed_pdf_registers_nothing_and_leaves_no_files() {
        let dir = setup("fail");
        let e = registrar_gerado(&dir, novo("cv_b"), |_h, _p| async { Err::<(), String>("boom".into()) }).await.unwrap_err();
        assert!(e.contains("boom"), "{e}");
        assert_eq!(count(&dir), 0);
        let docs = crate::documentos::docs_dir(&dir);
        assert!(!docs.join("cv_b.html").exists() && !docs.join("cv_b.pdf").exists());
    }

    #[tokio::test]
    async fn an_invalid_pdf_is_refused() {
        let dir = setup("invalid");
        let e = registrar_gerado(&dir, novo("cv_c"), |_h, p| async move { std::fs::write(p, b"<html>").map_err(|e| e.to_string()) }).await.unwrap_err();
        assert!(e.contains("inválido"), "{e}");
        assert_eq!(count(&dir), 0);
    }

    #[tokio::test]
    async fn a_name_with_a_path_is_refused() {
        let dir = setup("path");
        for bad in ["../cv", "..\\cv", "C:cv", "sub/cv", ""] {
            assert!(registrar_gerado(&dir, novo(bad), |_h, _p| async { Ok(()) }).await.is_err(), "{bad}");
        }
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.** `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-a cargo test --lib documentos::registrar` → FAIL (não compila).

- [ ] **Step 3: Implementar `registrar.rs`:**

```rust
//! A document the app generated (CV or cover letter): the preview HTML and the attachable PDF live side
//! by side in <app data>/documentos/ under bare names, and the row exists only once the PDF is real.
use super::biblioteca::{self, Tag};
use sha2::Digest;
use std::future::Future;
use std::path::{Path, PathBuf};

pub struct NovoGerado {
    pub tipo: String,
    pub nome: String,
    /// Bare file stem, e.g. "cv_classic-ats_2026-10-04_10-00-00": the files become <base>.html and <base>.pdf.
    pub base: String,
    pub html: String,
    pub idioma: Option<String>,
    pub variante_id: Option<String>,
    pub vaga_id: Option<i64>,
    pub tags: Vec<Tag>,
}

fn base_valida(base: &str) -> bool {
    !base.is_empty() && !base.contains(['/', '\\', ':']) && !base.starts_with('.')
}

/// Writes the HTML, asks `print` for the PDF, checks it, and only then registers the document.
/// Any failure leaves no row and no file behind. No database handle is held across the await, so the
/// future stays `Send` (Tauri async commands need that).
pub async fn registrar_gerado<F, Fut>(data_dir: &Path, novo: NovoGerado, print: F) -> Result<i64, String>
where
    F: FnOnce(PathBuf, PathBuf) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    if !base_valida(&novo.base) {
        return Err("nome de arquivo inválido".into());
    }
    let dir = super::docs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let html_name = format!("{}.html", novo.base);
    let pdf_name = format!("{}.pdf", novo.base);
    let html_path = dir.join(&html_name);
    let pdf_path = dir.join(&pdf_name);
    if html_path.exists() || pdf_path.exists() {
        return Err("já existe um documento com esse nome; tente de novo".into());
    }
    let limpar = || {
        let _ = std::fs::remove_file(&html_path);
        let _ = std::fs::remove_file(&pdf_path);
    };
    if let Err(e) = std::fs::write(&html_path, &novo.html) {
        limpar();
        return Err(e.to_string());
    }
    let printed = print(html_path.clone(), pdf_path.clone()).await;
    let bytes = match printed.and_then(|_| std::fs::read(&pdf_path).map_err(|_| "o PDF não foi gravado".to_string())) {
        Ok(b) if b.starts_with(b"%PDF-") => b,
        Ok(_) => {
            limpar();
            return Err("o PDF gerado é inválido".into());
        }
        Err(e) => {
            limpar();
            return Err(e);
        }
    };
    let sha = format!("{:x}", sha2::Sha256::digest(&bytes));
    let r = (|| -> Result<i64, String> {
        let mut conn = biblioteca::open_conn(data_dir)?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let id = super::inserir(
            &tx,
            &super::NovoDocumento {
                tipo: &novo.tipo,
                nome: &novo.nome,
                origem: "gerado",
                arquivo: &pdf_name,
                arquivo_html: Some(&html_name),
                formato: "pdf",
                idioma: novo.idioma.as_deref(),
                sha256: &sha,
                tamanho: bytes.len() as i64,
                variante_id: novo.variante_id.as_deref(),
            },
        )
        .map_err(|e| e.to_string())?;
        if let Some(v) = novo.vaga_id {
            tx.execute("UPDATE documentos SET vaga_id = ?1 WHERE id = ?2", rusqlite::params![v, id]).map_err(|e| e.to_string())?;
        }
        biblioteca::gravar_tags(&tx, id, &novo.tags)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    })();
    if r.is_err() {
        limpar();
    }
    r
}
```

- [ ] **Step 4: Rodar e ver passar.** `cargo test --lib documentos::registrar` → 4 passed.

- [ ] **Step 5: Mutações:** tirar os dois `limpar();` dos braços de erro do PDF → `a_failed_pdf_registers_nothing_and_leaves_no_files` FAILED; trocar `Ok(b) if b.starts_with(b"%PDF-") => b,` por `Ok(b) => b,` (e apagar o braço `Ok(_)`) → `an_invalid_pdf_is_refused` FAILED.

- [ ] **Step 6: `gerar_curriculo`** em `commands/curriculos.rs` — substituir o comando inteiro e **apagar** `listar_curriculos`, `abrir_curriculo`, `apagar_curriculo` e o struct `CurriculoInfo` (e o `use tauri_plugin_opener::OpenerExt;` se ficar sem uso):

```rust
fn template_nome(template_id: &str) -> &'static str {
    match template_id {
        "classic-ats" => "Classic ATS",
        "hybrid-skills" => "Hybrid Skills",
        _ => "Dev Compact",
    }
}

#[tauri::command]
pub async fn gerar_curriculo(
    app: AppHandle,
    template_id: String,
    cor_primaria: Option<String>,
    idioma: Option<String>,
    variante_id: Option<String>,
) -> Result<crate::documentos::biblioteca::DocumentoView, String> {
    use crate::documentos::{biblioteca, registrar};
    let raw = parse_candidato_base_interno(&app)?;
    let idioma_str = idioma.unwrap_or_else(|| "pt".to_string());
    let color = cor_primaria.unwrap_or_else(|| "#D97757".to_string());
    // The English translation shells out to the claude CLI: keep it off the async workers.
    let data = if idioma_str == "en" {
        tauri::async_runtime::spawn_blocking(move || traduzir_conteudo_en(&raw)).await.map_err(|e| e.to_string())?
    } else {
        raw
    };
    let lang = pick_lang(&idioma_str);
    let html = match template_id.as_str() {
        "classic-ats" => template_classic_ats(&data, &color, lang),
        "hybrid-skills" => template_hybrid_skills(&data, &color, lang),
        "dev-compact" => template_dev_compact(&data, &color, lang),
        _ => return Err(format!("Template desconhecido: {template_id}")),
    };
    let mut tags = vec![
        biblioteca::Tag { tipo: "template".into(), valor: template_id.clone() },
        biblioteca::Tag { tipo: "cor".into(), valor: color.clone() },
        biblioteca::Tag { tipo: "idioma".into(), valor: idioma_str.clone() },
    ];
    if let Some(v) = variante_id.as_deref().filter(|v| !v.is_empty()) {
        tags.insert(0, biblioteca::Tag { tipo: "variante".into(), valor: v.to_string() });
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let base = format!("cv_{template_id}_{}", Local::now().format("%Y-%m-%d_%H-%M-%S"));
    let novo = registrar::NovoGerado {
        tipo: "curriculo".into(),
        nome: format!("{} ({})", template_nome(&template_id), idioma_str.to_uppercase()),
        base,
        html,
        idioma: Some(idioma_str),
        variante_id: variante_id.filter(|v| !v.is_empty()),
        vaga_id: None,
        tags,
    };
    let printer = app.clone();
    let id = registrar::registrar_gerado(&data_dir, novo, move |h, p| async move {
        crate::documentos::pdf::html_to_pdf(&printer, &h, &p).await
    })
    .await?;
    biblioteca::ver(&biblioteca::open_conn(&data_dir)?, id)
}
```

`[depende de P7]`: se o usuário quiser conteúdo por variante, **parar** — vira spec própria.

- [ ] **Step 7: `gerar_cover_letter`** `[depende de P1]` em `commands/cover_letter.rs` — apagar `listar_cover_letters`, `abrir_cover_letter`, `apagar_cover_letter` e `CoverLetterInfo`. O comando ganha `variante_id: Option<String>` (último parâmetro) e o passa para `spawn_cover_letter_claude`. Dentro da thread, **depois** de `let html = build_cover_letter_html(...)`, trocar todo o trecho "Save file" e o `emit("cover-letter-done", info)` por:

```rust
        let data_dir = match app.path().app_data_dir() {
            Ok(d) => d,
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Error getting data directory: {e}"));
                return;
            }
        };
        let empresa_slug: String = empresa
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .chars()
            .take(30)
            .collect();
        let empresa_slug = if empresa_slug.is_empty() { "empresa".to_string() } else { empresa_slug };
        let mut tags = vec![
            crate::documentos::biblioteca::Tag { tipo: "cor".into(), valor: cor_primaria.clone() },
            crate::documentos::biblioteca::Tag { tipo: "idioma".into(), valor: idioma.clone() },
        ];
        if let Some(v) = variante_id.as_deref().filter(|v| !v.is_empty()) {
            tags.insert(0, crate::documentos::biblioteca::Tag { tipo: "variante".into(), valor: v.to_string() });
        }
        let novo = crate::documentos::registrar::NovoGerado {
            tipo: "carta".into(),
            nome: if cargo.is_empty() { format!("Carta {empresa}") } else { format!("Carta {empresa} – {cargo}") },
            base: format!("cl_{empresa_slug}_{}", Local::now().format("%Y-%m-%d_%H-%M-%S")),
            html,
            idioma: Some(idioma.clone()),
            variante_id: variante_id.filter(|v| !v.is_empty()),
            vaga_id: None,
            tags,
        };
        let printer = app.clone();
        // This is a plain std thread (the CLI stream is blocking): drive the async print to completion here.
        let registered = tauri::async_runtime::block_on(crate::documentos::registrar::registrar_gerado(&data_dir, novo, move |h, p| async move {
            crate::documentos::pdf::html_to_pdf(&printer, &h, &p).await
        }));
        let view = registered.and_then(|id| crate::documentos::biblioteca::ver(&crate::documentos::biblioteca::open_conn(&data_dir)?, id));
        match view {
            Ok(doc) => {
                let _ = app.emit("cover-letter-done", doc);
            }
            Err(e) => {
                let _ = app.emit("cover-letter-error", format!("Erro ao salvar a carta: {e}"));
            }
        }
```

`spawn_cover_letter_claude` ganha o parâmetro `variante_id: Option<String>`.

- [ ] **Step 8: Verificar.** `cargo test --lib` → exit 0; `cargo clippy --lib` sem aviso novo de código morto nos dois arquivos; `cargo build` → exit 0. Smoke no app **só com dados de teste não é possível** (o dev usa os dados reais): fica para a Tarefa 16 com autorização; aqui, registrar "não verificado no app".

- [ ] **Step 9: Commit.** `git commit -m "feat(documentos): generated CVs and cover letters become library documents (HTML preview + PDF, automatic tags); old path commands deleted"`

---

### Tarefa 10 (Onda 1, frente B1): variante da vaga escolhida pelo agente + variante principal

**Files:**
- Modify: `src-tauri/src/documentos/variantes.rs` (stub), `src-tauri/src/documentos/fonte.rs`, `src-tauri/src/commands/perfil.rs`, `src-tauri/src/mcp/tools/vagas.rs`, `src-tauri/src/mcp/tools/mod.rs`, `src-tauri/src/mcp/mod.rs`, `src-tauri/src/mcp/server.rs`

**Interfaces:**
- Produces:
  - `SearchVariant.principal: bool` (`#[serde(default)]`)
  - `variantes::ler(data_dir: &Path) -> Result<SearchVariants, String>` (arquivo ausente → vazio)
  - `variantes::validar(data_dir: &Path, id: &str) -> Result<(), String>`
  - `variantes::principal(data_dir: &Path) -> Option<String>`; `variantes::principal_de(sv: &SearchVariants) -> Option<String>`
  - `variantes::manter_uma_principal(sv: &mut SearchVariants, marcada: &str)`; `variantes::checar_uma_principal(sv: &SearchVariants) -> Result<(), String>`
  - `register_vaga(..., fonte_conexao: Option<&str>, variante_id: Option<&str>)`; `update_vaga_status(data_dir, vaga_id, status, detalhe, variante_id: Option<&str>)`; `set_vaga_variante(data_dir: &Path, vaga_id: i64, variante_id: &str) -> Result<String, String>`
  - Tool MCP `set_vaga_variante { vaga_id, variante_id }`; `register_vaga`/`update_vaga_status` aceitam `variante_id`.
- Consumes: `commands::perfil::{SearchVariant, SearchVariants}`.

- [ ] **Step 1: Testes que falham — `variantes.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn dir_with(yaml: &str) -> std::path::PathBuf {
        let d = crate::mcp::tools::test_support::temp_dir("variantes");
        std::fs::write(d.join("search_variants.yaml"), yaml).unwrap();
        d
    }

    const DUAS: &str = "variantes:\n  - id: backend\n    nome_exibicao: Backend\n  - id: devops\n    nome_exibicao: DevOps\n    principal: true\n";

    #[test]
    fn an_unknown_variant_is_refused_with_the_valid_ones() {
        let d = dir_with(DUAS);
        validar(&d, "backend").unwrap();
        let e = validar(&d, "frontend").unwrap_err();
        assert!(e.contains("frontend") && e.contains("backend") && e.contains("devops"), "{e}");
    }

    #[test]
    fn the_principal_is_read_and_a_missing_file_has_none() {
        assert_eq!(principal(&dir_with(DUAS)).as_deref(), Some("devops"));
        assert_eq!(principal(&crate::mcp::tools::test_support::temp_dir("sem-arquivo")), None);
    }

    #[test]
    fn marking_a_principal_unmarks_the_others() {
        let mut sv: SearchVariants = serde_yaml::from_str(DUAS).unwrap();
        manter_uma_principal(&mut sv, "backend");
        assert_eq!(principal_de(&sv).as_deref(), Some("backend"));
        assert_eq!(sv.variantes.iter().filter(|v| v.principal).count(), 1);
        sv.variantes[1].principal = true;
        assert!(checar_uma_principal(&sv).is_err(), "two principals are refused");
    }

    #[test]
    fn principal_survives_a_save_and_old_files_default_to_false() {
        let sv: SearchVariants = serde_yaml::from_str(DUAS).unwrap();
        let back: SearchVariants = serde_yaml::from_str(&serde_yaml::to_string(&sv).unwrap()).unwrap();
        assert!(back.variantes[1].principal && !back.variantes[0].principal);
    }
}
```

- [ ] **Step 2: Testes que falham — `vagas.rs`** (atualizar as chamadas existentes de `register_vaga`/`update_vaga_status` com o `None` novo; `test_support::seed_db` ganha `ALTER TABLE vagas ADD COLUMN variante_id TEXT;` logo depois do `fonte_conexao`):

```rust
    fn with_variants(dir: &std::path::Path) {
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n  - id: backend\n  - id: devops\n").unwrap();
    }

    fn variante_of(dir: &std::path::Path, id: i64) -> Option<String> {
        open_db(dir).unwrap().query_row("SELECT variante_id FROM vagas WHERE id = ?1", [id], |r| r.get(0)).unwrap()
    }

    #[test]
    fn the_agent_records_the_vagas_variant_three_ways() {
        let dir = temp_dir("vaga-var");
        seed_db(&dir);
        with_variants(&dir);
        register_vaga(&dir, "SRE", "Initech", "linkedin", "https://x/9", None, None, None, Some("devops")).unwrap();
        let id: i64 = open_db(&dir).unwrap().query_row("SELECT id FROM vagas WHERE url='https://x/9'", [], |r| r.get(0)).unwrap();
        assert_eq!(variante_of(&dir, id).as_deref(), Some("devops"));
        update_vaga_status(&dir, 1, "analisada", Some("ok"), Some("backend")).unwrap();
        assert_eq!(variante_of(&dir, 1).as_deref(), Some("backend"));
        set_vaga_variante(&dir, 1, "devops").unwrap();
        assert_eq!(variante_of(&dir, 1).as_deref(), Some("devops"));
    }

    #[test]
    fn an_unknown_variant_is_refused_and_nothing_is_written() {
        let dir = temp_dir("vaga-var-bad");
        seed_db(&dir);
        with_variants(&dir);
        assert!(register_vaga(&dir, "X", "Y", "l", "https://x/10", None, None, None, Some("frontend")).is_err());
        let n: i64 = open_db(&dir).unwrap().query_row("SELECT COUNT(*) FROM vagas WHERE url='https://x/10'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "no vaga with an invented variant");
        assert!(update_vaga_status(&dir, 1, "analisada", None, Some("frontend")).is_err());
        let st: String = open_db(&dir).unwrap().query_row("SELECT status FROM vagas WHERE id=1", [], |r| r.get(0)).unwrap();
        assert_eq!(st, "descoberta", "the status did not move either");
        assert!(set_vaga_variante(&dir, 1, "frontend").is_err());
        assert!(set_vaga_variante(&dir, 999, "backend").is_err(), "unknown vaga");
        assert_eq!(variante_of(&dir, 1), None);
    }
```

- [ ] **Step 3: Testes que falham — `fonte.rs`** (o teste `picks_by_the_vagas_variant_and_resolves_safely` passa a comparar com `starts_with("preferido da variante")`, porque a regra agora diz a variante):

```rust
    #[test]
    fn without_a_vaga_variant_the_principal_is_used() {
        let dir = setup("principal");
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n  - id: backend\n  - id: devops\n    principal: true\n").unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        c.execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status) VALUES (8, 't', 'e', 'p', 'https://x/8', datetime('now'), 'candidatando')", []).unwrap();
        let src = dir.join("ops.pdf");
        std::fs::write(&src, b"%PDF-1.7 ops").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, Some("Ops")).unwrap();
        crate::documentos::definir_preferido(&c, d.id, Some("devops")).unwrap();
        let a = DbDocumentSource { data_dir: dir.clone() }.for_upload(8, "curriculo", None).unwrap();
        assert_eq!(a.id, d.id);
        assert!(a.regra.contains("preferido da variante") && a.regra.contains("devops") && a.regra.contains("principal"), "{}", a.regra);
    }

    #[test]
    fn the_vagas_own_variant_beats_the_principal_and_the_result_says_so() {
        let dir = setup("own");
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n  - id: backend\n  - id: devops\n    principal: true\n").unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("be.pdf");
        std::fs::write(&src, b"%PDF-1.7 be").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, Some("BE")).unwrap();
        crate::documentos::definir_preferido(&c, d.id, Some("backend")).unwrap();
        let a = DbDocumentSource { data_dir: dir.clone() }.for_upload(7, "curriculo", None).unwrap();
        assert!(a.regra.contains("backend") && a.regra.contains("gravada na vaga"), "{}", a.regra);
    }

    #[test]
    fn no_variant_anywhere_is_said_too() {
        let dir = setup("none");
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        c.execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status) VALUES (9, 't', 'e', 'p', 'https://x/9', datetime('now'), 'candidatando')", []).unwrap();
        let src = dir.join("g.pdf");
        std::fs::write(&src, b"%PDF-1.7 g").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, None).unwrap();
        crate::documentos::definir_preferido(&c, d.id, None).unwrap();
        let a = DbDocumentSource { data_dir: dir }.for_upload(9, "curriculo", None).unwrap();
        assert!(a.regra.contains("preferido geral") && a.regra.contains("sem variante"), "{}", a.regra);
    }
```

- [ ] **Step 4: Teste de `dispatch`** (em `mcp/mod.rs`, `dispatch_tests`):

```rust
    #[test]
    fn set_vaga_variante_is_reachable_through_dispatch() {
        let dir = temp_dir("dispatch-var");
        seed_db(&dir);
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n  - id: backend\n").unwrap();
        let out = dispatch(&cfg_for(&dir), "set_vaga_variante", &serde_json::json!({ "vaga_id": 1, "variante_id": "backend" })).unwrap();
        assert!(out.contains("backend"), "{out}");
    }
```

- [ ] **Step 5: Rodar e ver falhar.** `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-b1 cargo test --lib variantes vagas fonte dispatch` → FAIL.

- [ ] **Step 6: Implementar.**

`commands/perfil.rs`, no struct `SearchVariant`, depois de `ativa`:

```rust
    /// The user's main variant: the default for a vaga the agent gave no variant to. At most one.
    #[serde(default)] pub principal: bool,
```

e em `guardar_variante_unica`, antes de serializar: `if sv.variantes.iter().any(|v| v.id == id_salvo && v.principal) { crate::documentos::variantes::manter_uma_principal(&mut sv, &id_salvo); }` (guardar `let id_salvo = variante.id.clone();` no início); em `guardar_search_variants`, antes de serializar: `crate::documentos::variantes::checar_uma_principal(&variantes)?;`.

`documentos/variantes.rs`:

```rust
//! The search variants as the document library and the vaga tools need them (spec 2.1): the agent
//! records a vaga's variant, validated against search_variants.yaml; the user's "principal" variant is
//! the default when the agent recorded none.
use crate::commands::perfil::SearchVariants;
use std::path::Path;

pub fn ler(data_dir: &Path) -> Result<SearchVariants, String> {
    match std::fs::read_to_string(data_dir.join("search_variants.yaml")) {
        Ok(s) => serde_yaml::from_str(&s).map_err(|e| format!("search_variants.yaml inválido: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SearchVariants::default()),
        Err(e) => Err(format!("erro ao ler search_variants.yaml: {e}")),
    }
}

pub fn validar(data_dir: &Path, id: &str) -> Result<(), String> {
    let sv = ler(data_dir)?;
    if sv.variantes.iter().any(|v| v.id == id) {
        return Ok(());
    }
    let ids: Vec<&str> = sv.variantes.iter().map(|v| v.id.as_str()).collect();
    Err(format!("variante desconhecida: {id}. Variantes existentes: {}", if ids.is_empty() { "nenhuma".to_string() } else { ids.join(", ") }))
}

/// The first variant marked principal (the save paths keep it to one).
pub fn principal_de(sv: &SearchVariants) -> Option<String> {
    sv.variantes.iter().find(|v| v.principal).map(|v| v.id.clone())
}

pub fn principal(data_dir: &Path) -> Option<String> {
    ler(data_dir).ok().as_ref().and_then(principal_de)
}

pub fn manter_uma_principal(sv: &mut SearchVariants, marcada: &str) {
    for v in sv.variantes.iter_mut() {
        v.principal = v.id == marcada;
    }
}

pub fn checar_uma_principal(sv: &SearchVariants) -> Result<(), String> {
    if sv.variantes.iter().filter(|v| v.principal).count() > 1 {
        return Err("só uma variante pode ser a principal".into());
    }
    Ok(())
}
```

`mcp/tools/vagas.rs`: `register_vaga` ganha `variante_id: Option<&str>` como último parâmetro; antes de abrir o banco, `if let Some(v) = variante_id { crate::documentos::variantes::validar(data_dir, v)?; }`; o `INSERT` inclui a coluna `variante_id` (`?8`). `update_vaga_status` ganha `variante_id: Option<&str>` como último parâmetro: valida **antes** de qualquer escrita; depois do `UPDATE` de status bem-sucedido, `if let Some(v) = variante_id { conn.execute("UPDATE vagas SET variante_id = ?1 WHERE id = ?2", params![v, vaga_id]) }`. Nova:

```rust
/// Records which variant the agent chose for a vaga (spec 2.1). Validated against search_variants.yaml.
pub fn set_vaga_variante(data_dir: &Path, vaga_id: i64, variante_id: &str) -> Result<String, String> {
    crate::documentos::variantes::validar(data_dir, variante_id)?;
    let conn = open_db(data_dir)?;
    let n = conn
        .execute("UPDATE vagas SET variante_id = ?1 WHERE id = ?2", rusqlite::params![variante_id, vaga_id])
        .map_err(|e| format!("erro ao gravar a variante: {e}"))?;
    if n == 0 {
        return Err(format!("Vaga {vaga_id} não encontrada."));
    }
    Ok(format!("Vaga {vaga_id}: variante {variante_id}."))
}
```

`mcp/tools/mod.rs`: exportar `set_vaga_variante` no `pub use vagas::{...}`. `mcp/mod.rs` `dispatch`: passar `args["variante_id"].as_str()` a `register_vaga` e `update_vaga_status`; braço novo:

```rust
        "set_vaga_variante" => {
            let vaga_id = args["vaga_id"].as_i64().ok_or("parâmetro 'vaga_id' em falta")?;
            let variante = args["variante_id"].as_str().ok_or("parâmetro 'variante_id' em falta")?;
            tools::set_vaga_variante(&cfg.data_dir, vaga_id, variante).inspect(|_| notify(cfg, "db"))
        }
```

`mcp/server.rs`: `RegisterVagaArgs` e `UpdateVagaStatusArgs` ganham `/// id da variante (de get_search_variants) que melhor serve esta vaga; omita se nenhuma serve\n variante_id: Option<String>,` e passam `"variante_id": args.variante_id` no `json!`. Novo:

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SetVagaVarianteArgs {
    /// ID da vaga
    vaga_id: i64,
    /// id da variante (de get_search_variants) que melhor serve esta vaga
    variante_id: String,
}
```

```rust
    #[tool(
        description = "Grava qual variante do candidato serve esta vaga (o id de get_search_variants). É ela que decide qual currículo browser_upload anexa. Sem variante gravada, vale a variante principal do usuário."
    )]
    async fn set_vaga_variante(&self, Parameters(a): Parameters<SetVagaVarianteArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.call("set_vaga_variante", serde_json::json!({ "vaga_id": a.vaga_id, "variante_id": a.variante_id }))
    }
```

`documentos/fonte.rs`, no braço `None =>`, trocar a leitura da variante por:

```rust
                let gravada: Option<String> = conn
                    .query_row("SELECT variante_id FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0))
                    .ok()
                    .flatten()
                    .filter(|v: &String| !v.is_empty());
                let (variante, de_onde) = match gravada {
                    Some(v) => (Some(v), "gravada na vaga"),
                    None => match super::variantes::principal(&self.data_dir) {
                        Some(p) => (Some(p), "a principal"),
                        None => (None, ""),
                    },
                };
                let e = super::escolha::escolher(&conn, tipo, variante.as_deref()).ok_or_else(|| {
                    format!("não há {tipo} para anexar: marque um como preferido no app (aba Currículos) ou gere um para a variante da vaga")
                })?;
                let regra = match &variante {
                    Some(v) => format!("{} (variante {v}, {de_onde})", e.regra),
                    None => format!("{} (sem variante)", e.regra),
                };
                (e.id, regra)
```

- [ ] **Step 7: Rodar e ver passar.** `cargo test --lib` → exit 0.

- [ ] **Step 8: Mutações:**
  - em `fonte.rs`, trocar o `None => match super::variantes::principal(...)` por `None => (None, "")` → `without_a_vaga_variant_the_principal_is_used` FAILED;
  - em `vagas.rs`, tirar a validação de `register_vaga` → `an_unknown_variant_is_refused_and_nothing_is_written` FAILED;
  - em `variantes.rs`, `manter_uma_principal` só marcando (sem desmarcar as outras: `if v.id == marcada { v.principal = true }`) → `marking_a_principal_unmarks_the_others` FAILED.

- [ ] **Step 9: Commit.** `git commit -m "feat(vagas): the agent records the vaga's variant (validated); principal variant as the default; upload result names the variant"`

---

### Tarefa 11 (Onda 1, frente B2): comandos completos da biblioteca + minors adiados

**Files:**
- Modify: `src-tauri/src/documentos/biblioteca.rs`, `src-tauri/src/documentos/importar.rs`, `src-tauri/src/documentos/seguranca.rs` (só `fn is_bare_name` → `pub(crate) fn is_bare_name`), `src-tauri/src/commands/documentos.rs`, `src-tauri/src/db/mod.rs`

**Interfaces:**
- Consumes: Onda 0 (`biblioteca::*`), `seguranca::is_bare_name`, `commands::perfil::ler_search_variants`
- Produces (em `biblioteca.rs`):
  - `pub fn dentro_da_pasta(data_dir: &Path, nome: &str) -> Option<PathBuf>`
  - `pub fn detalhe(conn: &Connection, data_dir: &Path, id: i64) -> Result<DocumentoDetalhe, String>`
  - `pub fn renomear(conn: &Connection, id: i64, nome: &str) -> Result<(), String>`
  - `pub fn remover_preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Result<(), String>`
  - `pub fn apagar(conn: &Connection, data_dir: &Path, id: i64) -> Result<(), String>`
  - os corpos de todos os comandos da tabela do contrato (todos `async`, trabalho em `spawn_blocking`)

- [ ] **Step 1: Testes que falham — `biblioteca.rs`** (no `mod tests` da Onda 0; acrescentar):

```rust
    fn on_disk(name: &str) -> (Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-bib-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir)).unwrap();
        // Schema through a throwaway connection: the one under test must get its FK setting from
        // open_conn alone (db::apply would switch it on by itself and hide a missing PRAGMA).
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        (open_conn(&dir).unwrap(), dir)
    }

    fn real(c: &Connection, dir: &Path, base: &str) -> i64 {
        let docs = crate::documentos::docs_dir(dir);
        std::fs::write(docs.join(format!("{base}.pdf")), b"%PDF-1.7 x").unwrap();
        std::fs::write(docs.join(format!("{base}.html")), "<p>cv</p>").unwrap();
        let pdf = format!("{base}.pdf");
        let html = format!("{base}.html");
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "gerado", arquivo: &pdf, arquivo_html: Some(&html), formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: None }).unwrap()
    }

    #[test]
    fn the_preview_is_read_only_from_inside_the_folder() {
        let (c, dir) = on_disk("preview");
        let ok = real(&c, &dir, "cv_ok");
        assert_eq!(detalhe(&c, &dir, ok).unwrap().preview_html.as_deref(), Some("<p>cv</p>"));
        std::fs::write(dir.join("fora.html"), "<p>segredo</p>").unwrap();
        let bad = inserir(&c, &NovoDocumento { tipo: "curriculo", nome: "X", origem: "gerado", arquivo: "x.pdf", arquivo_html: Some("..\\fora.html"), formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        assert_eq!(detalhe(&c, &dir, bad).unwrap().preview_html, None, "a name that escapes the folder is never read");
    }

    #[test]
    fn deleting_removes_its_files_and_its_preferred_marks_only() {
        let (c, dir) = on_disk("delete");
        let a = real(&c, &dir, "cv_a");
        let b = real(&c, &dir, "cv_b");
        crate::documentos::definir_preferido(&c, a, None).unwrap();
        crate::documentos::definir_preferido(&c, a, Some("backend")).unwrap();
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir).join("envio").join(a.to_string())).unwrap();
        apagar(&c, &dir, a).unwrap();
        let docs = crate::documentos::docs_dir(&dir);
        assert!(!docs.join("cv_a.pdf").exists() && !docs.join("cv_a.html").exists());
        assert!(!docs.join("envio").join(a.to_string()).exists(), "the copy handed to Chrome goes too");
        assert!(docs.join("cv_b.pdf").exists(), "the other document is untouched");
        assert_eq!(crate::documentos::preferido(&c, "curriculo", None), None, "the marks went with it (FK cascade)");
        assert_eq!(crate::documentos::preferido(&c, "curriculo", Some("backend")), None);
        assert!(ver(&c, b).is_ok());
        assert!(apagar(&c, &dir, a).is_err(), "deleting twice says it does not exist");
    }

    #[test]
    fn deleting_never_touches_a_file_outside_the_folder() {
        let (c, dir) = on_disk("outside");
        std::fs::write(dir.join("segredo.pdf"), b"%PDF-1.7 do usuario").unwrap();
        let id = inserir(&c, &NovoDocumento { tipo: "curriculo", nome: "X", origem: "importado", arquivo: "..\\segredo.pdf", arquivo_html: Some("../segredo.pdf"), formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        apagar(&c, &dir, id).unwrap();
        assert!(dir.join("segredo.pdf").exists(), "never deleted outside documentos/");
        assert!(ver(&c, id).is_err(), "the bad row itself is gone");
    }

    #[test]
    fn rename_and_unmark() {
        let (c, dir) = on_disk("rename");
        let a = real(&c, &dir, "cv_r");
        renomear(&c, a, "  Meu CV  ").unwrap();
        assert_eq!(ver(&c, a).unwrap().nome, "Meu CV");
        assert!(renomear(&c, a, "   ").is_err());
        assert!(renomear(&c, 999, "x").is_err());
        crate::documentos::definir_preferido(&c, a, None).unwrap();
        crate::documentos::definir_preferido(&c, a, Some("backend")).unwrap();
        remover_preferido(&c, "curriculo", Some("backend")).unwrap();
        let v = ver(&c, a).unwrap();
        assert!(v.preferido_global && v.preferido_variantes.is_empty(), "only that mark went");
        assert!(remover_preferido(&c, "foto", None).is_err());
    }
```

- [ ] **Step 2: Testes que falham — `importar.rs`** (minors: DOCX de verdade, tamanho depois de ler, nunca sobrescrever). Substituir `a_docx_is_accepted_by_its_zip_header` e acrescentar:

```rust
    fn docx_bytes(with_word: bool) -> Vec<u8> {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("[Content_Types].xml", o).unwrap();
        z.write_all(b"<Types/>").unwrap();
        if with_word {
            z.start_file("word/document.xml", o).unwrap();
            z.write_all(b"<w:document/>").unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn a_real_docx_is_accepted() {
        let (c, dir) = setup("docx");
        let src = file(&dir, "cv.docx", &docx_bytes(true));
        assert_eq!(importar(&c, &dir, "curriculo", &src, Some("Word")).unwrap().formato, "docx");
    }

    #[test]
    fn a_zip_renamed_docx_is_refused() {
        let (c, dir) = setup("zipdocx");
        let src = file(&dir, "cv.docx", &docx_bytes(false));
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("não é um DOCX"));
        let src = file(&dir, "cv2.docx", b"PK\x03\x04 not a zip at all");
        assert!(importar(&c, &dir, "curriculo", &src, None).is_err());
    }

    #[test]
    fn an_existing_destination_is_never_overwritten() {
        let (_c, dir) = setup("noover");
        let dest = dir.join("ja-existe.pdf");
        std::fs::write(&dest, b"%PDF-1.7 original").unwrap();
        assert!(gravar_novo(&dest, b"%PDF-1.7 outro").is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"%PDF-1.7 original");
    }

    #[test]
    fn the_size_limit_is_checked_on_the_bytes_read() {
        assert!(dentro_do_limite(MAX_BYTES) && !dentro_do_limite(MAX_BYTES + 1));
    }
```

- [ ] **Step 3: Teste que falha — `db/mod.rs`** (minor): no `schema_creates_all_tables`, a lista vira `["candidaturas", "documentos", "feedbacks", "pendencias", "preferidos", "propostas_perfil", "sessoes", "vagas"]`.

- [ ] **Step 4: Rodar e ver falhar.** `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-b2 cargo test --lib documentos db::tests` → FAIL (os novos não compilam; o do schema passa já, porque as tabelas existem: é teste de regressão, não de regra nova — a mutação abaixo prova que ele protege).

- [ ] **Step 5: Implementar — `biblioteca.rs`:**

```rust
/// A stored bare name resolved inside <app data>/documentos/, or None. Never follows a name that
/// escapes the folder (`..`, separators, drive letters, links pointing out).
pub fn dentro_da_pasta(data_dir: &Path, nome: &str) -> Option<std::path::PathBuf> {
    if !super::seguranca::is_bare_name(nome) {
        return None;
    }
    let base = std::fs::canonicalize(super::docs_dir(data_dir)).ok()?;
    let real = std::fs::canonicalize(base.join(nome)).ok()?;
    (real.starts_with(&base) && real.is_file()).then_some(real)
}

pub fn detalhe(conn: &Connection, data_dir: &Path, id: i64) -> Result<DocumentoDetalhe, String> {
    let d = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    let preview_html = d
        .arquivo_html
        .as_deref()
        .filter(|h| !h.is_empty())
        .and_then(|h| dentro_da_pasta(data_dir, h))
        .and_then(|p| std::fs::read_to_string(p).ok());
    Ok(DocumentoDetalhe { documento: view_de(conn, d)?, preview_html })
}

pub fn renomear(conn: &Connection, id: i64, nome: &str) -> Result<(), String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("o nome não pode ficar vazio".into());
    }
    let n = conn.execute("UPDATE documentos SET nome = ?1 WHERE id = ?2", params![nome, id]).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("o documento {id} não existe"));
    }
    Ok(())
}

pub fn remover_preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Result<(), String> {
    if tipo != "curriculo" && tipo != "carta" {
        return Err(format!("tipo inválido: {tipo}"));
    }
    conn.execute("DELETE FROM preferidos WHERE tipo = ?1 AND variante_id = ?2", params![tipo, variante_id.unwrap_or("")])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes by id only: the document's own files (only if they resolve inside the folder), the copy
/// handed to Chrome (envio/<id>/), then the row. Its preferred marks go by FK cascade (open_conn).
pub fn apagar(conn: &Connection, data_dir: &Path, id: i64) -> Result<(), String> {
    let d = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    for nome in [Some(d.arquivo.as_str()), d.arquivo_html.as_deref()].into_iter().flatten() {
        if let Some(p) = dentro_da_pasta(data_dir, nome) {
            std::fs::remove_file(&p).map_err(|e| format!("não foi possível apagar o arquivo: {e}"))?;
        }
    }
    let envio = super::docs_dir(data_dir).join("envio").join(id.to_string());
    if envio.is_dir() {
        let _ = std::fs::remove_dir_all(&envio);
    }
    conn.execute("DELETE FROM documentos WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
    Ok(())
}
```

**Atenção (Onda 3):** quando `candidaturas.documento_id` existir (Tarefa 15a), ele precisa ser `REFERENCES documentos(id) ON DELETE SET NULL`, senão o `DELETE` acima falha com FK ligada.

- [ ] **Step 6: Implementar — `importar.rs`.** Funções novas e uso:

```rust
pub(crate) fn dentro_do_limite(len: u64) -> bool {
    len <= MAX_BYTES
}

/// A .docx is a ZIP with a Word document inside, not any ZIP renamed.
fn parece_docx(bytes: &[u8]) -> bool {
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else { return false };
    let tipos = z.by_name("[Content_Types].xml").is_ok();
    let word = z.by_name("word/document.xml").is_ok();
    tipos && word
}

/// Creates the file; fails if anything already has that name (never overwrites).
pub(crate) fn gravar_novo(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(dest).map_err(|_| "conflito de nome ao importar; tente de novo".to_string())?;
    f.write_all(bytes).map_err(|e| {
        let _ = std::fs::remove_file(dest);
        e.to_string()
    })
}
```

Em `importar`: o `if meta.len() > MAX_BYTES` vira `if !dentro_do_limite(meta.len())`; logo depois de `let bytes = std::fs::read(...)`, `if !dentro_do_limite(bytes.len() as u64) { return Err("o arquivo passa de 10 MB".into()); }` (o arquivo pode crescer entre o `metadata` e o `read`); `let real = if formato == "pdf" { bytes.starts_with(b"%PDF-") } else { bytes.starts_with(b"PK\x03\x04") && parece_docx(&bytes) };`; o bloco `if dest.exists() {...}` + `std::fs::write(&dest, &bytes)` vira `gravar_novo(&dest, &bytes)?;`. Mensagens existentes ficam iguais.

- [ ] **Step 7: Implementar — `commands/documentos.rs`** (corpos; todos `async`, trabalho de disco/hash fora da thread principal — minor adiado):

```rust
use tauri_plugin_opener::OpenerExt;

/// Tauri runs sync commands on the main thread: a 10 MB hash there freezes the window. Every document
/// command is async and does its disk work here.
async fn bloqueante<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn listar_documentos(app: AppHandle, tipo: String) -> Result<Vec<DocumentoView>, String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::listar_view(&biblioteca::open_conn(&dir)?, &tipo)).await
}

#[tauri::command]
pub async fn obter_documento(app: AppHandle, id: i64) -> Result<DocumentoDetalhe, String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::detalhe(&biblioteca::open_conn(&dir)?, &dir, id)).await
}

#[tauri::command]
pub async fn importar_documento(app: AppHandle, tipo: String, caminho_origem: String, nome: Option<String>) -> Result<DocumentoView, String> {
    let dir = data_dir(&app)?;
    bloqueante(move || {
        let conn = biblioteca::open_conn(&dir)?;
        let d = documentos::importar::importar(&conn, &dir, &tipo, std::path::Path::new(&caminho_origem), nome.as_deref())?;
        biblioteca::ver(&conn, d.id)
    })
    .await
}

#[tauri::command]
pub async fn renomear_documento(app: AppHandle, id: i64, nome: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::renomear(&biblioteca::open_conn(&dir)?, id, &nome)).await
}

#[tauri::command]
pub async fn atualizar_tags_documento(app: AppHandle, id: i64, tags: Vec<Tag>) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::gravar_tags(&biblioteca::open_conn(&dir)?, id, &tags)).await
}

#[tauri::command]
pub async fn definir_preferido(app: AppHandle, id: i64, variante_id: Option<String>) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || documentos::definir_preferido(&biblioteca::open_conn(&dir)?, id, variante_id.as_deref())).await
}

#[tauri::command]
pub async fn remover_preferido(app: AppHandle, tipo: String, variante_id: Option<String>) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::remover_preferido(&biblioteca::open_conn(&dir)?, &tipo, variante_id.as_deref())).await
}

#[tauri::command]
pub async fn abrir_documento(app: AppHandle, id: i64) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let path = bloqueante(move || {
        let conn = biblioteca::open_conn(&dir)?;
        let d = documentos::obter(&conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
        biblioteca::dentro_da_pasta(&dir, &d.arquivo).ok_or_else(|| format!("o arquivo de \"{}\" sumiu: gere ou importe de novo", d.nome))
    })
    .await?;
    app.opener().open_path(path.to_string_lossy().to_string(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apagar_documento(app: AppHandle, id: i64) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::apagar(&biblioteca::open_conn(&dir)?, &dir, id)).await
}

#[tauri::command]
pub async fn listar_variantes(app: AppHandle) -> Result<Vec<VarianteResumo>, String> {
    let vs = crate::commands::perfil::ler_search_variants(app)?;
    Ok(vs.into_iter().map(|v| VarianteResumo { id: v.id, nome_exibicao: v.nome_exibicao, ativa: v.ativa }).collect())
}
```

(Apagar a constante `NOT_YET` quando não houver mais uso. `abrir_documento` não confere o `sha256`: abrir um arquivo que o usuário editou fora é legítimo; anexar não é — o upload continua com `resolve_attachable`.)

- [ ] **Step 8: Rodar e ver passar.** `cargo test --lib` → exit 0; `cargo build` → exit 0.

- [ ] **Step 9: Mutações:**
  - em `dentro_da_pasta`, tirar o `is_bare_name` **e** o `starts_with(&base)` → `deleting_never_touches_a_file_outside_the_folder` e `the_preview_is_read_only_from_inside_the_folder` FAILED;
  - tirar o `PRAGMA foreign_keys=ON` de `open_conn` → `deleting_removes_its_files_and_its_preferred_marks_only` FAILED (as marcas ficam; o `on_disk` não liga a FK por outro caminho);
  - tirar `&& parece_docx(&bytes)` → `a_zip_renamed_docx_is_refused` FAILED;
  - em `gravar_novo`, trocar `create_new(true)` por `create(true).truncate(true)` → `an_existing_destination_is_never_overwritten` FAILED;
  - em `db/schema.sql`, comentar o `CREATE TABLE ... preferidos` → `schema_creates_all_tables` FAILED.

- [ ] **Step 10: Commit.** `git commit -m "feat(documentos): full library commands (detail with safe preview, rename, tags, unmark, open, delete by id inside the folder); async off the main thread; real DOCX check; never overwrite on import"`

---

### Tarefa 13 (Onda 1, frente C): aba Currículos (sidebar, sub-abas, grade, página A4, Voltar)

**Files:**
- Create: `src/lib/documentos-api.ts`, `src/lib/documentos.mock.ts`, `src/components/documentos/index.tsx`, `Grade.tsx`, `Cartao.tsx`, `FolhaA4.tsx`, `PaginaDocumento.tsx`, `Gerar.tsx`, `logica.ts` (todos em `src/components/documentos/`), `tests/front/documentos-logica.test.ts`, `tests/front/folha-a4.test.ts`
- Modify: `src/types.ts`, `src/components/Sidebar.tsx`, `src/App.tsx`, `src/components/perfil/index.tsx`, `src/components/perfil/ResumoView.tsx`, `src/components/perfil/sections.tsx` (`VariantCard`), `src/components/perfil/types.ts` (sai `CurriculoInfo`/`CoverLetterInfo`; `PALETTE` e `DocLang` ficam), `src/i18n/pt.ts`, `src/i18n/en.ts`
- Delete: `src/components/perfil/CurriculosView.tsx`, `src/components/perfil/CoverLettersView.tsx`, `src/components/perfil/ParaAnexar.tsx`

Ler antes: `docs/padrao-design-claudia-rh.md` (tokens, navegação) e as telas atuais `CurriculosView.tsx`/`CoverLettersView.tsx` (o assistente de geração é movido, não reescrito).

**Interfaces:**
- Consumes: só o contrato (tabela de comandos + JSON `Documento`). O front **não** chama nenhum comando antigo.
- Produces: `View` ganha `"documentos"`; `docsApi` (abaixo); `Perfil` ganha a prop `onOpenDocumentos: () => void`.

- [ ] **Step 1: Tipos** em `src/types.ts`:

```ts
export type TipoDocumento = "curriculo" | "carta";
export type TipoTag = "variante" | "template" | "cor" | "idioma" | "livre";
export interface Tag { tipo: TipoTag; valor: string }

// Mirrors documentos::biblioteca::DocumentoView (spec section 6). No file names, no paths.
export interface Documento {
  id: number;
  tipo: TipoDocumento;
  nome: string;
  origem: "gerado" | "importado";
  formato: "pdf" | "docx";
  idioma: string | null;
  tags: Tag[];
  tamanho: number;
  criado_em: string;
  vaga_id: number | null;
  preferido_global: boolean;
  preferido_variantes: string[];
  tem_preview: boolean;
}
export interface DocumentoDetalhe extends Documento { preview_html: string | null }
export interface VarianteResumo { id: string; nome_exibicao: string; ativa: boolean }
```

e `SearchVariant` ganha `principal: boolean;`.

- [ ] **Step 2: Testes que falham — lógica pura** (`tests/front/documentos-logica.test.ts`, rodam com o Node 24 do projeto, sem dependência nova; ficam fora de `src/` para o `tsc` não exigir tipos do Node):

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { filtrar, valoresDeTag, ehPreferido, avisoAoApagar, preferidoDaVariante, comTag, semTag } from "../../src/components/documentos/logica.ts";

const doc = (id: number, extra: Partial<Record<string, unknown>> = {}) => ({
  id, tipo: "curriculo", nome: `CV ${id}`, origem: "gerado", formato: "pdf", idioma: "pt", tags: [], tamanho: 1,
  criado_em: "2026-10-04", vaga_id: null, preferido_global: false, preferido_variantes: [], tem_preview: true, ...extra,
}) as any;

const be = { tipo: "variante", valor: "backend" } as const;
const ats = { tipo: "template", valor: "classic-ats" } as const;

test("filters by every chosen tag kind at once", () => {
  const docs = [doc(1, { tags: [be, ats] }), doc(2, { tags: [be] }), doc(3, { tags: [ats] })];
  assert.deepEqual(filtrar(docs, { variante: "backend", template: "classic-ats" }).map((d: any) => d.id), [1]);
  assert.deepEqual(filtrar(docs, {}).map((d: any) => d.id), [1, 2, 3], "no filter, everything");
});

test("tag values for a filter are distinct and sorted", () => {
  const docs = [doc(1, { tags: [be] }), doc(2, { tags: [{ tipo: "variante", valor: "devops" }, be] })];
  assert.deepEqual(valoresDeTag(docs, "variante"), ["backend", "devops"]);
});

test("the star comes from the backend fields, never from local state", () => {
  assert.equal(ehPreferido(doc(1, { preferido_global: true }), null), true);
  assert.equal(ehPreferido(doc(1, { preferido_variantes: ["backend"] }), "backend"), true);
  assert.equal(ehPreferido(doc(1, { preferido_variantes: ["backend"] }), null), false);
});

test("deleting a preferred document warns, the general mark first", () => {
  assert.equal(avisoAoApagar(doc(1)), null);
  assert.equal(avisoAoApagar(doc(1, { preferido_variantes: ["x"] })), "preferido_variante");
  assert.equal(avisoAoApagar(doc(1, { preferido_global: true, preferido_variantes: ["x"] })), "preferido_global");
});

test("the variant picker shows the document marked for that variant", () => {
  const docs = [doc(1), doc(2, { preferido_variantes: ["backend"] })];
  assert.equal(preferidoDaVariante(docs, "backend")?.id, 2);
  assert.equal(preferidoDaVariante(docs, "devops"), null);
});

test("editing tags trims, refuses empty and never duplicates", () => {
  assert.deepEqual(comTag([be], { tipo: "variante", valor: " backend " }), [be]);
  assert.deepEqual(comTag([], { tipo: "livre", valor: "  " }), []);
  assert.deepEqual(semTag([be, ats], be), [ats]);
});
```

`tests/front/folha-a4.test.ts` (a regra de segurança da pré-visualização, conferida no fonte):

```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("the A4 sheet renders the preview in a sandbox that runs no scripts", () => {
  const src = readFileSync(new URL("../../src/components/documentos/FolhaA4.tsx", import.meta.url), "utf8");
  assert.match(src, /sandbox=""/, "an empty sandbox: no scripts, no same-origin");
  assert.doesNotMatch(src, /allow-scripts|allow-same-origin|dangerouslySetInnerHTML/);
  assert.match(src, /srcDoc=/);
});
```

- [ ] **Step 3: Rodar e ver falhar.** `node --test tests/front/` → FAIL (módulo inexistente).

- [ ] **Step 4: `src/components/documentos/logica.ts`:**

```ts
// Pure rules of the Currículos tab, tested with node:test (tests/front/). No React, no Tauri.
import type { Documento, Tag, TipoTag } from "../../types";

export type Filtro = Partial<Record<Exclude<TipoTag, "livre">, string>>;

export function filtrar(docs: Documento[], f: Filtro): Documento[] {
  const pedidos = Object.entries(f).filter(([, v]) => v) as [TipoTag, string][];
  return docs.filter((d) => pedidos.every(([tipo, valor]) => d.tags.some((t) => t.tipo === tipo && t.valor === valor)));
}

export function valoresDeTag(docs: Documento[], tipo: TipoTag): string[] {
  return [...new Set(docs.flatMap((d) => d.tags.filter((t) => t.tipo === tipo).map((t) => t.valor)))].sort();
}

export function ehPreferido(d: Documento, varianteId: string | null): boolean {
  return varianteId === null ? d.preferido_global : d.preferido_variantes.includes(varianteId);
}

export function avisoAoApagar(d: Documento): "preferido_global" | "preferido_variante" | null {
  if (d.preferido_global) return "preferido_global";
  if (d.preferido_variantes.length > 0) return "preferido_variante";
  return null;
}

export function preferidoDaVariante(docs: Documento[], varianteId: string): Documento | null {
  return docs.find((d) => d.preferido_variantes.includes(varianteId)) ?? null;
}

export function comTag(tags: Tag[], nova: Tag): Tag[] {
  const valor = nova.valor.trim();
  if (!valor || tags.some((t) => t.tipo === nova.tipo && t.valor === valor)) return tags;
  return [...tags, { tipo: nova.tipo, valor }];
}

export function semTag(tags: Tag[], fora: Tag): Tag[] {
  return tags.filter((t) => !(t.tipo === fora.tipo && t.valor === fora.valor));
}
```

- [ ] **Step 5: Rodar e ver passar.** `node --test tests/front/` → os dois arquivos ainda falham até o Step 7 (FolhaA4); a lógica passa.

- [ ] **Step 6: Uma única API** — `src/lib/documentos-api.ts`:

```ts
// The only place the front talks to the document commands (spec section 6). In `vite dev` outside
// Tauri (a plain browser), it answers from an in-memory mock so the screens can be built before the
// backend lands; production builds drop the mock (import.meta.env.DEV is false there).
import { invoke } from "@tauri-apps/api/core";
import type { DocLang } from "../components/perfil/types";
import type { Documento, DocumentoDetalhe, Tag, TipoDocumento, VarianteResumo } from "../types";

const semTauri = () => typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.DEV && semTauri()) {
    const { mockInvoke } = await import("./documentos.mock");
    return mockInvoke<T>(cmd, args ?? {});
  }
  return invoke<T>(cmd, args);
}

export const docsApi = {
  listar: (tipo: TipoDocumento) => call<Documento[]>("listar_documentos", { tipo }),
  obter: (id: number) => call<DocumentoDetalhe>("obter_documento", { id }),
  gerarCurriculo: (a: { templateId: string; corPrimaria: string; idioma: DocLang; varianteId: string | null }) =>
    call<Documento>("gerar_curriculo", a),
  gerarCarta: (a: { empresa: string; cargo: string; descricaoVaga: string | null; notaExtra: string | null; idioma: DocLang; corPrimaria: string; varianteId: string | null }) =>
    call<void>("gerar_cover_letter", a),
  importar: (tipo: TipoDocumento, caminhoOrigem: string, nome: string | null) =>
    call<Documento>("importar_documento", { tipo, caminhoOrigem, nome }),
  renomear: (id: number, nome: string) => call<void>("renomear_documento", { id, nome }),
  atualizarTags: (id: number, tags: Tag[]) => call<void>("atualizar_tags_documento", { id, tags }),
  definirPreferido: (id: number, varianteId: string | null) => call<void>("definir_preferido", { id, varianteId }),
  removerPreferido: (tipo: TipoDocumento, varianteId: string | null) => call<void>("remover_preferido", { tipo, varianteId }),
  abrir: (id: number) => call<void>("abrir_documento", { id }),
  apagar: (id: number) => call<void>("apagar_documento", { id }),
  listarVariantes: () => call<VarianteResumo[]>("listar_variantes"),
};
```

`src/lib/documentos.mock.ts`: começa com `export const MOCK_DOCUMENTOS = "mock-documentos-dev-only";` e exporta `mockInvoke<T>(cmd, args)` com um estado em memória (2 currículos — um `gerado` com `preview_html` de uma folha simples e tags `variante:backend`, `template:classic-ats`, `cor:#D97757`, `idioma:pt`; um `importado` PDF sem preview — e 1 carta), implementando **cada** comando da tabela do contrato com o mesmo formato de retorno (inclusive `preferido_global`/`preferido_variantes` recalculados ao marcar/desmarcar e erro `"o documento N não existe"`); `gerar_cover_letter` no mock emite nada e resolve `void`. Comando desconhecido → `Promise.reject("mock: comando desconhecido <cmd>")`.

- [ ] **Step 7: Componentes.** Regras (a disposição visual segue os tokens do `padrao-design-claudia-rh.md`; o que está abaixo é obrigatório):
  - `FolhaA4.tsx` — `export const FolhaA4: React.FC<{ html: string; escala?: number; titulo: string }>`: um contêiner com proporção A4 (210×297), sombra e margem; dentro, `<iframe sandbox="" srcDoc={html} title={titulo} style={{ width: 794, height: 1123, border: 0, transform: `scale(${escala ?? 1})`, transformOrigin: "top left", pointerEvents: escala && escala < 1 ? "none" : "auto" }} />`. Nunca `allow-scripts`, nunca `dangerouslySetInnerHTML` (teste do Step 2).
  - `Cartao.tsx` — props `{ doc: Documento; onAbrir: (id: number) => void }`. Miniatura: se `doc.tem_preview`, um `IntersectionObserver` (rootMargin `200px`) dispara **uma vez** `docsApi.obter(doc.id)` e mostra `<FolhaA4 html={preview_html} escala={0.22} />`; sem preview, um bloco com o ícone `FileText` e o formato em maiúsculas. Abaixo: nome, origem (`t.documents.origin.gerado|importado`), chips de tags, estrela cheia se `doc.preferido_global` (título: "Preferido geral") e um ponto/legenda se `preferido_variantes.length` (título lista as variantes). Clique no cartão → `onAbrir(doc.id)`. Teclado: `role="button"`, `tabIndex=0`, Enter abre.
  - `Gerar.tsx` — props `{ tipo: TipoDocumento; variantes: VarianteResumo[]; onFeito: () => void; onCancelar: () => void }`. Para `curriculo`: o seletor de template, idioma e cor que hoje está em `CurriculosView.tsx` (mover as constantes e o JSX) + um `<select>` de variante ("Sem variante" = `null`, depois as variantes ativas); botão Gerar chama `docsApi.gerarCurriculo(...)`, mostra `Loader2` enquanto roda e o erro do backend se falhar. Para `carta`: o formulário de `CoverLettersView.tsx` (empresa, cargo, descrição, nota, idioma, cor) + variante; escuta `cover-letter-stream` (mostra o texto chegando), `cover-letter-done` (`[depende de P1]`: payload `Documento`; chama `onFeito`) e `cover-letter-error`.
  - `Grade.tsx` — props `{ tipo: TipoDocumento; onAbrir: (id: number) => void }`. Carrega `docsApi.listar(tipo)` e `docsApi.listarVariantes()`; estados **carregando**, **erro** (mensagem do backend + "Tentar de novo"), **vazio** (texto + os dois botões) e **com dados** (grade `repeat(auto-fill, minmax(180px, 1fr))` de `Cartao`). Barra: quatro `<select>` de filtro (variante, template, cor, idioma; opções de `valoresDeTag`; "Todos" = sem filtro) aplicados com `filtrar`; botões **Gerar** (abre `Gerar` num painel) e **Adicionar o meu** (`open` de `@tauri-apps/plugin-dialog` com filtro `pdf`/`docx`, depois `docsApi.importar(tipo, caminho, null)` e recarrega; erro do backend aparece na barra). Recarrega ao voltar da página do documento.
  - `PaginaDocumento.tsx` — props `{ id: number; onVoltar: () => void }`. Topo: botão **← Voltar** (`t.common.back`) e o nome. Corpo: se `tem_preview`, `<FolhaA4 html={preview_html} />` em escala que caiba na largura; senão o **cartão do importado** (nome, formato, tamanho em KB, data, tags) e o botão **Abrir** (a spec deixa "PDF em iframe" para verificar; sem capability nova, fica o cartão). Lateral: metadados; **tags editáveis** (chips com ×, e um par `<select>` de tipo + campo de valor + Adicionar; salvar com `docsApi.atualizarTags` usando `comTag`/`semTag`, recarregar com `obter`); **preferido** — um checkbox "Preferido geral" (`definirPreferido(id, null)` / `removerPreferido(tipo, null)`) e um checkbox por variante ativa (`definirPreferido(id, v.id)` / `removerPreferido(tipo, v.id)`), o estado sempre relido do backend (`ehPreferido`) — é o minor "estrela local" resolvido; **Renomear** (campo inline, Enter salva); **Abrir** (`docsApi.abrir`); **Excluir** (`window.confirm` com `t.documents.confirmDelete` ou, se `avisoAoApagar` ≠ null, `t.documents.confirmDeletePreferred`; depois `docsApi.apagar` e `onVoltar()`).
  - `index.tsx` — `export const Documentos: React.FC`: estado `aba: TipoDocumento` (sub-abas **Currículos** / **Cover letters**) e `aberto: number | null`; com `aberto`, só a `PaginaDocumento`; sem, título + sub-abas + `Grade` da aba.

- [ ] **Step 8: Navegação.**
  - `Sidebar.tsx`: `View` ganha `"documentos"`; item `{ id: "documentos", label: t.nav.documents, Icon: FileText }` **entre** `perfil` e `historico`.
  - `App.tsx`: bloco `{/* Currículos */}` igual aos outros (`display: view === "documentos" ? "flex" : "none"`, `overflow: "hidden"`) com `<Documentos />`; `<Perfil ... onOpenDocumentos={() => setView("documentos")} />`.
  - `perfil/index.tsx`: saem os modos `curriculos` e `cover_letters`, os imports de `CurriculosView`/`CoverLettersView` e as funções `openCurriculos`/`openCoverLetters`; `ResumoView` recebe `onOpenDocumentos` no lugar de `onOpenCurriculos`/`onOpenCoverLetters`.
  - `ResumoView.tsx`: os dois botões viram **um** link "Currículos e cartas →" (`t.profile.goToDocuments`) que chama `onOpenDocumentos`. Carrega `docsApi.listar("curriculo")` e `docsApi.listar("carta")` uma vez (e de novo ao receber `db-atualizada`) e passa as listas ao `VariantCard`.
  - `sections.tsx` / `VariantCard`: props novas `curriculos: Documento[]`, `cartas: Documento[]`, `onPreferido: (tipo: TipoDocumento, docId: number | null) => void`. Dois `<select>` rotulados `t.profile.variantPreferredResume` e `t.profile.variantPreferredLetter`; valor = `preferidoDaVariante(lista, variant.id)?.id ?? ""`; opção vazia `t.profile.none`. `ResumoView` implementa `onPreferido`: `docId` → `docsApi.definirPreferido(docId, variant.id)`, `null` → `docsApi.removerPreferido(tipo, variant.id)`; depois recarrega as listas.
  - `[depende de P2]` No `VariantCard`, um botão "Principal" (`t.profile.principal`; ativo quando `variant.principal`) que chama `guardar_variante_unica` com `{ ...variant, principal: true }` (o backend desmarca as outras) e recarrega o Perfil.

- [ ] **Step 9: i18n** (pt e en, mesmas chaves; `en.ts` define o tipo `Strings`): `nav.documents` ("Currículos" / "Résumés"); bloco novo `documents`: `title, tabResumes, tabCoverLetters, generate, importMine, empty, emptyHint, loading, loadError, retry, filterAll, filterVariant, filterTemplate, filterColor, filterLanguage, preferredGlobal, preferredForVariant, rename, open, delete, confirmDelete, confirmDeletePreferred, tags, addTag, tagTypes: { variante, template, cor, idioma, livre }, origin: { gerado, importado }, size, createdAt, noVariant, variant, importedCard`; em `profile`: `goToDocuments, variantPreferredResume, variantPreferredLetter, none` (e `principal` se P2). Sai o bloco `profile.attach` (era do `ParaAnexar`) e as chaves que só as telas apagadas usavam (o `tsc` com `noUnusedLocals` não acusa chave sobrando: conferir com `grep` cada chave removida).

- [ ] **Step 10: Verificar.**
  - `node --test tests/front/` → exit 0 (7 testes).
  - `npx tsc --noEmit` → exit 0.
  - `grep -rnE "listar_curriculos|apagar_curriculo|abrir_curriculo|listar_cover_letters|abrir_cover_letter|apagar_cover_letter|ParaAnexar" src/` → **nenhuma** linha.
  - `npm run build` → exit 0; `grep -rl "mock-documentos-dev-only" dist/` → **nenhum** arquivo (o mock não vai para produção).
  - Com o mock, em `npm run dev` num browser comum (**não** o app): abrir a aba, as duas sub-abas, filtrar, abrir um cartão, Voltar, marcar preferido geral e por variante (a estrela muda depois de recarregar), editar tags, renomear, excluir um preferido (aparece o aviso). Prints nos dois idiomas. Isto não substitui o smoke da Tarefa 16 no app de verdade.

- [ ] **Step 11: Mutações:**
  - em `ehPreferido`, trocar por `return d.preferido_global || d.preferido_variantes.length > 0;` → `the star comes from the backend fields...` FAILED;
  - em `FolhaA4.tsx`, `sandbox="allow-scripts"` → `the A4 sheet renders the preview in a sandbox that runs no scripts` FAILED;
  - em `filtrar`, `every` → `some` → `filters by every chosen tag kind at once` FAILED.

- [ ] **Step 12: Commit.** `git commit -m "feat(ui): Currículos tab - sub-tabs, grid with lazy A4 thumbnails, document page with Back, tags, preferred per variant from the backend, import, generate; Profile links to it"`

---

### Tarefa 12 (Onda 2, frente D): migração dos currículos e cartas antigos

Depende de: Tarefa 9 (portão do PDF aprovado) e 9b (`registrar_gerado`) integradas.

**Files:**
- Modify: `src-tauri/src/documentos/migrar.rs` (stub da Onda 0; o gancho em `lib.rs` já existe)

**Interfaces:**
- Consumes: `registrar::{registrar_gerado, NovoGerado}`, `pdf::html_to_pdf`, `biblioteca::{open_conn, Tag}`
- Produces:
  - `pub struct Antigo { pub tipo: &'static str, pub origem: PathBuf, pub base: String, pub nome: String, pub tags: Vec<Tag> }`
  - `pub fn pendentes(data_dir: &Path) -> Result<Vec<Antigo>, String>`
  - `pub struct Relatorio { pub migrados: usize, pub falhas: Vec<String> }`
  - `pub async fn migrar_com<F, Fut>(data_dir: &Path, print: F) -> Relatorio where F: Fn(PathBuf, PathBuf) -> Fut, Fut: Future<Output = Result<(), String>>`
  - `pub async fn ao_iniciar(app: tauri::AppHandle)`

- [ ] **Step 1: Testes que falham:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-mig-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("curriculos")).unwrap();
        std::fs::create_dir_all(dir.join("cover_letters")).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        std::fs::write(dir.join("curriculos").join("cv_classic-ats_2026-09-30_10-00.html"), "<p>cv antigo</p>").unwrap();
        std::fs::write(dir.join("cover_letters").join("cl_acme_2026-09-30_11-00.html"), "<p>carta antiga</p>").unwrap();
        dir
    }

    async fn fake_pdf(_h: PathBuf, p: PathBuf) -> Result<(), String> {
        std::fs::write(p, b"%PDF-1.7 migrado").map_err(|e| e.to_string())
    }

    fn rows(dir: &Path) -> Vec<(String, String, String)> {
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let mut st = c.prepare("SELECT tipo, origem, arquivo FROM documentos ORDER BY id").unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(|r| r.unwrap()).collect()
    }

    #[tokio::test]
    async fn old_cvs_and_letters_become_documents_and_the_originals_stay() {
        let dir = setup("ok");
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (2, 0), "{:?}", r.falhas);
        assert_eq!(rows(&dir), vec![
            ("curriculo".into(), "gerado".into(), "migrado-cv_classic-ats_2026-09-30_10-00.pdf".into()),
            ("carta".into(), "gerado".into(), "migrado-cl_acme_2026-09-30_11-00.pdf".into()),
        ]);
        assert_eq!(std::fs::read_to_string(dir.join("curriculos").join("cv_classic-ats_2026-09-30_10-00.html")).unwrap(), "<p>cv antigo</p>", "nothing is deleted or changed");
        assert!(dir.join("cover_letters").join("cl_acme_2026-09-30_11-00.html").exists());
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let v = crate::documentos::biblioteca::ver(&c, 1).unwrap();
        assert!(v.tags.iter().any(|t| t.tipo == "template" && t.valor == "classic-ats"), "{:?}", v.tags);
    }

    #[tokio::test]
    async fn running_twice_migrates_once() {
        let dir = setup("twice");
        migrar_com(&dir, fake_pdf).await;
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (0, 0), "{:?}", r.falhas);
        assert_eq!(rows(&dir).len(), 2);
    }

    #[tokio::test]
    async fn a_failed_pdf_skips_that_file_and_the_rest_still_migrate() {
        let dir = setup("partial");
        let r = migrar_com(&dir, |h: PathBuf, p: PathBuf| async move {
            if h.to_string_lossy().contains("cl_acme") { Err("boom".to_string()) } else { fake_pdf(h, p).await }
        })
        .await;
        assert_eq!((r.migrados, r.falhas.len()), (1, 1));
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!(r.migrados, 1, "the failed one is retried on the next start");
        assert_eq!(rows(&dir).len(), 2);
    }

    #[tokio::test]
    async fn a_half_done_migration_is_retried() {
        let dir = setup("half");
        let docs = crate::documentos::docs_dir(&dir);
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("migrado-cv_classic-ats_2026-09-30_10-00.html"), "leftover").unwrap();
        let r = migrar_com(&dir, fake_pdf).await;
        assert_eq!((r.migrados, r.falhas.len()), (2, 0), "{:?}", r.falhas);
    }

    #[test]
    fn only_html_files_at_the_top_level_are_taken() {
        let dir = std::env::temp_dir().join(format!("claudia-mig-only-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("curriculos").join("sub")).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        std::fs::write(dir.join("curriculos").join("notas.txt"), "x").unwrap();
        std::fs::write(dir.join("curriculos").join("sub").join("cv_x_2026-01-01_00-00.html"), "x").unwrap();
        assert!(pendentes(&dir).unwrap().is_empty());
    }

    #[test]
    fn odd_file_names_never_panic() {
        let dir = std::env::temp_dir().join(format!("claudia-mig-odd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("cover_letters")).unwrap();
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        for name in ["cl_ção.html", "cl_empresa-ação_çççççççççç.html", "x.html"] {
            std::fs::write(dir.join("cover_letters").join(name), "x").unwrap();
        }
        assert_eq!(pendentes(&dir).unwrap().len(), 3);
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.** `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-d cargo test --lib documentos::migrar` → FAIL.

- [ ] **Step 3: Implementar `migrar.rs`:**

```rust
//! Old generated CVs (<data>/curriculos/*.html) and cover letters (<data>/cover_letters/*.html) join
//! the library: the HTML is copied (as text) into documentos/ and gets its PDF. Nothing in the old
//! folders is ever deleted, moved or changed. Idempotent: a file whose migrated PDF is registered is
//! skipped; a failure is retried on the next start.
use super::biblioteca::{self, Tag};
use super::registrar::{registrar_gerado, NovoGerado};
use std::future::Future;
use std::path::{Path, PathBuf};

pub struct Antigo {
    pub tipo: &'static str,
    pub origem: PathBuf,
    pub base: String,
    pub nome: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Default)]
pub struct Relatorio {
    pub migrados: usize,
    pub falhas: Vec<String>,
}

fn template_de(stem: &str) -> Option<(&'static str, &'static str)> {
    [("classic-ats", "Classic ATS"), ("hybrid-skills", "Hybrid Skills"), ("dev-compact", "Dev Compact")]
        .into_iter()
        .find(|(id, _)| stem.starts_with(&format!("cv_{id}_")))
}

/// "2026-09-30_10-00" at the end of the stem, shown as "2026-09-30 10:00". `get` (not indexing): a
/// stem with accents near the end must not panic on a char boundary.
fn data_de(stem: &str) -> String {
    match stem.len().checked_sub(16).and_then(|i| stem.get(i..)) {
        Some(d) if d.is_ascii() => format!("{} {}", &d[..10], d[11..].replace('-', ":")),
        _ => String::new(),
    }
}

fn ja_migrado(conn: &rusqlite::Connection, base: &str) -> bool {
    conn.query_row("SELECT 1 FROM documentos WHERE arquivo = ?1", [format!("{base}.pdf")], |_| Ok(()))
        .is_ok()
}

pub fn pendentes(data_dir: &Path) -> Result<Vec<Antigo>, String> {
    let conn = biblioteca::open_conn(data_dir)?;
    let mut out = Vec::new();
    for (pasta, tipo) in [("curriculos", "curriculo"), ("cover_letters", "carta")] {
        let Ok(entries) = std::fs::read_dir(data_dir.join(pasta)) else { continue };
        let mut files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        files.sort_by_key(|e| e.file_name());
        for e in files {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_file() || ft.is_symlink() {
                continue;
            }
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("html") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
            let base = format!("migrado-{stem}");
            if ja_migrado(&conn, &base) {
                continue;
            }
            let quando = data_de(&stem);
            let (nome, tags) = if tipo == "curriculo" {
                match template_de(&stem) {
                    Some((id, rotulo)) => (format!("{rotulo} ({quando})"), vec![Tag { tipo: "template".into(), valor: id.into() }]),
                    None => (format!("Currículo ({quando})"), vec![]),
                }
            } else {
                let empresa = stem.strip_prefix("cl_").unwrap_or(&stem);
                let empresa = empresa.len().checked_sub(17).and_then(|i| empresa.get(..i)).unwrap_or(empresa);
                (format!("Carta {} ({quando})", empresa.replace('-', " ")), vec![])
            };
            out.push(Antigo { tipo, origem: path, base, nome, tags });
        }
    }
    Ok(out)
}

pub async fn migrar_com<F, Fut>(data_dir: &Path, print: F) -> Relatorio
where
    F: Fn(PathBuf, PathBuf) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    let mut r = Relatorio::default();
    let lista = match pendentes(data_dir) {
        Ok(l) => l,
        Err(e) => {
            r.falhas.push(e);
            return r;
        }
    };
    let docs = super::docs_dir(data_dir);
    for a in lista {
        let html = match std::fs::read_to_string(&a.origem) {
            Ok(h) => h,
            Err(e) => {
                r.falhas.push(format!("{}: {e}", a.origem.display()));
                continue;
            }
        };
        // Leftovers of an interrupted run are ours (migrado-* has no row): clear them so it can retry.
        let _ = std::fs::remove_file(docs.join(format!("{}.html", a.base)));
        let _ = std::fs::remove_file(docs.join(format!("{}.pdf", a.base)));
        let novo = NovoGerado { tipo: a.tipo.into(), nome: a.nome, base: a.base.clone(), html, idioma: None, variante_id: None, vaga_id: None, tags: a.tags };
        match registrar_gerado(data_dir, novo, &print).await {
            Ok(_) => r.migrados += 1,
            Err(e) => r.falhas.push(format!("{}: {e}", a.base)),
        }
    }
    r
}

pub async fn ao_iniciar(app: tauri::AppHandle) {
    use tauri::Manager;
    let Ok(data_dir) = app.path().app_data_dir() else { return };
    let r = migrar_com(&data_dir, |h: PathBuf, p: PathBuf| {
        let app = app.clone();
        async move { super::pdf::html_to_pdf(&app, &h, &p).await }
    })
    .await;
    if r.migrados > 0 || !r.falhas.is_empty() {
        crate::diagnostics::emit_event("documentos", "migracao", None, &format!("migrados={} falhas={}", r.migrados, r.falhas.len()));
    }
}
```

(`registrar_gerado` recebe `FnOnce`; passar `&print` funciona porque `&F` onde `F: Fn` implementa `FnOnce`. Se o compilador pedir, usar `|h, p| print(h, p)`.)

`[depende de P3]` — acrescentar a `migrar_com`, depois do laço das pastas antigas, o processamento de `documentos/pendentes/`: para cada `<n>.json` com um `<n>.html` ao lado (nomes simples, só arquivos), ler o JSON `PendenteDoc { tipo, nome, idioma, variante_id, vaga_id, tags }` (contrato com a frente E), chamar `registrar_gerado` com `base = n` e, com sucesso, **apagar os dois arquivos de `pendentes/`** (são do próprio app, não do usuário); com falha, deixar para a próxima vez. Teste: `a_pending_letter_from_the_agent_becomes_a_document` (cria `pendentes/carta-1.html` + `.json`, roda, confere a linha `tipo = carta`, `vaga_id` gravado e `pendentes/` vazio).

- [ ] **Step 4: Rodar e ver passar.** `cargo test --lib` → exit 0.

- [ ] **Step 5: Mutações:**
  - tirar o `if ja_migrado(...) { continue; }` → `running_twice_migrates_once` FAILED (a segunda rodada reporta falhas de nome existente);
  - acrescentar `let _ = std::fs::remove_file(&a.origem);` depois do `Ok(_)` → `old_cvs_and_letters_become_documents_and_the_originals_stay` FAILED;
  - tirar a limpeza dos restos (`remove_file` do `migrado-*`) → `a_half_done_migration_is_retried` FAILED.

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): old generated CVs and cover letters join the library on startup (idempotent, originals untouched)"`

---

### Tarefa 14 (Onda 2, frente E): MCP de documentos, variantes e cover letter

Depende de: Tarefas 10 e 11 integradas (`variantes::*`, `biblioteca::detalhe`).

**Files:**
- Create: `src-tauri/src/mcp/tools/documentos.rs`, `src-tauri/src/mcp/tools/variantes.rs`
- Modify: `src-tauri/src/documentos/texto.rs` (stub), `src-tauri/src/mcp/tools/mod.rs`, `src-tauri/src/mcp/tools/profile.rs`, `src-tauri/src/mcp/mod.rs`, `src-tauri/src/mcp/server.rs`, `src-tauri/tests/mcp-browser-smoke.mjs`, `src-tauri/src/commands/cover_letter.rs` `[P3]`

**Interfaces:**
- Produces (todas `-> Result<String, String>`, saída em JSON legível; nenhuma aceita ou devolve caminho):
  - `list_variantes(data_dir)`, `get_variante(data_dir, id)`, `update_variante(data_dir, id, campos: &serde_json::Value, session)`
  - `list_documentos(data_dir, tipo)`, `get_documento(data_dir, id)`, `set_documento_preferido(data_dir, id, variante_id: Option<&str>, session)`, `save_cover_letter(...)` `[P3]`
  - `pub(crate) fn write_yaml_atomically(data_dir: &Path, file: &str, content: &str) -> Result<(), String>` em `profile.rs`
  - `pub fn html_para_texto(html: &str) -> String` em `documentos/texto.rs`

- [ ] **Step 0: Investigação (só leitura, spec 13).** Localizar a skill `tailor-application` (fora do repo: `~/.claude/skills/` ou plugins) e anotar no ledger onde ela grava arquivos. Se gravar fora da biblioteca, registrar no Linear ("currículo adaptado pela skill não entra na biblioteca") e propor a atualização da spec — **não** implementar nada aqui. Se não achar, escrever "não verificado".

- [ ] **Step 1: Testes que falham — `texto.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_scripts_and_styles_go_and_entities_are_decoded() {
        let t = html_para_texto("<html><style>p{color:red}</style><script>alert(1)</script><h1>João &amp; Cia</h1><p>Rust&nbsp;&lt;3</p><ul><li>A</li><li>B</li></ul></html>");
        assert_eq!(t, "João & Cia\nRust <3\nA\nB");
    }

    #[test]
    fn an_escaped_tag_stays_text() {
        assert_eq!(html_para_texto("<p>&amp;lt;b&amp;gt;</p>"), "&lt;b&gt;");
    }
}
```

- [ ] **Step 2: Testes que falham — `mcp/tools/variantes.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::temp_dir;

    const SV: &str = "variantes:\n  - id: backend\n    nome_exibicao: Backend\n    peso: 60\n    cv_gerado_path: C:\\\\Users\\\\x\\\\cv.html\n  - id: devops\n    nome_exibicao: DevOps\n    principal: true\n";

    fn dir() -> std::path::PathBuf {
        let d = temp_dir("mcp-var");
        std::fs::write(d.join("search_variants.yaml"), SV).unwrap();
        std::fs::write(d.join("candidate_base.yaml"), "competencias: [Rust, Kubernetes]\nexperiencia:\n  - empresa: Acme\n    cargo: SRE\n    tecnologias: [Terraform]\n").unwrap();
        d
    }

    #[test]
    fn reads_never_return_a_path() {
        let d = dir();
        let l = list_variantes(&d).unwrap();
        assert!(l.contains("backend") && l.contains("\"principal\": true"), "{l}");
        let g = get_variante(&d, "backend").unwrap();
        assert!(!g.contains("cv_gerado") && !g.contains("cv.html"), "{g}");
        assert!(get_variante(&d, "frontend").is_err());
    }

    #[test]
    fn update_variante_is_refused_in_the_execution_session() {
        let d = dir();
        let e = update_variante(&d, "backend", &serde_json::json!({ "peso": 10 }), SessionKind::Autonomous).unwrap_err();
        assert!(e.contains("sessão de Perfil"), "{e}");
        assert_eq!(std::fs::read_to_string(d.join("search_variants.yaml")).unwrap(), SV, "untouched");
    }

    #[test]
    fn update_variante_writes_with_a_backup_and_keeps_one_principal() {
        let d = dir();
        update_variante(&d, "backend", &serde_json::json!({ "peso": 70, "principal": true }), SessionKind::Interactive).unwrap();
        assert_eq!(std::fs::read_to_string(d.join("search_variants.yaml.bak-1")).unwrap(), SV, "the previous file is kept");
        let sv = crate::documentos::variantes::ler(&d).unwrap();
        assert_eq!(sv.variantes[0].peso, 70.0);
        assert_eq!(crate::documentos::variantes::principal_de(&sv).as_deref(), Some("backend"));
        assert_eq!(sv.variantes.iter().filter(|v| v.principal).count(), 1);
    }

    #[test]
    fn update_variante_refuses_unknown_fields_and_paths() {
        let d = dir();
        for campos in [serde_json::json!({ "cv_gerado_path": "C:\\x" }), serde_json::json!({ "id": "outra" }), serde_json::json!([1])] {
            assert!(update_variante(&d, "backend", &campos, SessionKind::Interactive).is_err(), "{campos}");
        }
        assert!(update_variante(&d, "frontend", &serde_json::json!({ "peso": 1 }), SessionKind::Interactive).is_err());
    }

    // [depende de P4]
    #[test]
    fn a_variant_cannot_claim_what_the_profile_does_not_have() {
        let d = dir();
        let e = update_variante(&d, "backend", &serde_json::json!({ "foco_competencias": ["Rust", "COBOL"] }), SessionKind::Interactive).unwrap_err();
        assert!(e.contains("COBOL"), "{e}");
        update_variante(&d, "backend", &serde_json::json!({ "foco_competencias": ["rust", "Terraform"], "foco_experiencia": ["Acme"] }), SessionKind::Interactive).unwrap();
    }
}
```

- [ ] **Step 3: Testes que falham — `mcp/tools/documentos.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::temp_dir;

    fn lib_with_two() -> (std::path::PathBuf, i64, i64) {
        let d = temp_dir("mcp-docs");
        let c = crate::documentos::biblioteca::open_conn(&d).unwrap();
        crate::db::apply(&c).unwrap();
        let docs = crate::documentos::docs_dir(&d);
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("cv_g.pdf"), b"%PDF-1.7 g").unwrap();
        std::fs::write(docs.join("cv_g.html"), "<h1>Ana Lima</h1><p>Rust &amp; SQL</p>").unwrap();
        let g = crate::documentos::inserir(&c, &crate::documentos::NovoDocumento { tipo: "curriculo", nome: "Gerado", origem: "gerado", arquivo: "cv_g.pdf", arquivo_html: Some("cv_g.html"), formato: "pdf", idioma: Some("pt"), sha256: "0", tamanho: 1, variante_id: None }).unwrap();
        let src = d.join("meu.pdf");
        std::fs::write(&src, b"%PDF-1.7 meu").unwrap();
        let i = crate::documentos::importar::importar(&c, &d, "curriculo", &src, Some("Meu")).unwrap().id;
        (d, g, i)
    }

    #[test]
    fn no_document_tool_returns_a_path() {
        let (d, g, i) = lib_with_two();
        let home = d.to_string_lossy().to_string();
        for out in [list_documentos(&d, "curriculo").unwrap(), get_documento(&d, g).unwrap(), get_documento(&d, i).unwrap()] {
            for leak in [home.as_str(), "cv_g.pdf", "cv_g.html", "importado-", "arquivo", "sha256"] {
                assert!(!out.contains(leak), "{leak} leaked: {out}");
            }
        }
    }

    #[test]
    fn a_generated_document_comes_with_its_text_an_imported_one_says_why_not() {
        let (d, g, i) = lib_with_two();
        assert!(get_documento(&d, g).unwrap().contains("Ana Lima\\nRust & SQL"));
        let imp = get_documento(&d, i).unwrap();
        assert!(imp.contains("candidate_base.yaml"), "{imp}");
        assert!(get_documento(&d, 999).is_err());
    }

    #[test]
    fn marking_preferred_is_a_profile_session_write() {
        let (d, g, _) = lib_with_two();
        assert!(set_documento_preferido(&d, g, Some("backend"), SessionKind::Autonomous).unwrap_err().contains("sessão de Perfil"));
        let c = crate::documentos::biblioteca::open_conn(&d).unwrap();
        assert_eq!(crate::documentos::preferido(&c, "curriculo", Some("backend")), None);
        set_documento_preferido(&d, g, Some("backend"), SessionKind::Interactive).unwrap();
        assert_eq!(crate::documentos::preferido(&c, "curriculo", Some("backend")), Some(g));
    }
}
```

E em `mcp/mod.rs` `dispatch_tests`:

```rust
    #[test]
    fn document_and_variant_tools_are_reachable_through_dispatch() {
        let dir = temp_dir("dispatch-docs");
        seed_db(&dir);
        crate::db::apply(&rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap()).unwrap();
        std::fs::write(dir.join("search_variants.yaml"), "variantes:\n  - id: backend\n").unwrap();
        let cfg = cfg_for(&dir);
        assert_eq!(dispatch(&cfg, "list_documentos", &serde_json::json!({ "tipo": "curriculo" })).unwrap().trim(), "[]");
        assert!(dispatch(&cfg, "list_variantes", &serde_json::json!({})).unwrap().contains("backend"));
        assert!(dispatch(&cfg, "get_variante", &serde_json::json!({ "id": "backend" })).unwrap().contains("backend"));
        assert!(dispatch(&cfg, "get_documento", &serde_json::json!({ "id": 1 })).is_err());
        assert!(dispatch(&cfg, "update_variante", &serde_json::json!({ "id": "backend", "campos": { "peso": 5 } })).unwrap().contains("backend"));
        assert!(dispatch(&cfg, "set_documento_preferido", &serde_json::json!({ "id": 1 })).is_err(), "no such document");
    }
```

(`seed_db` aplica só o `schema.sql` cru; o `db::apply` extra garante as tabelas novas. Se der conflito de coluna duplicada, ele é ignorado pelo próprio `apply`.)

- [ ] **Step 4: Rodar e ver falhar.** `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-e cargo test --lib texto mcp` → FAIL.

- [ ] **Step 5: Implementar `texto.rs`:**

```rust
//! Plain text of a generated document's HTML, for the agent to read (get_documento). Never runs
//! anything: drops tags, scripts and styles, and decodes the few entities our templates emit.
pub fn html_para_texto(html: &str) -> String {
    // ASCII lowercasing keeps byte offsets, so `lower` and `html` index the same characters.
    let lower = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len() / 2);
    let mut i = 0;
    while i < html.len() {
        let rest = &lower[i..];
        if rest.starts_with("<script") || rest.starts_with("<style") {
            let close = if rest.starts_with("<script") { "</script>" } else { "</style>" };
            match rest.find(close) {
                Some(j) => {
                    i += j + close.len();
                    continue;
                }
                None => break,
            }
        }
        if bytes[i] == b'<' {
            let block = ["<br", "<p", "</p", "<div", "</div", "<li", "<h1", "<h2", "<h3", "<h4", "</h", "<tr"].iter().any(|t| rest.starts_with(t));
            match html[i..].find('>') {
                Some(j) => {
                    if block {
                        out.push('\n');
                    }
                    i += j + 1;
                    continue;
                }
                None => break,
            }
        }
        let ch = html[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    decoded
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 6: Implementar o escritor genérico em `profile.rs`.** As funções `backup_path`, `check_slots_are_writable`, `slide_backups`, `archive_current` e `rotate_backups` ganham o parâmetro `file: &str` (nome do arquivo em `data_dir`), trocando o literal `"candidate_base.yaml"` por `file`. Mensagens: `format!("{file}.bak-{n} é uma pasta; remova-a para o backup poder ser criado")` e `format!("erro ao criar backup de {file}: {e}")`. Nova:

```rust
/// Backs up, then replaces `file` (in data_dir) atomically. Shared by the profile and the variants.
pub(crate) fn write_yaml_atomically(data_dir: &Path, file: &str, content: &str) -> Result<(), String> {
    rotate_backups(data_dir, file)?;
    let tmp = data_dir.join(format!("{file}.tmp"));
    std::fs::write(&tmp, content).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("erro ao escrever ficheiro temporário: {e}")
    })?;
    std::fs::rename(&tmp, data_dir.join(file)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("erro ao gravar {file}: {e}")
    })
}

pub(crate) fn write_profile_atomically(data_dir: &Path, yaml: &str) -> Result<(), String> {
    write_yaml_atomically(data_dir, "candidate_base.yaml", yaml)
}
```

Os testes existentes de `profile.rs` têm de continuar verdes **sem mudar**; se algum conferir a frase antiga do backup, manter a frase antiga quando `file == "candidate_base.yaml"`.

- [ ] **Step 7: Implementar `mcp/tools/variantes.rs`:**

```rust
//! Variant tools (spec section 7): reads in both sessions; writes only in the Profile session, with the
//! same backup-then-atomic-replace as update_profile. Never returns cv_gerado_path (a file path).
use crate::commands::perfil::SearchVariant;
use crate::documentos::variantes;
use crate::mcp::SessionKind;
use std::path::Path;

const EDITAVEIS: &[&str] = &["nome_exibicao", "peso", "ativa", "principal", "foco_competencias", "foco_experiencia", "regioes_aceitas", "modelos_trabalho", "idiomas_aplicacao"];

fn sem_caminho(v: &SearchVariant) -> serde_json::Value {
    let mut j = serde_json::to_value(v).unwrap_or_default();
    if let Some(o) = j.as_object_mut() {
        o.remove("cv_gerado_path");
        o.remove("cv_gerado_em");
    }
    j
}

pub fn list_variantes(data_dir: &Path) -> Result<String, String> {
    let sv = variantes::ler(data_dir)?;
    let out: Vec<serde_json::Value> = sv
        .variantes
        .iter()
        .map(|v| serde_json::json!({
            "id": v.id, "nome": v.nome_exibicao, "peso": v.peso, "ativa": v.ativa, "principal": v.principal,
            "foco_competencias": v.foco_competencias, "foco_experiencia": v.foco_experiencia,
        }))
        .collect();
    serde_json::to_string_pretty(&out).map_err(|e| e.to_string())
}

pub fn get_variante(data_dir: &Path, id: &str) -> Result<String, String> {
    let sv = variantes::ler(data_dir)?;
    let v = sv.variantes.iter().find(|v| v.id == id).ok_or_else(|| format!("variante desconhecida: {id}"))?;
    serde_json::to_string_pretty(&sem_caminho(v)).map_err(|e| e.to_string())
}

pub fn update_variante(data_dir: &Path, id: &str, campos: &serde_json::Value, session: SessionKind) -> Result<String, String> {
    if session == SessionKind::Autonomous {
        return Err("As variantes só podem ser alteradas numa sessão de Perfil, onde o usuário acompanha a conversa. Para sugerir uma mudança, use propose_profile_change.".into());
    }
    let obj = campos.as_object().ok_or("'campos' tem de ser um objeto, ex.: {\"peso\": 60}")?;
    if let Some(k) = obj.keys().find(|k| !EDITAVEIS.contains(&k.as_str())) {
        return Err(format!("campo não editável: {k}. Editáveis: {}", EDITAVEIS.join(", ")));
    }
    let mut sv = variantes::ler(data_dir)?;
    let pos = sv.variantes.iter().position(|v| v.id == id).ok_or_else(|| format!("variante desconhecida: {id}"))?;
    let mut atual = serde_json::to_value(&sv.variantes[pos]).map_err(|e| e.to_string())?;
    for (k, v) in obj {
        atual[k] = v.clone();
    }
    let nova: SearchVariant = serde_json::from_value(atual).map_err(|e| format!("valor inválido: {e}"))?;
    // [P4] fatos_do_perfil(data_dir, &nova, obj)?;
    let principal = nova.principal;
    sv.variantes[pos] = nova;
    if principal {
        variantes::manter_uma_principal(&mut sv, id);
    }
    let yaml = serde_yaml::to_string(&sv).map_err(|e| e.to_string())?;
    super::profile::write_yaml_atomically(data_dir, "search_variants.yaml", &yaml)?;
    Ok(format!("Variante {id} atualizada ({}). A versão anterior fica em search_variants.yaml.bak-1.", obj.keys().cloned().collect::<Vec<_>>().join(", ")))
}
```

`[depende de P4]` — e, chamada no lugar do comentário:

```rust
/// A variant highlights what the profile already has; it never states a new fact (spec 7). Only the
/// fields this call changes are checked.
fn fatos_do_perfil(data_dir: &Path, v: &SearchVariant, mudou: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    let raw = std::fs::read_to_string(data_dir.join("candidate_base.yaml")).map_err(|e| format!("erro ao ler candidate_base.yaml: {e}"))?;
    let base = crate::commands::perfil::parse_candidato_base_str(&raw)?;
    if mudou.contains_key("foco_competencias") {
        let mut conhecidas: Vec<String> = base.competencias.iter().map(|c| c.to_lowercase()).collect();
        for e in &base.experiencia {
            conhecidas.extend(e.tecnologias.iter().map(|t| t.to_lowercase()));
        }
        for p in &base.projetos {
            conhecidas.extend(p.tecnologias.iter().map(|t| t.to_lowercase()));
        }
        if let Some(c) = v.foco_competencias.iter().find(|c| !conhecidas.contains(&c.to_lowercase())) {
            return Err(format!("\"{c}\" não está no perfil (candidate_base.yaml): uma variante só destaca o que o perfil já tem"));
        }
    }
    if mudou.contains_key("foco_experiencia") {
        let conhecidas: Vec<String> = base.experiencia.iter().flat_map(|e| [e.empresa.to_lowercase(), e.cargo.to_lowercase()]).collect();
        if let Some(x) = v.foco_experiencia.iter().find(|x| !conhecidas.contains(&x.to_lowercase())) {
            return Err(format!("\"{x}\" não é uma experiência do perfil (use a empresa ou o cargo como estão no candidate_base.yaml)"));
        }
    }
    Ok(())
}
```

- [ ] **Step 8: Implementar `mcp/tools/documentos.rs`:**

```rust
//! Document tools (spec section 7). Metadata and tags only: never a file name or a path. Marking a
//! preferred document is a Profile-session write.
use crate::documentos::{self, biblioteca};
use crate::mcp::SessionKind;
use std::path::Path;

fn tipo_ok(tipo: &str) -> Result<(), String> {
    if tipo == "curriculo" || tipo == "carta" { Ok(()) } else { Err(format!("tipo inválido: {tipo} (use curriculo ou carta)")) }
}

pub fn list_documentos(data_dir: &Path, tipo: &str) -> Result<String, String> {
    tipo_ok(tipo)?;
    let v = biblioteca::listar_view(&biblioteca::open_conn(data_dir)?, tipo)?;
    serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
}

pub fn get_documento(data_dir: &Path, id: i64) -> Result<String, String> {
    let d = biblioteca::detalhe(&biblioteca::open_conn(data_dir)?, data_dir, id)?;
    let texto = match (&d.documento.origem[..], &d.preview_html) {
        ("gerado", Some(h)) => documentos::texto::html_para_texto(h),
        ("gerado", None) => "(o HTML deste documento sumiu: gere de novo para ler o texto)".to_string(),
        _ => "(documento importado: o texto não é extraído; responda formulários pelo candidate_base.yaml)".to_string(),
    };
    serde_json::to_string_pretty(&serde_json::json!({ "documento": d.documento, "texto": texto })).map_err(|e| e.to_string())
}

pub fn set_documento_preferido(data_dir: &Path, id: i64, variante_id: Option<&str>, session: SessionKind) -> Result<String, String> {
    if session == SessionKind::Autonomous {
        return Err("Marcar o documento preferido só numa sessão de Perfil, onde o usuário acompanha a conversa.".into());
    }
    if let Some(v) = variante_id {
        crate::documentos::variantes::validar(data_dir, v)?;
    }
    documentos::definir_preferido(&biblioteca::open_conn(data_dir)?, id, variante_id)?;
    Ok(match variante_id {
        Some(v) => format!("Documento {id} é o preferido da variante {v}."),
        None => format!("Documento {id} é o preferido geral."),
    })
}
```

`[depende de P3]` — `save_cover_letter(data_dir, texto, vaga_id: Option<i64>, idioma, variante_id: Option<&str>) -> Result<String, String>`: valida `idioma` ∈ {`pt`,`en`} e a variante; lê `candidate_base.yaml` (`parse_candidato_base_str`); empresa/cargo vêm da vaga (`SELECT empresa, titulo FROM vagas WHERE id = ?`) ou ficam vazios; o HTML vem de `crate::commands::cover_letter::render_carta(&texto, &base, &empresa, &cargo, &idioma)` (wrapper `pub(crate)` novo em `cover_letter.rs` sobre `build_cover_letter_html` com a cor padrão `#D97757`); grava `documentos/pendentes/carta-<nanos>.html` e `.json` (`{ tipo: "carta", nome, idioma, variante_id, vaga_id, tags }`, mesmo formato que a frente D lê); `dispatch` chama `notify(cfg, "documentos")`; a tool consulta o banco a cada 500 ms, por até 20 s, por `arquivo = 'carta-<nanos>.pdf'` e responde `"Carta salva como documento N (PDF pronto)."` ou `"Carta salva; o PDF fica pronto quando o app processar (abra o app)."`. Testes: `save_cover_letter_leaves_a_pending_letter_and_no_path_in_the_answer` (sem app: resposta "fica pronto", arquivos em `pendentes/`, resposta sem caminho) e `save_cover_letter_refuses_an_unknown_variant`.

- [ ] **Step 9: Registrar.** `mcp/tools/mod.rs`: `mod documentos; mod variantes;` e `pub use documentos::{get_documento, list_documentos, set_documento_preferido}; pub use variantes::{get_variante, list_variantes, update_variante};`. `mcp/mod.rs` `dispatch`, braços novos:

```rust
        "list_variantes" => tools::list_variantes(&cfg.data_dir),
        "get_variante" => {
            let id = args["id"].as_str().ok_or("parâmetro 'id' em falta")?;
            tools::get_variante(&cfg.data_dir, id)
        }
        "update_variante" => {
            let id = args["id"].as_str().ok_or("parâmetro 'id' em falta")?;
            tools::update_variante(&cfg.data_dir, id, &args["campos"], cfg.session).inspect(|_| notify(cfg, "perfil"))
        }
        "list_documentos" => {
            let tipo = args["tipo"].as_str().unwrap_or("curriculo");
            tools::list_documentos(&cfg.data_dir, tipo)
        }
        "get_documento" => {
            let id = args["id"].as_i64().ok_or("parâmetro 'id' em falta")?;
            tools::get_documento(&cfg.data_dir, id)
        }
        "set_documento_preferido" => {
            let id = args["id"].as_i64().ok_or("parâmetro 'id' em falta")?;
            tools::set_documento_preferido(&cfg.data_dir, id, args["variante_id"].as_str(), cfg.session).inspect(|_| notify(cfg, "db"))
        }
```

`mcp/server.rs`: structs de argumentos (`IdTextoArgs { id: String }`, `UpdateVarianteArgs { id: String, campos: serde_json::Value }`, `TipoArgs { tipo: Option<String> }`, `IdArgs { id: i64 }`, `SetPreferidoArgs { id: i64, variante_id: Option<String> }`) com doc-comments em pt-BR e as tools:
- `list_variantes` — "Lista as variantes do candidato (id, nome, peso, ativa, principal, foco). A variante principal é a padrão quando nenhuma serve melhor à vaga."
- `get_variante` — "Devolve uma variante completa pelo id."
- `update_variante` — "Altera campos de uma variante (só na sessão de Perfil). 'campos' é um objeto só com o que muda, ex.: {\"peso\": 60}. Faz backup. Nunca acrescente competência ou experiência que não esteja no perfil."
- `list_documentos` — "Lista os currículos (tipo 'curriculo') ou cartas ('carta') da biblioteca: id, nome, origem, idioma, tags e marcas de preferido. Não há caminho de arquivo: para anexar, use browser_upload."
- `get_documento` — "Devolve os metadados de um documento e, se foi gerado pela Claudia, o texto dele. De um documento importado não há texto: use o candidate_base.yaml."
- `set_documento_preferido` — "Marca o documento preferido (geral, ou de uma variante com variante_id). Só na sessão de Perfil."
- `save_cover_letter` `[P3]`.

`tests/mcp-browser-smoke.mjs`: acrescentar as tools novas à lista exigida em `tools/list`.

- [ ] **Step 10: Rodar e ver passar.** `cargo test --lib` → exit 0; `cargo build` e `node src-tauri/tests/mcp-browser-smoke.mjs` → TUDO OK.

- [ ] **Step 11: Mutações:**
  - tirar o `if session == SessionKind::Autonomous` de `update_variante` → `update_variante_is_refused_in_the_execution_session` FAILED;
  - tirar o `o.remove("cv_gerado_path")` → `reads_never_return_a_path` FAILED;
  - em `write_yaml_atomically`, comentar `rotate_backups(...)` → `update_variante_writes_with_a_backup_and_keeps_one_principal` FAILED;
  - em `get_documento`, serializar o `Documento` cru (`documentos::obter`) em vez da view → `no_document_tool_returns_a_path` FAILED;
  - `[P4]` tirar a chamada de `fatos_do_perfil` → `a_variant_cannot_claim_what_the_profile_does_not_have` FAILED.

- [ ] **Step 12: Commit.** `git commit -m "feat(mcp): document and variant tools (read in both sessions, write only in the Profile session with backup; no paths ever)"`

---

### Tarefa 14c (Onda 2, frente F): acabamento do `browser_upload` (minors adiados)

**Files:**
- Modify: `src-tauri/src/mcp/tools/browser.rs`, `src-tauri/tests/browser_live.rs`

**Interfaces:** nenhuma pública muda. Dentro de `browser.rs`: `async fn finish_with(&self, tool, started, r, c, alvo: Option<(String, String)>)` (o `finish` atual delega com `None`).

- [ ] **Step 1: Testes live que falham** (em `browser_live.rs`, perto dos testes de upload; usam `Mem`, `tools_with_sink`, `fake_doc`, `fixture_file`, `ref_of` existentes):

```rust
#[tokio::test]
#[ignore]
async fn an_upload_result_ends_with_the_state_line() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap();
    assert!(out.lines().last().is_some_and(|l| l.starts_with("estado: ")), "{out}");
}

#[tokio::test]
#[ignore]
async fn a_failed_upload_is_logged_with_its_target() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone()).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Botão que não abre nada"), None, None).await.unwrap_err();
    let rec = mem.actions.lock().unwrap().iter().rev().find(|r| r.tool == "browser_upload").cloned().unwrap();
    assert_eq!(rec.target.as_deref(), Some("Botão que não abre nada"), "{rec:?}");
    assert!(rec.role.is_some());
}

#[tokio::test]
#[ignore]
async fn the_upload_message_and_the_log_use_the_same_size_unit() {
    let l = launch().await;
    let mem = Mem::new();
    let t = tools_with_sink(&l, mem.clone()).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap();
    let rec = mem.actions.lock().unwrap().iter().rev().find(|r| r.tool == "browser_upload").cloned().unwrap();
    let bytes = std::fs::metadata(&fake_doc("pdf").path).unwrap().len();
    assert!(out.contains(&format!("{bytes} bytes")), "{out}");
    assert!(rec.text.as_deref().unwrap_or("").contains(&format!("{bytes} bytes")), "{rec:?}");
}
```

(Se `ActionRecord` não for `Clone`/`Debug`, usar os campos diretamente dentro do `lock`; não mudar a visibilidade do tipo de produção.)

- [ ] **Step 2: Rodar e ver falhar.** `CLAUDIA_TEST_CHROME=%TEMP%\cft\chrome-win64\chrome.exe CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-f cargo test --test browser_live upload -- --ignored --test-threads=2` → os três novos FAILED.

- [ ] **Step 3: Implementar.**
  - Em `upload`, antes do `async {`: `let mut alvo: Option<(String, String)> = None;`; logo depois de `let n = self.resolve_ref(...)` : `alvo = Some((n.name.clone(), n.role.clone()));`; no fim: `self.finish_with("browser_upload", started, r, Some(&c), alvo).await`.
  - `finish_with` = o corpo atual de `finish`, com uma linha a mais no braço `Err(e)`: `if let Some((t, r)) = alvo { rec.target = Some(t); rec.role = Some(r); }`; `finish(...)` passa a ser `self.finish_with(tool, started, r, c, None).await`.
  - Tamanho em bytes nos dois lugares: a mensagem vira `format!("Anexou \"{}\" (arquivo {}, {}, {} bytes) pela regra: {}.", att.nome, file_name, att.formato, att.tamanho, att.regra)`; o `text` do log vira `format!("{} | {} | {} bytes | sha256 {}", ...)`.
  - Linha `estado:` (spec 8): depois de montar `message`, `if let Ok(ax) = c.call("Accessibility.getFullAXTree", json!({})).await { message.push('\n'); message.push_str(&Self::state_line(&c, &ax).await); }` (mesmo padrão de `open`).
  - Ajustar os testes live existentes que conferem `KB` para `bytes`.

- [ ] **Step 4: Rodar e ver passar.** Suíte live inteira: `CLAUDIA_TEST_CHROME=... cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon` → exit 0. `cargo test --lib` → exit 0.

- [ ] **Step 5: Mutações:** tirar a linha `rec.target = Some(t)` → `a_failed_upload_is_logged_with_its_target` FAILED; tirar o bloco do `estado:` → `an_upload_result_ends_with_the_state_line` FAILED.

- [ ] **Step 6: Commit.** `git commit -m "fix(upload): result ends with the estado line; failed uploads are logged with their target; one size unit in the message and the log"`

---

### Tarefa 15a (Onda 3, frente G): onde o documento aparece — backend `[depende de P5]`

**Files:**
- Modify: `src-tauri/src/db/schema.sql`, `src-tauri/src/db/mod.rs`, `src-tauri/src/db/queries.rs`, `src-tauri/src/mcp/tools/vagas.rs`, `src-tauri/src/mcp/tools/browser.rs`, `src-tauri/src/documentos/fonte.rs`

**Interfaces (contrato G ↔ H):**
- Tabela `anexos (id INTEGER PRIMARY KEY AUTOINCREMENT, vaga_id INTEGER NOT NULL REFERENCES vagas(id), tipo TEXT NOT NULL CHECK (tipo IN ('curriculo','carta')), documento_id INTEGER REFERENCES documentos(id) ON DELETE SET NULL, anexado_em TEXT NOT NULL DEFAULT (datetime('now')))`.
- Migração idempotente em `db::apply`: `ALTER TABLE candidaturas ADD COLUMN documento_id INTEGER REFERENCES documentos(id) ON DELETE SET NULL;` (na lista de `ALTER` com "duplicate column" ignorado).
- `trait DocumentSource` ganha `fn attached(&self, vaga_id: i64, tipo: &str, documento_id: i64) {}` (método com corpo padrão vazio: `NoDocuments` e o `FakeDocs` dos testes live não mudam); `DbDocumentSource::attached` grava em `anexos`; `upload` chama `self.docs.attached(vaga, tipo, att.id)` só no caminho de sucesso.
- `register_candidatura` grava `documento_id` = o `documento_id` do `anexos` mais recente com `tipo = 'curriculo'` daquela vaga (ou `NULL`).
- `VagaAtual` (JSON de `vaga_atual_sessao`) ganha `anexos: { tipo: "curriculo" | "carta", documento_id: number | null, nome: string | null, formato: string | null }[]` (o mais recente de cada tipo).
- `Candidatura` (JSON de `listar_candidaturas`) ganha `documento_id: number | null` e `documento_nome: string | null` (LEFT JOIN em `documentos`).

- [ ] **Step 1: Testes que falham:**
  - `db/mod.rs`: `schema_creates_all_tables` inclui `"anexos"`; `apply_is_idempotent` continua.
  - `fonte.rs`: `an_attachment_is_recorded_for_the_vaga` (`DbDocumentSource::attached(7, "curriculo", id)` → uma linha em `anexos`).
  - `vagas.rs`: `the_candidatura_remembers_the_cv_that_was_attached` (insere dois `anexos` de currículo e um de carta para a vaga 1; `register_candidatura` → `candidaturas.documento_id` = o currículo mais recente) e `without_an_attachment_the_candidatura_has_no_document`.
  - `queries.rs`: `the_current_vaga_lists_its_latest_attachments` e `history_names_the_document_used` (com o documento apagado depois: `documento_id` NULL e `documento_nome` NULL, sem erro — prova o `ON DELETE SET NULL`).
  - `browser_live.rs` não muda nesta tarefa (o `attached` padrão é vazio); a chamada no sucesso é conferida pela mutação abaixo com um teste unitário em `browser.rs` se já houver um `DocumentSource` de teste lá; se não houver, pela rodada de agente da Tarefa 16 (anotar "conferido no smoke").
- [ ] **Step 2: Rodar e ver falhar.** `CARGO_TARGET_DIR=C:/Users/joaog/.cache/claudia-p2-g cargo test --lib` → os novos FAILED.
- [ ] **Step 3: Implementar** conforme as interfaces acima (SQL na `schema.sql` com `CREATE TABLE IF NOT EXISTS anexos`; `register_candidatura` dentro da transação existente: `INSERT INTO candidaturas (..., documento_id) VALUES (..., (SELECT documento_id FROM anexos WHERE vaga_id = ?1 AND tipo = 'curriculo' ORDER BY id DESC LIMIT 1))`).
- [ ] **Step 4: Rodar e ver passar.** `cargo test --lib` → exit 0; suíte live → exit 0.
- [ ] **Step 5: Mutações:** tirar o `ON DELETE SET NULL` da coluna/ tabela → `history_names_the_document_used` FAILED (o `apagar_documento` falha com FK); trocar `ORDER BY id DESC` por `ASC` → `the_candidatura_remembers_the_cv_that_was_attached` FAILED.
- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): remember which document was attached to each vaga and candidatura"`

### Tarefa 15b (Onda 3, frente H): onde o documento aparece — frontend

**Files:** `src/components/dashboard/index.tsx`, `src/components/dashboard/types.ts`, `src/components/Vagas.tsx`, `src/components/Pendencias.tsx`, `src/components/ActivityLogScreen.tsx`, `src/components/Welcome.tsx`, `src/i18n/pt.ts`, `src/i18n/en.ts`

**Interfaces:** consome o contrato G ↔ H acima, `docsApi.listar` (Welcome) e a categoria `sem_documento` `[P6]`. Navegar para a aba: `onNavigate("documentos")` (Dashboard e Pendências já recebem um `onNavigate`/`onNavigateToPerfil`; Pendências ganha `onNavigateToDocumentos`, passado por `App.tsx` — a frente C já integrou `App.tsx` na Onda 1; nesta onda só a frente H o toca, uma linha).

- [ ] **Step 1: Teste que falha** — `tests/front/onde-aparece.test.ts` sobre funções puras novas em `src/components/dashboard/anexos.ts`: `rotuloAnexo(a)` → `"CV Backend (PDF)"`; sem nome → `t.documents...` não (função pura recebe o texto "documento apagado" como argumento); `ehSemDocumento(categoria)` → `true` para `sem_documento`.
- [ ] **Step 2: Implementar:**
  - **Dashboard / vaga atual:** abaixo do título da vaga em `candidatando`, uma linha por anexo (`rotuloAnexo`), clicável → aba Currículos.
  - **Histórico (`Vagas.tsx`):** cada candidatura mostra `documento_nome` (ou "documento apagado" se `documento_id` nulo e houve anexo; "—" se nunca houve).
  - **Pendências:** `CATEGORIA_META` ganha `sem_documento` (ícone `FileWarning`, cor `warning`) com o botão "Abrir Currículos" que chama `onNavigateToDocumentos`.
  - **Activity log:** nas linhas `browser_upload`, a coluna de alvo mostra `r.text` (nome | formato) no lugar de `r.target` quando existir; o detalhe mostra os dois.
  - **Welcome:** no passo final, se `docsApi.listar("curriculo")` vier vazio, um aviso "Para anexar a candidaturas, gere ou importe ao menos um currículo" com o botão que leva à aba (via `onComplete("documentos")`).
  - i18n pt/en para tudo.
- [ ] **Step 3: Verificar.** `node --test tests/front/` → exit 0; `npx tsc --noEmit` → exit 0; `npm run build` → exit 0.
- [ ] **Step 4: Mutação:** `ehSemDocumento` retornando `false` sempre → teste FAILED.
- [ ] **Step 5: Commit.** `git commit -m "feat(ui): the attached document shows on the Dashboard, History, Pendências, Activity log; Welcome asks for a CV"`

---

### Tarefa 14b (Onda 3, frente I): ensinar o agente — **BLOQUEADA até autorização explícita do usuário**

**Files:** `src-tauri/src/prompt_sistema_runtime.md`, `src-tauri/src/commands/prompts.rs`

- [ ] **Step 1: Pedir autorização**, mostrando o texto exato abaixo. Sem um "pode editar o prompt", **pular** e registrar no ledger que o agente não conhece as tools novas (elas existem mas o agente não é instruído a usá-las).
- [ ] **Step 2: Texto proposto.**
  - Na linha 28 (`... use o CV dessa variante (campo cv_gerado_path), gerando-o via skill tailor-application se ainda não existir ...`), trocar o trecho do `cv_gerado_path` por: "ao processar uma vaga, decida qual variante ativa melhor corresponde (por área/região/modelo de trabalho) e grave-a com `set_vaga_variante` (ou no `variante_id` de `register_vaga`/`update_vaga_status`). Se nenhuma servir melhor, não grave: vale a variante principal do usuário. É a variante gravada que decide qual currículo `browser_upload` anexa." (o resto da linha, sobre `preferencias_globais` e `red_lines`, fica; o destino da menção à `tailor-application` depende do Step 0 da Tarefa 14.)
  - Depois do bullet de `browser_upload`: "- `list_documentos` e `get_documento` mostram os currículos e cartas da biblioteca (id, tags, preferidos) e o texto dos que a Claudia gerou; use-os para responder perguntas do formulário que dependem do currículo. Nunca há caminho de arquivo."
  - `[P3]` "- Se a vaga pedir carta e não houver uma adequada, escreva-a e grave com `save_cover_letter` (texto, `vaga_id`, idioma, variante); depois anexe com `browser_upload` e `tipo: \"carta\"`."
  - `[P6]` No bullet de `browser_upload`, trocar "registre uma pendência" por "registre uma pendência com a categoria `sem_documento`".
- [ ] **Step 3: Teste do prompt** em `commands/prompts.rs`: acrescentar `"set_vaga_variante"`, `"list_documentos"`, `"get_documento"` (e `"save_cover_letter"` com P3) à lista de `the_shipped_runtime_prompt_no_longer_sends_the_agent_to_the_extension`, e um `assert!(!p.contains("cv_gerado_path"), "the agent no longer looks for a CV path")`. Rodar → PASS.
- [ ] **Step 4: Mutação:** devolver a menção a `cv_gerado_path` → o teste FAILED.
- [ ] **Step 5: Commit.** `git commit -m "docs(prompt): teach the vaga variant and the document tools (approved by the user)"`

---

### Tarefa 16 (Onda 4, orquestrador Opus): rodadas de teste da Parte 2 e entrega

Seguir o `CLAUDE.md`, seções 3 a 6. O orquestrador **roda** cada comando e confere o exit code; não repassa "passou" de agente.

- [ ] **Step 1: Unitários e regressão:** `cd src-tauri && cargo test --lib` (exit 0, anotar a contagem), `cargo clippy --lib` (sem aviso novo), `npx tsc --noEmit` (0), `node --test tests/front/` (0), `npm run build` (0) e `grep -rl "mock-documentos-dev-only" dist/` (nenhum).
- [ ] **Step 2: Integração:** suíte live inteira (`--skip icon`, exit 0), `node src-tauri/tests/mcp-browser-smoke.mjs` (TUDO OK).
- [ ] **Step 3: Rodada de agente** (Haiku executa, Opus observa; Chrome de teste, pasta de dados temporária, `CLAUDIA_ALLOW_LOCAL_URLS`, MCP do binário): numa sessão de Perfil, `list_variantes`, `update_variante` (peso), `list_documentos`, `set_documento_preferido`; numa sessão de execução, `register_vaga` com `variante_id`, `set_vaga_variante` com variante inventada (recusa), `update_vaga_status` para `candidatando`, `browser_upload` em `upload-form.html` (a regra diz a variante e termina em `estado:`), `update_variante` (recusa). O Opus cruza transcript, Activity log (linha de upload com nome | formato | bytes | sha curto, **sem conteúdo**; falha com alvo) e prints.
- [ ] **Step 4: Smoke de uso real no app** (`/claudia-teste`), **com autorização do usuário para escrever nos dados reais nesta rodada** (gerar e importar documentos escreve em `%APPDATA%`; antes, backup de `claudia_rh.db`, `search_variants.yaml` e da pasta `documentos/`): sidebar → Currículos; sub-abas; estado vazio; importar um PDF; gerar um currículo (nenhuma janela aparece; o PDF existe; `banco.mjs "SELECT id, nome, origem, formato, arquivo FROM documentos"`); abrir a página, Voltar; tags; preferido geral e por variante (a estrela persiste ao reabrir o app); renomear; excluir um preferido (aviso; arquivos somem da pasta; `preferidos` sem a linha); Perfil → seletores de preferido da variante; migração (os `curriculos/*.html` antigos aparecem como documentos; a pasta antiga continua igual: `dir` antes e depois); os dois idiomas; prints de cada estado. Sem autorização: declarar "não verificado no app real" e rodar só o Step 10 da Tarefa 13 (mock).
- [ ] **Step 5: Upload num site real — só com autorização do usuário para a rodada**, nunca enviando. Sem ela: "não testado em site real".
- [ ] **Step 6: Review Opus** do diff da Parte 2 inteiro (segurança do apagar e do preview, ausência de caminho no MCP, sessão nas escritas, idempotência da migração, PDF sem janela). Corrigir ou registrar os achados no ledger e no Linear.
- [ ] **Step 7: Linear:** tasks para o que ficou de fora (o TOCTOU entre o hash e a leitura preguiçosa do Chrome, minor adiado da Parte 1, segue registrado como risco aceito; flake `a_wide_but_short_window_grows_in_height_only`; o que o Step 0 da Tarefa 14 achar).
- [ ] **Step 8: Entrega.** Relatório ao usuário: o que foi feito, evidências (exit codes, linhas `FAILED` das mutações, prints), o que não foi verificado, as PROPOSTAS e o que se decidiu. **Integração (merge, push, PR) é decisão do usuário.**

---

## Review Focus da Parte 2

1. **Apagar um documento que é o preferido** (ou um registro estragado apontando para fora da pasta) → só os arquivos daquele documento, dentro de `documentos/`, somem; as marcas de preferido vão junto; nada fora da pasta é tocado. (Tarefa 11: `deleting_removes_its_files_and_its_preferred_marks_only`, `deleting_never_touches_a_file_outside_the_folder`; Onda 0: `open_conn_turns_foreign_keys_on`.)
2. **Migração na abertura do app** que roda duas vezes, para no meio ou falha num arquivo → nenhum original apagado ou alterado, nenhum documento duplicado, o que falhou é tentado de novo. (Tarefa 12: `old_cvs_and_letters_become_documents_and_the_originals_stay`, `running_twice_migrates_once`, `a_failed_pdf_skips_that_file_and_the_rest_still_migrate`, `a_half_done_migration_is_retried`.)
3. **PDF que falha ou sai inválido** (WebView2 travado, disco cheio) → nenhum documento meio-registrado que o upload tentaria anexar, nenhum arquivo órfão, erro claro na tela; e nenhuma janela aparece. (Tarefa 9b: `a_failed_pdf_registers_nothing_and_leaves_no_files`, `an_invalid_pdf_is_refused`; Tarefa 9, Step 5: print da área de trabalho.)
4. **Variante errada ou ausente** → o agente não consegue gravar uma variante inventada (e o status não anda junto); sem variante na vaga vale a principal, e o resultado do upload diz qual variante e por quê; duas principais é impossível. (Tarefa 10: `an_unknown_variant_is_refused_and_nothing_is_written`, `without_a_vaga_variant_the_principal_is_used`, `the_vagas_own_variant_beats_the_principal_and_the_result_says_so`, `marking_a_principal_unmarks_the_others`.)
5. **A sessão de execução tentando mudar o perfil, ou uma tool vazando caminho** → `update_variante` e `set_documento_preferido` recusam fora da sessão de Perfil sem tocar no arquivo; nenhuma resposta de tool traz pasta, nome de arquivo ou `cv_gerado_path`. (Tarefa 14: `update_variante_is_refused_in_the_execution_session`, `marking_preferred_is_a_profile_session_write`, `no_document_tool_returns_a_path`, `reads_never_return_a_path`.)

## Self-Review (writing-plans)

**Cobertura da spec:**

| Spec | Onde |
|---|---|
| 2: aba própria, sub-abas, grade, página com Voltar, A4 HTML | 13 |
| 2: importar o próprio; preferido por variante; tags (variante, template, cor, idioma, livres) | Parte 1 (importar) + 0 (tags) + 9b (tags automáticas) + 11 + 13 |
| 2: MCP de variantes, cartas, documentos; `browser_upload` | Parte 1 + 14 + 14c |
| 2.1.1: agente decide a variante; principal como padrão | 10 (+ 13 [P2]) |
| 2.1.2: PDF pelo WebView2, sem Chrome nem runtime | 9 |
| 2.1.3: importado sem extração de texto | 14 (`get_documento` explica) |
| 2.1.5: Chrome nunca falha por upload | Parte 1 (mantido; 14c não mexe na mecânica) |
| 3: tabelas, `variante_id` como campo, `principal`, migração sem apagar | Parte 1 + 10 + 12 |
| 4: regra de escolha e "diz qual variante e regra" | Parte 1 + 10 (`fonte.rs`) |
| 5: HTML com moldura A4, `iframe sandbox` `srcdoc`, miniatura lazy, importado como cartão | 13 |
| 6: todos os comandos e o JSON `Documento`; remoção dos antigos | 0 + 9b + 11 + 13 |
| 7: tools e sessão; nenhuma com caminho | 14 (+ `save_cover_letter` [P3]) + 10 (`set_vaga_variante`) |
| 8: `estado:`, log sem conteúdo | 14c |
| 9: prompt só com aprovação | 14b (bloqueada) |
| 10: Sidebar, aba, página, Perfil, Dashboard, Histórico, Pendências, Activity log, Welcome, i18n | 13 + 15b |
| 11: segurança (id, canonicalize, tipo real, sandbox, log) | 0 + 11 + 13 + 14 + 14c |
| 13: investigar `tailor-application` | 14, Step 0 |
| 14: unidade, live, mutação, `claudia-teste` | cada tarefa + 16 |

Fora do plano por decisão do usuário: extração de texto de importados, vagas acumuladas em `candidatando` (CLA-35).

**Sem placeholders:** os pontos que a spec não decide estão nomeados (P1–P7) e cada passo que depende deles está marcado; os layouts da Tarefa 13 são descritos por regra (estado, chamada, texto) e conferidos pelo smoke, não por código. Nenhum "TBD"/"implementar depois".

**Consistência de tipos entre tarefas:**
- `DocumentoView` (Rust, Onda 0) ↔ `Documento` (TS, 13): mesmos 13 campos, mesmos nomes; `idioma` `Option<String>` ↔ `string | null`; `tags: Vec<Tag>` ↔ `Tag[]`.
- `registrar_gerado(data_dir, NovoGerado, print)` igual em 9b (produz) e 12 (consome); `print` é `FnOnce(PathBuf, PathBuf)` (12 passa `&F` com `F: Fn`).
- `biblioteca::open_conn` usado por 9b, 11, 12, 14 — sempre com FK ligada.
- `variantes::{ler, validar, principal, principal_de, manter_uma_principal}` (10) consumidos por 14 com as mesmas assinaturas.
- Comandos: nomes e chaves camelCase da tabela do contrato = `documentos-api.ts` (13) = `lib.rs` (Onda 0).
- `regra` do upload: string livre (Parte 1), agora com "(variante X, gravada na vaga | a principal)" ou "(sem variante)" — conferido por `contains`, não por igualdade, nos testes de 10.
- `register_vaga`/`update_vaga_status` ganham `variante_id: Option<&str>` como **último** parâmetro em 10; a chamada em `dispatch` e os testes existentes são atualizados na mesma tarefa.
