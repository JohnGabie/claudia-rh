//! Plain text of a generated document's HTML, for the agent to read (get_documento). Never runs
//! anything: drops tags, scripts and styles, and decodes the few entities our templates emit.
pub fn html_para_texto(html: &str) -> String {
    // ASCII lowercasing keeps byte offsets, so `lower` and `html` index the same characters.
    let lower = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len() / 2);
    let mut i = 0;
    while i < html.len() {
        let rest = &lower[i..];
        if rest.starts_with("<script") || rest.starts_with("<style") {
            let close = if rest.starts_with("<script") { "</script>" } else { "</style>" };
            match rest.find(close) {
                Some(j) => {
                    i += j + close.len();
                    continue;
                }
                None => break,
            }
        }
        if bytes[i] == b'<' {
            let block = ["<br", "<p", "</p", "<div", "</div", "<li", "<h1", "<h2", "<h3", "<h4", "</h", "<tr"].iter().any(|t| rest.starts_with(t));
            match html[i..].find('>') {
                Some(j) => {
                    if block {
                        out.push('\n');
                    }
                    i += j + 1;
                    continue;
                }
                None => break,
            }
        }
        let ch = html[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    decoded
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_scripts_and_styles_go_and_entities_are_decoded() {
        let t = html_para_texto("<html><style>p{color:red}</style><script>alert(1)</script><h1>João &amp; Cia</h1><p>Rust&nbsp;&lt;3</p><ul><li>A</li><li>B</li></ul></html>");
        assert_eq!(t, "João & Cia\nRust <3\nA\nB");
    }

    #[test]
    fn an_escaped_tag_stays_text() {
        assert_eq!(html_para_texto("<p>&amp;lt;b&amp;gt;</p>"), "&lt;b&gt;");
    }
}
