use super::*;

fn provider(id: &str, cost: i64, tokens: u64, per_mtok: Option<i64>) -> ProviderSpend {
    ProviderSpend {
        provider: id.into(),
        provider_name: id.into(),
        cost_usd_micros: cost,
        total_tokens: tokens,
        partial: false,
        models: Vec::new(),
        models_other: None,
        cost_per_mtok_usd_micros: per_mtok,
    }
}

fn period(cost: i64, tokens: u64) -> PeriodSpend {
    PeriodSpend {
        cost_usd_micros: cost,
        total_tokens: tokens,
        partial: false,
        by_provider: vec![
            provider("claude", cost - 1_000_000, tokens - 100, Some(330_000)),
            provider("codex", 1_000_000, 100, None),
        ],
        cost_per_mtok_usd_micros: Some(330_000),
        projects: None,
        projects_other: None,
        models: None,
        models_other: None,
    }
}

fn spend(with_week: bool, with_projects: bool) -> Spend {
    let mut today = period(3_000_000, 1000);
    if with_projects {
        today.projects = Some(Vec::new());
    }
    Spend {
        today,
        yesterday: period(2_000_000, 500),
        last_7_days: with_week.then(|| period(5_000_000, 5000)),
        last_30_days: period(9_000_000, 9000),
    }
}

#[test]
fn choices_follow_the_display_then_fall_back() {
    let display = Display {
        spend_period: SpendPeriod::Last7Days,
        spend_unit: SpendUnit::CostPerMtok,
        spend_breakdown: SpendBreakdown::Projects,
        ..Display::default()
    };
    let recent = SpendChoice::resolve(&display, SpendOverride::default(), &spend(true, true), true);
    assert_eq!(recent.period, SpendPeriod::Last7Days);
    assert_eq!(recent.unit, SpendUnit::CostPerMtok);
    assert_eq!(recent.breakdown, SpendBreakdown::Projects);
    let old = SpendChoice::resolve(
        &display,
        SpendOverride::default(),
        &spend(false, false),
        false,
    );
    assert_eq!(old.period, SpendPeriod::Last30Days);
    assert_eq!(old.unit, SpendUnit::Cost);
    assert_eq!(old.breakdown, SpendBreakdown::Models);
    let chosen = SpendOverride {
        period: Some(SpendPeriod::Today),
        unit: Some(SpendUnit::Tokens),
        breakdown: None,
    };
    let picked = SpendChoice::resolve(&display, chosen, &spend(false, false), false);
    assert_eq!(
        (picked.period, picked.unit),
        (SpendPeriod::Today, SpendUnit::Tokens)
    );
    assert_eq!(picked.basis(), Basis::Tokens);
}

#[test]
fn periods_and_units_depend_on_the_daemon() {
    assert_eq!(periods(&spend(false, false)).len(), 3);
    assert_eq!(periods(&spend(true, false))[2], SpendPeriod::Last7Days);
    assert_eq!(units(false), [SpendUnit::Cost, SpendUnit::Tokens]);
    assert_eq!(units(true).len(), 3);
}

#[test]
fn the_breakdown_follows_the_setting_and_the_payload() {
    let shown = Display::default();
    assert!(shows_breakdown(&shown, &spend(false, true)));
    assert!(!shows_breakdown(&shown, &spend(false, false)));
    let hidden = Display {
        show_breakdown: false,
        ..Display::default()
    };
    assert!(!shows_breakdown(&hidden, &spend(false, true)));
}

#[test]
fn ring_centers_per_unit() {
    let period = period(127_650_000, 182_900_000);
    let cost = ring_center(Lang::En, SpendUnit::Cost, &period);
    assert_eq!((cost.amount.as_str(), cost.unit_line), ("$128", None));
    let tokens = ring_center(Lang::En, SpendUnit::Tokens, &period);
    assert_eq!(tokens.amount, "183M");
    assert_eq!(tokens.unit_line, Some("tokens"));
    let rate = ring_center(Lang::En, SpendUnit::CostPerMtok, &period);
    assert_eq!(rate.amount, "$0.33");
    assert_eq!(rate.unit_line, Some("blended"));
    assert_eq!(per_mtok_text(None), "—");
}

#[test]
fn slices_follow_the_basis() {
    let period = period(3_000_000, 1000);
    let cost = SpendChoice {
        period: SpendPeriod::Today,
        unit: SpendUnit::Cost,
        breakdown: SpendBreakdown::Models,
    };
    assert_eq!(slice_values(cost, &period), [2_000_000.0, 1_000_000.0]);
    let tokens = SpendChoice {
        unit: SpendUnit::Tokens,
        ..cost
    };
    assert_eq!(slice_values(tokens, &period), [900.0, 100.0]);
    let mut free = period.clone();
    free.cost_usd_micros = 0;
    assert_eq!(basis_of(Basis::Cost, &free), Basis::Tokens);
}

#[test]
fn shares_round_down_to_tenths() {
    assert_eq!(permille(378, 1000), 378);
    assert_eq!(permille(2, 3), 666);
    assert_eq!(permille(5, 0), 0);
    assert_eq!(share_text(Lang::En, 378), "37.8%");
    assert_eq!(share_text(Lang::Ru, 43), "4,3%");
    assert_eq!(whole_share_text(629), "62%");
    let figures = Figures {
        cost_micros: 19_800_000,
        tokens: 52_100_000,
        per_mtok: None,
    };
    assert_eq!(unit_value(Lang::En, SpendUnit::Cost, figures), "$19.80");
    assert_eq!(unit_value(Lang::En, SpendUnit::Tokens, figures), "52.1M");
    assert_eq!(unit_value(Lang::En, SpendUnit::CostPerMtok, figures), "—");
}

#[test]
fn titles_translate() {
    assert_eq!(
        unit_title(Lang::En, SpendUnit::CostPerMtok),
        "Cost per MTok"
    );
    assert_eq!(period_title(Lang::Ru, SpendPeriod::Last7Days), "7 дней");
    assert_eq!(
        period_heading(Lang::En, SpendPeriod::Last30Days),
        "Last 30 days"
    );
    assert!(!unit_subtitle(Lang::Ru, SpendUnit::Tokens).is_empty());
}
