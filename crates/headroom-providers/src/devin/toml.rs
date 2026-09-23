pub(super) fn top_level_string(text: &str, key: &str) -> Option<String> {
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            return None;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if unquote_key(name.trim()) == key {
            return string_value(value.trim());
        }
    }
    None
}

fn unquote_key(name: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|quote| {
            name.strip_prefix(*quote)
                .and_then(|rest| rest.strip_suffix(*quote))
        })
        .unwrap_or(name)
}

fn string_value(value: &str) -> Option<String> {
    if let Some(rest) = value.strip_prefix('"') {
        basic_string(rest)
    } else if let Some(rest) = value.strip_prefix('\'') {
        rest.split_once('\'').map(|(inner, _)| inner.to_owned())
    } else {
        None
    }
}

fn basic_string(rest: &str) -> Option<String> {
    let mut output = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(output),
            '\\' => output.push(escaped(chars.next()?)?),
            c => output.push(c),
        }
    }
    None
}

fn escaped(c: char) -> Option<char> {
    match c {
        '"' => Some('"'),
        '\\' => Some('\\'),
        'n' => Some('\n'),
        't' => Some('\t'),
        '/' => Some('/'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_and_literal_strings_are_read() {
        let text = "# Devin\nwindsurf_api_key = \"devin-session-token$abc\" # note\n\
                    api_server_url = 'https://example.test/'\n";
        assert_eq!(
            top_level_string(text, "windsurf_api_key").as_deref(),
            Some("devin-session-token$abc")
        );
        assert_eq!(
            top_level_string(text, "api_server_url").as_deref(),
            Some("https://example.test/")
        );
    }

    #[test]
    fn escapes_and_quoted_keys_are_understood() {
        let text = "\"windsurf_api_key\" = \"a\\\"b\\\\c\"";
        assert_eq!(
            top_level_string(text, "windsurf_api_key").as_deref(),
            Some("a\"b\\c")
        );
    }

    #[test]
    fn keys_inside_tables_bare_values_and_broken_strings_are_ignored() {
        assert_eq!(top_level_string("[other]\nkey = \"x\"", "key"), None);
        assert_eq!(top_level_string("key = 42", "key"), None);
        assert_eq!(top_level_string("key = \"unterminated", "key"), None);
        assert_eq!(top_level_string("key = \"bad \\q escape\"", "key"), None);
        assert_eq!(top_level_string("other = \"x\"", "key"), None);
    }
}
