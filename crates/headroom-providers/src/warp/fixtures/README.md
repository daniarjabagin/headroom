# Warp fixtures

Synthetic but documented: no Warp account was available when the provider was written. The bodies
follow the `GetRequestLimitInfo` query in Warp's public client
(`crates/graphql/src/api/queries/get_request_limit_info.rs`) and the fields CodexBar decodes
(`WarpUsageFetcher.swift`); the numbers and times are made up. Replace them with anonymised real
responses once one is captured.

- `request_limits.json`: a limited plan with a personal and a workspace bonus grant.
- `unlimited.json`: an unlimited plan without grants.
- `user_facing_error.json`: the `UserFacingError` branch of the `user` union.
- `graphql_errors.json`: a top-level GraphQL error without data.
