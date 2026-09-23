use std::path::Path;

use headroom_core::usage::PriceBook;
use jiff::tz::TimeZone;

use crate::clock::Clock;
use crate::error::DaemonError;
use crate::home::HomeDisplay;
use crate::model::Model;
use crate::state::payload::StatePayload;
use crate::state::{AssembleContext, assemble};
use crate::storage::{Storage, cursors};
use crate::usage::summary::summarize;

pub struct OnceContext<'a> {
    pub price_book: &'a dyn PriceBook,
    pub clock: &'a dyn Clock,
    pub tz: &'a TimeZone,
    pub homes: &'a HomeDisplay,
}

pub fn assemble_state_once(
    db_path: &Path,
    ctx: &OnceContext<'_>,
) -> Result<StatePayload, DaemonError> {
    let storage = Storage::open(db_path)?;
    let now = ctx.clock.now();
    let model = storage.blocking(|conn| {
        let mut model = Model::load(conn)?;
        model.usage_homes = cursors::homes(conn)?
            .into_iter()
            .filter(|home| home.home.is_dir())
            .collect();
        for home in &model.usage_homes {
            let summary = summarize(conn, home, ctx.price_book, ctx.tz, now)?;
            model.usage.insert(home.clone(), summary);
        }
        Ok(model)
    })?;
    let assemble_ctx = AssembleContext {
        now,
        tz: ctx.tz,
        homes: ctx.homes,
    };
    Ok(assemble(&model, &assemble_ctx))
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderKind;
    use headroom_core::cursor::LogCursors;

    use super::*;
    use crate::clock::testing::ManualClock;
    use crate::home::UsageHome;
    use crate::state::payload::{AccountStatus, DataSource};
    use crate::storage::{accounts, events, snapshots};
    use crate::testing::{FlatPrices, account, event, session, snapshot, ts, usage_home_of};

    fn ingest(storage: &Storage, home: &UsageHome, key: &str) {
        let used = event(key, "2026-09-23T09:00:00Z", "gpt-5.5", 100, 20);
        storage
            .blocking(|conn| events::ingest(conn, home, &[used], &LogCursors::default()))
            .unwrap();
    }

    fn once(path: &Path) -> StatePayload {
        let clock = ManualClock::at("2026-09-23T10:00:00Z");
        let homes = HomeDisplay::new(Some("/home/ada".into()));
        let ctx = OnceContext {
            price_book: &FlatPrices,
            clock: &clock,
            tz: &TimeZone::UTC,
            homes: &homes,
        };
        assemble_state_once(path, &ctx).unwrap()
    }

    #[test]
    fn assembles_from_the_database_without_a_daemon() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("headroom.db");
        let mut work = account(ProviderKind::Codex, "work");
        work.home = dir.path().join("codex");
        std::fs::create_dir_all(&work.home).unwrap();
        let limits = snapshot(
            vec![session(40.0, "2026-09-23T12:00:00Z")],
            "2026-09-23T09:55:00Z",
        );
        let storage = Storage::open(&path).unwrap();
        storage
            .blocking(|conn| {
                accounts::sync_provider(
                    conn,
                    ProviderKind::Codex,
                    std::slice::from_ref(&work),
                    ts("2026-09-23T09:00:00Z"),
                )?;
                snapshots::save(conn, &work.id, &limits)
            })
            .unwrap();
        ingest(&storage, &usage_home_of(&work), "r1");
        drop(storage);
        let state = once(&path);
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].status, AccountStatus::Fresh);
        assert_eq!(state.accounts[0].source, Some(DataSource::Cache));
        assert_eq!(state.usage[0].today.tokens.total, 120);
        assert_eq!(state.usage[0].today.cost_usd_micros, 240);
        assert_eq!(state.headline.unwrap().account_id, work.id.0);
    }

    #[test]
    fn cached_usage_covers_existing_homes_with_or_without_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("headroom.db");
        let api_key = UsageHome {
            provider: ProviderKind::Claude,
            home: dir.path().join("claude-api"),
        };
        std::fs::create_dir_all(&api_key.home).unwrap();
        let removed = UsageHome {
            provider: ProviderKind::Codex,
            home: dir.path().join("removed"),
        };
        let storage = Storage::open(&path).unwrap();
        ingest(&storage, &api_key, "a");
        ingest(&storage, &removed, "b");
        drop(storage);
        let state = once(&path);
        assert!(state.accounts.is_empty());
        let listed: Vec<_> = state.usage.iter().map(|u| u.provider).collect();
        assert_eq!(listed, [ProviderKind::Claude]);
        assert_eq!(state.spend.today.total_tokens, 120);
    }
}
