CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    home TEXT NOT NULL,
    owner TEXT NOT NULL,
    label TEXT,
    hidden INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL,
    email TEXT,
    plan TEXT,
    last_seen INTEGER NOT NULL,
    gone INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE limits_snapshots (
    account_id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL,
    fetched_at INTEGER NOT NULL
);

CREATE TABLE usage_events (
    provider TEXT NOT NULL,
    usage_home TEXT NOT NULL,
    key TEXT NOT NULL,
    at INTEGER NOT NULL,
    model TEXT NOT NULL,
    tier TEXT NOT NULL,
    input INTEGER NOT NULL,
    cache_read INTEGER NOT NULL,
    cache_write_5m INTEGER NOT NULL,
    cache_write_1h INTEGER NOT NULL,
    output INTEGER NOT NULL,
    reasoning INTEGER NOT NULL,
    total INTEGER NOT NULL,
    web_search INTEGER NOT NULL,
    PRIMARY KEY (provider, usage_home, key)
) WITHOUT ROWID;

CREATE INDEX usage_events_by_home_time ON usage_events (provider, usage_home, at);

CREATE INDEX usage_events_by_time ON usage_events (at);

CREATE TABLE log_cursors (
    provider TEXT NOT NULL,
    usage_home TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (provider, usage_home)
);

CREATE TABLE notification_state (
    account_id TEXT NOT NULL,
    window_id TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (account_id, window_id)
);

CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    payload TEXT NOT NULL
);
