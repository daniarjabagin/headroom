use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::tokens::TokenCounts;

pub(crate) fn event(
    model: &str,
    tier: ServiceTier,
    tokens: &TokenCounts,
    web_search: u32,
) -> UsageEvent {
    UsageEvent {
        key: EventKey("resp_test".into()),
        at: "2026-09-23T10:00:00Z".parse().unwrap(),
        model: model.into(),
        tier,
        tokens: *tokens,
        web_search_requests: web_search,
    }
}
