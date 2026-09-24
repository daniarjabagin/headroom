use std::borrow::Cow;

pub fn printable(text: &str) -> Cow<'_, str> {
    if text.chars().any(char::is_control) {
        Cow::Owned(text.chars().filter(|c| !c.is_control()).collect())
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_borrowed_unchanged() {
        assert!(matches!(
            printable("Codex · ada@example.com"),
            Cow::Borrowed(_)
        ));
        assert_eq!(printable("né 5h"), "né 5h");
    }

    #[test]
    fn c0_c1_escape_and_osc_sequences_lose_their_control_characters() {
        assert_eq!(printable("a\x1b[2Jb"), "a[2Jb");
        assert_eq!(printable("\x1b]0;pwned\x07title"), "]0;pwnedtitle");
        assert_eq!(printable("x\u{9b}31my\u{9d}z"), "x31myz");
        assert_eq!(printable("line\nbreak\r\ttab\x7f"), "linebreaktab");
    }
}
