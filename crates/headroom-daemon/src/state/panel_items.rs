use std::cmp::Ordering;

use headroom_core::pace::Tone;

use super::headline::{Candidate, candidates, select, share};
use super::payload::{AccountView, CombinedView, PanelItem};
use crate::settings::{PanelLimit, PanelMode, Settings, ValueMode};

const AUTO_ITEMS: usize = 2;
const MAX_ITEMS: usize = 3;

#[must_use]
pub fn panel_items(
    accounts: &[AccountView],
    combined: &[CombinedView],
    settings: &Settings,
) -> Vec<PanelItem> {
    let candidates = candidates(accounts, combined);
    let display = &settings.display;
    let chosen = match display.panel_mode {
        PanelMode::Headline => select(&candidates, &settings.headline)
            .into_iter()
            .collect(),
        PanelMode::Several if display.panel_limits.is_empty() => most_critical(&candidates),
        PanelMode::Several => pinned(&candidates, &display.panel_limits),
        PanelMode::Icon => Vec::new(),
    };
    chosen
        .into_iter()
        .map(|candidate| item(candidate, display.value_mode))
        .collect()
}

#[must_use]
pub fn panel_tone(accounts: &[AccountView], combined: &[CombinedView]) -> Option<Tone> {
    candidates(accounts, combined)
        .into_iter()
        .map(Candidate::tone)
        .max()
}

fn most_critical<'a>(candidates: &[Candidate<'a>]) -> Vec<Candidate<'a>> {
    let mut ranked = candidates.to_vec();
    ranked.sort_by(|a, b| by_criticality(*a, *b));
    ranked.truncate(AUTO_ITEMS);
    ranked
}

fn by_criticality(a: Candidate<'_>, b: Candidate<'_>) -> Ordering {
    b.tone()
        .cmp(&a.tone())
        .then_with(|| a.remaining_percent().total_cmp(&b.remaining_percent()))
}

fn pinned<'a>(candidates: &[Candidate<'a>], limits: &[PanelLimit]) -> Vec<Candidate<'a>> {
    let mut chosen: Vec<Candidate<'a>> = Vec::new();
    for limit in limits {
        let found = candidates
            .iter()
            .copied()
            .find(|c| c.is(&limit.account_id, &limit.window));
        if let Some(candidate) = found.filter(|c| !chosen.iter().any(|seen| same(*seen, *c))) {
            chosen.push(candidate);
        }
    }
    chosen.truncate(MAX_ITEMS);
    chosen
}

fn same(a: Candidate<'_>, b: Candidate<'_>) -> bool {
    match (a, b) {
        (Candidate::Single(_, w), Candidate::Single(_, v)) => std::ptr::eq(w, v),
        (Candidate::Combined(_, w), Candidate::Combined(_, v)) => std::ptr::eq(w, v),
        _ => false,
    }
}

fn item(candidate: Candidate<'_>, value_mode: ValueMode) -> PanelItem {
    let headline = candidate.headline();
    let value_percent = match value_mode {
        ValueMode::Left => headline.remaining_percent,
        ValueMode::Used => headline.used_percent,
    };
    PanelItem {
        logo: headline.provider.to_string(),
        value_percent,
        even_pace_percent: even_pace_percent(candidate),
        headline,
    }
}

fn even_pace_percent(candidate: Candidate<'_>) -> Option<f64> {
    match candidate {
        Candidate::Single(_, window) => window.pace.even_pace_percent,
        Candidate::Combined(_, window) => window
            .pace
            .even_pace_percent
            .map(|value| share(window, value)),
    }
}

#[cfg(test)]
#[path = "panel_items_tests.rs"]
mod tests;
