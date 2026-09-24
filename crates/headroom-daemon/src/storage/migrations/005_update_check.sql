CREATE TABLE update_check (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    checked_at INTEGER NOT NULL,
    etag TEXT,
    version TEXT,
    url TEXT,
    published_at INTEGER
);
