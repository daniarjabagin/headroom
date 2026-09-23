use jiff::SignedDuration;

const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_DAY: i64 = 86_400;
const NOISE_SEGMENTS: [&str; 2] = ["gpt", "codex"];

pub(super) fn plan_label(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "prolite" => "Pro 5x".to_owned(),
        "pro" => "Pro 20x".to_owned(),
        other => other
            .split('_')
            .filter(|part| !part.is_empty())
            .map(capitalize)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

pub(super) fn limit_label(limit_name: &str) -> String {
    let meaningful: Vec<&str> = limit_name
        .split(['-', '_', ' '])
        .filter(|segment| !segment.is_empty() && !is_noise(segment))
        .collect();
    if meaningful.is_empty() {
        limit_name.trim().to_owned()
    } else {
        meaningful
            .into_iter()
            .map(capitalize)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(super) fn period_label(period: SignedDuration) -> String {
    let seconds = period.as_secs();
    if seconds > 0 && seconds % SECONDS_PER_DAY == 0 {
        format!("{}d", seconds / SECONDS_PER_DAY)
    } else if seconds > 0 && seconds % SECONDS_PER_HOUR == 0 {
        format!("{}h", seconds / SECONDS_PER_HOUR)
    } else if seconds > 0 && seconds % SECONDS_PER_MINUTE == 0 {
        format!("{}m", seconds / SECONDS_PER_MINUTE)
    } else {
        format!("{seconds}s")
    }
}

fn is_noise(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    NOISE_SEGMENTS.contains(&lower.as_str()) || lower.starts_with(|c: char| c.is_ascii_digit())
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_labels_follow_codex_naming() {
        assert_eq!(plan_label("prolite"), "Pro 5x");
        assert_eq!(plan_label("pro"), "Pro 20x");
        assert_eq!(plan_label("plus"), "Plus");
        assert_eq!(plan_label("enterprise_cbp_usage"), "Enterprise Cbp Usage");
    }

    #[test]
    fn limit_labels_drop_vendor_and_version_segments() {
        assert_eq!(limit_label("GPT-5.3-Codex-Spark"), "Spark");
        assert_eq!(limit_label("GPT-Codex-Spark"), "Spark");
        assert_eq!(limit_label("codex_spark_preview"), "Spark Preview");
        assert_eq!(limit_label("GPT-5"), "GPT-5");
    }

    #[test]
    fn period_labels_use_largest_whole_unit() {
        assert_eq!(period_label(SignedDuration::from_hours(24)), "1d");
        assert_eq!(period_label(SignedDuration::from_hours(3)), "3h");
        assert_eq!(period_label(SignedDuration::from_mins(90)), "90m");
        assert_eq!(period_label(SignedDuration::from_secs(45)), "45s");
    }
}
