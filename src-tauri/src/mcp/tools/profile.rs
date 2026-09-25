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
    fn writes_valid_yaml() {
        let dir = temp_dir("prof-valid");
        let yaml = "dados_pessoais:\n  nome_completo: \"Maria\"\nexperiencia:\n  - empresa: \"ACME\"\n    cargo: \"Dev\"\n";
        let msg = update_profile(&dir, yaml).unwrap();
        assert!(msg.contains("1 experiência"), "got: {msg}");
        let written = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
        assert_eq!(written, yaml);
    }
}
