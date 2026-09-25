use jiff::civil::Date;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{SpendPeriod, Tokens};
use crate::preferences::names::spend_period_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendGroup {
    Model,
    Project,
    Provider,
    Day,
}

impl SpendGroup {
    fn name(self) -> &'static str {
        match self {
            SpendGroup::Model => "model",
            SpendGroup::Project => "project",
            SpendGroup::Provider => "provider",
            SpendGroup::Day => "day",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendRange {
    Period(SpendPeriod),
    Since { since: Date, until: Option<Date> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendQuery {
    pub range: SpendRange,
    pub by: SpendGroup,
    pub provider: Option<String>,
}

impl SpendQuery {
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut query = Map::new();
        match self.range {
            SpendRange::Period(period) => {
                query.insert("period".into(), json!(spend_period_name(period)));
            }
            SpendRange::Since { since, until } => {
                query.insert("since".into(), json!(since.to_string()));
                if let Some(until) = until {
                    query.insert("until".into(), json!(until.to_string()));
                }
            }
        }
        query.insert("by".into(), json!(self.by.name()));
        if let Some(provider) = &self.provider {
            query.insert("provider".into(), json!(provider));
        }
        Value::Object(query).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SpendReport {
    pub since: Date,
    pub until: Date,
    pub by: String,
    pub rows: Vec<SpendRow>,
    pub total: SpendRow,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SpendRow {
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    pub tokens: Tokens,
    pub cost_usd_micros: i64,
    pub partial: bool,
    #[serde(default)]
    pub unpriced_tokens: u64,
    #[serde(default)]
    pub cost_per_mtok_usd_micros: Option<i64>,
    #[serde(default)]
    pub share_permille: u32,
}

pub fn parse_spend_report(json: &str) -> Result<SpendReport, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_name_a_period_or_a_range() {
        let period = SpendQuery {
            range: SpendRange::Period(SpendPeriod::Last7Days),
            by: SpendGroup::Project,
            provider: None,
        };
        assert_eq!(period.to_json(), r#"{"by":"project","period":"7d"}"#);
        let range = SpendQuery {
            range: SpendRange::Since {
                since: Date::new(2026, 9, 17).unwrap(),
                until: Some(Date::new(2026, 9, 20).unwrap()),
            },
            by: SpendGroup::Model,
            provider: Some("claude".into()),
        };
        assert_eq!(
            range.to_json(),
            r#"{"by":"model","provider":"claude","since":"2026-09-17","until":"2026-09-20"}"#
        );
    }

    #[test]
    fn parses_the_documented_report() {
        let json = r#"{
          "since": "2026-09-17", "until": "2026-09-23", "by": "model",
          "rows": [
            { "key": "claude-opus-4-5", "provider": "claude",
              "tokens": { "input": 1200000, "cache_read": 180000000, "cache_write": 9000000, "output": 2100000, "reasoning": 0, "total": 192300000 },
              "cost_usd_micros": 151200000, "partial": false, "unpriced_tokens": 0, "cost_per_mtok_usd_micros": 786271,
              "share_permille": 767 }
          ],
          "total": { "key": null, "provider": null,
              "tokens": { "input": 1, "cache_read": 0, "cache_write": 0, "output": 1, "reasoning": 0, "total": 2 },
              "cost_usd_micros": 197100000, "partial": true, "unpriced_tokens": 40, "cost_per_mtok_usd_micros": null,
              "share_permille": 1000 }
        }"#;
        let report = parse_spend_report(json).unwrap();
        assert_eq!(report.until, Date::new(2026, 9, 23).unwrap());
        assert_eq!(report.rows[0].tokens.total, 192_300_000);
        assert_eq!(report.rows[0].cost_per_mtok_usd_micros, Some(786_271));
        assert_eq!(report.total.key, None);
        assert!(report.total.partial);
        assert!(parse_spend_report("{}").is_err());
    }
}
