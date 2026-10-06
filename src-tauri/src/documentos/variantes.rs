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

/// Edits search_variants.yaml as a YAML value, touching only `campos` of variant `id` (and, when the
/// edit marks it principal, un-marking the others that carry `principal: true`). Everything the
/// SearchVariants struct does not model (descricao, unknown keys, top-level blocks) survives, and no
/// default-valued field is added. Returns the new file text; the caller writes it with a backup.
pub fn editar_campos(data_dir: &Path, id: &str, campos: &serde_json::Map<String, serde_json::Value>) -> Result<String, String> {
    use serde_yaml::Value;
    let texto = std::fs::read_to_string(data_dir.join("search_variants.yaml"))
        .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { format!("variante desconhecida: {id}") } else { format!("erro ao ler search_variants.yaml: {e}") })?;
    let mut doc: Value = serde_yaml::from_str(&texto).map_err(|e| format!("search_variants.yaml inválido: {e}"))?;
    let lista = doc.get_mut("variantes").and_then(Value::as_sequence_mut).ok_or_else(|| format!("variante desconhecida: {id}"))?;
    let pos = lista.iter().position(|v| v.get("id").and_then(Value::as_str) == Some(id)).ok_or_else(|| format!("variante desconhecida: {id}"))?;
    let alvo = lista[pos].as_mapping_mut().ok_or("variante inválida em search_variants.yaml")?;
    for (k, v) in campos {
        alvo.insert(Value::from(k.as_str()), serde_yaml::to_value(v).map_err(|e| e.to_string())?);
    }
    // Same type check the struct gave before: a bad value is refused, nothing is written.
    let nova: crate::commands::perfil::SearchVariant =
        serde_yaml::from_value(lista[pos].clone()).map_err(|e| format!("valor inválido: {e}"))?;
    if nova.principal {
        for (i, outra) in lista.iter_mut().enumerate() {
            if i != pos {
                if let Some(m) = outra.as_mapping_mut() {
                    if m.get("principal").and_then(Value::as_bool) == Some(true) {
                        m.insert(Value::from("principal"), Value::Bool(false));
                    }
                }
            }
        }
    }
    serde_yaml::to_string(&doc).map_err(|e| e.to_string())
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
