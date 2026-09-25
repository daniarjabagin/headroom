ALTER TABLE usage_events ADD COLUMN project TEXT;

UPDATE log_cursors SET payload = '{}' WHERE provider IN ('claude', 'codex');
