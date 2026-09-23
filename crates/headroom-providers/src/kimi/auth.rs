use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use jiff::{SignedDuration, Timestamp};

use super::client::KimiClient;
use super::credentials;

const REFRESH_MARGIN: SignedDuration = SignedDuration::from_mins(5);

pub(super) async fn oauth_token(
    client: &KimiClient,
    account: &AccountRef,
    now: Timestamp,
) -> Result<SecretString, ProviderError> {
    let tokens = credentials::load(&account.home)?;
    if tokens.valid_for(now, REFRESH_MARGIN) {
        return Ok(tokens.access);
    }
    if account.owner == CredentialOwner::Cli {
        return if tokens.valid_for(now, SignedDuration::ZERO) {
            Ok(tokens.access)
        } else {
            Err(ProviderError::SignInExpired)
        };
    }
    let refreshed = client.refresh(tokens.refresh.expose(), now).await?;
    credentials::save_refreshed(&account.home, &tokens, &refreshed, now)?;
    Ok(refreshed.access())
}
