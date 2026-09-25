// Profile tools — validate-then-write for candidate_base.yaml.

use std::path::Path;

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

/// Validates the full candidate_base.yaml content against the serde structs
/// BEFORE writing. Invalid YAML never reaches disk; the parse error goes back
/// to the model so it can self-correct.
pub fn update_profile(data_dir: &Path, yaml: &str) -> Result<String, String> {
    if yaml.trim().is_empty() {
        return Err("YAML vazio — envie o conteúdo completo do candidate_base.yaml".to_string());
    }
    let parsed = crate::commands::perfil::parse_candidato_base_str(yaml)?;

    let path = data_dir.join("candidate_base.yaml");
    let tmp = data_dir.join("candidate_base.yaml.tmp");
    std::fs::write(&tmp, yaml).map_err(|e| format!("erro ao escrever ficheiro temporário: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("erro ao gravar candidate_base.yaml: {e}"))?;

    Ok(format!(
        "Perfil atualizado com sucesso: {} experiência(s), {} projeto(s), {} formação(ões), {} competência(s), {} idioma(s).",
        parsed.experiencia.len(),
        parsed.projetos.len(),
        parsed.formacao.len(),
        parsed.competencias.len(),
        parsed.idiomas.len(),
    ))
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
