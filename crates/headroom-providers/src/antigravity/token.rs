use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use headroom_core::secret::SecretString;
use jiff::{SignedDuration, Timestamp};
use serde_json::{Map, Value};

const GO_KEYRING_PREFIX: &str = "go-keyring-base64:";
const BOM: char = '\u{feff}';
const ACCESS_KEYS: [&str; 8] = [
    "access_token",
    "accessToken",
    "token",
    "id_token",
    "idToken",
    "bearerToken",
    "auth_token",
    "authToken",
];
const EXPIRY_KEYS: [&str; 3] = ["expiry", "expires_at", "expiresAt"];
const NESTED_KEYS: [&str; 5] = ["tokens", "oauth", "oauth2", "credentials", "auth"];
const EXPIRY_MARGIN: SignedDuration = SignedDuration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AccessToken {
    pub secret: SecretString,
    pub expires_at: Option<Timestamp>,
}

impl AccessToken {
    pub(super) fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at
            .is_some_and(|expiry| expiry.duration_since(now) <= EXPIRY_MARGIN)
    }
}

pub(super) fn parse(raw: &str) -> Option<AccessToken> {
    let text = unwrap_go_keyring(trim(raw))?;
    let text = trim(&text);
    if text.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(object)) => from_object(&object),
        Ok(Value::String(token)) => bare(&token),
        Ok(_) => None,
        Err(_) if text.starts_with('{') || text.starts_with('[') => None,
        Err(_) => bare(text.strip_prefix("Bearer ").unwrap_or(text)),
    }
}

fn trim(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || c == BOM)
}

fn unwrap_go_keyring(text: &str) -> Option<String> {
    match text.strip_prefix(GO_KEYRING_PREFIX) {
        Some(encoded) => String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok(),
        None => Some(text.to_owned()),
    }
}

fn bare(token: &str) -> Option<AccessToken> {
    let token = token.trim();
    (!token.is_empty()).then(|| AccessToken {
        secret: SecretString::new(token.to_owned()),
        expires_at: None,
    })
}

fn from_object(object: &Map<String, Value>) -> Option<AccessToken> {
    let source = object
        .get("token")
        .and_then(Value::as_object)
        .unwrap_or(object);
    match first_text(source, &ACCESS_KEYS) {
        Some(access) => Some(AccessToken {
            secret: SecretString::new(access.to_owned()),
            expires_at: first_text(source, &EXPIRY_KEYS).and_then(|text| text.parse().ok()),
        }),
        None => NESTED_KEYS
            .iter()
            .filter_map(|key| object.get(*key).and_then(Value::as_object))
            .find_map(from_object),
    }
}

fn first_text<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access(raw: &str) -> Option<(String, Option<Timestamp>)> {
        parse(raw).map(|token| (token.secret.expose().to_owned(), token.expires_at))
    }

    #[test]
    fn the_agy_shape_behind_the_go_keyring_prefix_is_read() {
        let json = r#"{"token":{"access_token":"ya29.fake","refresh_token":"1//fake","expiry":"2026-09-23T11:00:00Z"}}"#;
        let raw = format!("\u{feff} {GO_KEYRING_PREFIX}{}\n", STANDARD.encode(json));
        assert_eq!(
            access(&raw),
            Some((
                "ya29.fake".into(),
                Some("2026-09-23T11:00:00Z".parse().unwrap())
            ))
        );
    }

    #[test]
    fn root_and_nested_objects_are_searched() {
        assert_eq!(
            access(r#"{"accessToken":"a","expiresAt":"bad"}"#),
            Some(("a".into(), None))
        );
        assert_eq!(
            access(r#"{"oauth2":{"access_token":"b"}}"#),
            Some(("b".into(), None))
        );
        assert_eq!(access(r#"{"refresh_token":"only"}"#), None);
    }

    #[test]
    fn plain_strings_and_bearer_values_are_tokens() {
        assert_eq!(access("\"c\""), Some(("c".into(), None)));
        assert_eq!(access("Bearer d"), Some(("d".into(), None)));
        assert_eq!(access("e"), Some(("e".into(), None)));
    }

    #[test]
    fn broken_or_empty_material_is_never_a_token() {
        for raw in [
            "",
            "  ",
            "{\"access_token\":",
            "[1,2]",
            "42",
            "go-keyring-base64:!!",
        ] {
            assert_eq!(access(raw), None, "{raw}");
        }
    }

    #[test]
    fn tokens_close_to_expiry_count_as_expired() {
        let now: Timestamp = "2026-09-23T10:00:00Z".parse().unwrap();
        let at = |expiry: &str| AccessToken {
            secret: SecretString::new("t".into()),
            expires_at: Some(expiry.parse().unwrap()),
        };
        assert!(at("2026-09-23T10:01:00Z").is_expired(now));
        assert!(!at("2026-09-23T10:01:01Z").is_expired(now));
        assert!(!bare("t").unwrap().is_expired(now));
    }
}
