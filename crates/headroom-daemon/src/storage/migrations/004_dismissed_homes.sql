CREATE TABLE dismissed_homes (
    provider TEXT NOT NULL,
    account_id TEXT NOT NULL,
    home TEXT NOT NULL,
    PRIMARY KEY (account_id, home)
) WITHOUT ROWID;

INSERT OR IGNORE INTO dismissed_homes (provider, account_id, home)
SELECT accounts.provider, accounts.id, accounts.home
FROM settings
JOIN json_each(
    CASE WHEN json_valid(settings.payload) THEN settings.payload ELSE '{}' END,
    '$.dismissed_accounts'
) AS dismissed
JOIN accounts ON accounts.id = dismissed.value
WHERE settings.id = 1 AND accounts.owner = 'cli';

UPDATE settings
SET payload = json_remove(payload, '$.dismissed_accounts')
WHERE id = 1 AND json_valid(payload);
