CREATE TABLE quota_samples (
    account_id TEXT NOT NULL,
    window_id TEXT NOT NULL,
    used_percent REAL NOT NULL,
    at INTEGER NOT NULL,
    PRIMARY KEY (account_id, window_id, at)
) WITHOUT ROWID;
CREATE INDEX quota_samples_at ON quota_samples (at);
