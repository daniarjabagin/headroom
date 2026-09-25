use headroom_daemon::state::payload::TokensView;

use super::*;
use crate::providers;

fn tokens(input: u64, output: u64, cache_read: u64, cache_write: u64) -> TokensView {
    TokensView {
        input,
        cache_read,
        cache_write,
        output,
        reasoning: 0,
        total: input + output + cache_read + cache_write,
    }
}

fn row(key: Option<&str>, provider: Option<&str>, tokens: TokensView, cost: i64) -> ReportRow {
    ReportRow {
        key: key.map(str::to_owned),
        provider: provider.map(|id| ProviderId::parse(id).unwrap()),
        tokens,
        cost_usd_micros: cost,
        partial: false,
        unpriced_tokens: 0,
        cost_per_mtok_usd_micros: None,
        share_permille: 0,
    }
}

fn shared(mut row: ReportRow, share: u32) -> ReportRow {
    row.share_permille = share;
    row
}

fn report(by: GroupBy, rows: Vec<ReportRow>, total: ReportRow) -> SpendReport {
    SpendReport {
        since: "2026-09-19".parse().unwrap(),
        until: "2026-09-25".parse().unwrap(),
        by,
        rows,
        total,
    }
}

fn models() -> SpendReport {
    let mut mystery = row(
        Some("mystery-1"),
        Some("codex"),
        tokens(4_000, 1_000, 0, 0),
        0,
    );
    mystery.partial = true;
    mystery.unpriced_tokens = 5_000;
    let mut total = row(
        None,
        None,
        tokens(5_104_000, 1_462_000, 119_500_000, 3_100_000),
        134_550_000,
    );
    total.partial = true;
    total.unpriced_tokens = 5_000;
    report(
        GroupBy::Model,
        vec![
            row(
                Some("claude-opus-5-5"),
                Some("claude"),
                tokens(1_900_000, 612_000, 48_200_000, 3_100_000),
                96_400_000,
            ),
            row(
                Some("gpt-5.5-codex"),
                Some("codex"),
                tokens(3_200_000, 849_000, 71_300_000, 0),
                38_150_000,
            ),
            mystery,
        ],
        total,
    )
}

fn render(report: &SpendReport, palette: Palette) -> String {
    render_breakdown(report, &providers::catalog(), palette)
}

const MODELS_PLAIN: &str = "\
Spend by model  Sep 19 – Sep 25 · estimated from local logs

Model            Provider  Input  Output  Cache read  Cache write     Cost
claude-opus-5-5  Claude     1.9M    612K       48.2M         3.1M   $96.40
gpt-5.5-codex    Codex      3.2M    849K       71.3M            0   $38.15
mystery-1        Codex        4K      1K           0            0        —  partial
───────────────────────────────────────────────────────────────────────────────────
Total                       5.1M    1.5M        120M         3.1M  $134.55  partial
partial: some usage has no known price and is left out of the cost
";

#[test]
fn models_show_token_kinds_cost_and_partial_rows() {
    assert_eq!(render(&models(), Palette::plain()), MODELS_PLAIN);
}

fn projects() -> SpendReport {
    let project = |key: Option<&str>, total: u64, cost: i64, share: u32| {
        shared(row(key, None, tokens(total, 0, 0, 0), cost), share)
    };
    report(
        GroupBy::Project,
        vec![
            project(Some("~/code/app"), 118_000_000, 88_340_000, 448),
            project(Some("~/code/headroom"), 82_100_000, 61_020_000, 310),
            project(None, 12_800_000, 13_380_000, 68),
        ],
        project(None, 212_900_000, 162_740_000, 1_000),
    )
}

const PROJECTS_PLAIN: &str = "\
Spend by project  Sep 19 – Sep 25 · project = working directory of the session

Project          Tokens     Cost   Share
~/code/app         118M   $88.34   44.8%  ████████████
~/code/headroom   82.1M   $61.02   31.0%  ████████
(no project)      12.8M   $13.38    6.8%  ██
──────────────────────────────────────────────────────
Total              213M  $162.74  100.0%
";

#[test]
fn projects_show_tokens_cost_and_a_share_bar() {
    assert_eq!(render(&projects(), Palette::plain()), PROJECTS_PLAIN);
}

#[test]
fn colors_mark_cost_bars_and_the_total() {
    let out = render(&projects(), Palette::colored());
    assert!(out.contains("\x1b[1m $88.34\x1b[0m"));
    assert!(out.contains("\x1b[38;2;0;145;255m████████████\x1b[0m"));
    assert!(out.contains("\x1b[1mTotal          \x1b[0m"));
    assert!(!out.contains(" \x1b[0m\n"));
}

#[test]
fn days_are_named_and_one_day_ranges_have_one_date() {
    let mut day = report(
        GroupBy::Day,
        vec![row(Some("2026-09-21"), None, tokens(10, 5, 0, 0), 40)],
        row(None, None, tokens(10, 5, 0, 0), 40),
    );
    day.since = "2026-09-21".parse().unwrap();
    day.until = day.since;
    let out = render(&day, Palette::plain());
    assert!(out.starts_with("Spend by day  Sep 21 · estimated from local logs\n"));
    assert!(out.contains("Mon Sep 21     10       5           0            0  $0.00\n"));
}

#[test]
fn an_empty_breakdown_says_so() {
    let empty = report(
        GroupBy::Provider,
        Vec::new(),
        row(None, None, tokens(0, 0, 0, 0), 0),
    );
    assert_eq!(
        render(&empty, Palette::plain()),
        "Spend by provider  Sep 19 – Sep 25 · estimated from local logs\n\nNo local usage in Sep 19 – Sep 25.\n"
    );
}
