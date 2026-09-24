# Kilo Code fixtures

Synthetic but documented: no Kilo account was available when the provider was written. The shapes
come from Kilo's open-source client and server, the values are made up:

- `balance.json`, `balance_depleted.json`: `GET /api/profile/balance`, `{ balance, isDepleted }`
  (`Kilo-Org/cloud` `apps/web/src/app/api/profile/balance/route.ts`). The server computes `balance`
  as micro-USD / 1 000 000, so it never has more than six decimals.
- `profile.json`: `GET /api/profile` (`Kilo-Org/kilocode` `packages/kilo-gateway/src/api/profile.ts`).
- `auth.json`, `auth_organization.json`: the `kilo` entry that `kilo auth login` writes to
  `$XDG_DATA_HOME/kilo/auth.json` (`packages/kilo-gateway/src/auth/device-auth-tui.ts`,
  `packages/opencode/src/auth/index.ts`); `accountId` is set when an organization was chosen.

Replace them with anonymised real responses once one is captured.
