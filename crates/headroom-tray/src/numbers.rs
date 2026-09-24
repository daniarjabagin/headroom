use crate::i18n::{Lang, fill};

const MICROS_PER_CENT: i64 = 10_000;
const COMPACT_FROM: u64 = 1000;
const UNITS: [(u128, &str); 3] = [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")];
const RU_GROUP: char = '\u{a0}';
const TOKEN_FORMS: [&str; 2] = ["{tokens} token", "{tokens} tokens"];

fn ru_unit(suffix: &str) -> &'static str {
    match suffix {
        "B" => "\u{a0}млрд",
        "M" => "\u{a0}млн",
        "K" => "\u{a0}тыс.",
        _ => "",
    }
}

fn token_digits(whole: u128) -> u32 {
    u32::from(whole < 100)
}

fn money_digits(whole: u128) -> u32 {
    match whole {
        100.. => 0,
        10.. => 1,
        _ => 2,
    }
}

fn scaled_text(value: u128, size: u128, digits: u32) -> String {
    let factor = 10u128.pow(digits);
    let rounded = (value * factor * 2 + size) / (size * 2);
    if digits == 0 {
        return rounded.to_string();
    }
    let whole = rounded / factor;
    let fraction = rounded % factor;
    if fraction == 0 {
        return whole.to_string();
    }
    format!("{whole}.{fraction:0width$}", width = digits as usize)
}

fn abbreviate(value: u128, unit: u128, digits_for: fn(u128) -> u32) -> (String, &'static str) {
    for (size, suffix) in UNITS {
        let size = size * unit;
        if value >= size {
            return (scaled_text(value, size, digits_for(value / size)), suffix);
        }
    }
    (scaled_text(value, unit, digits_for(value / unit)), "")
}

fn group_digits(value: u128, separator: char) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit);
    }
    out
}

#[must_use]
pub fn compact_tokens(lang: Lang, count: u64) -> String {
    let (text, suffix) = abbreviate(u128::from(count), 1, token_digits);
    match lang {
        Lang::En => format!("{text}{suffix}"),
        Lang::Ru => format!("{}{}", text.replace('.', ","), ru_unit(suffix)),
    }
}

#[must_use]
pub fn exact_tokens(lang: Lang, count: u64) -> String {
    let separator = if lang == Lang::Ru { RU_GROUP } else { ',' };
    group_digits(u128::from(count), separator)
}

#[must_use]
pub fn compact_tokens_text(lang: Lang, count: u64) -> String {
    let word_count = if count < COMPACT_FROM { count } else { 0 };
    let template = lang.tr_plural(TOKEN_FORMS, word_count);
    fill(template, &[("tokens", &compact_tokens(lang, count))])
}

#[must_use]
pub fn exact_tokens_text(lang: Lang, count: u64) -> String {
    let template = lang.tr_plural(TOKEN_FORMS, count);
    fill(template, &[("tokens", &exact_tokens(lang, count))])
}

fn cents_of(micros: i64) -> i128 {
    let micros = i128::from(micros);
    let per_cent = i128::from(MICROS_PER_CENT);
    let half = per_cent / 2;
    if micros >= 0 {
        (micros + half) / per_cent
    } else {
        (micros - half) / per_cent
    }
}

fn signed(negative: bool, text: String) -> String {
    if negative { format!("-{text}") } else { text }
}

#[must_use]
pub fn exact_usd(micros: i64) -> String {
    let cents = cents_of(micros);
    let magnitude = cents.unsigned_abs();
    let dollars = group_digits(magnitude / 100, ',');
    signed(cents < 0, format!("${dollars}.{:02}", magnitude % 100))
}

#[must_use]
pub fn usd(micros: i64) -> String {
    let cents = cents_of(micros);
    let magnitude = cents.unsigned_abs();
    if magnitude < 100_000 {
        return exact_usd(micros);
    }
    let (text, suffix) = abbreviate(magnitude, 100, money_digits);
    signed(cents < 0, format!("${text}{suffix}"))
}

#[must_use]
pub fn ring_usd(micros: i64) -> String {
    let cents = cents_of(micros);
    let magnitude = cents.unsigned_abs();
    if (10_000..1_000_000).contains(&magnitude) {
        return signed(cents < 0, format!("${}", (magnitude + 50) / 100));
    }
    usd(micros)
}

#[must_use]
pub fn spend_line(lang: Lang, cost_micros: i64, total_tokens: u64) -> String {
    if cost_micros == 0 && total_tokens == 0 {
        return lang.tr("No data").to_owned();
    }
    format!(
        "{} · {}",
        usd(cost_micros),
        compact_tokens_text(lang, total_tokens)
    )
}

#[must_use]
pub fn exact_spend_line(lang: Lang, cost_micros: i64, total_tokens: u64, partial: bool) -> String {
    let suffix = if partial {
        lang.tr(" · some models unpriced")
    } else {
        ""
    };
    format!(
        "{} · {}{suffix}",
        exact_usd(cost_micros),
        exact_tokens_text(lang, total_tokens)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_token_counts() {
        let cases = [
            (0, "0"),
            (999, "999"),
            (1_000, "1K"),
            (1_250, "1.3K"),
            (35_812_904, "35.8M"),
            (150_000_000, "150M"),
            (1_500_000_000, "1.5B"),
        ];
        for (count, expected) in cases {
            assert_eq!(compact_tokens(Lang::En, count), expected, "{count}");
        }
        assert_eq!(compact_tokens(Lang::Ru, 35_812_904), "35,8\u{a0}млн");
    }

    #[test]
    fn exact_token_counts() {
        assert_eq!(exact_tokens(Lang::En, 4_812_000), "4,812,000");
        assert_eq!(exact_tokens(Lang::Ru, 4_812_000), "4\u{a0}812\u{a0}000");
        assert_eq!(exact_tokens_text(Lang::En, 1), "1 token");
        assert_eq!(exact_tokens_text(Lang::Ru, 5), "5 токенов");
        assert_eq!(compact_tokens_text(Lang::En, 1_200_000), "1.2M tokens");
        assert_eq!(
            compact_tokens_text(Lang::Ru, 1_200_000),
            "1,2\u{a0}млн токенов"
        );
    }

    #[test]
    fn money() {
        let cases = [
            (0, "$0.00", "$0.00", "$0.00"),
            (4_080_000, "$4.08", "$4.08", "$4.08"),
            (22_005_000, "$22.01", "$22.01", "$22.01"),
            (463_400_000, "$463.40", "$463.40", "$463"),
            (2_060_000_000, "$2,060.00", "$2.06K", "$2060"),
            (12_345_678_000_000, "$12,345,678.00", "$12.3M", "$12.3M"),
        ];
        for (micros, exact, short, ring) in cases {
            assert_eq!(exact_usd(micros), exact, "{micros}");
            assert_eq!(usd(micros), short, "{micros}");
            assert_eq!(ring_usd(micros), ring, "{micros}");
        }
        assert_eq!(exact_usd(-1_500_000), "-$1.50");
    }

    #[test]
    fn spend_lines() {
        assert_eq!(spend_line(Lang::En, 0, 0), "No data");
        assert_eq!(
            spend_line(Lang::En, 4_080_000, 1_200_000),
            "$4.08 · 1.2M tokens"
        );
        assert_eq!(
            exact_spend_line(Lang::En, 4_080_000, 1_200_000, true),
            "$4.08 · 1,200,000 tokens · some models unpriced"
        );
    }
}
