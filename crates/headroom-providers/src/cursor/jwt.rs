use base64::Engine;
use base64::alphabet::URL_SAFE;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use jiff::Timestamp;
use serde_json::Value;

const BASE64URL: GeneralPurpose = GeneralPurpose::new(
    &URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Claims {
    pub(super) subject: Option<String>,
    pub(super) expires_at: Option<Timestamp>,
}

pub(super) fn claims(token: &str) -> Claims {
    let Some(payload) = decode_payload(token) else {
        return Claims::default();
    };
    Claims {
        subject: payload
            .get("sub")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|sub| !sub.is_empty())
            .map(str::to_owned),
        expires_at: payload
            .get("exp")
            .and_then(Value::as_i64)
            .and_then(|seconds| Timestamp::from_second(seconds).ok()),
    }
}

pub(super) fn user_id(subject: &str) -> &str {
    subject.split_once('|').map_or(subject, |(_, user)| user)
}

fn decode_payload(token: &str) -> Option<Value> {
    let mut parts = token.trim().split('.');
    let (_header, payload, _signature) = (parts.next()?, parts.next()?, parts.next()?);
    let bytes = BASE64URL.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
pub(super) fn unsigned_token(payload: &Value) -> String {
    let encode =
        |value: &Value| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value.to_string());
    format!(
        "{}.{}.signature",
        encode(&serde_json::json!({ "alg": "none" })),
        encode(payload)
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reads_subject_and_expiry() {
        let token = unsigned_token(&json!({ "sub": "auth0|user_01", "exp": 1_790_000_000 }));
        assert_eq!(
            claims(&token),
            Claims {
                subject: Some("auth0|user_01".into()),
                expires_at: Some("2026-09-21T14:13:20Z".parse().unwrap()),
            }
        );
    }

    #[test]
    fn user_id_is_the_part_after_the_connection() {
        assert_eq!(user_id("google-oauth2|user_01"), "user_01");
        assert_eq!(user_id("user_01"), "user_01");
    }

    #[test]
    fn malformed_tokens_have_no_claims() {
        assert_eq!(claims("opaque"), Claims::default());
        assert_eq!(claims("a.!!!.c"), Claims::default());
        let blank = unsigned_token(&json!({ "sub": "  " }));
        assert_eq!(claims(&blank).subject, None);
    }
}
