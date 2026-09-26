use std::collections::BTreeMap;

use headroom_core::usage::{
    ModelUsage, PeriodUsage, ProjectUsage, UsageSummary, UsageTotals, most_expensive_first,
    top_projects,
};
use jiff::civil::Date;

#[must_use]
pub fn merge_summaries(summaries: &[&UsageSummary]) -> UsageSummary {
    UsageSummary {
        today: merge_periods(summaries, |s| &s.today),
        yesterday: merge_periods(summaries, |s| &s.yesterday),
        last_7_days: merge_periods(summaries, |s| &s.last_7_days),
        last_30_days: merge_periods(summaries, |s| &s.last_30_days),
        daily: merge_daily(summaries),
    }
}

fn merge_periods(
    summaries: &[&UsageSummary],
    pick: fn(&UsageSummary) -> &PeriodUsage,
) -> PeriodUsage {
    let mut totals = UsageTotals::default();
    let mut models: BTreeMap<String, UsageTotals> = BTreeMap::new();
    let mut projects: BTreeMap<Option<String>, UsageTotals> = BTreeMap::new();
    for period in summaries.iter().map(|summary| pick(summary)) {
        totals.absorb(&period.totals);
        for usage in &period.models {
            models
                .entry(usage.model.clone())
                .or_default()
                .absorb(&usage.totals);
        }
        for usage in &period.projects {
            projects
                .entry(usage.project.clone())
                .or_default()
                .absorb(&usage.totals);
        }
    }
    PeriodUsage {
        totals,
        models: ranked_models(models),
        projects: ranked_projects(projects),
    }
}

fn ranked_models(models: BTreeMap<String, UsageTotals>) -> Vec<ModelUsage> {
    let mut ranked: Vec<ModelUsage> = models
        .into_iter()
        .map(|(model, totals)| ModelUsage { model, totals })
        .collect();
    ranked.sort_by(|a, b| {
        most_expensive_first(&a.totals, &b.totals).then_with(|| a.model.cmp(&b.model))
    });
    ranked
}

fn ranked_projects(projects: BTreeMap<Option<String>, UsageTotals>) -> Vec<ProjectUsage> {
    let all: Vec<ProjectUsage> = projects
        .into_iter()
        .map(|(project, totals)| ProjectUsage { project, totals })
        .collect();
    let count = all.len();
    top_projects(all, count).top
}

fn merge_daily(summaries: &[&UsageSummary]) -> Vec<(Date, UsageTotals)> {
    let mut daily: BTreeMap<Date, UsageTotals> = BTreeMap::new();
    for (date, totals) in summaries.iter().flat_map(|summary| &summary.daily) {
        daily.entry(*date).or_default().absorb(totals);
    }
    daily.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use headroom_core::usage::aggregate;
    use jiff::tz::TimeZone;

    use super::*;
    use crate::testing::{FlatPrices, event, ts};

    #[test]
    fn merging_equals_aggregating_all_events_at_once() {
        let now = ts("2026-09-23T10:00:00Z");
        let mut first = vec![
            event("a", "2026-09-23T09:00:00Z", "opus", 100, 10),
            event("b", "2026-09-22T09:00:00Z", "sonnet", 40, 4),
        ];
        first[0].project = Some("/p/one".into());
        let mut second = vec![
            event("c", "2026-09-23T08:00:00Z", "opus", 7, 3),
            event("d", "2026-09-10T08:00:00Z", "unknown", 5, 5),
        ];
        second[0].project = Some("/p/two".into());
        let tz = TimeZone::UTC;
        let one = aggregate(&first, &FlatPrices, &tz, now);
        let two = aggregate(&second, &FlatPrices, &tz, now);
        let all: Vec<_> = first.into_iter().chain(second).collect();
        let whole = aggregate(&all, &FlatPrices, &tz, now);
        assert_eq!(merge_summaries(&[&one, &two]), whole);
    }

    #[test]
    fn a_single_summary_merges_to_itself() {
        let events = [event("a", "2026-09-23T09:00:00Z", "opus", 100, 10)];
        let one = aggregate(
            &events,
            &FlatPrices,
            &TimeZone::UTC,
            ts("2026-09-23T10:00:00Z"),
        );
        assert_eq!(merge_summaries(&[&one]), one);
    }
}
