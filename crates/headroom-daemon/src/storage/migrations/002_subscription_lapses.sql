CREATE TABLE subscription_lapses (
    account_id TEXT PRIMARY KEY NOT NULL,
    detail TEXT NOT NULL,
    notified INTEGER NOT NULL DEFAULT 0
);
