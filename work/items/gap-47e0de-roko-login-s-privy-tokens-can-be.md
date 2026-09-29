+++
id = "gap-47e0de"
kind = "gap"
title = "roko login's Privy tokens can be allow-listed only by role or org_id claims; there is no user-id allow-list"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/auth", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d5c1dc6be"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "follow-up to bug-7eef96 (3fd7bfc76); its fix direction named a user-id allow-list that was not built"
anchors = ["crates/roko-serve/src/routes/middleware.rs::try_privy_jwt", "crates/roko-serve/src/routes/middleware.rs::privy_allow_list_configured", "crates/roko-serve/src/jwks.rs::PrivyClaims", "crates/roko-core/src/config/serve.rs::privy_allowed_roles", "crates/roko-cli/src/commands/auth.rs::cmd_login_browser"]
links = { depends_on = [], blocks = [], related = ["bug-7eef96", "find-a1284b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'privy_allowed_users' crates/roko-core/src/config/serve.rs && grep -rqw 'fn privy_jwt_user_allow_list_admits_only_listed_users' crates/roko-serve/src && cargo test -p roko-serve --lib privy_jwt_user_allow_list_admits_only_listed_users"
+++

## Problem

Since bug-7eef96's fix, `roko serve` accepts a Privy JWT only when `serve.auth.privy_allowed_roles` or
`serve.auth.privy_workspace_id` is set, and both match claims inside the token:

- With `privy_workspace_id`, the `org_id` claim must equal it, or the token is rejected.
- With `privy_allowed_roles`, a token whose `role` claim is missing or not listed gets `read` scope.

`roko login` (the browser flow) stores the Privy access token the dashboard hands back. Privy documents its
access-token claims as `sid`, `sub`, `iss`, `aud`, `iat` and `exp`. `role` and `org_id` are not among them. This was
not checked against a live `roko login` token in this session. So unless the Privy app adds those claims, a user who
signs in with `roko login` is rejected, or at best gets read scope. No setting can grant one named person write
access. The one per-user value in the token, `sub` (`did:privy:…`, which `roko whoami` prints), cannot be matched by
any setting.

## Why it matters

Goal `release`: `roko login [url]` is the documented way to reach a deployed `roko serve`. After the fail-closed fix
(which was right), its browser path gives no usable access, and operators must fall back to API keys. bug-7eef96's
own fix direction named "a new `privy_allowed_user_ids`"; it was not built.

## Where

- `crates/roko-serve/src/routes/middleware.rs`:
  - `privy_allow_list_configured` (:39-41) counts only roles and workspace; `PRIVY_ALLOW_LIST_HINT` (:30-33)
    names those two settings.
  - `try_privy_jwt` (:438-491): workspace check (:456-469), role check (:472-488). It returns `claims.sub` as the
    user id.
- `crates/roko-serve/src/jwks.rs::PrivyClaims` (:61-76): `org_id` and `role` are optional.
- `crates/roko-core/src/config/serve.rs`: `privy_app_id` (:219), `privy_workspace_id` (:230) and
  `privy_allowed_roles` (:238) on `ServeAuthConfig`.
- `crates/roko-cli/src/commands/auth.rs::cmd_login_browser` (:98) stores `access_token` with method `privy` and
  `privy_user_id`. `roko whoami` shows the user id (:293-294).
- Docs: `docs/v3/24-AUTH.md` §7.2 (JWT validation chain) and the config example near :513.

## Current state

Checked at `d5c1dc6be` by reading the code. The fix is `3fd7bfc76` (bug-7eef96, done). Its tests build ES256
tokens with chosen `role` and `org_id` claims (`privy_test_jwt` in `middleware.rs`), so they do not show what a
real Privy token contains.

## Plan

1. Decode the payload of a real `roko login` token (base64, no signature check needed) and record which claims it
   has. If it does carry `role` or `org_id`, write down where they come from and narrow this item.
2. Add `privy_allowed_users: Vec<String>` (Privy user ids, matched against `sub`) to `ServeAuthConfig`. Count it in
   `privy_allow_list_configured`, and name it in `PRIVY_ALLOW_LIST_HINT` and the docs.
3. In `try_privy_jwt`, a listed `sub` gets `admin`. Decide how this combines with `privy_workspace_id`:
   - independent: a listed user needs no `org_id`. Simplest, and what `roko login` users need.
   - AND: listed and a member.
   Recommended: independent, with a startup log line naming how many users are listed.
4. Tests in roko-serve: `privy_jwt_user_allow_list_admits_only_listed_users` (a listed `sub` gets admin; an
   unlisted `sub` is rejected when no other allow-list is set), plus the chosen interaction with roles and workspace.

## Done when

- An operator can grant specific Privy users write access by user id, and everyone else stays fail-closed.
- The `[[verify]]` command passes.

## Notes

- Keep fail-closed: an empty `privy_allowed_users` must not widen access.
- find-a1284b (the default Privy JWKS URL returns 404) also affects Privy auth on deployed servers.
