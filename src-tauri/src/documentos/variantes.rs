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
    Err(format!(
        "variante desconhecida: {id}. Variantes existentes: {}",
        if ids.is_empty() { "nenhuma".to_string() } else { ids.join(", ") }
    ))
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
