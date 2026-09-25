use std::path::PathBuf;

use headroom_core::account::ProviderId;
use headroom_core::event::UsageEvent;
use headroom_core::usage::aggregate;

use super::payload::{OtherProjectsView, PeriodSpendView, ProjectProviderView};
use super::*;
use crate::home::UsageHome;
use crate::testing::{CLAUDE, CODEX, FlatPrices, catalog, event, ts};

const NOW: &str = "2026-09-23T10:00:00Z";
const TODAY: &str = "2026-09-23T08:00:00Z";

fn placed(key: &str, project: Option<&str>, tokens: u64) -> UsageEvent {
    UsageEvent {
        project: project.map(str::to_owned),
        ..event(key, TODAY, "gpt-5.5", tokens, 0)
    }
}

fn model_with(homes: Vec<(ProviderId, &str, Vec<UsageEvent>)>) -> Model {
    let mut model = Model::default();
    for (provider, dir, events) in homes {
        let home = UsageHome {
            provider,
            home: PathBuf::from(dir),
        };
        let summary = aggregate(&events, &FlatPrices, &TimeZone::UTC, ts(NOW));
        model.usage_homes.insert(home.clone());
        model.usage.insert(home, summary);
    }
    model
}

fn today(model: &Model) -> PeriodSpendView {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    assemble(model, &ctx).spend.today
}

fn listed(period: &PeriodSpendView) -> Vec<(Option<&str>, i64, u64, u32)> {
    period
        .projects
        .iter()
        .map(|p| {
            let name = p.project.as_deref();
            (name, p.cost_usd_micros, p.total_tokens, p.share_permille)
        })
        .collect()
}

fn split(provider: ProviderId, name: &str, cost: i64, tokens: u64) -> ProjectProviderView {
    ProjectProviderView {
        provider,
        provider_name: name.into(),
        cost_usd_micros: cost,
        total_tokens: tokens,
    }
}

#[test]
fn projects_are_merged_across_homes_before_the_cut() {
    let codex = vec![
        placed("c1", Some("/home/ada/work/app"), 300),
        placed("c2", Some("/home/ada/work/api"), 60),
        placed("c3", Some("/srv/tools"), 40),
    ];
    let claude = vec![
        placed("a1", Some("/home/ada/work/api"), 200),
        placed("a2", None, 150),
        placed("a3", Some("/home/ada/work/app"), 100),
        placed("a4", Some("/home/ada/scratch"), 45),
        placed("a5", Some("/home/ada/notes"), 5),
    ];
    let period = today(&model_with(vec![
        (CODEX, "/home/ada/.codex", codex),
        (CLAUDE, "/home/ada/.claude", claude),
    ]));
    assert_eq!(period.cost_usd_micros, 1_800);
    assert_eq!(
        listed(&period),
        [
            (Some("~/work/app"), 800, 400, 444),
            (Some("~/work/api"), 520, 260, 288),
            (None, 300, 150, 166),
            (Some("~/scratch"), 90, 45, 50),
        ]
    );
    assert_eq!(
        period.projects_other,
        Some(OtherProjectsView {
            count: 2,
            cost_usd_micros: 90,
            total_tokens: 45,
            partial: false,
            share_permille: 50,
        })
    );
    assert_eq!(
        period.projects[0].by_provider,
        [
            split(CODEX, "Codex", 600, 300),
            split(CLAUDE, "Claude", 200, 100)
        ]
    );
    let parts = period
        .projects
        .iter()
        .map(|p| p.cost_usd_micros)
        .sum::<i64>();
    assert_eq!(parts + 90, period.cost_usd_micros);
}

#[test]
fn at_most_five_projects_are_listed() {
    let events: Vec<UsageEvent> = (0..7)
        .map(|n| {
            let dir = format!("/home/ada/p{n}");
            placed(&format!("e{n}"), Some(&dir), 100 - n)
        })
        .collect();
    let period = today(&model_with(vec![(CODEX, "/home/ada/.codex", events)]));
    let names: Vec<_> = listed(&period).into_iter().map(|row| row.0).collect();
    assert_eq!(
        names,
        [
            Some("~/p0"),
            Some("~/p1"),
            Some("~/p2"),
            Some("~/p3"),
            Some("~/p4")
        ]
    );
    let other = period.projects_other.unwrap();
    assert_eq!((other.count, other.total_tokens), (2, 95 + 94));
}

#[test]
fn unpriced_projects_are_partial_and_empty_periods_have_no_projects() {
    let events = vec![
        placed("a", Some("/home/ada/app"), 100),
        UsageEvent {
            model: "unknown".into(),
            ..placed("b", Some("/home/ada/app"), 10)
        },
    ];
    let model = model_with(vec![(CODEX, "/home/ada/.codex", events)]);
    let period = today(&model);
    assert!(period.projects[0].partial);
    assert_eq!(period.projects[0].share_permille, 1_000);
    assert_eq!(period.projects_other, None);
    let homes = HomeDisplay::default();
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    let spend = assemble(&model, &ctx).spend;
    assert!(spend.yesterday.projects.is_empty());
    assert_eq!(spend.yesterday.projects_other, None);
    assert_eq!(
        spend.last_7_days.projects[0].project.as_deref(),
        Some("/home/ada/app")
    );
}
