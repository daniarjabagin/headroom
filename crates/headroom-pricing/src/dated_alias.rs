use std::collections::BTreeMap;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct DatedAlias {
    from: Option<Date>,
    model: String,
}

#[derive(Debug, Default)]
pub(crate) struct DatedAliases(BTreeMap<String, Vec<DatedAlias>>);

impl DatedAliases {
    pub(crate) fn new(raw: BTreeMap<String, Vec<DatedAlias>>) -> DatedAliases {
        DatedAliases(
            raw.into_iter()
                .map(|(name, ranges)| (name.to_lowercase(), newest_first(ranges)))
                .collect(),
        )
    }

    pub(crate) fn resolve(&self, name: &str, at: Timestamp) -> Option<&str> {
        let day = TimeZone::UTC.to_datetime(at).date();
        self.0
            .get(name)?
            .iter()
            .find(|range| range.from.is_none_or(|from| from <= day))
            .map(|range| range.model.as_str())
    }
}

fn newest_first(mut ranges: Vec<DatedAlias>) -> Vec<DatedAlias> {
    for range in &mut ranges {
        range.model = range.model.to_lowercase();
    }
    ranges.sort_by_key(|range| std::cmp::Reverse(range.from));
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aliases() -> DatedAliases {
        let raw = serde_json::from_str(
            r#"{ "Auto": [
                { "model": "Base" },
                { "from": "2026-04-23", "model": "new" },
                { "from": "2026-03-05", "model": "mid" }
            ] }"#,
        )
        .unwrap();
        DatedAliases::new(raw)
    }

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn picks_the_newest_range_that_started_by_the_event_day() {
        let aliases = aliases();
        let cases = [
            ("2026-09-23T10:00:00Z", Some("new")),
            ("2026-04-23T00:00:00Z", Some("new")),
            ("2026-04-22T23:59:59Z", Some("mid")),
            ("2026-03-05T00:00:00Z", Some("mid")),
            ("2026-03-04T23:59:59Z", Some("base")),
            ("2020-01-01T00:00:00Z", Some("base")),
        ];
        for (time, expected) in cases {
            assert_eq!(aliases.resolve("auto", at(time)), expected, "{time}");
        }
    }

    #[test]
    fn boundaries_use_the_utc_day() {
        let aliases = aliases();
        let late_local = at("2026-04-23T01:00:00+05:00");
        assert_eq!(aliases.resolve("auto", late_local), Some("mid"));
    }

    #[test]
    fn names_without_ranges_are_not_aliased() {
        assert_eq!(aliases().resolve("other", at("2026-09-23T10:00:00Z")), None);
    }

    #[test]
    fn ranges_without_an_open_start_leave_older_events_unaliased() {
        let raw = serde_json::from_str(r#"{ "auto": [{ "from": "2026-01-01", "model": "x" }] }"#)
            .unwrap();
        let aliases = DatedAliases::new(raw);
        assert_eq!(aliases.resolve("auto", at("2025-12-31T12:00:00Z")), None);
    }
}
