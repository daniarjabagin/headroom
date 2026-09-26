use std::borrow::Cow;

use headroom_core::usage::UsageSummary;

use super::spend::HomeSummary;
use crate::home::UsageHome;
use crate::model::Model;
use crate::usage::merge::merge_summaries;

pub type ShownUsage<'a> = (&'a UsageHome, Cow<'a, UsageSummary>);

#[must_use]
pub fn shown_usage<'a>(model: &Model, listed: &[HomeSummary<'a>]) -> Vec<ShownUsage<'a>> {
    let absorbed = model.absorbed_homes();
    listed
        .iter()
        .filter(|(home, _)| !absorbed.contains_key(*home))
        .map(|&(home, summary)| {
            let members: Vec<&UsageSummary> = listed
                .iter()
                .filter(|(member, _)| absorbed.get(*member) == Some(home))
                .map(|&(_, member)| member)
                .collect();
            (home, with_members(summary, &members))
        })
        .collect()
}

fn with_members<'a>(summary: &'a UsageSummary, members: &[&UsageSummary]) -> Cow<'a, UsageSummary> {
    if members.is_empty() {
        return Cow::Borrowed(summary);
    }
    let all: Vec<&UsageSummary> = std::iter::once(summary)
        .chain(members.iter().copied())
        .collect();
    Cow::Owned(merge_summaries(&all))
}
