use rusqlite::Connection;

use crate::error::StorageError;

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_subscription_lapses.sql"),
    include_str!("migrations/003_reported_cost.sql"),
    include_str!("migrations/004_dismissed_homes.sql"),
    include_str!("migrations/005_update_check.sql"),
    include_str!("migrations/006_project.sql"),
    include_str!("migrations/007_held_alerts.sql"),
    include_str!("migrations/008_status_cache.sql"),
    include_str!("migrations/009_onboarding_existing.sql"),
    include_str!("migrations/010_quota_samples.sql"),
];

pub fn migrate(conn: &mut Connection) -> Result<(), StorageError> {
    migrate_through(conn, MIGRATIONS.len())
}

fn migrate_through(conn: &mut Connection, target: usize) -> Result<(), StorageError> {
    let current = user_version(conn)?;
    if current > MIGRATIONS.len() {
        return Err(StorageError::FutureSchema {
            found: current,
            supported: MIGRATIONS.len(),
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().take(target).skip(current) {
        apply(conn, index + 1, sql)?;
    }
    Ok(())
}

pub fn user_version(conn: &Connection) -> Result<usize, StorageError> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    usize::try_from(version).map_err(|_| StorageError::OutOfRange("user_version"))
}

fn apply(conn: &mut Connection, version: usize, sql: &str) -> Result<(), StorageError> {
    let tx = conn.transaction()?;
    tx.execute_batch(sql)?;
    let version = i64::try_from(version).map_err(|_| StorageError::OutOfRange("user_version"))?;
    tx.pragma_update(None, "user_version", version)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
pub fn latest() -> usize {
    MIGRATIONS.len()
}

#[cfg(test)]
#[path = "migrations_tests.rs"]
mod tests;
