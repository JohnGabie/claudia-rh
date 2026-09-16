// Profile tools — validate-then-write for candidate_base.yaml.

use std::path::Path;

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
    fn writes_valid_yaml() {
        let dir = temp_dir("prof-valid");
        let yaml = "dados_pessoais:\n  nome_completo: \"Maria\"\nexperiencia:\n  - empresa: \"ACME\"\n    cargo: \"Dev\"\n";
        let msg = update_profile(&dir, yaml).unwrap();
        assert!(msg.contains("1 experiência"), "got: {msg}");
        let written = std::fs::read_to_string(dir.join("candidate_base.yaml")).unwrap();
        assert_eq!(written, yaml);
    }
}
