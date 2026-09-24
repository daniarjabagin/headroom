# Poe fixtures

Synthetic but documented: no Poe account was available when the provider was written, so these
bodies follow the shapes in Poe's API reference
(`https://creator.poe.com/api-reference/getCurrentBalance`) instead of captured responses.
Replace them with anonymised real responses once one is captured.

- `current_balance.json`: `GET /usage/current_balance`.
- `invalid_key.json`: the 401 `authentication_error` body.
