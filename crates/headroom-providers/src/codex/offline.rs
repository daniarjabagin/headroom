use std::path::PathBuf;

use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{LimitsSnapshot, Notice};
use jiff::Timestamp;

use super::auth::Credentials;
use super::rate_limits;

pub(super) const SIGN_IN_EXPIRED_NOTICE: &str =
    "Sign-in expired — open Codex to sign in again. Showing limits from local logs.";
pub(super) const OFFLINE_NOTICE: &str = "Offline — showing limits from local logs.";

pub(super) async fn limits_from_logs(
    home: PathBuf,
    credentials: Credentials,
    now: Timestamp,
    error: ProviderError,
) -> Result<LimitsSnapshot, ProviderError> {
    let Some(notice) = fallback_notice(&error) else {
        return Err(error);
    };
    let Credentials {
        identity,
        signed_in_at,
        ..
    } = credentials;
    let lookup = tokio::task::spawn_blocking(move || {
        rate_limits::latest_snapshot(&home, identity, signed_in_at, now)
    })
    .await;
    match lookup {
        Ok(Ok(Some(mut snapshot))) => {
            snapshot.notices.push(notice);
            Ok(snapshot)
        }
        Ok(Ok(None)) => Err(error),
        Ok(Err(local)) => {
            tracing::warn!(error = %local, "codex offline limits unavailable");
            Err(error)
        }
        Err(join) => {
            tracing::warn!(error = %join, "codex offline limits lookup failed");
            Err(error)
        }
    }
}

fn fallback_notice(error: &ProviderError) -> Option<Notice> {
    let text = match error {
        ProviderError::SignInExpired => SIGN_IN_EXPIRED_NOTICE,
        ProviderError::Network(_) => OFFLINE_NOTICE,
        _ => return None,
    };
    Some(Notice {
        tone: Tone::Warning,
        text: text.to_owned(),
    })
}
