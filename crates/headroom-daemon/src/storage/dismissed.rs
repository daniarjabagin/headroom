use headroom_core::account::{AccountId, ProviderId};
use rusqlite::{Connection, params};

use super::codec::{enum_from_sql, enum_to_sql, path_from_sql, path_to_sql};
use crate::dismissed::{DismissedHome, DismissedHomes};
use crate::error::StorageError;

pub fn load_all(conn: &Connection) -> Result<DismissedHomes, StorageError> {
    let mut statement =
        conn.prepare("SELECT provider, account_id, home FROM dismissed_homes ORDER BY account_id")?;
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<(String, String, String)>, _>>()?;
    rows.into_iter()
        .map(|(provider, account, home)| {
            Ok(DismissedHome {
                provider: enum_from_sql("provider", provider)?,
                account: AccountId(account),
                home: path_from_sql(home),
            })
        })
        .collect()
}

pub fn insert(conn: &Connection, home: &DismissedHome) -> Result<(), StorageError> {
    conn.execute(
        "INSERT OR IGNORE INTO dismissed_homes (provider, account_id, home) VALUES (?1, ?2, ?3)",
        params![
            enum_to_sql(&home.provider)?,
            home.account.0,
            path_to_sql(&home.home)?
        ],
    )?;
    Ok(())
}

pub fn restore(conn: &Connection, provider: Option<&ProviderId>) -> Result<(), StorageError> {
    match provider {
        None => conn.execute("DELETE FROM dismissed_homes", [])?,
        Some(provider) => conn.execute(
            "DELETE FROM dismissed_homes WHERE provider = ?1",
            [enum_to_sql(provider)?],
        )?,
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::storage::{Storage, migrations, settings};
    use crate::testing::{CLAUDE, CODEX};

    const VERSION_3: [&str; 3] = [
        include_str!("migrations/001_init.sql"),
        include_str!("migrations/002_subscription_lapses.sql"),
        include_str!("migrations/003_reported_cost.sql"),
    ];

    const LEGACY_ACCOUNTS: &str = "INSERT INTO accounts \
        (id, provider, home, owner, sort_order, last_seen) VALUES \
        ('grok:cli', 'grok', '/home/ada/.grok', 'cli', 0, 0), \
        ('grok:own', 'grok', '/data/headroom/accounts/grok/1', 'headroom', 1, 0)";

    fn version_3_database(settings: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        for sql in VERSION_3 {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute_batch(LEGACY_ACCOUNTS).unwrap();
        conn.execute(
            "INSERT INTO settings (id, payload) VALUES (1, ?1)",
            [settings],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();
        conn
    }

    fn home(provider: ProviderId, path: &str) -> DismissedHome {
        DismissedHome {
            account: AccountId(format!("{provider}:ada")),
            provider,
            home: PathBuf::from(path),
        }
    }

    #[test]
    fn dismissed_homes_are_stored_once_and_restored_per_provider() {
        let storage = Storage::open_in_memory().unwrap();
        let codex = home(CODEX, "/home/ada/.codex");
        let claude = home(CLAUDE, "/home/ada/.claude");
        let loaded = storage
            .blocking(|conn| {
                insert(conn, &codex)?;
                insert(conn, &codex)?;
                insert(conn, &claude)?;
                load_all(conn)
            })
            .unwrap();
        let listed: Vec<_> = loaded.iter().cloned().collect();
        assert_eq!(listed, [claude.clone(), codex.clone()]);
        let left = storage
            .blocking(|conn| {
                restore(conn, Some(&CODEX))?;
                load_all(conn)
            })
            .unwrap();
        assert_eq!(left.iter().cloned().collect::<Vec<_>>(), [claude]);
        let none = storage
            .blocking(|conn| {
                restore(conn, None)?;
                load_all(conn)
            })
            .unwrap();
        assert_eq!(none, DismissedHomes::default());
    }

    #[test]
    fn legacy_dismissed_ids_become_the_cli_homes_they_named() {
        let legacy =
            r#"{"reduced_motion":true,"dismissed_accounts":["grok:cli","grok:own","grok:gone"]}"#;
        let mut conn = version_3_database(legacy);
        migrations::migrate(&mut conn).unwrap();
        let listed: Vec<_> = load_all(&conn).unwrap().iter().cloned().collect();
        assert_eq!(
            listed,
            [DismissedHome {
                provider: ProviderId::parse("grok").unwrap(),
                account: AccountId("grok:cli".into()),
                home: PathBuf::from("/home/ada/.grok"),
            }]
        );
        let payload: String = conn
            .query_row("SELECT payload FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(payload, r#"{"reduced_motion":true}"#);
        assert!(settings::load(&conn).unwrap().reduced_motion);
    }

    #[test]
    fn settings_without_dismissals_migrate_to_an_empty_list() {
        for stored in [r#"{"reduced_motion":true}"#, "not json"] {
            let mut conn = version_3_database(stored);
            migrations::migrate(&mut conn).unwrap();
            assert_eq!(load_all(&conn).unwrap(), DismissedHomes::default());
        }
    }
}
