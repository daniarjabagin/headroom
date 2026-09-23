use base64::Engine;
use base64::alphabet::URL_SAFE;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use jiff::Timestamp;
use serde_json::Value;

use super::timestamp::from_epoch_seconds;

const AUTH_CLAIMS: &str = "https://api.openai.com/auth";
const BASE64URL: GeneralPurpose = GeneralPurpose::new(
    &URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct IdClaims {
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub user_id: Option<String>,
    pub plan: Option<String>,
}

pub(super) fn id_claims(token: &str) -> IdClaims {
    let Some(payload) = decode_payload(token) else {
        return IdClaims::default();
    };
    let auth = payload.get(AUTH_CLAIMS);
    let auth_text = |key: &str| auth.and_then(|claims| text(claims, key));
    IdClaims {
        email: text(&payload, "email"),
        account_id: auth_text("chatgpt_account_id"),
        user_id: auth_text("chatgpt_user_id")
            .or_else(|| auth_text("user_id"))
            .or_else(|| text(&payload, "sub")),
        plan: auth_text("chatgpt_plan_type"),
    }
}

pub(super) fn expires_at(token: &str) -> Option<Timestamp> {
    decode_payload(token)?
        .get("exp")?
        .as_f64()
        .and_then(from_epoch_seconds)
}

fn decode_payload(token: &str) -> Option<Value> {
    let mut parts = token.trim().split('.');
    let (_header, payload, _signature) = (parts.next()?, parts.next()?, parts.next()?);
    let bytes = BASE64URL.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
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
    fn reads_openai_auth_claims() {
        let token = unsigned_token(&json!({
            "email": "someone@example.com",
            "sub": "auth0|fallback",
            AUTH_CLAIMS: {
                "chatgpt_account_id": "acc-1",
                "chatgpt_user_id": "user-1",
                "chatgpt_plan_type": "prolite",
            }
        }));
        assert_eq!(
            id_claims(&token),
            IdClaims {
                email: Some("someone@example.com".into()),
                account_id: Some("acc-1".into()),
                user_id: Some("user-1".into()),
                plan: Some("prolite".into()),
            }
        );
    }

    #[test]
    fn user_id_falls_back_to_user_id_then_sub() {
        let legacy = unsigned_token(&json!({ "sub": "s", AUTH_CLAIMS: { "user_id": "u" } }));
        assert_eq!(id_claims(&legacy).user_id.as_deref(), Some("u"));
        let bare = unsigned_token(&json!({ "sub": "s" }));
        assert_eq!(id_claims(&bare).user_id.as_deref(), Some("s"));
    }

    #[test]
    fn padded_payload_is_accepted() {
        let payload = base64::engine::general_purpose::URL_SAFE.encode(r#"{"exp":1790000000}"#);
        let token = format!("h.{payload}.s");
        assert_eq!(
            expires_at(&token).unwrap().to_string(),
            "2026-09-21T14:13:20Z"
        );
    }

    #[test]
    fn malformed_tokens_yield_nothing() {
        assert_eq!(id_claims("not-a-jwt"), IdClaims::default());
        assert_eq!(id_claims("a.!!!.c"), IdClaims::default());
        assert_eq!(expires_at("a.e30.c"), None);
    }
}
