pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                for d in chars.by_ref() {
                    if d == '\u{7}' || d == '\u{9c}' {
                        break;
                    }
                    if d == '\u{1b}' {
                        let _ = chars.next();
                        break;
                    }
                }
            }
            Some(_) => {
                let _ = chars.next();
            }
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;
    #[test]
    fn joins_letters_split_by_csi() {
        assert_eq!(strip_ansi("W\u{1b}[0maiting"), "Waiting");
    }
}
