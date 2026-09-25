// Profile tools — validate-then-write for candidate_base.yaml.

use std::path::Path;

use crate::commands::perfil::CandidatoBase;

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

/// Slides candidate_base.yaml.bak-N to .bak-(N+1) and moves the current profile
/// into .bak-1. The oldest backup falls off the end.
///
/// A missing profile is not an error: the first write has nothing to preserve.
fn rotate_backups(data_dir: &Path) -> Result<(), String> {
    let current = data_dir.join("candidate_base.yaml");
    if !current.exists() {
        return Ok(());
    }
    // Walk down so each slot is free before we move into it.
    for n in (1..BACKUP_DEPTH).rev() {
        let from = backup_path(data_dir, n);
        if from.exists() {
            std::fs::rename(&from, backup_path(data_dir, n + 1))
                .map_err(|e| format!("erro ao rodar backup {n}: {e}"))?;
        }
    }
    std::fs::rename(&current, backup_path(data_dir, 1))
        .map_err(|e| format!("erro ao criar backup do perfil: {e}"))
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

/// Builds the sentence the model gets back after a write.
///
/// Counts, not a textual diff: what matters is what disappeared, not how the
/// YAML was reformatted. The minus sign is U+2212, so it survives terminals
/// that would swallow a leading hyphen.
fn describe_change(prev: Option<&CandidatoBase>, next: &CandidatoBase) -> ChangeSummary {
    let Some(prev) = prev else {
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

    let text = if changes.is_empty() {
        "Perfil gravado, sem alterações de conteúdo.".to_string()
    } else {
        format!("Perfil gravado. {}.", changes.join(", "))
    };
    ChangeSummary { text, lossy }
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
pub fn update_profile(data_dir: &Path, yaml: &str) -> Result<String, String> {
    if yaml.trim().is_empty() {
        return Err("YAML vazio — envie o conteúdo completo do candidate_base.yaml".to_string());
    }
    let parsed = crate::commands::perfil::parse_candidato_base_str(yaml)?;

    let path = data_dir.join("candidate_base.yaml");
    let previous = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| crate::commands::perfil::parse_candidato_base_str(&raw).ok());

    // No backup, no write.
    rotate_backups(data_dir)?;

    let tmp = data_dir.join("candidate_base.yaml.tmp");
    std::fs::write(&tmp, yaml).map_err(|e| format!("erro ao escrever ficheiro temporário: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("erro ao gravar candidate_base.yaml: {e}"))?;

    let summary = describe_change(previous.as_ref(), &parsed);
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
        let err = update_profile(&dir, "experiencia: [ { empresa: 'x'").unwrap_err();
        assert!(err.contains("YAML parse error"), "got: {err}");
        assert!(!dir.join("candidate_base.yaml").exists(), "invalid YAML must not be written");
    }

    #[test]
    fn rejects_empty_yaml() {
        let dir = temp_dir("prof-empty");
        assert!(update_profile(&dir, "   \n").is_err());
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
        let out = describe_change(Some(&prev), &next);
        assert!(out.text.contains("−1 experiência"), "got: {}", out.text);
        assert!(out.text.contains("−2 competências"), "got: {}", out.text);
        assert!(out.lossy);
    }

    #[test]
    fn diff_reports_gains() {
        let prev = base_from("experiencia:\n  - empresa: A\n");
        let next = base_from("experiencia:\n  - empresa: A\n  - empresa: B\n");
        let out = describe_change(Some(&prev), &next);
        assert!(out.text.contains("+1 experiência"), "got: {}", out.text);
        assert!(!out.lossy, "a pure addition is not lossy");
    }

    /// Review Focus 3: re-saving the same content must not invent movement.
    #[test]
    fn diff_says_nothing_changed_when_nothing_changed() {
        let yaml = "experiencia:\n  - empresa: A\ncompetencias:\n  - Rust\n";
        let prev = base_from(yaml);
        let next = base_from(yaml);
        let out = describe_change(Some(&prev), &next);
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
        let out = describe_change(Some(&prev), &next);
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
        let out = describe_change(None, &next);
        assert!(out.text.contains("Perfil criado"), "got: {}", out.text);
        assert!(!out.lossy, "a first write cannot lose anything");
    }

    #[test]
    fn write_keeps_the_previous_profile_in_bak1() {
        let dir = temp_dir("prof-keeps");
        let v1 = "experiencia:\n  - empresa: A\n  - empresa: B\n";
        update_profile(&dir, v1).unwrap();
        update_profile(&dir, "experiencia: []\n").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), v1);
    }

    #[test]
    fn write_that_removes_content_says_so_and_points_at_the_backup() {
        let dir = temp_dir("prof-says");
        update_profile(&dir, "experiencia:\n  - empresa: A\n  - empresa: B\n").unwrap();
        let msg = update_profile(&dir, "experiencia: []\n").unwrap();
        assert!(msg.contains("−2 experiências"), "got: {msg}");
        assert!(msg.contains("bak-1"), "a lossy write must point at the backup; got: {msg}");
    }

    #[test]
    fn write_without_losses_does_not_mention_the_backup() {
        let dir = temp_dir("prof-nomention");
        update_profile(&dir, "experiencia:\n  - empresa: A\n").unwrap();
        let msg = update_profile(&dir, "experiencia:\n  - empresa: A\n  - empresa: B\n").unwrap();
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
        )
        .unwrap();
        let msg = update_profile(&dir, "dados_pessoais:\n  nome_completo: Maria\n").unwrap();
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
            update_profile(&dir, first).unwrap();
        }
        let blocker = backup_path(&dir, BACKUP_DEPTH);
        std::fs::create_dir(&blocker).unwrap();
        std::fs::write(blocker.join("occupied"), "x").unwrap();

        assert!(update_profile(&dir, "experiencia: []\n").is_err());
        let still = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
        assert!(still.contains("empresa: A"), "profile must be untouched; got: {still}");
    }

    #[test]
    fn rotation_moves_current_to_bak1() {
        let dir = temp_dir("prof-rot-1");
        std::fs::write(dir.join("candidate_base.yaml"), "v1").unwrap();
        rotate_backups(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("candidate_base.yaml.bak-1")).unwrap(), "v1");
        assert!(!dir.join("candidate_base.yaml").exists(), "current must have been moved");
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
        let msg = update_profile(&dir, yaml).unwrap();
        assert!(msg.contains("1 experiência"), "got: {msg}");
        let written = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
        assert_eq!(written, yaml);
    }
}
