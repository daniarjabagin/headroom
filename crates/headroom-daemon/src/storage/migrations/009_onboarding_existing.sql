INSERT INTO settings (id, payload)
SELECT 1, '{"onboarding":{"completed":true}}'
WHERE NOT EXISTS (SELECT 1 FROM settings)
  AND (
    EXISTS (SELECT 1 FROM accounts)
    OR EXISTS (SELECT 1 FROM log_cursors)
    OR EXISTS (SELECT 1 FROM usage_events)
  );
