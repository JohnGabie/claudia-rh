// Profile tools — validate-then-write for candidate_base.yaml.

use std::path::Path;

use crate::commands::perfil::CandidatoBase;
use crate::mcp::SessionKind;

/// Reads one of the candidate's configuration files from data_dir.
///
/// Fails loud when the file is absent. The prompt path these tools replace used
/// unwrap_or_default(), so a missing file became an empty string and the model
/// worked from a blank profile without ever knowing — inventing CV data is worse
/// than refusing to start.
fn read_data_file(data_dir: &Path, name: &str) -> Result<String, String> {
    std::fs::read_to_string(data_dir.join(name))
        .map_err(|e| format!("erro ao ler {name}: {e}"))
}

/// Reads candidate_base.yaml so the runtime prompt no longer has to carry it.
pub fn get_candidate_profile(data_dir: &Path) -> Result<String, String> {
    read_data_file(data_dir, "candidate_base.yaml")
}

/// Reads search_variants.yaml — the search terms the session should try.
pub fn get_search_variants(data_dir: &Path) -> Result<String, String> {
    read_data_file(data_dir, "search_variants.yaml")
}

/// Reads strategy.md — the candidate's standing preferences about what to apply to.
pub fn get_strategy(data_dir: &Path) -> Result<String, String> {
    read_data_file(data_dir, "strategy.md")
}

/// How many previous versions of the profile we keep.
const BACKUP_DEPTH: usize = 5;

fn backup_path(data_dir: &Path, n: usize) -> std::path::PathBuf {
    data_dir.join(format!("candidate_base.yaml.bak-{n}"))
}

/// Rejects the rotation before it mutates anything if a slot is occupied by a
/// directory. Renaming a file onto a non-empty directory fails, and the sliding
/// loop has no rollback — without this check a mid-loop failure would destroy
/// the oldest generation and leave a hole in the numbering, for a write that
/// never happened.
fn check_slots_are_writable(data_dir: &Path) -> Result<(), String> {
    for n in 1..=BACKUP_DEPTH {
        let slot = backup_path(data_dir, n);
        if slot.is_dir() {
            return Err(format!(
                "candidate_base.yaml.bak-{n} é uma pasta; remova-a para o backup poder ser criado"
            ));
        }
    }
    Ok(())
}

/// Slides candidate_base.yaml.bak-N to .bak-(N+1). The oldest backup falls off
/// the end. Does not touch the live profile — see `archive_current`.
fn slide_backups(data_dir: &Path) -> Result<(), String> {
    // Walk down so each slot is free before we move into it.
    for n in (1..BACKUP_DEPTH).rev() {
        let from = backup_path(data_dir, n);
        if from.exists() {
            std::fs::rename(&from, backup_path(data_dir, n + 1))
                .map_err(|e| format!("erro ao rodar backup {n}: {e}"))?;
        }
    }
    Ok(())
}

/// Copies the live profile into .bak-1.
///
/// A copy, not a move: the profile must never be absent from its canonical
/// path. Moving it away meant a later failure — a full disk, a scanner holding
/// the file — left the app with no profile at all, and the error message said
/// nothing about where it went.
fn archive_current(data_dir: &Path) -> Result<(), String> {
    std::fs::copy(data_dir.join("candidate_base.yaml"), backup_path(data_dir, 1))
        .map(|_| ())
        .map_err(|e| format!("erro ao criar backup do perfil: {e}"))
}

/// Preserves the current profile before it is overwritten.
///
/// A missing profile is not an error: the first write has nothing to preserve.
fn rotate_backups(data_dir: &Path) -> Result<(), String> {
    if !data_dir.join("candidate_base.yaml").exists() {
        return Ok(());
    }
    check_slots_are_writable(data_dir)?;
    slide_backups(data_dir)?;
    archive_current(data_dir)
}

/// Backs up, then replaces candidate_base.yaml atomically.
///
/// Every path that overwrites the profile goes through here — the MCP tool and
/// the Perfil tab's form saves alike. The form path used to be a bare
/// fs::write: no backup, and a crash mid-save left a truncated profile with
/// nothing to fall back on.
pub(crate) fn write_profile_atomically(data_dir: &Path, yaml: &str) -> Result<(), String> {
    // No backup, no write.
    rotate_backups(data_dir)?;

    let tmp = data_dir.join("candidate_base.yaml.tmp");
    std::fs::write(&tmp, yaml).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("erro ao escrever ficheiro temporário: {e}")
    })?;
    std::fs::rename(&tmp, data_dir.join("candidate_base.yaml")).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("erro ao gravar candidate_base.yaml: {e}")
    })
}

/// The blocks whose size we report. Names are the user-facing singular/plural
/// pair, in pt-BR, because this string is read by the model and shown to the user.
fn block_counts(base: &CandidatoBase) -> [(&'static str, &'static str, usize); 6] {
    [
        ("experiência", "experiências", base.experiencia.len()),
        ("projeto", "projetos", base.projetos.len()),
        ("formação", "formações", base.formacao.len()),
        ("competência", "competências", base.competencias.len()),
        ("idioma", "idiomas", base.idiomas.len()),
        ("gap", "gaps", base.gaps_conhecidos.len()),
    ]
}

/// Personal fields worth naming when they go from filled to blank. `links` is a
/// list and is not covered here; losing every link is rare and the block counts
/// do not track it — accepted gap, documented rather than silently ignored.
fn personal_fields(base: &CandidatoBase) -> [(&'static str, &str); 8] {
    let d = &base.dados_pessoais;
    [
        ("nome_completo", d.nome_completo.as_str()),
        ("email", d.email.as_str()),
        ("telefone", d.telefone.as_str()),
        ("localizacao_atual", d.localizacao_atual.as_str()),
        ("endereco", d.endereco.as_str()),
        ("nacionalidade", d.nacionalidade.as_str()),
        ("data_nascimento", d.data_nascimento.as_str()),
        ("cpf", d.cpf.as_str()),
    ]
}

/// What a write did, and whether anything was lost doing it.
struct ChangeSummary {
    text: String,
    lossy: bool,
}

/// What was on disk before this write.
enum Previous<'a> {
    /// No profile existed.
    None,
    /// A profile existed but could not be parsed, so there is nothing to diff
    /// against. Distinct from None: this write REPLACES something.
    Unreadable,
    Parsed(&'a CandidatoBase),
}

/// Builds the sentence the model gets back after a write.
///
/// Counts, not a textual diff: what matters is what disappeared, not how the
/// YAML was reformatted. The minus sign is U+2212, so it survives terminals
/// that would swallow a leading hyphen.
///
/// `bytes_identical` is what separates "nothing happened" from "everything was
/// rewritten and the counts happen to match" — the counts alone cannot tell
/// those apart, and the second one is a data loss.
fn describe_change(
    prev: Previous<'_>,
    next: &CandidatoBase,
    bytes_identical: bool,
) -> ChangeSummary {
    let prev = match prev {
        Previous::None => {
            let created: Vec<String> = block_counts(next)
                .iter()
                .filter(|(_, _, n)| *n > 0)
                .map(|(one, many, n)| format!("{n} {}", if *n == 1 { one } else { many }))
                .collect();
            let text = if created.is_empty() {
                "Perfil criado, ainda sem conteúdo.".to_string()
            } else {
                format!("Perfil criado: {}.", created.join(", "))
            };
            return ChangeSummary { text, lossy: false };
        }
        Previous::Unreadable => {
            return ChangeSummary {
                text: "Perfil gravado. O ficheiro anterior não era legível, por isso não há \
                       comparação — verifique o backup antes de continuar."
                    .to_string(),
                lossy: true,
            };
        }
        Previous::Parsed(p) => p,
    };

    let before = block_counts(prev);
    let after = block_counts(next);
    let mut changes: Vec<String> = Vec::new();
    let mut lossy = false;
    for (i, (one, many, new_n)) in after.iter().enumerate() {
        let old_n = before[i].2;
        let delta = *new_n as i64 - old_n as i64;
        if delta == 0 {
            continue;
        }
        let magnitude = delta.unsigned_abs();
        let noun = if magnitude == 1 { one } else { many };
        let sign = if delta > 0 {
            "+"
        } else {
            lossy = true;
            "−"
        };
        changes.push(format!("{sign}{magnitude} {noun}"));
    }

    let prev_personal = personal_fields(prev);
    let blanked: Vec<&str> = personal_fields(next)
        .iter()
        .enumerate()
        .filter(|(i, (_, value))| value.trim().is_empty() && !prev_personal[*i].1.trim().is_empty())
        .map(|(_, (name, _))| *name)
        .collect();
    if !blanked.is_empty() {
        lossy = true;
        changes.push(format!("apagado(s) em dados_pessoais: {}", blanked.join(", ")));
    }

    if changes.is_empty() {
        // Counts and personal scalars see only a slice of the profile: nothing
        // here tracks descriptions, achievements, technologies, respostas_modelo
        // or links. A body gutted entry by entry moves none of them. Saying
        // "sem alterações" there would be a false statement about the one thing
        // the user relies on this message for.
        return if bytes_identical {
            ChangeSummary {
                text: "Perfil gravado, sem alterações de conteúdo.".to_string(),
                lossy: false,
            }
        } else {
            ChangeSummary {
                text: "Perfil gravado. O conteúdo foi reescrito sem mudar as contagens — \
                       confirme o que ficou."
                    .to_string(),
                lossy: true,
            }
        };
    }
    ChangeSummary { text: format!("Perfil gravado. {}.", changes.join(", ")), lossy }
}

/// Validates the full candidate_base.yaml content against the serde structs
/// BEFORE writing. Invalid YAML never reaches disk; the parse error goes back
/// to the model so it can self-correct.
///
/// Parsing alone is not enough. Every field of CandidatoBase carries
/// #[serde(default)] (see commands/perfil.rs) so that legacy Claude-written
/// YAMLs keep loading — which means a one-line document deserializes into a
/// complete, empty profile. That is exactly how the 2026-09-25 wipe happened.
/// The defenses here are the backup and the diff, not the parse.
///
/// Only an interactive session may write: the diff lands in a conversation the
/// user is reading, which is what makes a destructive write visible at all.
pub fn update_profile(
    data_dir: &Path,
    yaml: &str,
    session: SessionKind,
) -> Result<String, String> {
    if session == SessionKind::Autonomous {
        return Err(
            "O perfil só pode ser alterado numa sessão de Perfil, onde o usuário acompanha a \
             conversa. Para registar uma mudança que o usuário deve rever, use \
             propose_profile_change."
                .to_string(),
        );
    }
    if yaml.trim().is_empty() {
        return Err("YAML vazio — envie o conteúdo completo do candidate_base.yaml".to_string());
    }
    let parsed = crate::commands::perfil::parse_candidato_base_str(yaml)?;

    let path = data_dir.join("candidate_base.yaml");
    let previous_raw = std::fs::read_to_string(&path).ok();
    let previous_parsed = previous_raw
        .as_deref()
        .and_then(|raw| crate::commands::perfil::parse_candidato_base_str(raw).ok());
    let previous = match (&previous_raw, &previous_parsed) {
        (None, _) => Previous::None,
        (Some(_), None) => Previous::Unreadable,
        (Some(_), Some(p)) => Previous::Parsed(p),
    };
    let bytes_identical = previous_raw.as_deref() == Some(yaml);

    write_profile_atomically(data_dir, yaml)?;

    let summary = describe_change(previous, &parsed, bytes_identical);
    Ok(if summary.lossy {
        format!("{}\nAnterior em candidate_base.yaml.bak-1.", summary.text)
    } else {
        summary.text
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::temp_dir;

    #[test]
    fn rejects_invalid_yaml() {
        let dir = temp_dir("prof-invalid");
        let err = update_profile(&dir, "experiencia: [ { empresa: 'x'", SessionKind::Interactive).unwrap_err();
        assert!(err.contains("YAML parse error"), "got: {err}");
        assert!(!dir.join("candidate_base.yaml").exists(), "invalid YAML must not be written");
    }

    #[test]
    fn rejects_empty_yaml() {
        let dir = temp_dir("prof-empty");
        assert!(update_profile(&dir, "   \n", SessionKind::Interactive).is_err());
    }

    #[test]
    fn get_profile_returns_file_contents() {
        let dir = temp_dir("prof-get");
        let yaml = "dados_pessoais:\n  nome_completo: \"Maria\"\n";
        std::fs::write(dir.join("candidate_base.yaml"), yaml).unwrap();
        assert_eq!(get_candidate_profile(&dir).unwrap(), yaml);
    }

    /// The prompt path used unwrap_or_default(), so a missing profile became an
    /// empty string and the model invented data without knowing. A read tool
    /// must fail loud instead.
    #[test]
    fn get_profile_fails_when_file_missing() {
        let dir = temp_dir("prof-get-missing");
        let err = get_candidate_profile(&dir).unwrap_err();
        assert!(err.contains("candidate_base.yaml"), "got: {err}");
    }

    #[test]
    fn get_search_variants_returns_file_contents() {
        let dir = temp_dir("variants-get");
        let yaml = "variantes:\n  - termo: \"rust developer\"\n";
        std::fs::write(dir.join("search_variants.yaml"), yaml).unwrap();
        assert_eq!(get_search_variants(&dir).unwrap(), yaml);
    }

    #[test]
    fn get_search_variants_fails_when_file_missing() {
        let dir = temp_dir("variants-missing");
        let err = get_search_variants(&dir).unwrap_err();
        assert!(err.contains("search_variants.yaml"), "got: {err}");
    }

    #[test]
    fn get_strategy_returns_file_contents() {
        let dir = temp_dir("strategy-get");
        let md = "# Estratégia\n\nPriorizar vagas remotas.\n";
        std::fs::write(dir.join("strategy.md"), md).unwrap();
        assert_eq!(get_strategy(&dir).unwrap(), md);
    }

    #[test]
    fn get_strategy_fails_when_file_missing() {
        let dir = temp_dir("strategy-missing");
        let err = get_strategy(&dir).unwrap_err();
        assert!(err.contains("strategy.md"), "got: {err}");
    }

    fn base_from(yaml: &str) -> CandidatoBase {
        crate::commands::perfil::parse_candidato_base_str(yaml).unwrap()
    }

    #[test]
    fn diff_reports_losses_per_block() {
        let prev = base_from(
            "experiencia:\n  - empresa: A\n  - empresa: B\ncompetencias:\n  - Rust\n  - Go\n",
        );
        let next = base_from("experiencia:\n  - empresa: A\n");
        let out = describe_change(Previous::Parsed(&prev), &next, false);
        assert!(out.text.contains("−1 experiência"), "got: {}", out.text);
        assert!(out.text.contains("−2 competências"), "got: {}", out.text);
        assert!(out.lossy);
    }

    #[test]
    fn diff_reports_gains() {
        let prev = base_from("experiencia:\n  - empresa: A\n");
        let next = base_from("experiencia:\n  - empresa: A\n  - empresa: B\n");
        let out = describe_change(Previous::Parsed(&prev), &next, false);
        assert!(out.text.contains("+1 experiência"), "got: {}", out.text);
        assert!(!out.lossy, "a pure addition is not lossy");
    }

    /// Review Focus 3: re-saving the same content must not invent movement.
    #[test]
    fn diff_says_nothing_changed_when_nothing_changed() {
        let yaml = "experiencia:\n  - empresa: A\ncompetencias:\n  - Rust\n";
        let prev = base_from(yaml);
        let next = base_from(yaml);
        let out = describe_change(Previous::Parsed(&prev), &next, true);
        assert!(out.text.contains("sem alterações"), "got: {}", out.text);
        assert!(!out.lossy);
    }

    /// Review Focus 4: block counts are unchanged, but personal data vanished.
    /// This is the case that a "does the text contain a minus sign" check would miss.
    #[test]
    fn diff_flags_personal_fields_that_went_blank() {
        let prev =
            base_from("dados_pessoais:\n  nome_completo: Maria\n  cpf: \"000.111.222-33\"\n");
        let next = base_from("dados_pessoais:\n  nome_completo: Maria\n");
        let out = describe_change(Previous::Parsed(&prev), &next, false);
        assert!(out.text.contains("cpf"), "got: {}", out.text);
        assert!(
            !out.text.contains("nome_completo"),
            "unchanged field must stay quiet; got: {}",
            out.text
        );
        assert!(out.lossy, "losing a personal field is a loss even with no block change");
    }

    /// Review Focus 1: first write ever — there is no previous profile to compare against.
    #[test]
    fn diff_describes_a_first_write() {
        let next = base_from("experiencia:\n  - empresa: A\n");
        let out = describe_change(Previous::None, &next, false);
        assert!(out.text.contains("Perfil criado"), "got: {}", out.text);
        assert!(!out.lossy, "a first write cannot lose anything");
    }

    /// I1: the Perfil tab's form saves went through a bare fs::write — no
    /// backup, no temp file. Roughly half the profile writes a real user makes.
    /// Spec objective 1 is unqualified: no write destroys the previous state
    /// without leaving a copy.
    #[test]
    fn the_shared_writer_backs_up_and_is_atomic() {
        let dir = temp_dir("prof-shared-writer");
        let original = "experiencia:\n  - empresa: A\n";
        write_profile_atomically(&dir, original).unwrap();
        write_profile_atomically(&dir, "experiencia: []\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(backup_path(&dir, 1)).unwrap(),
            original,
            "the shared writer must leave a backup"
        );

        std::fs::create_dir(dir.join("candidate_base.yaml.tmp")).unwrap();
        assert!(write_profile_atomically(&dir, "experiencia:\n  - empresa: C\n").is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap(),
            "experiencia: []\n",
            "a failed write must leave the profile where it was"
        );
    }

    /// C1: block counts and personal scalars can both be unchanged while the
    /// body of every entry is gutted. Claiming "sem alterações" there is worse
    /// than the wipe this branch fixes — that one at least announced zeros.
    #[test]
    fn write_that_guts_entry_bodies_is_not_reported_as_unchanged() {
        let dir = temp_dir("prof-gutted");
        let full = "experiencia:\n  - empresa: A\n    cargo: Dev\n    descricao: \"Liderou a migração\"\n    conquistas:\n      - \"Reduziu custos 40%\"\nrespostas_modelo:\n  pretensao_salarial_texto: \"8000 EUR\"\n";
        let gutted = "experiencia:\n  - empresa: A\n    cargo: Dev\n";
        update_profile(&dir, full, SessionKind::Interactive).unwrap();
        let msg = update_profile(&dir, gutted, SessionKind::Interactive).unwrap();
        assert!(!msg.contains("sem alterações"), "content was lost; got: {msg}");
        assert!(msg.contains("bak-1"), "a lossy write must point at the backup; got: {msg}");
    }

    /// C1, other half: a byte-identical re-save must still stay quiet.
    #[test]
    fn identical_bytes_are_still_reported_as_unchanged() {
        let dir = temp_dir("prof-identical");
        let yaml = "experiencia:\n  - empresa: A\n    descricao: \"x\"\n";
        update_profile(&dir, yaml, SessionKind::Interactive).unwrap();
        let msg = update_profile(&dir, yaml, SessionKind::Interactive).unwrap();
        assert!(msg.contains("sem alterações"), "got: {msg}");
        assert!(!msg.contains("bak-1"), "nothing was lost; got: {msg}");
    }

    /// C2: the profile must never be absent from its canonical path. Rotating the
    /// live file away before writing turned a harmless disk error into a profile
    /// that vanished — with an error message that never named the backup.
    #[test]
    fn profile_survives_a_failed_write() {
        let dir = temp_dir("prof-survives");
        let original = "experiencia:\n  - empresa: A\n";
        update_profile(&dir, original, SessionKind::Interactive).unwrap();
        // A directory where the temp file goes makes the write fail.
        std::fs::create_dir(dir.join("candidate_base.yaml.tmp")).unwrap();
        assert!(update_profile(&dir, "experiencia: []\n", SessionKind::Interactive).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap(),
            original,
            "the profile must still be at its canonical path after a failed write"
        );
    }

    /// I4: "there was no profile" and "there was one and I could not read it" are
    /// very different states. Reporting the second as "Perfil criado" gives the
    /// most alarming case the most reassuring message.
    #[test]
    fn unreadable_previous_profile_is_not_announced_as_a_creation() {
        let dir = temp_dir("prof-unreadable");
        std::fs::write(dir.join("candidate_base.yaml"), "experiencia: [ { unclosed").unwrap();
        let msg =
            update_profile(&dir, "experiencia:\n  - empresa: A\n", SessionKind::Interactive)
                .unwrap();
        assert!(!msg.contains("Perfil criado"), "a replacement is not a creation; got: {msg}");
        assert!(msg.contains("bak-1"), "got: {msg}");
    }

    /// I5: the property is "no backup, no write". The obstacle-in-the-last-slot
    /// test proves the sliding loop aborts; this one proves the archive step does.
    #[test]
    fn write_aborts_when_the_current_profile_cannot_be_archived() {
        let dir = temp_dir("prof-noarchive");
        let original = "experiencia:\n  - empresa: A\n";
        update_profile(&dir, original, SessionKind::Interactive).unwrap();
        // A non-empty directory in the .bak-1 slot cannot be replaced by a file.
        let blocker = backup_path(&dir, 1);
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("occupied"), "x").unwrap();
        assert!(update_profile(&dir, "experiencia: []\n", SessionKind::Interactive).is_err());
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap(), original);
    }

    /// I6: a rotation that fails halfway must not have eaten a generation of
    /// history for a write that never happened.
    #[test]
    fn failed_rotation_does_not_destroy_older_backups() {
        let dir = temp_dir("prof-partial");
        let yaml = "experiencia:\n  - empresa: A\n";
        update_profile(&dir, yaml, SessionKind::Interactive).unwrap();
        update_profile(&dir, yaml, SessionKind::Interactive).unwrap(); // .bak-1 exists
        std::fs::write(backup_path(&dir, 2), "generation-2").unwrap();

        let blocker = backup_path(&dir, 3);
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("occupied"), "x").unwrap();

        assert!(update_profile(&dir, yaml, SessionKind::Interactive).is_err());
        assert_eq!(
            std::fs::read_to_string(backup_path(&dir, 2)).unwrap(),
            "generation-2",
            "an aborted rotation must leave the numbering intact"
        );
    }

    #[test]
    fn autonomous_session_cannot_write_the_profile() {
        let dir = temp_dir("prof-auto");
        let yaml = "experiencia:\n  - empresa: A\n";
        let err = update_profile(&dir, yaml, SessionKind::Autonomous).unwrap_err();
        assert!(err.contains("propose_profile_change"), "must name the alternative; got: {err}");
        assert!(!dir.join("candidate_base.yaml").exists(), "autonomous write must not land");
    }

    #[test]
    fn autonomous_refusal_does_not_touch_an_existing_profile() {
        let dir = temp_dir("prof-auto-keep");
        let yaml = "experiencia:\n  - empresa: A\n";
        update_profile(&dir, yaml, SessionKind::Interactive).unwrap();
        assert!(update_profile(&dir, "experiencia: []\n", SessionKind::Autonomous).is_err());
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap(), yaml);
        assert!(
            !dir.join("candidate_base.yaml.bak-1").exists(),
            "a refusal must not rotate backups"
        );
    }

    #[test]
    fn write_keeps_the_previous_profile_in_bak1() {
        let dir = temp_dir("prof-keeps");
        let v1 = "experiencia:\n  - empresa: A\n  - empresa: B\n";
        update_profile(&dir, v1, SessionKind::Interactive).unwrap();
        update_profile(&dir, "experiencia: []\n", SessionKind::Interactive).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), v1);
    }

    #[test]
    fn write_that_removes_content_says_so_and_points_at_the_backup() {
        let dir = temp_dir("prof-says");
        update_profile(&dir, "experiencia:\n  - empresa: A\n  - empresa: B\n", SessionKind::Interactive).unwrap();
        let msg = update_profile(&dir, "experiencia: []\n", SessionKind::Interactive).unwrap();
        assert!(msg.contains("−2 experiências"), "got: {msg}");
        assert!(msg.contains("bak-1"), "a lossy write must point at the backup; got: {msg}");
    }

    #[test]
    fn write_without_losses_does_not_mention_the_backup() {
        let dir = temp_dir("prof-nomention");
        update_profile(&dir, "experiencia:\n  - empresa: A\n", SessionKind::Interactive).unwrap();
        let msg = update_profile(&dir, "experiencia:\n  - empresa: A\n  - empresa: B\n", SessionKind::Interactive).unwrap();
        assert!(!msg.contains("bak-1"), "nothing was lost; got: {msg}");
    }

    /// A loss with no change in block counts must still point at the backup.
    /// This is the case the `lossy` flag exists for.
    #[test]
    fn write_that_blanks_a_personal_field_points_at_the_backup() {
        let dir = temp_dir("prof-blank");
        update_profile(
            &dir,
            "dados_pessoais:\n  nome_completo: Maria\n  cpf: \"000.111.222-33\"\n",
            SessionKind::Interactive,
        )
        .unwrap();
        let msg = update_profile(&dir, "dados_pessoais:\n  nome_completo: Maria\n", SessionKind::Interactive).unwrap();
        assert!(msg.contains("cpf"), "got: {msg}");
        assert!(msg.contains("bak-1"), "got: {msg}");
    }

    /// Review Focus 2: no backup, no write. Losing the rollback silently is the
    /// failure mode this whole task exists to prevent.
    ///
    /// The obstacle is a NON-EMPTY directory in the OLDEST slot. Two properties
    /// of rotation make the other slots useless for this: an empty directory is
    /// renamed aside like any file, and the loop walks downward precisely to
    /// clear each slot before using it. Only the last slot is never vacated.
    #[test]
    fn write_aborts_when_the_backup_cannot_be_made() {
        let dir = temp_dir("prof-noback");
        let first = "experiencia:\n  - empresa: A\n";
        // Fill the rotation so the slot before the oldest is occupied.
        for _ in 0..BACKUP_DEPTH {
            update_profile(&dir, first, SessionKind::Interactive).unwrap();
        }
        let blocker = backup_path(&dir, BACKUP_DEPTH);
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("occupied"), "x").unwrap();

        assert!(update_profile(&dir, "experiencia: []\n", SessionKind::Interactive).is_err());
        let still = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
        assert!(still.contains("empresa: A"), "profile must be untouched; got: {still}");
    }

    #[test]
    fn rotation_copies_current_into_bak1_without_removing_it() {
        let dir = temp_dir("prof-rot-1");
        std::fs::write(dir.join("candidate_base.yaml"), "v1").unwrap();
        rotate_backups(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), "v1");
        assert_eq!(
            std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap(),
            "v1",
            "the live profile must stay in place — moving it away is what left users with none"
        );
    }

    #[test]
    fn rotation_slides_older_backups_and_drops_the_sixth() {
        let dir = temp_dir("prof-rot-slide");
        for i in 1..=6 {
            std::fs::write(dir.join("candidate_base.yaml"), format!("v{i}")).unwrap();
            rotate_backups(&dir).unwrap();
        }
        // Newest write is v6; it is now .bak-1, and v1 fell off the end.
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), "v6");
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-5")).unwrap(), "v2");
        assert!(!dir.join("candidate_base.yaml.bak-6").exists(), "only five backups are kept");
    }

    /// Review Focus 1: first write ever — nothing to rotate, and that is not an error.
    #[test]
    fn rotation_is_a_noop_without_a_current_file() {
        let dir = temp_dir("prof-rot-none");
        rotate_backups(&dir).unwrap();
        assert!(!dir.join("candidate_base.yaml.bak-1").exists());
    }

    #[test]
    fn writes_valid_yaml() {
        let dir = temp_dir("prof-valid");
        let yaml = "dados_pessoais:\n  nome_completo: \"Maria\"\nexperiencia:\n  - empresa: \"ACME\"\n    cargo: \"Dev\"\n";
        let msg = update_profile(&dir, yaml, SessionKind::Interactive).unwrap();
        assert!(msg.contains("1 experiência"), "got: {msg}");
        let written = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
        assert_eq!(written, yaml);
    }
}
