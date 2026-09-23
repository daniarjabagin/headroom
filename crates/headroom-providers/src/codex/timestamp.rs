use jiff::{SignedDuration, Timestamp};

const MAX_FRACTION_DIGITS: usize = 9;
const DATE_LEN: usize = 10;

pub(super) fn parse_log_timestamp(raw: &str) -> Option<Timestamp> {
    normalize(raw).parse().ok()
}

pub(super) fn from_epoch_seconds(seconds: f64) -> Option<Timestamp> {
    let duration = SignedDuration::try_from_secs_f64(seconds).ok()?;
    Timestamp::from_duration(duration).ok()
}

fn normalize(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut text = trimmed
        .strip_suffix(" UTC")
        .map_or_else(|| trimmed.to_owned(), |base| format!("{base}Z"));
    if text.as_bytes().get(DATE_LEN) == Some(&b' ') {
        text.replace_range(DATE_LEN..=DATE_LEN, "T");
    }
    let text = truncate_fraction(&text);
    if has_zone(&text) {
        text
    } else {
        format!("{text}Z")
    }
}

fn truncate_fraction(text: &str) -> String {
    let Some(dot) = text.find('.') else {
        return text.to_owned();
    };
    let digits = text[dot + 1..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digits <= MAX_FRACTION_DIGITS {
        return text.to_owned();
    }
    let keep_end = dot + 1 + MAX_FRACTION_DIGITS;
    let rest_start = dot + 1 + digits;
    format!("{}{}", &text[..keep_end], &text[rest_start..])
}

fn has_zone(text: &str) -> bool {
    let Some(time) = text.get(DATE_LEN..) else {
        return false;
    };
    time.ends_with('Z') || time.ends_with('z') || time.contains('+') || time.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(raw: &str) -> String {
        parse_log_timestamp(raw).unwrap().to_string()
    }

    #[test]
    fn parses_rfc3339_with_millis() {
        assert_eq!(
            parsed("2026-09-23T06:21:40.235Z"),
            "2026-09-23T06:21:40.235Z"
        );
    }

    #[test]
    fn accepts_any_fraction_length() {
        assert_eq!(parsed("2026-09-23T06:21:40.2Z"), "2026-09-23T06:21:40.2Z");
        assert_eq!(
            parsed("2026-09-23T06:21:40.123456789123Z"),
            "2026-09-23T06:21:40.123456789Z"
        );
    }

    #[test]
    fn adds_missing_zone_and_accepts_space_separator() {
        assert_eq!(parsed("2026-09-23 06:21:40"), "2026-09-23T06:21:40Z");
        assert_eq!(parsed("2026-09-23 06:21:40 UTC"), "2026-09-23T06:21:40Z");
    }

    #[test]
    fn converts_offsets_to_utc() {
        assert_eq!(parsed("2026-09-23T11:21:40+05:00"), "2026-09-23T06:21:40Z");
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_log_timestamp("yesterday"), None);
        assert_eq!(parse_log_timestamp(""), None);
    }

    #[test]
    fn epoch_seconds_accept_fractions() {
        assert_eq!(
            from_epoch_seconds(1_790_000_000.5).unwrap().to_string(),
            "2026-09-21T14:13:20.5Z"
        );
        assert_eq!(from_epoch_seconds(f64::NAN), None);
    }
}
