// The send guard. It lives in app code, not in a prompt: the model can be
// wrong, injected or lazy, this cannot.
const PATTERNS: &str = include_str!("../../submit-patterns.txt");

use unicode_normalization::UnicodeNormalization;

/// Lowercase, strip accents, remove invisible/format characters, and collapse whitespace
/// so "Enviar\u{200B}candidatura", "Enviar  solicitação", and "enviarcandidatura" all normalize equally.
/// Uses NFKD normalization, removes combining marks and format characters, handles special letters.
fn fold(s: &str) -> String {
    // Step 1: NFKD normalize to decompose fullwidth and other compatibility characters
    let normalized: String = s.nfkd().collect();

    // Step 2: Process each character
    let processed: String = normalized
        .chars()
        .filter_map(|c| {
            // Skip combining marks (U+0300..U+036F) - they're combining, not spacing
            if ('\u{0300}'..='\u{036F}').contains(&c) {
                return None;
            }

            // Soft hyphen and other format characters are removed silently
            if c == '\u{00AD}' ||                        // Soft hyphen
               ('\u{2060}'..='\u{206F}').contains(&c) {   // Format characters
                return None;
            }

            // Zero-width and no-break space characters become spaces to preserve word boundaries
            if ('\u{200B}'..='\u{200F}').contains(&c) ||  // Zero-width characters
               c == '\u{FEFF}' {                          // Zero-width no-break space
                return Some(" ".to_string());
            }

            // Lowercase and map special letters
            let c_lower = c.to_lowercase().next().unwrap_or(c);
            let mapped: String = match c_lower {
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => "a".to_string(),
                'é' | 'è' | 'ê' | 'ë' => "e".to_string(),
                'í' | 'ì' | 'î' | 'ï' => "i".to_string(),
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' => "o".to_string(),
                'ú' | 'ù' | 'û' | 'ü' => "u".to_string(),
                'ç' => "c".to_string(),
                'æ' => "ae".to_string(),
                'ß' => "ss".to_string(),
                'đ' => "d".to_string(),
                'ł' => "l".to_string(),
                c if c.is_alphanumeric() => c.to_string(),
                _ => " ".to_string(),  // Turn non-alphanumeric into space
            };
            Some(mapped)
        })
        .collect();

    // Step 3: Collapse whitespace
    processed.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn looks_like_submit(accessible_name: &str) -> bool {
    let name = fold(accessible_name);
    if name.is_empty() {
        return false;
    }

    let name_no_spaces = name.replace(" ", "");

    PATTERNS.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .any(|p| {
            if let Some(exact_pattern) = p.strip_prefix('=') {
                // Exact match pattern (without the '=' prefix)
                name == exact_pattern
            } else {
                // Substring match pattern, with or without spaces
                name.contains(p) || name_no_spaces.contains(&p.replace(" ", ""))
            }
        })
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

pub fn check_submit(name: &str, page_url: &str, confirm: Option<i64>, ctx: &dyn GuardContext) -> Verdict {
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
    if limit <= 0 {
        return Verdict::Refuse("RECUSADO: o orçamento diário está em 0.".into());
    }
    if ctx.applications_today() >= limit {
        return Verdict::Refuse(format!("RECUSADO: o orçamento diário de {limit} candidaturas já foi usado."));
    }
    if !ctx.site_allowed(page_url) {
        return Verdict::Refuse("RECUSADO: este site não está permitido para envio.".into());
    }
    Verdict::Allow
}

#[cfg(test)]
mod tests {
    use super::*;

    // Import PATTERNS for the extra tests
    use super::PATTERNS;

    struct Ctx { status: Option<&'static str>, today: i64, limit: i64, allowed: bool, received_url: Option<&'static str> }
    impl GuardContext for Ctx {
        fn vaga_status(&self, _id: i64) -> Option<String> { self.status.map(String::from) }
        fn applications_today(&self) -> i64 { self.today }
        fn daily_limit(&self) -> i64 { self.limit }
        fn site_allowed(&self, _url: &str) -> bool { self.allowed }
    }
    fn ok() -> Ctx { Ctx { status: Some("candidatando"), today: 2, limit: 10, allowed: true, received_url: None } }

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
        let ctx = Ctx { status: None, today: 99, limit: 1, allowed: false, received_url: None };
        assert_eq!(check_submit("Avançar", "https://example.com", None, &ctx), Verdict::Allow);
    }

    #[test]
    fn submit_without_confirmation_is_refused() {
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", None, &ok()), Verdict::Refuse(m) if m.contains("confirm_submit")));
    }

    #[test]
    fn submit_needs_the_vaga_to_be_candidatando() {
        let ctx = Ctx { status: Some("analisada"), ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx), Verdict::Refuse(m) if m.contains("candidatando")));
        let none = Ctx { status: None, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &none), Verdict::Refuse(_)));
    }

    #[test]
    fn submit_stops_at_the_daily_budget() {
        let ctx = Ctx { today: 10, limit: 10, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx), Verdict::Refuse(m) if m.contains("orçamento")));
    }

    #[test]
    fn a_zero_or_negative_limit_refuses_every_submit() {
        // Zero limit means budget is exhausted
        let ctx_zero = Ctx { today: 0, limit: 0, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx_zero), Verdict::Refuse(m) if m.contains("está em 0")));

        // Negative limit also means budget is exhausted
        let ctx_negative = Ctx { today: 0, limit: -5, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx_negative), Verdict::Refuse(m) if m.contains("está em 0")));
    }

    #[test]
    fn submit_is_refused_on_a_site_that_is_not_allowed() {
        let ctx = Ctx { allowed: false, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx), Verdict::Refuse(_)));
    }

    #[test]
    fn submit_passes_when_all_three_conditions_hold() {
        assert_eq!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ok()), Verdict::Allow);
    }

    #[test]
    fn patterns_file_is_already_folded() {
        for line in PATTERNS.lines() {
            let line = line.trim();
            if !line.is_empty() {
                if line.starts_with('=') {
                    // Exact patterns: check the part after '='
                    let pattern = &line[1..];
                    assert_eq!(pattern, fold(pattern), "pattern '{}' is not already folded", line);
                    assert!(pattern.chars().all(|c| c.is_ascii_lowercase()),
                        "pattern '{}' contains non-ASCII-lowercase characters", line);
                } else {
                    // Substring patterns
                    assert_eq!(line, fold(line), "pattern '{}' is not already folded", line);
                    assert!(line.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
                        "pattern '{}' contains non-ASCII-lowercase characters", line);
                }
            }
        }
    }

    #[test]
    fn every_pattern_is_recognised() {
        for line in PATTERNS.lines() {
            let line = line.trim();
            if !line.is_empty() {
                if line.starts_with('=') {
                    // Exact patterns: test matching the bare name (without the '=' prefix)
                    let pattern = &line[1..];
                    assert!(looks_like_submit(pattern), "pattern '{}' not recognised by looks_like_submit", line);
                } else {
                    // Substring patterns: test both bare and in longer labels
                    assert!(looks_like_submit(line), "pattern '{}' not recognised by looks_like_submit", line);
                    // Also test that it's recognised inside a longer label
                    let label = format!("Clique aqui: {} agora", line);
                    assert!(looks_like_submit(&label), "pattern '{}' not recognised in longer label '{}'", line, label);
                }
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
        assert_eq!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx), Verdict::Allow);
    }

    #[test]
    fn budget_is_checked_against_applications_today_not_the_limit_alone() {
        // today = limit - 1 is allowed
        let ctx = Ctx { today: 9, limit: 10, ..ok() };
        assert_eq!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx), Verdict::Allow);

        // today = limit is refused
        let ctx2 = Ctx { today: 10, limit: 10, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", "https://example.com", Some(7), &ctx2), Verdict::Refuse(_)));
    }

    #[test]
    fn fold_removes_zero_width_space() {
        // U+200B is zero-width space
        assert_eq!(fold("Enviar\u{200B}candidatura"), "enviar candidatura");
    }

    #[test]
    fn fold_removes_soft_hyphen() {
        // U+00AD is soft hyphen
        assert_eq!(fold("Enviar candi\u{00AD}datura"), "enviar candidatura");
    }

    #[test]
    fn fold_normalizes_decomposed_accents() {
        // "solicitação" with combining tilde and acute
        let decomposed = "solicitac\u{0327}a\u{0303}o"; // ç + combining tilde on a
        assert_eq!(fold(decomposed), "solicitacao");
    }

    #[test]
    fn fold_handles_fullwidth_characters() {
        // U+FF33 is fullwidth 'S', U+FF35 is fullwidth 'U', U+FF2D is fullwidth 'M'
        assert_eq!(fold("Ｓｕｂｍｉｔ application"), "submit application");
    }

    #[test]
    fn fold_treats_special_letters() {
        assert_eq!(fold("ø"), "o");
        assert_eq!(fold("æ"), "ae");
        assert_eq!(fold("ß"), "ss");
        assert_eq!(fold("đ"), "d");
        assert_eq!(fold("ł"), "l");
    }

    #[test]
    fn fold_handles_hyphens_and_underscores_as_spaces() {
        assert_eq!(fold("Submit-application"), "submit application");
        assert_eq!(fold("Submit_application"), "submit application");
    }

    #[test]
    fn fold_handles_no_break_space() {
        // U+00A0 is no-break space
        assert_eq!(fold("Enviar\u{00A0}candidatura"), "enviar candidatura");
    }

    #[test]
    fn fold_handles_no_spaces() {
        assert_eq!(fold("enviarcandidatura"), "enviarcandidatura");
    }

    #[test]
    fn looks_like_submit_matches_pattern_with_or_without_spaces() {
        // Pattern is "enviar candidatura", test name without spaces should still match
        assert!(looks_like_submit("enviarcandidatura"));
        // And also with spaces
        assert!(looks_like_submit("enviar candidatura"));
        // And with extra non-alphanumeric chars that become spaces
        assert!(looks_like_submit("enviar-candidatura"));
        // And as part of a longer label
        assert!(looks_like_submit("Clique para enviar candidatura agora"));
    }

    #[test]
    fn exact_patterns_match_only_exact_whole_names() {
        // Exact pattern "=submit" only matches when name folds to exactly "submit"
        assert!(looks_like_submit("Submit"));
        assert!(looks_like_submit("SUBMIT"));
        // But not when it's part of a longer name (unlike substring patterns)
        // Note: "Submit application" matches substring pattern "submit application"
    }

    #[test]
    fn exact_pattern_send_does_not_match_longer_labels() {
        // "=send" matches only exact "send", not "Send application" or "Send message"
        assert!(looks_like_submit("Send"));
        // "Send message" is not a submit pattern (no substring match for it)
        assert!(!looks_like_submit("Send message"));
    }

    #[test]
    fn substring_pattern_enviar_candidatura_matches_full_labels() {
        // "enviar candidatura" (substring pattern) matches within longer labels
        assert!(looks_like_submit("Enviar candidatura"));
        assert!(looks_like_submit("Clique para Enviar candidatura"));
    }

    #[test]
    fn new_german_locales() {
        assert!(looks_like_submit("Bewerbung senden"));
        assert!(looks_like_submit("Bewerbung abschicken"));
        assert!(looks_like_submit("Bewerbung einreichen"));
    }

    #[test]
    fn new_spanish_and_french_locales() {
        assert!(looks_like_submit("Enviar solicitud"));
        assert!(looks_like_submit("Envoyer la candidature"));
        assert!(looks_like_submit("Envoyer ma candidature"));
    }

    #[test]
    fn new_danish_locales() {
        assert!(looks_like_submit("Indsend din ansøgning"));
    }

    #[test]
    fn page_url_is_passed_to_site_allowed() {
        use std::sync::Mutex;

        // Create a custom context that captures the received URL
        struct UrlCapturingCtx { received_url: Mutex<Option<String>> }
        impl GuardContext for UrlCapturingCtx {
            fn vaga_status(&self, _id: i64) -> Option<String> { Some("candidatando".to_string()) }
            fn applications_today(&self) -> i64 { 0 }
            fn daily_limit(&self) -> i64 { 10 }
            fn site_allowed(&self, url: &str) -> bool {
                *self.received_url.lock().unwrap() = Some(url.to_string());
                true
            }
        }

        let ctx = UrlCapturingCtx { received_url: Mutex::new(None) };
        let test_url = "https://example.com/apply";
        check_submit("Enviar candidatura", test_url, Some(7), &ctx);

        assert_eq!(*ctx.received_url.lock().unwrap(), Some(test_url.to_string()));
    }
}
