// What state the page is in, as far as the URL and the accessibility tree can tell: the line
// `estado: <value>` that ends the result of browser_open / browser_read / browser_text.
//
// These are HEURISTICS that INFORM the model; nothing here ever blocks an action. A false positive
// costs one line of text, a false negative is the behavior before this existed (the model loops on a
// page it cannot use). The patterns are DATA (`PATTERNS`), already in the folded form `guard::fold`
// produces (lowercase, no accents, punctuation as a space), so a test can swap them.
use std::collections::HashMap;

use serde_json::Value;

use crate::browser::guard::fold;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    Ok,
    LoginWall,
    Captcha,
    Vazio,
    Carregando,
}

impl PageState {
    pub fn label(self) -> &'static str {
        match self {
            PageState::Ok => "ok",
            PageState::LoginWall => "login_wall",
            PageState::Captcha => "captcha",
            PageState::Vazio => "vazio",
            PageState::Carregando => "carregando",
        }
    }

    /// The closing line of a tool result.
    pub fn line(self) -> String {
        format!("estado: {}", self.label())
    }
}

pub struct Patterns {
    /// URL path pieces of a login wall. A match needs a boundary after it ("/login?x" yes, "/login-tips" no),
    /// unless the pattern itself ends in "/".
    pub login_urls: &'static [&'static str],
    /// Words that, on a button, link or heading INSIDE a dialog, make the dialog a login prompt. Folded.
    pub login_words: &'static [&'static str],
    /// URL pieces of a captcha page.
    pub captcha_urls: &'static [&'static str],
    /// Phrases that mean a captcha wherever they appear. Folded.
    pub captcha_strict: &'static [&'static str],
    /// Looser words, only looked for in the NAMES of controls, frames and headings (not in page prose:
    /// a job description may legitimately mention "captcha"). Folded.
    pub captcha_loose: &'static [&'static str],
}

pub const PATTERNS: Patterns = Patterns {
    login_urls: &["/login", "/signin", "/sign-in", "/authwall", "/checkpoint", "/uas/"],
    login_words: &[
        "entrar", "sign in", "log in", "login", "continuar com o google", "continue with google", "cadastre se",
        "sign up", "join now", "inscreva se", "entre para",
    ],
    captcha_urls: &["/captcha"],
    captcha_strict: &[
        "nao sou um robo", "i m not a robot", "i am not a robot", "verify you are human", "verifique que voce e humano",
        "verifique se voce e humano", "confirme que voce e humano", "are you a robot",
    ],
    captcha_loose: &["captcha", "recaptcha", "hcaptcha", "turnstile"],
};

/// Roles whose NAME is looked at for the loose captcha words.
const CONTROL_ROLES: &[&str] =
    &["Iframe", "iframe", "checkbox", "button", "heading", "dialog", "alertdialog", "RootWebArea", "region", "group", "main"];

/// Roles that carry text the user can read, for the "empty page" count.
const TEXT_ROLES: &[&str] = &["StaticText", "button", "link", "heading", "textbox", "image", "img", "checkbox", "radio"];

/// Fewer visible characters than this on a finished page = `vazio`.
const EMPTY_BELOW_CHARS: usize = 10;

fn role(n: &Value) -> &str {
    n["role"]["value"].as_str().unwrap_or("")
}

fn name(n: &Value) -> &str {
    n["name"]["value"].as_str().unwrap_or("")
}

fn ignored(n: &Value) -> bool {
    n["ignored"] == true
}

/// True when `url` (lowercase) holds `pat` with a boundary after it.
fn url_has_segment(url: &str, pat: &str) -> bool {
    let mut from = 0;
    while let Some(i) = url[from..].find(pat) {
        let end = from + i + pat.len();
        if pat.ends_with('/') {
            return true;
        }
        match url[end..].chars().next() {
            None => return true,
            Some(c) if matches!(c, '/' | '?' | '#' | '.' | '&' | '=') => return true,
            Some(_) => {}
        }
        from = end;
    }
    false
}

fn contains_any(folded: &str, words: &[&str]) -> bool {
    words.iter().any(|w| folded.contains(w))
}

/// Names (folded) of the dialog itself and of the buttons, links and headings below it.
fn dialog_names(dialog: &Value, by_id: &HashMap<&str, &Value>) -> Vec<String> {
    let mut out = vec![fold(name(dialog))];
    let mut stack: Vec<&Value> = vec![dialog];
    let mut seen = std::collections::HashSet::new();
    while let Some(n) = stack.pop() {
        let id = n["nodeId"].as_str().unwrap_or("");
        if !seen.insert(id.to_string()) {
            continue;
        }
        if !ignored(n) && matches!(role(n), "button" | "link" | "heading") {
            out.push(fold(name(n)));
        }
        for c in n["childIds"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            if let Some(child) = by_id.get(c) {
                stack.push(child);
            }
        }
    }
    out
}

fn visible_chars(nodes: &[Value]) -> usize {
    nodes
        .iter()
        .filter(|n| !ignored(n) && TEXT_ROLES.contains(&role(n)))
        .map(|n| name(n).trim().chars().count())
        .sum()
}

pub fn detect(url: &str, ready_state: &str, ax: &Value) -> PageState {
    detect_with(&PATTERNS, url, ready_state, ax)
}

/// Captcha, then login wall, then still loading, then empty, else ok: a positive finding wins over
/// "it is still loading" (a challenge page is the answer even if the document never settles).
pub fn detect_with(p: &Patterns, url: &str, ready_state: &str, ax: &Value) -> PageState {
    let nodes: &[Value] = ax["nodes"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let url_lower = url.to_lowercase();

    let captcha = p.captcha_urls.iter().any(|u| url_lower.contains(u))
        || nodes.iter().filter(|n| !ignored(n)).any(|n| {
            let nm = fold(name(n));
            !nm.is_empty() && (contains_any(&nm, p.captcha_strict) || (CONTROL_ROLES.contains(&role(n)) && contains_any(&nm, p.captcha_loose)))
        });
    if captcha {
        return PageState::Captcha;
    }

    let by_id: HashMap<&str, &Value> = nodes.iter().filter_map(|n| n["nodeId"].as_str().map(|id| (id, n))).collect();
    let login = p.login_urls.iter().any(|u| url_has_segment(&url_lower, u))
        || nodes
            .iter()
            .filter(|n| !ignored(n) && matches!(role(n), "dialog" | "alertdialog"))
            .any(|d| dialog_names(d, &by_id).iter().any(|nm| contains_any(nm, p.login_words)));
    if login {
        return PageState::LoginWall;
    }

    if !ready_state.is_empty() && ready_state != "complete" {
        return PageState::Carregando;
    }
    if visible_chars(nodes) < EMPTY_BELOW_CHARS {
        return PageState::Vazio;
    }
    PageState::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A tiny AX tree: (id, role, name, ignored, children).
    fn tree(spec: &[(&str, &str, &str, bool, &[&str])]) -> Value {
        let nodes: Vec<Value> = spec
            .iter()
            .map(|(id, role, name, ign, kids)| {
                json!({ "nodeId": id, "ignored": ign, "role": {"value": role}, "name": {"value": name}, "childIds": kids })
            })
            .collect();
        json!({ "nodes": nodes })
    }

    fn prose_page() -> Value {
        tree(&[
            ("1", "RootWebArea", "Vaga", false, &["2", "3"]),
            ("2", "heading", "Engenheiro de Software", false, &[]),
            ("3", "StaticText", "Você vai construir sistemas de agendamento para fábricas.", false, &[]),
        ])
    }

    #[test]
    fn an_ordinary_page_is_ok_and_the_line_has_the_agreed_format() {
        let s = detect("https://example.com/jobs/1", "complete", &prose_page());
        assert_eq!(s, PageState::Ok);
        assert_eq!(s.line(), "estado: ok");
        assert_eq!(PageState::LoginWall.line(), "estado: login_wall");
        assert_eq!(PageState::Captcha.label(), "captcha");
        assert_eq!(PageState::Vazio.label(), "vazio");
        assert_eq!(PageState::Carregando.label(), "carregando");
    }

    #[test]
    fn a_login_url_is_a_login_wall_but_only_at_a_path_boundary() {
        for url in [
            "https://www.linkedin.com/authwall?trk=x",
            "https://www.linkedin.com/login",
            "https://www.linkedin.com/uas/login?session_redirect=y",
            "https://www.linkedin.com/checkpoint/challenge/abc",
            "https://example.com/Login?next=/jobs",
            "https://example.com/app/#/signin",
            "file:///C:/fixtures/authwall.html",
        ] {
            assert_eq!(detect(url, "complete", &prose_page()), PageState::LoginWall, "{url}");
        }
        for url in [
            "https://example.com/blog/login-tips",
            "https://example.com/jobs/blogin",
            "https://example.com/checkpointing",
            "https://example.com/ruas/1",
        ] {
            assert_eq!(detect(url, "complete", &prose_page()), PageState::Ok, "{url} is not a login page");
        }
    }

    #[test]
    fn a_dialog_that_asks_to_sign_in_is_a_login_wall() {
        let ax = tree(&[
            ("1", "RootWebArea", "Vagas", false, &["2", "5"]),
            ("2", "dialog", "", false, &["3", "4"]),
            ("3", "heading", "Entre para ver mais vagas", false, &[]),
            ("4", "button", "Continuar com o Google", false, &[]),
            ("5", "StaticText", "Resultados da busca de emprego para você ler com calma.", false, &[]),
        ]);
        assert_eq!(detect("https://www.linkedin.com/jobs/search/", "complete", &ax), PageState::LoginWall);
        let en = tree(&[
            ("1", "RootWebArea", "Jobs", false, &["2"]),
            ("2", "alertdialog", "Sign in to continue", false, &[]),
        ]);
        assert_eq!(detect("https://x.com/", "complete", &en), PageState::LoginWall);
    }

    #[test]
    fn a_sign_in_button_outside_a_dialog_or_a_dialog_without_login_words_is_not_a_wall() {
        let header = tree(&[
            ("1", "RootWebArea", "Vagas", false, &["2", "3"]),
            ("2", "button", "Entrar", false, &[]),
            ("3", "StaticText", "Lista pública de vagas com texto suficiente para não ser vazia.", false, &[]),
        ]);
        assert_eq!(detect("https://www.linkedin.com/jobs/", "complete", &header), PageState::Ok, "a header 'Entrar' is not a wall");
        let cookies = tree(&[
            ("1", "RootWebArea", "Vagas", false, &["2", "5"]),
            ("2", "dialog", "Cookies", false, &["3"]),
            ("3", "button", "Aceitar tudo", false, &[]),
            ("5", "StaticText", "Lista pública de vagas com texto suficiente para não ser vazia.", false, &[]),
        ]);
        assert_eq!(detect("https://x.com/", "complete", &cookies), PageState::Ok);
    }

    #[test]
    fn a_captcha_is_found_by_frame_name_by_checkbox_or_by_a_strict_phrase() {
        let frame = tree(&[("1", "RootWebArea", "Segurança", false, &["2"]), ("2", "Iframe", "reCAPTCHA", false, &[])]);
        assert_eq!(detect("https://x.com/", "complete", &frame), PageState::Captcha);
        let box_ = tree(&[("1", "RootWebArea", "x", false, &["2"]), ("2", "checkbox", "Não sou um robô", false, &[])]);
        assert_eq!(detect("https://x.com/", "complete", &box_), PageState::Captcha);
        let prose = tree(&[("1", "RootWebArea", "x", false, &["2"]), ("2", "StaticText", "Please verify you are human to continue", false, &[])]);
        assert_eq!(detect("https://x.com/", "complete", &prose), PageState::Captcha);
        assert_eq!(detect("https://x.com/captcha/page", "complete", &prose_page()), PageState::Captcha);
    }

    #[test]
    fn the_word_captcha_in_job_prose_is_not_a_captcha() {
        let job = tree(&[
            ("1", "RootWebArea", "Vaga", false, &["2"]),
            ("2", "StaticText", "Experiência com sistemas anti-bot, captcha e rate limiting é um diferencial.", false, &[]),
        ]);
        assert_eq!(detect("https://example.com/jobs/1", "complete", &job), PageState::Ok);
    }

    #[test]
    fn captcha_wins_over_login_wall_and_both_win_over_loading() {
        let ax = tree(&[
            ("1", "RootWebArea", "x", false, &["2", "3"]),
            ("2", "Iframe", "hCaptcha", false, &[]),
            ("3", "StaticText", "texto longo o bastante para a página não estar vazia", false, &[]),
        ]);
        assert_eq!(detect("https://www.linkedin.com/checkpoint/challenge", "loading", &ax), PageState::Captcha);
        assert_eq!(detect("https://www.linkedin.com/checkpoint/challenge", "loading", &prose_page()), PageState::LoginWall);
    }

    #[test]
    fn a_page_still_loading_says_so_and_an_empty_finished_page_says_vazio() {
        assert_eq!(detect("https://example.com/", "loading", &prose_page()), PageState::Carregando);
        assert_eq!(detect("https://example.com/", "interactive", &prose_page()), PageState::Carregando);
        let empty = tree(&[("1", "RootWebArea", "", false, &[])]);
        assert_eq!(detect("https://example.com/", "complete", &empty), PageState::Vazio);
        assert_eq!(detect("https://example.com/", "loading", &empty), PageState::Carregando, "still loading beats empty");
        // Hidden (ignored) text does not count as content.
        let hidden = tree(&[("1", "RootWebArea", "", false, &["2"]), ("2", "StaticText", "texto escondido bem comprido", true, &[])]);
        assert_eq!(detect("https://example.com/", "complete", &hidden), PageState::Vazio);
        // No readyState known: not reported as loading.
        assert_eq!(detect("https://example.com/", "", &prose_page()), PageState::Ok);
    }

    #[test]
    fn the_patterns_are_data_a_test_can_replace() {
        let mine = Patterns {
            login_urls: &["/acesso"],
            login_words: &["identifique se"],
            captcha_urls: &[],
            captcha_strict: &["prova de humanidade"],
            captcha_loose: &[],
        };
        assert_eq!(detect_with(&mine, "https://x.com/acesso", "complete", &prose_page()), PageState::LoginWall);
        assert_eq!(detect_with(&mine, "https://x.com/login", "complete", &prose_page()), PageState::Ok, "the built-in /login is not in this set");
        let ax = tree(&[("1", "RootWebArea", "x", false, &["2"]), ("2", "StaticText", "Faça a prova de humanidade", false, &[])]);
        assert_eq!(detect_with(&mine, "https://x.com/", "complete", &ax), PageState::Captcha);
    }

    #[test]
    fn every_built_in_word_is_already_folded_so_it_can_actually_match() {
        for w in PATTERNS.login_words.iter().chain(PATTERNS.captcha_strict).chain(PATTERNS.captcha_loose) {
            assert_eq!(fold(w), *w, "pattern {w:?} must be in folded form (lowercase, no accents, punctuation as spaces)");
        }
    }
}
