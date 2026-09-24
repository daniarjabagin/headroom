use std::path::Path;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use serde_json::{Map, Value};

use super::auth::{Credentials, Secret, bearer_value, read_file};
use super::client::ClineClient;
use super::raw::RawTokens;
use crate::fsio::write_private;

pub(super) struct Stored<'a> {
    pub(super) path: &'a Path,
    pub(super) text: &'a str,
}

pub(super) async fn refresh_owned(
    client: &ClineClient,
    stored: &Stored<'_>,
    credentials: &Credentials,
) -> Result<Secret, ProviderError> {
    let refresh_token = credentials
        .refresh_token
        .as_ref()
        .map(|token| Secret::new(token.expose().to_owned()))
        .ok_or(ProviderError::SignInExpired)?;
    let client = client.clone();
    let path = stored.path.to_path_buf();
    let text = stored.text.to_owned();
    let detached = tokio::spawn(async move {
        let stored = Stored {
            path: &path,
            text: &text,
        };
        refresh_and_save(&client, &stored, &refresh_token).await
    });
    detached.await.map_err(|_| {
        ProviderError::LocalData("the Cline sign-in refresh was interrupted".to_owned())
    })?
}

async fn refresh_and_save(
    client: &ClineClient,
    stored: &Stored<'_>,
    refresh_token: &Secret,
) -> Result<Secret, ProviderError> {
    let tokens = client.refresh(refresh_token).await?;
    let patched = patch_credentials(stored.text, &tokens)?;
    write_back(stored, &patched)?;
    Ok(Secret::new(bearer_value(&tokens.access_token)))
}

pub(super) fn patch_credentials(text: &str, tokens: &RawTokens) -> Result<String, ProviderError> {
    let expires_at: Timestamp = tokens.expires_at.parse().map_err(|_| {
        ProviderError::InvalidResponse("Cline token refresh returned an invalid expiry".to_owned())
    })?;
    let mut file: Value = serde_json::from_str(text)
        .map_err(|_| ProviderError::LocalData("cannot parse Cline providers.json".to_owned()))?;
    let auth = auth_object(&mut file).ok_or(ProviderError::NotSignedIn)?;
    auth.insert(
        "accessToken".to_owned(),
        Value::String(bearer_value(&tokens.access_token)),
    );
    if let Some(refresh) = tokens
        .refresh_token
        .as_deref()
        .filter(|t| !t.trim().is_empty())
    {
        auth.insert("refreshToken".to_owned(), Value::String(refresh.to_owned()));
    }
    auth.insert(
        "expiresAt".to_owned(),
        Value::from(expires_at.as_millisecond()),
    );
    serde_json::to_string_pretty(&file)
        .map(|json| json + "\n")
        .map_err(|_| ProviderError::LocalData("cannot write Cline providers.json".to_owned()))
}

fn auth_object(file: &mut Value) -> Option<&mut Map<String, Value>> {
    file.get_mut("providers")?
        .get_mut("cline")?
        .get_mut("settings")?
        .get_mut("auth")?
        .as_object_mut()
}

fn write_back(stored: &Stored<'_>, patched: &str) -> Result<(), ProviderError> {
    let current = read_file(stored.path)?;
    if current.as_deref() != Some(stored.text) {
        tracing::warn!(
            path = %stored.path.display(),
            "Cline credentials changed during a token refresh; keeping the newer file"
        );
        return Ok(());
    }
    write_private(stored.path, patched.as_bytes()).map_err(|error| {
        ProviderError::LocalData(format!(
            "cannot save refreshed Cline sign-in to {}: {}",
            stored.path.display(),
            error.kind()
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::auth::parse_credentials;
    use super::super::client::parse_envelope;
    use super::*;

    const SIGNED_IN: &str = include_str!("fixtures/providers.json");
    const REFRESH: &str = include_str!("fixtures/refresh.json");

    fn tokens() -> RawTokens {
        parse_envelope(REFRESH.as_bytes()).unwrap()
    }

    #[test]
    fn refreshed_tokens_replace_only_the_auth_fields() {
        let patched = patch_credentials(SIGNED_IN, &tokens()).unwrap();
        let credentials = parse_credentials(&patched).unwrap();
        assert_eq!(credentials.access_token.expose(), "workos:fresh.jwt.token");
        assert_eq!(
            credentials.refresh_token.unwrap().expose(),
            "rotated-refresh"
        );
        assert_eq!(
            credentials.expires_at,
            Some("2026-09-23T11:00:00Z".parse().unwrap())
        );
        let before: Value = serde_json::from_str(SIGNED_IN).unwrap();
        let after: Value = serde_json::from_str(&patched).unwrap();
        assert_eq!(
            before["providers"]["cline"]["settings"]["model"],
            after["providers"]["cline"]["settings"]["model"]
        );
        assert_eq!(
            before["providers"]["cline"]["settings"]["auth"]["metadata"],
            after["providers"]["cline"]["settings"]["auth"]["metadata"]
        );
        assert_eq!(before["lastUsedProvider"], after["lastUsedProvider"]);
    }

    #[test]
    fn a_missing_rotation_keeps_the_old_refresh_token() {
        let mut kept = tokens();
        kept.refresh_token = None;
        let patched = patch_credentials(SIGNED_IN, &kept).unwrap();
        let credentials = parse_credentials(&patched).unwrap();
        assert_eq!(credentials.refresh_token.unwrap().expose(), "fake-refresh");
        let mut broken = tokens();
        broken.expires_at = "soon".to_owned();
        assert!(matches!(
            patch_credentials(SIGNED_IN, &broken),
            Err(ProviderError::InvalidResponse(_))
        ));
    }

    #[test]
    fn write_back_only_replaces_an_unchanged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("providers.json");
        fs::write(&path, SIGNED_IN).unwrap();
        let stored = Stored {
            path: &path,
            text: SIGNED_IN,
        };
        write_back(&stored, "{\"patched\":true}\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"patched\":true}\n");
        write_back(&stored, "{\"again\":true}\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"patched\":true}\n");
    }
}
