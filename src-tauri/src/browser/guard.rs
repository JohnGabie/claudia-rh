// The send guard. It lives in app code, not in a prompt: the model can be
// wrong, injected or lazy, this cannot.
const PATTERNS: &str = include_str!("../../submit-patterns.txt");

/// Lowercase, strip accents and collapse whitespace so "Enviar  solicitação"
/// and "enviar solicitacao" compare equal.
fn fold(s: &str) -> String {
    let mapped: String = s
        .chars()
        .flat_map(|c| {
            let c = c.to_lowercase().next().unwrap_or(c);
            let r: &str = match c {
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => "a",
                'é' | 'è' | 'ê' | 'ë' => "e",
                'í' | 'ì' | 'î' | 'ï' => "i",
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
                'ú' | 'ù' | 'û' | 'ü' => "u",
                'ç' => "c",
                'æ' => "ae",
                _ => return vec![c],
            };
            r.chars().collect::<Vec<_>>()
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn looks_like_submit(accessible_name: &str) -> bool {
    let name = fold(accessible_name);
    if name.is_empty() {
        return false;
    }
    PATTERNS.lines().map(str::trim).filter(|l| !l.is_empty()).any(|p| name.contains(p))
}

pub trait GuardContext: Send + Sync {
    fn vaga_status(&self, vaga_id: i64) -> Option<String>;
    fn applications_today(&self) -> i64;
    /// 0 means "no cap".
    fn daily_limit(&self) -> i64;
    fn site_allowed(&self, url: &str) -> bool;
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Allow,
    Refuse(String),
}

pub fn check_submit(name: &str, confirm: Option<i64>, ctx: &dyn GuardContext) -> Verdict {
    if !looks_like_submit(name) {
        return Verdict::Allow;
    }
    let Some(vaga_id) = confirm else {
        return Verdict::Refuse(
            "RECUSADO: este botão envia a candidatura. Repita a chamada com confirm_submit = <id da vaga> \
             (a vaga precisa estar em 'candidatando')."
                .into(),
        );
    };
    match ctx.vaga_status(vaga_id).as_deref() {
        Some("candidatando") => {}
        other => {
            return Verdict::Refuse(format!(
                "RECUSADO: a vaga {vaga_id} está em '{}', não em 'candidatando'.",
                other.unwrap_or("inexistente")
            ))
        }
    }
    let limit = ctx.daily_limit();
    if limit > 0 && ctx.applications_today() >= limit {
        return Verdict::Refuse(format!("RECUSADO: o orçamento diário de {limit} candidaturas já foi usado."));
    }
    if !ctx.site_allowed("") {
        return Verdict::Refuse("RECUSADO: este site não está permitido para envio.".into());
    }
    Verdict::Allow
}

#[cfg(test)]
mod tests {
    use super::*;

    // Import PATTERNS for the extra tests
    use super::PATTERNS;

    struct Ctx { status: Option<&'static str>, today: i64, limit: i64, allowed: bool }
    impl GuardContext for Ctx {
        fn vaga_status(&self, _id: i64) -> Option<String> { self.status.map(String::from) }
        fn applications_today(&self) -> i64 { self.today }
        fn daily_limit(&self) -> i64 { self.limit }
        fn site_allowed(&self, _url: &str) -> bool { self.allowed }
    }
    fn ok() -> Ctx { Ctx { status: Some("candidatando"), today: 2, limit: 10, allowed: true } }

    #[test]
    fn recognises_submit_buttons_in_several_languages_and_cases() {
        for n in ["Enviar candidatura", "ENVIAR CANDIDATURA", "Submit application", "Enviar solicitação",
                  "  Send   application ", "Indsend ansøgning", "Postuler"] {
            assert!(looks_like_submit(n), "{n}");
        }
    }

    #[test]
    fn ordinary_buttons_are_not_submit() {
        for n in ["Avançar", "Avaliar", "Revisar", "Enviar mensagem", "Próximo", "Continuar", ""] {
            assert!(!looks_like_submit(n), "{n}");
        }
    }

    #[test]
    fn non_submit_clicks_are_always_allowed() {
        let ctx = Ctx { status: None, today: 99, limit: 1, allowed: false };
        assert_eq!(check_submit("Avançar", None, &ctx), Verdict::Allow);
    }

    #[test]
    fn submit_without_confirmation_is_refused() {
        assert!(matches!(check_submit("Enviar candidatura", None, &ok()), Verdict::Refuse(m) if m.contains("confirm_submit")));
    }

    #[test]
    fn submit_needs_the_vaga_to_be_candidatando() {
        let ctx = Ctx { status: Some("analisada"), ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Refuse(m) if m.contains("candidatando")));
        let none = Ctx { status: None, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &none), Verdict::Refuse(_)));
    }

    #[test]
    fn submit_stops_at_the_daily_budget() {
        let ctx = Ctx { today: 10, limit: 10, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Refuse(m) if m.contains("orçamento")));
    }

    #[test]
    fn a_zero_limit_means_no_budget_cap() {
        let ctx = Ctx { today: 500, limit: 0, ..ok() };
        assert_eq!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Allow);
    }

    #[test]
    fn submit_is_refused_on_a_site_that_is_not_allowed() {
        let ctx = Ctx { allowed: false, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Refuse(_)));
    }

    #[test]
    fn submit_passes_when_all_three_conditions_hold() {
        assert_eq!(check_submit("Enviar candidatura", Some(7), &ok()), Verdict::Allow);
    }

    #[test]
    fn patterns_file_is_already_folded() {
        for line in PATTERNS.lines() {
            let line = line.trim();
            if !line.is_empty() {
                assert_eq!(line, fold(line), "pattern '{}' is not already folded", line);
                assert!(line.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
                    "pattern '{}' contains non-ASCII-lowercase characters", line);
            }
        }
    }

    #[test]
    fn every_pattern_is_recognised() {
        for line in PATTERNS.lines() {
            let line = line.trim();
            if !line.is_empty() {
                assert!(looks_like_submit(line), "pattern '{}' not recognised by looks_like_submit", line);
                // Also test that it's recognised inside a longer label
                let label = format!("Clique aqui: {} agora", line);
                assert!(looks_like_submit(&label), "pattern '{}' not recognised in longer label '{}'", line, label);
            }
        }
    }

    #[test]
    fn fold_handles_danish_and_portuguese_letters() {
        assert_eq!(fold("Indsend ansøgning"), "indsend ansogning");
        assert_eq!(fold("Enviar solicitação"), "enviar solicitacao");
        assert_eq!(fold("  A   B "), "a b");
    }

    #[test]
    fn a_vaga_in_candidatando_with_zero_applications_today_is_allowed() {
        let ctx = Ctx { today: 0, limit: 10, ..ok() };
        assert_eq!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Allow);
    }

    #[test]
    fn budget_is_checked_against_applications_today_not_the_limit_alone() {
        // today = limit - 1 is allowed
        let ctx = Ctx { today: 9, limit: 10, ..ok() };
        assert_eq!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Allow);

        // today = limit is refused
        let ctx2 = Ctx { today: 10, limit: 10, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx2), Verdict::Refuse(_)));
    }
}
