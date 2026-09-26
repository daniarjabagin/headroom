use headroom_core::cursor::LogCursors;
use jiff::tz::TimeZone;

use super::*;
use crate::clock::testing::ManualClock;
use crate::dismissed::DismissedHome;
use crate::home::HomeDisplay;
use crate::once::{OnceContext, assemble_state_once};
use crate::state::payload::StatePayload;
use crate::storage::{Storage, accounts, dismissed, events};
use crate::testing::{FlatPrices, catalog, ts};

async fn ingest_at(harness: &Harness, fake: &FakeProvider, home: &Path, key: &str, input: u64) {
    fake.usage
        .lock()
        .unwrap()
        .push(event(key, "2026-09-23T09:00:00Z", "model", input, 10));
    let home = UsageHome {
        provider: CLAUDE,
        home: home.to_path_buf(),
    };
    ingest::pass(&harness.core, &home, &mut None).await;
}

#[tokio::test]
async fn logs_in_both_homes_merge_into_the_account_entry_and_spend_is_unchanged() {
    let (harness, fake) = signed_in_twice(&CLAUDE, "ada").await;
    let cli_dir = PathBuf::from(cli_home(&CLAUDE));
    let own = owned(&CLAUDE, "ada").home;
    *fake.homes.lock().unwrap() = vec![cli_dir.clone(), own.clone()];
    dismiss_cli(&harness, &CLAUDE, "ada").await;
    ingest_at(&harness, &fake, &cli_dir, "cli", 100).await;
    ingest_at(&harness, &fake, &own, "own", 40).await;
    let state = harness.core.state();
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.accounts[0].usage_home, "~/.claude");
    assert_eq!(state.usage.len(), 1);
    let usage = &state.usage[0];
    assert_eq!(usage.usage_home, "~/.claude");
    assert_eq!(usage.today.tokens.total, 160);
    assert_eq!(usage.today.cost_usd_micros, 320);
    let today = usage.daily.last().unwrap();
    assert_eq!((today.total_tokens, today.cost_usd_micros), (160, 320));
    assert_eq!(state.spend.today.total_tokens, 160);
    assert_eq!(state.spend.today.cost_usd_micros, 320);
}

fn store_dismissed_link(path: &Path, cli_dir: &Path) -> AccountRef {
    let cli_ref = AccountRef {
        home: cli_dir.to_path_buf(),
        ..cli(&CLAUDE, "ada", "/unused")
    };
    let own = owned(&CLAUDE, "ada");
    let cli_usage = UsageHome {
        provider: CLAUDE,
        home: cli_dir.to_path_buf(),
    };
    let stored = own.clone();
    let storage = Storage::open(path).unwrap();
    storage
        .blocking(move |conn| {
            accounts::sync_provider(conn, &CLAUDE, &[stored], ts(NOW))?;
            dismissed::insert(conn, &DismissedHome::of(&cli_ref))?;
            let used = [event("r1", "2026-09-23T09:00:00Z", "model", 100, 20)];
            events::ingest(conn, &cli_usage, &used, &LogCursors::default())
        })
        .unwrap();
    own
}

fn once_state(path: &Path) -> StatePayload {
    let clock = ManualClock::at(NOW);
    let homes = HomeDisplay::new(Some("/home/ada".into()));
    let ctx = OnceContext {
        price_book: &FlatPrices,
        clock: &clock,
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    assemble_state_once(path, &ctx).unwrap()
}

#[test]
fn stored_dismissals_seed_the_links_before_the_first_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    let cli_dir = dir.path().join(".claude");
    std::fs::create_dir_all(&cli_dir).unwrap();
    let own = store_dismissed_link(&path, &cli_dir);
    let storage = Storage::open(&path).unwrap();
    let model = storage.blocking(|conn| Model::load(conn)).unwrap();
    assert_eq!(model.linked_log_homes(&own), [cli_dir]);
}

#[test]
fn the_daemonless_status_attributes_the_dismissed_cli_home() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    let cli_dir = dir.path().join(".claude");
    std::fs::create_dir_all(&cli_dir).unwrap();
    store_dismissed_link(&path, &cli_dir);
    let state = once_state(&path);
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.accounts[0].usage_home, cli_dir.display().to_string());
    assert_eq!(state.usage.len(), 1);
    assert_eq!(state.usage[0].usage_home, state.accounts[0].usage_home);
    assert_eq!(state.usage[0].today.tokens.total, 120);
}
