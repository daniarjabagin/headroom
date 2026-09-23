use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use headroom_core::provider::ProviderError;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

const VALUE_QUERY: &str = "SELECT value FROM ItemTable WHERE key = ?1";

pub(super) fn read_items(
    path: &Path,
    keys: &[&str],
) -> Result<BTreeMap<String, String>, ProviderError> {
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let connection = open_untouched(path).map_err(|error| db_error(&error))?;
    let mut statement = connection
        .prepare(VALUE_QUERY)
        .map_err(|error| db_error(&error))?;
    let mut items = BTreeMap::new();
    for key in keys {
        let value = statement
            .query_row([key], |row| Ok(text(row.get_ref(0)?)))
            .optional()
            .map_err(|error| db_error(&error))?
            .flatten();
        if let Some(value) = value {
            items.insert((*key).to_owned(), value);
        }
    }
    Ok(items)
}

fn open_untouched(path: &Path) -> rusqlite::Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Connection::open_with_flags(immutable_uri(path), flags)
}

fn immutable_uri(path: &Path) -> String {
    let mut uri = String::from("file:");
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(byte) {
            uri.push(char::from(*byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri.push_str("?mode=ro&immutable=1");
    uri
}

fn text(value: ValueRef<'_>) -> Option<String> {
    let bytes = match value {
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => bytes,
        ValueRef::Null | ValueRef::Integer(_) | ValueRef::Real(_) => return None,
    };
    let text = std::str::from_utf8(bytes).ok()?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn db_error(error: &rusqlite::Error) -> ProviderError {
    ProviderError::LocalData(format!("cannot read the Cursor state database: {error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn create_db(path: &Path, rows: &[(&str, &str)]) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
            )
            .unwrap();
        for (key, value) in rows {
            connection
                .execute("INSERT INTO ItemTable VALUES (?1, ?2)", [key, value])
                .unwrap();
        }
    }

    #[test]
    fn reads_requested_keys_and_skips_blank_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.vscdb");
        create_db(
            &path,
            &[
                ("cursorAuth/accessToken", " token-1 \n"),
                ("cursorAuth/stripeMembershipType", ""),
                ("other", "ignored"),
            ],
        );
        let items = read_items(
            &path,
            &[
                "cursorAuth/accessToken",
                "cursorAuth/stripeMembershipType",
                "missing",
            ],
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items["cursorAuth/accessToken"], "token-1");
    }

    #[test]
    fn blob_values_are_read_as_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.vscdb");
        create_db(&path, &[]);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "INSERT INTO ItemTable VALUES ('k', ?1)",
                [b"blob-value".as_slice()],
            )
            .unwrap();
        drop(connection);
        assert_eq!(read_items(&path, &["k"]).unwrap()["k"], "blob-value");
    }

    #[test]
    fn reading_leaves_the_database_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.vscdb");
        create_db(&path, &[("k", "v")]);
        let before = fs::read(&path).unwrap();
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        read_items(&path, &["k"]).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["state.vscdb"]);
    }

    #[test]
    fn missing_database_has_no_items() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            read_items(&dir.path().join("none.vscdb"), &["k"])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn garbage_file_is_a_local_data_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.vscdb");
        fs::write(
            &path,
            b"not a database at all, just text that is long enough",
        )
        .unwrap();
        assert!(matches!(
            read_items(&path, &["k"]),
            Err(ProviderError::LocalData(_))
        ));
    }

    #[test]
    fn paths_with_uri_characters_are_escaped() {
        assert_eq!(
            immutable_uri(Path::new("/tmp/a b?#%/state.vscdb")),
            "file:/tmp/a%20b%3F%23%25/state.vscdb?mode=ro&immutable=1"
        );
        let dir = tempfile::tempdir().unwrap();
        let odd = dir.path().join("we?ird #dir");
        fs::create_dir(&odd).unwrap();
        let path = odd.join("state.vscdb");
        create_db(&path, &[("k", "v")]);
        assert_eq!(read_items(&path, &["k"]).unwrap()["k"], "v");
    }
}
