use headroom_core::provider::ProviderError;

use super::auth::Credentials;
use super::mapper::MappedUsage;

pub(super) fn require_subscription(
    credentials: &Credentials,
    fetched: Result<MappedUsage, ProviderError>,
) -> Result<MappedUsage, ProviderError> {
    if credentials.subscribed {
        return fetched;
    }
    let lapsed = || no_subscription(credentials.plan.as_deref());
    match fetched {
        Ok(mapped) if mapped.windows.is_empty() && mapped.balances.is_empty() => Err(lapsed()),
        Err(ProviderError::SignInExpired | ProviderError::NoSubscription { .. }) => Err(lapsed()),
        other => other,
    }
}

pub(super) fn no_subscription(plan: Option<&str>) -> ProviderError {
    let detail = match plan {
        Some(plan) => format!("No active Claude subscription ({plan} plan)."),
        None => "No active Claude subscription.".to_owned(),
    };
    ProviderError::NoSubscription { detail }
}

#[cfg(test)]
#[path = "subscription_tests.rs"]
mod tests;
