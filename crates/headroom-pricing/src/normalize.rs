const PROVIDER_PREFIXES: [&str; 2] = ["anthropic/", "openai/"];
const DASHED_DATE: &str = "-0000-00-00";

pub(crate) fn normalize(model: &str) -> String {
    let lowered = model.trim().to_lowercase();
    let mut name = lowered.as_str();
    for prefix in PROVIDER_PREFIXES {
        name = name.strip_prefix(prefix).unwrap_or(name);
    }
    name = name.split_once('@').map_or(name, |(head, _)| head);
    name = strip_bracket_suffix(name);
    name.trim().to_owned()
}

pub(crate) fn strip_date(name: &str) -> &str {
    compact_date_stem(name)
        .or_else(|| dashed_date_stem(name))
        .unwrap_or(name)
}

pub(crate) fn stems<'a>(name: &'a str, qualifiers: &[String]) -> Vec<&'a str> {
    let mut stems = vec![name];
    let mut current = strip_date(name);
    push_unique(&mut stems, current);
    while let Some(shorter) = strip_qualifier(current, qualifiers) {
        current = strip_date(shorter);
        push_unique(&mut stems, current);
    }
    stems
}

fn strip_bracket_suffix(name: &str) -> &str {
    match name
        .strip_suffix(']')
        .and_then(|rest| rest.rsplit_once('['))
    {
        Some((head, _)) => head,
        None => name,
    }
}

fn compact_date_stem(name: &str) -> Option<&str> {
    let (stem, date) = name.rsplit_once('-')?;
    (date.len() == 8 && all_digits(date) && !stem.is_empty()).then_some(stem)
}

fn dashed_date_stem(name: &str) -> Option<&str> {
    let split = name.len().checked_sub(DASHED_DATE.len())?;
    let (stem, date) = (name.get(..split)?, name.get(split..)?);
    let shaped = date
        .bytes()
        .zip(DASHED_DATE.bytes())
        .all(|(byte, shape)| match shape {
            b'-' => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    (shaped && !stem.is_empty()).then_some(stem)
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

fn strip_qualifier<'a>(name: &'a str, qualifiers: &[String]) -> Option<&'a str> {
    qualifiers
        .iter()
        .filter_map(|suffix| name.strip_suffix(suffix.as_str()))
        .find(|stem| !stem.is_empty())
}

fn push_unique<'a>(stems: &mut Vec<&'a str>, stem: &'a str) {
    if !stems.contains(&stem) {
        stems.push(stem);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_prefix_case_brackets_and_at_suffix() {
        let cases = [
            ("claude-opus-5-5", "claude-opus-5-5"),
            ("Claude-Sonnet-5", "claude-sonnet-5"),
            ("anthropic/claude-sonnet-5", "claude-sonnet-5"),
            ("openai/gpt-5.5", "gpt-5.5"),
            ("claude-opus-5[1m]", "claude-opus-5"),
            ("claude-haiku-4-5@20251001", "claude-haiku-4-5"),
            ("  gpt-5.5  ", "gpt-5.5"),
            ("claude-sonnet-4-5-20250929", "claude-sonnet-4-5-20250929"),
        ];
        for (raw, expected) in cases {
            assert_eq!(normalize(raw), expected, "{raw}");
        }
    }

    #[test]
    fn strip_date_handles_both_date_shapes() {
        let cases = [
            ("claude-haiku-4-5-20251001", "claude-haiku-4-5"),
            ("gpt-5.5-2026-04-23", "gpt-5.5"),
            ("gpt-5.5", "gpt-5.5"),
            ("claude-sonnet-4-5", "claude-sonnet-4-5"),
            ("model-1234567", "model-1234567"),
            ("20251001", "20251001"),
            ("gpt-2026-04-2x", "gpt-2026-04-2x"),
        ];
        for (raw, expected) in cases {
            assert_eq!(strip_date(raw), expected, "{raw}");
        }
    }

    #[test]
    fn stems_drop_dates_then_known_qualifiers_only() {
        let qualifiers = vec!["-high".to_owned(), "-thinking".to_owned()];
        assert_eq!(
            stems("claude-opus-5-thinking-high", &qualifiers),
            vec![
                "claude-opus-5-thinking-high",
                "claude-opus-5-thinking",
                "claude-opus-5"
            ]
        );
        assert_eq!(
            stems("gpt-5.5-2026-04-23", &qualifiers),
            vec!["gpt-5.5-2026-04-23", "gpt-5.5"]
        );
        assert_eq!(stems("gpt-5.5-mini", &qualifiers), vec!["gpt-5.5-mini"]);
        assert_eq!(stems("-high", &qualifiers), vec!["-high"]);
    }
}
