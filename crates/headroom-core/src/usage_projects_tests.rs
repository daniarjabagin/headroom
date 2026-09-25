use super::tests::{FlatPrices, event, ts, tz};
use super::*;

const APP: &str = "/home/user/work/app";

fn in_project(key: &str, at: &str, model: &str, output: u64, project: Option<&str>) -> UsageEvent {
    UsageEvent {
        project: project.map(str::to_owned),
        ..event(key, at, model, output)
    }
}

fn summary_at(events: &[UsageEvent], now: &str) -> UsageSummary {
    aggregate(events, &FlatPrices, &TimeZone::UTC, ts(now))
}

#[test]
fn seven_day_window_follows_local_midnights() {
    let rows = [
        (
            "Asia/Almaty",
            "2026-09-23T10:00:00Z",
            "2026-09-16T18:59:59Z",
            "2026-09-16T19:00:00Z",
            "2026-09-23T18:59:59Z",
            "2026-09-23T19:00:00Z",
        ),
        (
            "America/Los_Angeles",
            "2026-09-23T10:00:00Z",
            "2026-09-17T06:59:59Z",
            "2026-09-17T07:00:00Z",
            "2026-09-24T06:59:59Z",
            "2026-09-24T07:00:00Z",
        ),
        (
            "Europe/Berlin",
            "2026-10-27T10:00:00Z",
            "2026-10-20T21:59:59Z",
            "2026-10-20T22:00:00Z",
            "2026-10-27T22:59:59Z",
            "2026-10-27T23:00:00Z",
        ),
    ];
    for (zone, now, before, first, last, after) in rows {
        let events = [
            event("before", before, "m", 0),
            event("first", first, "m", 1),
            event("last", last, "m", 2),
            event("after", after, "m", 4),
        ];
        let summary = aggregate(&events, &FlatPrices, &tz(zone), ts(now));
        let week = &summary.last_7_days.totals;
        assert_eq!(week.tokens.input, Tokens(200), "{zone}");
        assert_eq!(week.tokens.output, Tokens(3), "{zone}");
        let month = &summary.last_30_days.totals;
        assert_eq!(month.tokens.input, Tokens(300), "{zone}");
    }
}

#[test]
fn seven_day_period_has_models_and_projects() {
    let events = [
        in_project("a", "2026-09-23T01:00:00Z", "gpt-5.5", 10, Some(APP)),
        in_project("b", "2026-09-17T01:00:00Z", "unknown", 0, None),
        in_project("c", "2026-09-16T23:59:59Z", "old", 0, Some(APP)),
    ];
    let week = summary_at(&events, "2026-09-23T10:00:00Z").last_7_days;
    assert_eq!(week.totals.tokens.total(), Tokens(210));
    assert!(week.totals.is_partial());
    let models: Vec<&str> = week.models.iter().map(|m| m.model.as_str()).collect();
    assert_eq!(models, ["gpt-5.5", "unknown"]);
    let projects: Vec<(Option<&str>, i64)> = week
        .projects
        .iter()
        .map(|p| (p.project.as_deref(), p.totals.cost.0))
        .collect();
    assert_eq!(projects, [(Some(APP), 110), (None, 0)]);
}

fn project_rows(period: &PeriodUsage) -> Vec<(Option<String>, u64, i64)> {
    period
        .projects
        .iter()
        .map(|p| {
            (
                p.project.clone(),
                p.totals.tokens.total().0,
                p.totals.cost.0,
            )
        })
        .collect()
}

#[test]
fn projects_are_broken_down_per_period() {
    let events = [
        in_project("a", "2026-09-23T01:00:00Z", "m", 10, Some(APP)),
        in_project("b", "2026-09-22T01:00:00Z", "m", 20, Some("/srv/api")),
        in_project("c", "2026-09-22T02:00:00Z", "m", 0, Some(APP)),
    ];
    let summary = summary_at(&events, "2026-09-23T10:00:00Z");
    assert_eq!(project_rows(&summary.today), [(Some(APP.into()), 110, 110)]);
    assert_eq!(
        project_rows(&summary.yesterday),
        [
            (Some("/srv/api".into()), 120, 120),
            (Some(APP.into()), 100, 100)
        ]
    );
    assert_eq!(
        project_rows(&summary.last_30_days),
        [
            (Some(APP.into()), 210, 210),
            (Some("/srv/api".into()), 120, 120)
        ]
    );
}

#[test]
fn projects_sort_by_cost_then_tokens_then_name() {
    let at = "2026-09-23T01:00:00Z";
    let events = [
        in_project("a", at, "unknown", 900, Some("/z-unpriced")),
        in_project("b", at, "m", 0, Some("/b-tie")),
        in_project("c", at, "m", 0, Some("/a-tie")),
        in_project("d", at, "m", 0, None),
        in_project("e", at, "m", 50, Some("/y-top")),
    ];
    let summary = summary_at(&events, "2026-09-23T10:00:00Z");
    let names: Vec<Option<&str>> = summary
        .today
        .projects
        .iter()
        .map(|p| p.project.as_deref())
        .collect();
    assert_eq!(
        names,
        [
            Some("/y-top"),
            None,
            Some("/a-tie"),
            Some("/b-tie"),
            Some("/z-unpriced")
        ]
    );
}

fn eight_projects() -> PeriodUsage {
    let mut events: Vec<UsageEvent> = (0..7u64)
        .map(|index| {
            let key = format!("k{index}");
            let project = format!("/home/user/work/p{index}");
            let output = index * 7 + 3;
            in_project(&key, "2026-09-23T01:00:00Z", "m", output, Some(&project))
        })
        .collect();
    events.push(in_project("u", "2026-09-23T02:00:00Z", "unknown", 11, None));
    events.push(in_project("v", "2026-09-23T03:00:00Z", "m", 5, None));
    summary_at(&events, "2026-09-23T10:00:00Z").today
}

#[test]
fn top_projects_and_other_sum_to_the_period_total() {
    let period = eight_projects();
    let split = top_projects(period.projects.clone(), TOP_PROJECTS);
    assert_eq!(split.top[..], period.projects[..TOP_PROJECTS]);
    let other = split.other.clone().unwrap();
    let mut sum = other.clone();
    split.top.iter().for_each(|p| sum.absorb(&p.totals));
    assert_eq!(sum, period.totals);
    assert_eq!(other.tokens.total(), Tokens(110 + 216 + 103));
    assert_eq!(other.cost, MicroUsd(110 + 105 + 103));
    assert_eq!(other.unpriced_tokens, Tokens(111));
    assert!(other.is_partial());
}

#[test]
fn top_projects_without_a_remainder_has_no_other() {
    let period = eight_projects();
    let all = top_projects(period.projects.clone(), 8);
    assert_eq!(all.top, period.projects);
    assert_eq!(all.other, None);
    assert_eq!(
        top_projects(Vec::new(), TOP_PROJECTS),
        TopProjects::default()
    );
    let none = top_projects(period.projects, 0);
    assert!(none.top.is_empty());
    assert_eq!(none.other, Some(period.totals));
}

fn priced_project(name: &str, cost: i64) -> ProjectUsage {
    ProjectUsage {
        project: Some(name.into()),
        totals: UsageTotals {
            cost: MicroUsd(cost),
            ..UsageTotals::default()
        },
    }
}

#[test]
fn top_projects_sorts_unsorted_input() {
    let cheap = priced_project("/cheap", 1);
    let pricey = priced_project("/pricey", 9);
    let split = top_projects(vec![cheap.clone(), pricey.clone()], 1);
    assert_eq!(split.top, [pricey]);
    assert_eq!(split.other, Some(cheap.totals));
}

fn totals(cost: i64, total: u64, unpriced: u64) -> UsageTotals {
    UsageTotals {
        tokens: TokenCounts {
            input: Tokens(total),
            ..TokenCounts::default()
        },
        cost: MicroUsd(cost),
        unpriced_tokens: Tokens(unpriced),
        unpriced_models: BTreeSet::new(),
    }
}

#[test]
fn cost_per_mtok_uses_integer_math_over_priced_tokens() {
    let rows = [
        (1_500_000, 1_000_000, 0, Some(1_500_000)),
        (1, 3, 0, Some(333_333)),
        (2, 3, 0, Some(666_667)),
        (1, 2_000_000, 0, Some(1)),
        (1, 2_000_001, 0, Some(0)),
        (-1, 2_000_000, 0, Some(-1)),
        (300, 400, 100, Some(1_000_000)),
        (0, 500, 0, Some(0)),
        (0, 500, 500, None),
        (0, 0, 0, None),
        (i64::MAX, 1, 0, Some(i64::MAX)),
        (i64::MIN, 1, 0, Some(i64::MIN)),
    ];
    for (cost, total, unpriced, expected) in rows {
        assert_eq!(
            totals(cost, total, unpriced).cost_per_mtok(),
            expected.map(MicroUsd),
            "{cost} / {total} with {unpriced} unpriced"
        );
    }
}

#[test]
fn absorb_adds_every_field() {
    let mut left = totals(5, 10, 2);
    left.unpriced_models.insert("a".into());
    let mut right = totals(7, 20, 3);
    right.unpriced_models.insert("b".into());
    left.absorb(&right);
    assert_eq!(left.cost, MicroUsd(12));
    assert_eq!(left.tokens.total(), Tokens(30));
    assert_eq!(left.unpriced_tokens, Tokens(5));
    assert_eq!(left.priced_tokens(), Tokens(25));
    assert_eq!(left.unpriced_models.len(), 2);
}

#[test]
fn summaries_without_the_new_fields_still_deserialize() {
    let mut value = serde_json::to_value(UsageSummary::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("last_7_days");
    for period in ["today", "yesterday", "last_30_days"] {
        object[period].as_object_mut().unwrap().remove("projects");
    }
    let summary: UsageSummary = serde_json::from_value(value).unwrap();
    assert_eq!(summary, UsageSummary::default());
}
