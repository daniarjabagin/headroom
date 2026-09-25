CREATE TABLE status_cache (
    url TEXT PRIMARY KEY,
    fetched_at INTEGER NOT NULL,
    etag TEXT,
    body TEXT NOT NULL
);
