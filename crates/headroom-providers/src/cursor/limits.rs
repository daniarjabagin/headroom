use headroom_core::provider::ProviderError;
use jiff::Timestamp;

use super::auth::{AccessToken, Credentials};
use super::client::{CursorClient, Session};
use super::extras::{credits, grok_bot_window, request_window};
use super::mapper::{MappedUsage, PlanUsage, map_period_usage, plan_label};
use super::raw::RawPlanInfoResponse;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Fetched {
    pub(super) plan: Option<String>,
    pub(super) usage: MappedUsage,
}

pub(super) async fn fetch(
    client: &CursorClient,
    credentials: &Credentials,
    token: &AccessToken,
    now: Timestamp,
) -> Result<Fetched, ProviderError> {
    let period = client.period_usage(token, now).await?;
    let session = Session::new(&credentials.subject, token);
    let (plan_info, grants, stripe, grok_bot) = tokio::join!(
        client.plan_info(token, now),
        client.credit_grants(token, now),
        client.stripe(&session, now),
        client.grok_bot_usage(token, now),
    );
    let plan = optional("plan", plan_info)
        .as_ref()
        .and_then(plan_name)
        .or_else(|| credentials.membership.clone());
    let mut usage = match map_period_usage(&period, plan.as_deref())? {
        PlanUsage::Mapped(usage) => usage,
        PlanUsage::RequestBased => request_based(client, &session, now).await?,
    };
    usage.windows.extend(
        optional("Grok Bot usage", grok_bot)
            .as_ref()
            .and_then(grok_bot_window),
    );
    usage.balances.extend(credits(
        optional("credit grants", grants).as_ref(),
        optional("prepaid balance", stripe).as_ref(),
    ));
    Ok(Fetched {
        plan: plan.as_deref().and_then(plan_label),
        usage,
    })
}

async fn request_based(
    client: &CursorClient,
    session: &Session,
    now: Timestamp,
) -> Result<MappedUsage, ProviderError> {
    let raw = client.request_usage(session, now).await?;
    let window = request_window(&raw).ok_or_else(|| {
        ProviderError::InvalidResponse(
            "Cursor reported neither plan usage nor request counts".into(),
        )
    })?;
    Ok(MappedUsage {
        windows: vec![window],
        balances: Vec::new(),
    })
}

fn plan_name(response: &RawPlanInfoResponse) -> Option<String> {
    let name = response.plan_info.as_ref()?.plan_name.as_deref()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

fn optional<T>(what: &'static str, result: Result<T, ProviderError>) -> Option<T> {
    result
        .inspect_err(|error| tracing::warn!(%error, "optional Cursor {what} lookup failed"))
        .ok()
}
