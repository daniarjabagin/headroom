use headroom_core::account::ProviderId;

use crate::model::Model;
use crate::state::AssembleContext;
use crate::state::payload::ProviderStatusView;
use crate::status::{ProviderStatus, policy, providers_in_use};

pub fn provider_status(model: &Model, ctx: &AssembleContext<'_>) -> Vec<ProviderStatusView> {
    if !model.settings.status_pages.enabled {
        return Vec::new();
    }
    let mut in_use: Vec<_> = providers_in_use(model).into_iter().collect();
    in_use.sort_by_key(|provider| ctx.catalog.rank(provider));
    in_use
        .into_iter()
        .filter_map(|provider| {
            let status = model.provider_status.get(&provider)?;
            if !policy::is_fresh(status.fetched_at, ctx.now) {
                return None;
            }
            let page = ctx.catalog.descriptor(&provider)?.links.status?;
            Some(view(provider, status, page))
        })
        .collect()
}

fn view(provider: ProviderId, status: &ProviderStatus, page: &str) -> ProviderStatusView {
    let indicator = status.assessment.indicator;
    let event = status.assessment.event.as_ref();
    ProviderStatusView {
        provider,
        indicator,
        tone: indicator.tone(),
        title: event.map(|event| event.title.clone()),
        stage: event.and_then(|event| event.stage.clone()),
        started_at: event.and_then(|event| event.started_at),
        url: event
            .and_then(|event| event.url.clone())
            .unwrap_or_else(|| page.to_owned()),
    }
}

#[cfg(test)]
#[path = "provider_status_tests.rs"]
mod tests;
