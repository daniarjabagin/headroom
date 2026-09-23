use serde_json::Value;

const PLAN_WORDS: [&str; 8] = [
    "plan",
    "plans",
    "subscription",
    "subscriptions",
    "subscribe",
    "subscribed",
    "unsubscribed",
    "billing",
];
const MAX_DEPTH: usize = 4;

pub(crate) fn mentions_plan(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body).is_ok_and(|value| value_mentions_plan(&value, 0))
}

fn value_mentions_plan(value: &Value, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    match value {
        Value::String(text) => text_mentions_plan(text),
        Value::Array(items) => items
            .iter()
            .any(|item| value_mentions_plan(item, depth + 1)),
        Value::Object(fields) => fields
            .values()
            .any(|field| value_mentions_plan(field, depth + 1)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn text_mentions_plan(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .any(|word| PLAN_WORDS.contains(&word.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_wording_in_nested_errors_is_found() {
        let bodies = [
            r#"{"type":"error","error":{"type":"permission_error","message":"This account does not have an active subscription."}}"#,
            r#"{"detail":{"code":"no_active_subscription"}}"#,
            r#"{"error":{"code":"plan_required","message":"Upgrade required"}}"#,
            r#"{"detail":"Your plan does not include Codex."}"#,
        ];
        for body in bodies {
            assert!(mentions_plan(body.as_bytes()), "{body}");
        }
    }

    #[test]
    fn unrelated_errors_and_non_json_are_ignored() {
        let bodies = [
            r#"{"type":"error","error":{"type":"authentication_error","message":"OAuth token has expired."}}"#,
            r#"{"detail":"Could not parse your authentication token. Please try signing in again."}"#,
            r#"{"error":{"message":"Explanation: planned maintenance"}}"#,
            "<html>Your plan has lapsed</html>",
            "",
        ];
        for body in bodies {
            assert!(!mentions_plan(body.as_bytes()), "{body}");
        }
    }
}
