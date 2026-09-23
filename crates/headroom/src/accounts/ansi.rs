use std::iter::Peekable;
use std::str::Chars;

const ESC: char = '\u{1b}';
const BEL: char = '\u{7}';
const HYPERLINK_PREFIX: &str = "8;";
const URL_SCHEMES: [&str; 2] = ["https://", "http://"];
const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];
const TRAILING_PUNCTUATION: &[char] = &['.', ',', ';', ':', '!', '?', ')', ']', '}', '\'', '"'];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CleanLine {
    pub text: String,
    pub links: Vec<String>,
}

impl CleanLine {
    pub fn parse(raw: &str) -> CleanLine {
        let mut line = CleanLine::default();
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                ESC => line.escape(&mut chars),
                '\r' => line.text.clear(),
                '\t' => line.text.push(c),
                c if c.is_control() => {}
                c => line.text.push(c),
            }
        }
        line
    }

    pub fn url(&self) -> Option<String> {
        first_url(&self.text).or_else(|| self.links.iter().find_map(|link| first_url(link)))
    }

    fn escape(&mut self, chars: &mut Peekable<Chars<'_>>) {
        match chars.next() {
            Some('[') => skip_csi(chars),
            Some(']') => {
                let payload = osc_payload(chars);
                if let Some(link) = hyperlink_target(&payload) {
                    self.links.push(link.to_owned());
                }
            }
            Some('(' | ')' | '*' | '+') => {
                chars.next();
            }
            _ => {}
        }
    }
}

fn skip_csi(chars: &mut Peekable<Chars<'_>>) {
    for c in chars.by_ref() {
        if ('@'..='~').contains(&c) {
            return;
        }
    }
}

fn osc_payload(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut payload = String::new();
    while let Some(c) = chars.next() {
        match c {
            BEL => break,
            ESC => {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                break;
            }
            c => payload.push(c),
        }
    }
    payload
}

fn hyperlink_target(payload: &str) -> Option<&str> {
    let rest = payload.strip_prefix(HYPERLINK_PREFIX)?;
    let (_, target) = rest.split_once(';')?;
    (!target.is_empty()).then_some(target)
}

fn first_url(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some((start, scheme)) = earliest_scheme(rest) {
        let candidate = url_at(&rest[start..]);
        if candidate.len() > scheme.len() && !is_loopback(&candidate[scheme.len()..]) {
            return Some(candidate.to_owned());
        }
        rest = &rest[start + scheme.len()..];
    }
    None
}

fn earliest_scheme(text: &str) -> Option<(usize, &'static str)> {
    URL_SCHEMES
        .iter()
        .filter_map(|scheme| text.find(scheme).map(|at| (at, *scheme)))
        .min_by_key(|(at, _)| *at)
}

fn url_at(text: &str) -> &str {
    let end = text
        .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`'))
        .unwrap_or(text.len());
    text[..end].trim_end_matches(TRAILING_PUNCTUATION)
}

fn is_loopback(after_scheme: &str) -> bool {
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    let host = if authority.starts_with('[') {
        authority.split_inclusive(']').next().unwrap_or(authority)
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    LOOPBACK_HOSTS.contains(&host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_colors_cursor_moves_and_charsets() {
        let line = CleanLine::parse("\u{1b}[1;32mSigned\u{1b}[0m in \u{1b}(Bnow\u{1b}[2K\u{7}");
        assert_eq!(line.text, "Signed in now");
        assert!(line.links.is_empty());
    }

    #[test]
    fn carriage_returns_keep_the_last_redraw() {
        assert_eq!(CleanLine::parse("Waiting.\rWaiting..\rDone").text, "Done");
    }

    #[test]
    fn hyperlinks_keep_their_label_and_remember_the_target() {
        let raw =
            "Open \u{1b}]8;;https://claude.ai/oauth?x=1\u{1b}\\this link\u{1b}]8;;\u{1b}\\ now";
        let line = CleanLine::parse(raw);
        assert_eq!(line.text, "Open this link now");
        assert_eq!(line.links, ["https://claude.ai/oauth?x=1"]);
        assert_eq!(line.url().as_deref(), Some("https://claude.ai/oauth?x=1"));
        let bel = CleanLine::parse("\u{1b}]8;id=1;https://a.example/\u{7}x\u{1b}]8;;\u{7}");
        assert_eq!(bel.links, ["https://a.example/"]);
    }

    #[test]
    fn finds_the_first_public_url_in_a_line() {
        let cases = [
            (
                "visit: https://auth.openai.com/oauth/authorize?a=1&b=2",
                Some("https://auth.openai.com/oauth/authorize?a=1&b=2"),
            ),
            (
                "Go to (https://example.com/path).",
                Some("https://example.com/path"),
            ),
            (
                "\"http://example.com/a\" or https://b.example",
                Some("http://example.com/a"),
            ),
            (
                "Starting local login server on http://localhost:1455.",
                None,
            ),
            (
                "callback http://127.0.0.1:8080/cb then https://x.example/y",
                Some("https://x.example/y"),
            ),
            ("ipv6 http://[::1]:9/ only", None),
            ("https:// alone", None),
            ("no link here", None),
        ];
        for (text, expected) in cases {
            assert_eq!(CleanLine::parse(text).url().as_deref(), expected, "{text}");
        }
    }

    #[test]
    fn visible_urls_win_over_hyperlink_targets() {
        let raw = "\u{1b}]8;;https://hidden.example/\u{7}https://shown.example/\u{1b}]8;;\u{7}";
        assert_eq!(
            CleanLine::parse(raw).url().as_deref(),
            Some("https://shown.example/")
        );
    }
}
