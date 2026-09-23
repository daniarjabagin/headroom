use headroom_core::account::{AccountId, AccountRef, ProviderKind};
use jiff::Timestamp;
use rusqlite::{Connection, Row, params};

use super::codec::{
    enum_from_sql, enum_to_sql, path_from_sql, path_to_sql, timestamp_from_sql, timestamp_to_sql,
};
use crate::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub reference: AccountRef,
    pub label: Option<String>,
    pub hidden: bool,
    pub sort_order: i64,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub last_seen: Timestamp,
    pub gone: bool,
}

impl AccountRecord {
    #[must_use]
    pub fn id(&self) -> &AccountId {
        &self.reference.id
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        !self.hidden && !self.gone
    }
}

struct RawAccount {
    id: String,
    provider: String,
    home: String,
    owner: String,
    label: Option<String>,
    hidden: bool,
    sort_order: i64,
    email: Option<String>,
    plan: Option<String>,
    last_seen: i64,
    gone: bool,
}

const SELECT: &str = "SELECT id, provider, home, owner, label, hidden, sort_order, email, plan, \
                      last_seen, gone FROM accounts ORDER BY sort_order, id";

const UPSERT: &str = "INSERT INTO accounts (id, provider, home, owner, sort_order, last_seen, gone) \
     VALUES (?1, ?2, ?3, ?4, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM accounts), ?5, 0) \
     ON CONFLICT(id) DO UPDATE SET provider = excluded.provider, home = excluded.home, \
     owner = excluded.owner, last_seen = excluded.last_seen, gone = 0";

pub fn load_all(conn: &Connection) -> Result<Vec<AccountRecord>, StorageError> {
    let mut statement = conn.prepare(SELECT)?;
    let raw = statement
        .query_map([], read_raw)?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter().map(decode).collect()
}

fn read_raw(row: &Row<'_>) -> rusqlite::Result<RawAccount> {
    Ok(RawAccount {
        id: row.get(0)?,
        provider: row.get(1)?,
        home: row.get(2)?,
        owner: row.get(3)?,
        label: row.get(4)?,
        hidden: row.get(5)?,
        sort_order: row.get(6)?,
        email: row.get(7)?,
        plan: row.get(8)?,
        last_seen: row.get(9)?,
        gone: row.get(10)?,
    })
}

fn decode(raw: RawAccount) -> Result<AccountRecord, StorageError> {
    Ok(AccountRecord {
        reference: AccountRef {
            id: AccountId(raw.id),
            provider: enum_from_sql("provider", raw.provider)?,
            home: path_from_sql(raw.home),
            owner: enum_from_sql("owner", raw.owner)?,
        },
        label: raw.label,
        hidden: raw.hidden,
        sort_order: raw.sort_order,
        email: raw.email,
        plan: raw.plan,
        last_seen: timestamp_from_sql(raw.last_seen)?,
        gone: raw.gone,
    })
}

pub fn sync_provider(
    conn: &mut Connection,
    provider: ProviderKind,
    discovered: &[AccountRef],
    now: Timestamp,
) -> Result<(), StorageError> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE accounts SET gone = 1 WHERE provider = ?1",
        [enum_to_sql(&provider)?],
    )?;
    let seen = timestamp_to_sql(now)?;
    for account in discovered.iter().filter(|a| a.provider == provider) {
        tx.execute(
            UPSERT,
            params![
                account.id.0,
                enum_to_sql(&account.provider)?,
                path_to_sql(&account.home)?,
                enum_to_sql(&account.owner)?,
                seen
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn set_identity(
    conn: &Connection,
    id: &AccountId,
    email: Option<&str>,
    plan: Option<&str>,
) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE accounts SET email = ?2, plan = ?3 WHERE id = ?1",
        params![id.0, email, plan],
    )?;
    Ok(())
}

pub fn set_label(
    conn: &Connection,
    id: &AccountId,
    label: Option<&str>,
) -> Result<bool, StorageError> {
    let changed = conn.execute(
        "UPDATE accounts SET label = ?2 WHERE id = ?1",
        params![id.0, label],
    )?;
    Ok(changed > 0)
}

pub fn set_hidden(conn: &Connection, id: &AccountId, hidden: bool) -> Result<bool, StorageError> {
    let changed = conn.execute(
        "UPDATE accounts SET hidden = ?2 WHERE id = ?1",
        params![id.0, hidden],
    )?;
    Ok(changed > 0)
}

pub fn set_order(conn: &mut Connection, ordered: &[AccountId]) -> Result<(), StorageError> {
    let tx = conn.transaction()?;
    for (index, id) in ordered.iter().enumerate() {
        let order = i64::try_from(index).map_err(|_| StorageError::OutOfRange("sort order"))?;
        tx.execute(
            "UPDATE accounts SET sort_order = ?2 WHERE id = ?1",
            params![id.0, order],
        )?;
    }
    tx.commit()?;
    Ok(())
}
