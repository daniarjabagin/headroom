use jiff::Span;
use jiff::civil::Date;
use serde_json::{Map, Value};

const EXPECTED: &str = "expected today, yesterday, a number of days like 14d, or a date YYYY-MM-DD";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Since {
    Today,
    Yesterday,
    Days(u64),
    Date(Date),
}

pub fn parse_since(value: &str) -> Result<Since, String> {
    let value = value.trim();
    match value {
        "today" => Ok(Since::Today),
        "yesterday" => Ok(Since::Yesterday),
        _ => match value.strip_suffix('d') {
            Some(count) if is_number(count) => parse_days(count),
            _ if looks_like_date(value) => parse_date(value),
            _ => Err(EXPECTED.to_owned()),
        },
    }
}

impl Since {
    pub fn insert_into(self, query: &mut Map<String, Value>, today: Date) {
        match self {
            Since::Today => insert(query, "period", "today"),
            Since::Yesterday => insert(query, "period", "yesterday"),
            Since::Days(7) => insert(query, "period", "7d"),
            Since::Days(30) => insert(query, "period", "30d"),
            Since::Days(days) => insert(query, "since", &first_day(today, days).to_string()),
            Since::Date(date) => insert(query, "since", &date.to_string()),
        }
    }
}

fn insert(query: &mut Map<String, Value>, key: &str, value: &str) {
    query.insert(key.into(), value.into());
}

fn first_day(today: Date, days: u64) -> Date {
    let back = i64::try_from(days.saturating_sub(1)).unwrap_or(i64::MAX);
    Span::new()
        .try_days(back)
        .map_or(Date::MIN, |span| today.saturating_sub(span))
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn parse_days(count: &str) -> Result<Since, String> {
    let days = count.parse::<u64>().unwrap_or(u64::MAX);
    if days == 0 {
        return Err("the number of days must be at least 1, like 1d for today".to_owned());
    }
    Ok(Since::Days(days))
}

fn looks_like_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    matches!(parts.as_slice(), [year, month, day]
        if year.len() == 4 && month.len() == 2 && day.len() == 2
            && parts.iter().all(|part| is_number(part)))
}

fn parse_date(value: &str) -> Result<Since, String> {
    value
        .parse::<Date>()
        .map(Since::Date)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "since_tests.rs"]
mod tests;
