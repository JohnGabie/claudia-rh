pub fn redact(s: &str) -> String {
    let mut out = s.to_string();
    out = mask_emails(&out);
    out = mask_phones(&out);
    out = mask_bearer(&out);
    out = mask_sk_tokens(&out);
    out = mask_assignment(&out, "password=");
    out = mask_assignment(&out, "senha=");
    out
}

fn mask_emails(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        let split_at = rest
            .find(|c: char| c.is_whitespace() || is_email_boundary(c))
            .unwrap_or(rest.len());
        let (token, after) = rest.split_at(split_at);
        if is_email_token(token) {
            out.push_str("[email]");
        } else {
            out.push_str(token);
        }
        if after.is_empty() {
            break;
        }
        let boundary_len = after.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        out.push_str(&after[..boundary_len]);
        rest = &after[boundary_len..];
    }
    out
}

fn is_email_boundary(c: char) -> bool {
    matches!(c, ',' | ';' | ':' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | '<' | '>')
}

fn is_email_token(token: &str) -> bool {
    let Some((local, domain)) = token.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn mask_phones(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '+' || chars[i].is_ascii_digit() {
            let start = i;
            let mut digit_count = 0;
            let mut j = i;
            if chars[j] == '+' {
                j += 1;
            }
            while j < chars.len() {
                let c = chars[j];
                if c.is_ascii_digit() {
                    digit_count += 1;
                    j += 1;
                } else if matches!(c, ' ' | '-' | '(' | ')') {
                    j += 1;
                } else {
                    break;
                }
            }
            // Trim trailing separators that are not part of the phone number.
            while j > start {
                let prev = chars[j - 1];
                if matches!(prev, ' ' | '-' | '(' | ')') {
                    j -= 1;
                } else {
                    break;
                }
            }
            if digit_count >= 9 {
                out.push_str("[phone]");
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn mask_bearer(s: &str) -> String {
    // Replace "Bearer <token>" with "Bearer [token]" (preserve original Bearer casing).
    let mut out = String::with_capacity(s.len());
    let lower = s.to_ascii_lowercase();
    let mut i = 0;
    while i < s.len() {
        if lower[i..].starts_with("bearer ") {
            let prefix_end = i + "bearer ".len();
            out.push_str(&s[i..prefix_end]);
            out.push_str("[token]");
            i = prefix_end;
            while let Some(ch) = s[i..].chars().next() {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }
            continue;
        }
        let ch = s[i..].chars().next().expect("i is a char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn mask_sk_tokens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if let Some(prefix_len) = sk_prefix_len(&s[i..]) {
            let after_prefix = i + prefix_len;
            let mut j = after_prefix;
            let mut alnum_count = 0;
            while let Some(ch) = s[j..].chars().next() {
                if ch.is_ascii_alphanumeric() {
                    alnum_count += 1;
                    j += ch.len_utf8();
                } else {
                    break;
                }
            }
            if alnum_count >= 20 {
                out.push_str("[token]");
                i = j;
                continue;
            }
        }
        let ch = s[i..].chars().next().expect("i is a char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn sk_prefix_len(s: &str) -> Option<usize> {
    if s.starts_with("sk-ant-") {
        Some("sk-ant-".len())
    } else if s.starts_with("sk-") {
        Some("sk-".len())
    } else {
        None
    }
}

fn mask_assignment(s: &str, key: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let lower = s.to_ascii_lowercase();
    let key_lower = key.to_ascii_lowercase();
    let mut i = 0;
    while i < s.len() {
        if lower[i..].starts_with(&key_lower) {
            // Keep "password=" / "senha=" then replace value with [redacted].
            let eq_end = i + key.len();
            out.push_str(&s[i..eq_end]);
            out.push_str("[redacted]");
            i = eq_end;
            while let Some(ch) = s[i..].chars().next() {
                if ch.is_whitespace() {
                    break;
                }
                i += ch.len_utf8();
            }
            continue;
        }
        let ch = s[i..].chars().next().expect("i is a char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn masks_email_phone_and_token() {
        let raw = "write joao@mail.com or +5511999887766 with sk-ant-abcdefghijklmnopqrstuv";
        let out = redact(raw);
        assert!(out.contains("[email]"), "{out}");
        assert!(out.contains("[phone]"), "{out}");
        assert!(out.contains("[token]"), "{out}");
        assert!(!out.contains("joao@mail.com"));
        assert!(!out.contains("sk-ant-abcdefghijklmnopqrstuv"));
    }

    #[test]
    fn leaves_clean_text() {
        assert_eq!(redact("Acme Backend vaga"), "Acme Backend vaga");
    }

    #[test]
    fn masks_bearer_and_password_assignment() {
        let out = redact("Authorization: Bearer abcdefghijklmnopqrstuvwxyz12 password=secret123");
        assert!(out.contains("Bearer [token]"), "{out}");
        assert!(out.contains("[redacted]"), "{out}");
        assert!(!out.contains("abcdefghijklmnopqrstuvwxyz12"), "{out}");
        assert!(!out.contains("secret123"));
    }

    #[test]
    fn redacts_utf8_portuguese_logs() {
        let raw = "José de São Paulo: maria@empresa.com.br senha=segredo123";
        let out = redact(raw);
        assert!(out.contains("José de São Paulo"), "{out}");
        assert!(out.contains("[email]"), "{out}");
        assert!(out.contains("[redacted]"), "{out}");
        assert!(!out.contains("maria@empresa.com.br"), "{out}");
        assert!(!out.contains("segredo123"), "{out}");
    }
}
