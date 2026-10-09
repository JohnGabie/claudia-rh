//! Variant tools (spec section 7): reads in both sessions; writes only in the Profile session, with the
//! same backup-then-atomic-replace as update_profile. Never returns cv_gerado_path (a file path).
//! No automatic fact check on foco_* (user decision P4): the Profile prompt tells the agent to change
//! a variant only when the user asks.
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
        .map(|v| {
            serde_json::json!({
                "id": v.id, "nome": v.nome_exibicao, "peso": v.peso, "ativa": v.ativa, "principal": v.principal,
                "foco_competencias": v.foco_competencias, "foco_experiencia": v.foco_experiencia,
            })
        })
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
    // Edit the YAML itself, not a SearchVariants round trip: that dropped every key the struct does not model.
    let yaml = variantes::editar_campos(data_dir, id, obj)?;
    super::profile::write_yaml_atomically(data_dir, "search_variants.yaml", &yaml)?;
    Ok(format!(
        "Variante {id} atualizada ({}). A versão anterior fica guardada como cópia de segurança.",
        obj.keys().cloned().collect::<Vec<_>>().join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::temp_dir;

    const SV: &str = "variantes:\n  - id: backend\n    nome_exibicao: Backend\n    peso: 60\n    cv_gerado_path: C:\\\\Users\\\\x\\\\cv.html\n  - id: devops\n    nome_exibicao: DevOps\n    principal: true\n";

    fn dir() -> std::path::PathBuf {
        let d = temp_dir("mcp-var");
        std::fs::write(d.join("search_variants.yaml"), SV).unwrap();
        d
    }

    #[test]
    fn reads_never_return_a_path() {
        let d = dir();
        let l = list_variantes(&d).unwrap();
        assert!(l.contains("backend") && l.contains("\"principal\": true"), "{l}");
        assert!(!l.contains("cv_gerado") && !l.contains("cv.html"), "{l}");
        let g = get_variante(&d, "backend").unwrap();
        assert!(g.contains("backend") && !g.contains("cv_gerado") && !g.contains("cv.html"), "{g}");
        assert!(get_variante(&d, "frontend").is_err());
    }

    #[test]
    fn update_variante_is_refused_in_the_execution_session() {
        let d = dir();
        let e = update_variante(&d, "backend", &serde_json::json!({ "peso": 10 }), SessionKind::Autonomous).unwrap_err();
        assert!(e.contains("sessão de Perfil"), "{e}");
        assert_eq!(std::fs::read_to_string(d.join("search_variants.yaml")).unwrap(), SV, "untouched");
        assert!(!d.join("search_variants.yaml.bak-1").exists());
    }

    #[test]
    fn update_variante_writes_with_a_backup_and_keeps_one_principal() {
        let d = dir();
        let msg = update_variante(&d, "backend", &serde_json::json!({ "peso": 70, "principal": true }), SessionKind::Interactive).unwrap();
        assert!(!msg.contains(".yaml") && !msg.contains(".bak"), "no file name in the answer: {msg}");
        assert_eq!(std::fs::read_to_string(d.join("search_variants.yaml.bak-1")).unwrap(), SV, "the previous file is kept");
        let sv = crate::documentos::variantes::ler(&d).unwrap();
        assert_eq!(sv.variantes[0].peso, 70.0);
        assert_eq!(crate::documentos::variantes::principal_de(&sv).as_deref(), Some("backend"));
        assert_eq!(sv.variantes.iter().filter(|v| v.principal).count(), 1);
        assert_eq!(sv.variantes[0].cv_gerado_path, "C:\\\\Users\\\\x\\\\cv.html", "fields the tool does not touch survive");
    }

    #[test]
    fn update_variante_refuses_unknown_fields_and_paths() {
        let d = dir();
        for campos in [serde_json::json!({ "cv_gerado_path": "C:\\x" }), serde_json::json!({ "id": "outra" }), serde_json::json!([1])] {
            assert!(update_variante(&d, "backend", &campos, SessionKind::Interactive).is_err(), "{campos}");
        }
        assert!(update_variante(&d, "frontend", &serde_json::json!({ "peso": 1 }), SessionKind::Interactive).is_err());
        assert_eq!(std::fs::read_to_string(d.join("search_variants.yaml")).unwrap(), SV, "nothing written");
    }

    #[test]
    fn update_variante_does_not_check_foco_against_the_profile() {
        // P4 (user decision): no automatic fact check; the Profile prompt is what limits the agent.
        let d = dir();
        update_variante(&d, "backend", &serde_json::json!({ "foco_competencias": ["COBOL"] }), SessionKind::Interactive).unwrap();
    }

    const RICO: &str = "variantes:\n  - id: devops-sre\n    nome_exibicao: DevOps\n    peso: 50\n    descricao: Plataforma e SRE\n    campo_futuro: [a, b]\n  - id: backend\n    nome_exibicao: Backend\n    descricao: APIs\npreferencias_globais:\n  faixa_salarial:\n    minimo: 45000\n    moeda: DKK\n    nota_livre: x\n  extra_pref: 1\nred_lines:\n  - sem plantao\nperguntas_pendentes:\n  - pergunta: viaja?\n    prioridade: alta\nchave_desconhecida:\n  k: v\n";

    fn dir_rico() -> std::path::PathBuf {
        let d = temp_dir("mcp-var-rico");
        std::fs::write(d.join("search_variants.yaml"), RICO).unwrap();
        d
    }

    fn yaml_de(d: &std::path::Path) -> serde_yaml::Value {
        serde_yaml::from_str(&std::fs::read_to_string(d.join("search_variants.yaml")).unwrap()).unwrap()
    }

    #[test]
    fn update_variante_preserves_unknown_keys_everywhere() {
        let d = dir_rico();
        update_variante(&d, "devops-sre", &serde_json::json!({ "peso": 70 }), SessionKind::Interactive).unwrap();
        let antes: serde_yaml::Value = serde_yaml::from_str(RICO).unwrap();
        let depois = yaml_de(&d);
        for i in 0..2 {
            assert_eq!(depois["variantes"][i]["descricao"], antes["variantes"][i]["descricao"], "descricao of variant {i}");
        }
        assert_eq!(depois["variantes"][0]["campo_futuro"], antes["variantes"][0]["campo_futuro"]);
        for k in ["preferencias_globais", "red_lines", "perguntas_pendentes", "chave_desconhecida"] {
            assert_eq!(depois[k], antes[k], "{k}");
        }
        assert_eq!(depois["variantes"][0]["peso"].as_f64(), Some(70.0));
    }

    #[test]
    fn update_variante_changes_only_the_requested_field() {
        let d = dir_rico();
        update_variante(&d, "devops-sre", &serde_json::json!({ "peso": 70 }), SessionKind::Interactive).unwrap();
        let antes: serde_yaml::Value = serde_yaml::from_str(RICO).unwrap();
        let depois = yaml_de(&d);
        assert_eq!(depois["variantes"][1], antes["variantes"][1], "the untouched variant is identical");
        let mut esperado = antes["variantes"][0].clone();
        esperado["peso"] = serde_yaml::Value::from(70);
        assert_eq!(depois["variantes"][0], esperado, "the touched one differs only in peso");
        let texto = std::fs::read_to_string(d.join("search_variants.yaml")).unwrap();
        assert!(!texto.contains("cv_gerado"), "no empty field is introduced:\n{texto}");
        assert_eq!(std::fs::read_to_string(d.join("search_variants.yaml.bak-1")).unwrap(), RICO, "backup still made");
    }
}
