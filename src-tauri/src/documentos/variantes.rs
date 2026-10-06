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

fn completar(dest: &mut serde_yaml::Mapping, de: &serde_yaml::Mapping) {
    for (k, v) in de {
        if !dest.contains_key(k) {
            dest.insert(k.clone(), v.clone());
        }
    }
}

/// What the Perfil screen sends is a typed SearchVariants; whatever the file holds beyond the struct
/// (descricao, unknown keys at any level) is copied back from the file currently on disk, so a screen
/// save never drops it. Keys the screen did send win.
pub fn preservar_extras(novo: &mut SearchVariants, antigo: &SearchVariants) {
    completar(&mut novo.extra, &antigo.extra);
    completar(&mut novo.preferencias_globais.extra, &antigo.preferencias_globais.extra);
    completar(&mut novo.preferencias_globais.faixa_salarial.extra, &antigo.preferencias_globais.faixa_salarial.extra);
    for v in novo.variantes.iter_mut() {
        if let Some(a) = antigo.variantes.iter().find(|a| a.id == v.id) {
            completar(&mut v.extra, &a.extra);
        }
    }
    for (i, p) in novo.perguntas_pendentes.iter_mut().enumerate() {
        if let Some(a) = antigo.perguntas_pendentes.get(i).filter(|a| a.pergunta == p.pergunta) {
            completar(&mut p.extra, &a.extra);
        }
    }
}

/// Whole-file save from the Perfil screen (guardar_search_variants).
pub fn salvar_todas(data_dir: &Path, mut sv: SearchVariants) -> Result<(), String> {
    checar_uma_principal(&sv)?;
    preservar_extras(&mut sv, &ler(data_dir).unwrap_or_default());
    gravar(data_dir, &sv)
}

/// One variant from the Perfil screen (guardar_variante_unica): replaces or appends it, the rest stays.
pub fn salvar_unica(data_dir: &Path, variante: crate::commands::perfil::SearchVariant) -> Result<(), String> {
    let id = variante.id.clone();
    let mut sv = ler(data_dir)?;
    match sv.variantes.iter().position(|v| v.id == id) {
        Some(pos) => {
            let mut v = variante;
            completar(&mut v.extra, &sv.variantes[pos].extra);
            sv.variantes[pos] = v;
        }
        None => sv.variantes.push(variante),
    }
    if sv.variantes.iter().any(|v| v.id == id && v.principal) {
        manter_uma_principal(&mut sv, &id);
    }
    gravar(data_dir, &sv)
}

/// Weights from the dashboard sliders (guardar_pesos_variantes).
pub fn salvar_pesos(data_dir: &Path, pesos: &std::collections::HashMap<String, f64>) -> Result<(), String> {
    if !data_dir.join("search_variants.yaml").exists() {
        return Ok(());
    }
    let mut sv = ler(data_dir)?;
    for v in sv.variantes.iter_mut() {
        if let Some(&peso) = pesos.get(&v.id) {
            v.peso = (peso * 10.0).round() / 10.0; // 1 decimal place
        }
    }
    gravar(data_dir, &sv)
}

fn gravar(data_dir: &Path, sv: &SearchVariants) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let yaml = serde_yaml::to_string(sv).map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("search_variants.yaml"), yaml).map_err(|e| e.to_string())
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

    const RICO: &str = "variantes:\n  - id: devops-sre\n    nome_exibicao: DevOps\n    descricao: Plataforma e SRE\n    campo_futuro: [a, b]\n  - id: backend\n    nome_exibicao: Backend\n    descricao: APIs\npreferencias_globais:\n  faixa_salarial:\n    minimo: 45000\n    moeda: DKK\n    nota_livre: x\n  extra_pref: 1\nred_lines:\n  - sem plantao\nperguntas_pendentes:\n  - pergunta: viaja?\n    prioridade: alta\nchave_desconhecida:\n  k: v\n";

    fn yaml(d: &std::path::Path) -> serde_yaml::Value {
        serde_yaml::from_str(&std::fs::read_to_string(d.join("search_variants.yaml")).unwrap()).unwrap()
    }

    fn confere_preservado(d: &std::path::Path) {
        let antes: serde_yaml::Value = serde_yaml::from_str(RICO).unwrap();
        let depois = yaml(d);
        for i in 0..2 {
            assert_eq!(depois["variantes"][i]["descricao"], antes["variantes"][i]["descricao"], "descricao {i}");
        }
        assert_eq!(depois["variantes"][0]["campo_futuro"], antes["variantes"][0]["campo_futuro"]);
        assert_eq!(depois["preferencias_globais"]["extra_pref"], antes["preferencias_globais"]["extra_pref"]);
        assert_eq!(depois["preferencias_globais"]["faixa_salarial"]["nota_livre"], antes["preferencias_globais"]["faixa_salarial"]["nota_livre"]);
        assert_eq!(depois["perguntas_pendentes"][0]["prioridade"], antes["perguntas_pendentes"][0]["prioridade"]);
        assert_eq!(depois["red_lines"], antes["red_lines"]);
        assert_eq!(depois["chave_desconhecida"], antes["chave_desconhecida"]);
        let texto = std::fs::read_to_string(d.join("search_variants.yaml")).unwrap();
        assert!(!texto.contains("cv_gerado"), "no empty cv field is introduced:\n{texto}");
    }

    #[test]
    fn the_screen_whole_file_save_keeps_what_the_struct_does_not_model() {
        let d = dir_with(RICO);
        // The screen sends the typed value; here one stripped of the extras, as an older screen would.
        let mut vindo: SearchVariants = serde_yaml::from_str(RICO).unwrap();
        vindo.extra.clear();
        vindo.preferencias_globais.extra.clear();
        vindo.preferencias_globais.faixa_salarial.extra.clear();
        vindo.perguntas_pendentes[0].extra.clear();
        for v in vindo.variantes.iter_mut() {
            v.extra.clear();
        }
        vindo.variantes[0].peso = 80.0;
        salvar_todas(&d, vindo).unwrap();
        confere_preservado(&d);
        assert_eq!(yaml(&d)["variantes"][0]["peso"].as_f64(), Some(80.0));
    }

    #[test]
    fn the_screen_single_variant_save_and_weights_keep_the_rest() {
        let d = dir_with(RICO);
        let mut v: crate::commands::perfil::SearchVariant = serde_yaml::from_str("id: backend\nnome_exibicao: Backend 2\n").unwrap();
        v.peso = 33.0;
        salvar_unica(&d, v).unwrap();
        confere_preservado(&d);
        assert_eq!(yaml(&d)["variantes"][1]["nome_exibicao"].as_str(), Some("Backend 2"));
        salvar_pesos(&d, &std::collections::HashMap::from([("devops-sre".to_string(), 61.26)])).unwrap();
        confere_preservado(&d);
        assert_eq!(yaml(&d)["variantes"][0]["peso"].as_f64(), Some(61.3));
    }

    #[test]
    fn principal_survives_a_save_and_old_files_default_to_false() {
        let sv: SearchVariants = serde_yaml::from_str(DUAS).unwrap();
        let back: SearchVariants = serde_yaml::from_str(&serde_yaml::to_string(&sv).unwrap()).unwrap();
        assert!(back.variantes[1].principal && !back.variantes[0].principal);
    }
}
