mod ru;

use crate::payload::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

impl Lang {
    #[must_use]
    pub fn resolve(setting: Language, system_locale: Option<&str>) -> Self {
        match setting {
            Language::En => Lang::En,
            Language::Ru => Lang::Ru,
            Language::System => match system_locale {
                Some(locale) if locale.starts_with("ru") => Lang::Ru,
                _ => Lang::En,
            },
        }
    }

    #[must_use]
    pub fn tr(self, msgid: &'static str) -> &'static str {
        match self {
            Lang::En => msgid,
            Lang::Ru => ru::lookup(msgid).unwrap_or(msgid),
        }
    }

    #[must_use]
    pub fn tr_plural(self, forms: [&'static str; 2], count: u64) -> &'static str {
        let english = forms[usize::from(count != 1)];
        match self {
            Lang::En => english,
            Lang::Ru => ru::plural(forms[0]).map_or(english, |ru| ru[russian_plural(count)]),
        }
    }
}

fn russian_plural(count: u64) -> usize {
    let last_digit = count % 10;
    let last_two = count % 100;
    if last_digit == 1 && last_two != 11 {
        0
    } else if (2..=4).contains(&last_digit) && !(12..=14).contains(&last_two) {
        1
    } else {
        2
    }
}

#[must_use]
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        match values.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            None => out.push_str(&rest[open..=open + close + 1]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

#[must_use]
pub fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_named_placeholders() {
        assert_eq!(fill("{a} and {b}", &[("a", "1"), ("b", "2")]), "1 and 2");
        assert_eq!(fill("keep {x}", &[]), "keep {x}");
        assert_eq!(fill("open { brace", &[]), "open { brace");
    }

    #[test]
    fn resolves_the_language() {
        assert_eq!(
            Lang::resolve(Language::System, Some("ru_RU.UTF-8")),
            Lang::Ru
        );
        assert_eq!(Lang::resolve(Language::System, Some("de_DE")), Lang::En);
        assert_eq!(Lang::resolve(Language::System, None), Lang::En);
        assert_eq!(Lang::resolve(Language::Ru, Some("en_US")), Lang::Ru);
    }

    #[test]
    fn russian_plural_forms() {
        let cases = [(1, 0), (2, 1), (5, 2), (11, 2), (21, 0), (22, 1), (112, 2)];
        for (count, index) in cases {
            assert_eq!(russian_plural(count), index, "{count}");
        }
        let forms = ["{tokens} token", "{tokens} tokens"];
        assert_eq!(Lang::En.tr_plural(forms, 1), "{tokens} token");
        assert_eq!(Lang::En.tr_plural(forms, 0), "{tokens} tokens");
        assert_eq!(Lang::Ru.tr_plural(forms, 3), "{tokens} токена");
    }

    #[test]
    fn translates_known_strings_only() {
        assert_eq!(Lang::Ru.tr("Session"), "Сессия");
        assert_eq!(Lang::Ru.tr("Unknown text"), "Unknown text");
        assert_eq!(Lang::En.tr("Session"), "Session");
    }
}
