//! Pure URL reading and domain matching. No I/O, so every rule is testable without a database.

/// Host and path of an http(s) URL as a browser would read them, both lowercased. `None` when it is
/// not a plain http(s) URL with an ASCII host (IP literals and IDNs are refused on purpose).
pub fn host_and_path(url: &str) -> Option<(String, String)> {
    // Checked on the RAW input: a trailing newline or a space inside the host is a refusal, not
    // something to clean up.
    let u = url;
    if u.is_empty() || u.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let lower = u.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"))?;
    // In special schemes a browser treats "\" as "/": "https://evil.com\@good.com" is host evil.com.
    let rest = rest.replace('\\', "/");
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    // The host is what follows the LAST "@": everything before it is user info.
    let host_port = authority.rsplit('@').next()?;
    if host_port.starts_with('[') {
        return None;
    }
    let mut parts = host_port.split(':');
    let host = parts.next()?.trim_end_matches('.');
    let port = parts.next().unwrap_or("");
    if parts.next().is_some() || !port.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if host.is_empty() || host.starts_with('.') || host.contains("..") || !host.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-') {
        return None;
    }
    let path = if tail.starts_with('/') { tail.split(['?', '#']).next().unwrap_or("/") } else { "/" };
    Some((host.to_string(), path.to_string()))
}

/// `host` is `domain` or a subdomain of it, matched only at a label boundary.
pub fn host_matches(host: &str, domain: &str) -> bool {
    let d = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    !d.is_empty() && (host == d || host.ends_with(&format!(".{d}")))
}

/// `path` is `prefix` or lives under it, at a segment boundary. An empty prefix matches everything.
pub fn path_under(path: &str, prefix: &str) -> bool {
    // Fail closed: a browser resolves dot-segments and percent-encoded dots/slashes, so such a path
    // may point outside the prefix it appears to be under.
    if path.contains('%') || path.split('/').any(|s| s == "." || s == "..") {
        return false;
    }
    let p = prefix.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() {
        return true;
    }
    let p = if p.starts_with('/') { p } else { format!("/{p}") };
    path == p || path.starts_with(&format!("{p}/"))
}

pub struct DomainRules<'a> {
    pub dominios: &'a [String],
    pub dominios_ats: &'a [String],
    pub caminho_ats: Option<&'a str>,
}

/// A career-site domain matches any path; an ATS domain matches only under the company path.
pub fn url_matches(rules: &DomainRules, url: &str) -> bool {
    let Some((host, path)) = host_and_path(url) else { return false };
    if rules.dominios.iter().any(|d| host_matches(&host, d)) {
        return true;
    }
    rules.dominios_ats.iter().any(|d| host_matches(&host, d))
        && rules.caminho_ats.is_none_or(|p| path_under(&path, p))
}

/// Stable key of a user-added site: its host without "www.", dots turned into dashes.
pub fn chave_from_url(url: &str) -> Option<String> {
    let (host, _) = host_and_path(url)?;
    let host = host.strip_prefix("www.").unwrap_or(&host);
    Some(host.replace('.', "-"))
}

/// A bare, lowercase domain with at least one dot: the shape stored in the seed.
pub fn valid_domain(d: &str) -> bool {
    !d.is_empty()
        && d.contains('.')
        && !d.starts_with('.')
        && !d.ends_with('.')
        && !d.contains("..")
        && d.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules<'a>(d: &'a [String], a: &'a [String], p: Option<&'a str>) -> DomainRules<'a> {
        DomainRules { dominios: d, dominios_ats: a, caminho_ats: p }
    }
    fn v(xs: &[&str]) -> Vec<String> { xs.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn the_real_host_is_the_one_after_the_last_at_sign() {
        assert_eq!(host_and_path("https://linkedin.com@evil.com/x").unwrap().0, "evil.com");
    }

    #[test]
    fn a_backslash_counts_as_a_slash_like_in_a_browser() {
        // Chrome reads this as host evil.com, path "/@linkedin.com".
        assert_eq!(host_and_path("https://evil.com\\@linkedin.com").unwrap().0, "evil.com");
    }

    #[test]
    fn host_is_lowercased_and_loses_port_and_trailing_dot() {
        assert_eq!(host_and_path("HTTPS://LinkedIn.COM.:443/Jobs?q=1#x").unwrap(), ("linkedin.com".to_string(), "/jobs".to_string()));
    }

    #[test]
    fn only_plain_http_and_https_with_a_clean_host_are_read() {
        for bad in ["", "ftp://a.com", "javascript:alert(1)", "https://", "https://[::1]/", "https://a b.com", "https://a.com/\n", "https://bücher.de/"] {
            assert!(host_and_path(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_domain_matches_itself_and_its_subdomains_only_at_a_label_boundary() {
        assert!(host_matches("linkedin.com", "linkedin.com"));
        assert!(host_matches("www.linkedin.com", "linkedin.com"));
        assert!(!host_matches("evil-linkedin.com", "linkedin.com"));
        assert!(!host_matches("linkedin.com.evil.com", "linkedin.com"));
        assert!(!host_matches("notlinkedin.com", "linkedin.com"));
        assert!(!host_matches("linkedin.com", ""));
    }

    #[test]
    fn a_path_prefix_matches_at_a_segment_boundary() {
        assert!(path_under("/canonical", "/canonical"));
        assert!(path_under("/canonical/jobs/1", "/canonical"));
        assert!(!path_under("/canonicalx", "/canonical"));
        assert!(!path_under("/other/canonical", "/canonical"));
        assert!(path_under("/anything", ""));
    }

    #[test]
    fn ats_domains_need_the_company_path_but_career_domains_do_not() {
        let dom = v(&["canonical.com"]);
        let ats = v(&["job-boards.greenhouse.io"]);
        let r = rules(&dom, &ats, Some("/canonical"));
        assert!(url_matches(&r, "https://canonical.com/careers"));
        assert!(url_matches(&r, "https://job-boards.greenhouse.io/canonical/jobs/1"));
        assert!(!url_matches(&r, "https://job-boards.greenhouse.io/anthropic"));
        assert!(!url_matches(&r, "https://greenhouse.io/canonical"));
    }

    #[test]
    fn tricky_urls_never_match_the_trusted_domain() {
        let dom = v(&["linkedin.com"]);
        let r = rules(&dom, &[], None);
        for url in ["https://linkedin.com@evil.com/", "https://evil.com\\@linkedin.com", "https://linkedin.com.evil.com/", "https://evil.com/linkedin.com"] {
            assert!(!url_matches(&r, url), "{url}");
        }
        for url in ["https://LINKEDIN.COM./", "https://linkedin.com:443/jobs", "http://www.linkedin.com/jobs"] {
            assert!(url_matches(&r, url), "{url}");
        }
    }

    #[test]
    fn the_key_of_a_user_added_site_comes_from_its_domain() {
        assert_eq!(chave_from_url("https://www.Jobindex.dk/jobsoegning").as_deref(), Some("jobindex-dk"));
        assert_eq!(chave_from_url("not a url"), None);
    }

    #[test]
    fn dot_segments_and_percent_encoding_cannot_escape_the_ats_company_path() {
        let dom = v(&["canonical.com"]);
        let ats = v(&["job-boards.greenhouse.io"]);
        let r = rules(&dom, &ats, Some("/canonical"));
        for url in [
            "https://job-boards.greenhouse.io/canonical/../anthropic",
            "https://job-boards.greenhouse.io/canonical/%2e%2e/anthropic",
            "https://job-boards.greenhouse.io/canonical/..%2fanthropic",
        ] {
            assert!(!url_matches(&r, url), "{url}");
        }
        assert!(url_matches(&r, "https://job-boards.greenhouse.io/canonical/jobs/1"));
        assert!(url_matches(&r, "https://canonical.com/careers/a%20b"));
    }

    #[test]
    fn empty_labels_and_malformed_ports_are_refused() {
        for bad in ["https://.linkedin.com/", "https://a..com/", "https://linkedin.com:abc/", "https://linkedin.com:443:x/"] {
            assert!(host_and_path(bad).is_none(), "{bad}");
        }
        assert_eq!(host_and_path("https://linkedin.com:443/").unwrap().0, "linkedin.com");
    }

    #[test]
    fn a_domain_for_the_seed_is_bare_lowercase_with_a_dot() {
        assert!(valid_domain("linkedin.com"));
        assert!(valid_domain("lego.wd103.myworkdayjobs.com"));
        for bad in ["", "localhost", "https://a.com", "a.com/x", "A.com", "a..com", ".a.com", "a.com."] {
            assert!(!valid_domain(bad), "{bad}");
        }
    }
}
