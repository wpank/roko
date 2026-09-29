+++
id = "bug-7eef96"
kind = "bug"
title = "Privy JWT grants admin to any Nunchi Privy user on a publicly bound roko serve"
status = "open"
triage = "verified"
severity = "p0"
hold = "Deferred by Will on 2026-09-28 (\"file it for later\"); do not pick until he lifts the hold"
goal = "release"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md#security-finding"
discovered_from = "session:roko-55 (tmp/cybernetic-harness/research/F-fix-inventory.md), verified by session roko-b6"
anchors = ["crates/roko-serve/src/lib.rs::build_app_state", "crates/roko-serve/src/routes/middleware.rs::try_privy_jwt", "crates/roko-serve/src/jwks.rs::NUNCHI_PRIVY_APP_ID"]
doc = ""
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = "grep -n 'No role filter configured' crates/roko-serve/src/routes/middleware.rs"

[[verify]]
command = "! grep -q 'No role filter configured' crates/roko-serve/src/routes/middleware.rs && cargo test -p roko-serve --lib privy_jwt_without_allow_list_is_not_authenticated"
+++

`roko-serve/src/lib.rs` always sets `serve.auth.privy_app_id` to the Nunchi Privy app id
and auto-enables auth for any non-loopback bind. `routes/middleware.rs::try_privy_jwt`
then grants the `admin` scope to any valid Privy JWT when `privy_allowed_roles` is empty
and `privy_workspace_id` is unset ("No role filter configured — grant admin (legacy
behaviour)"). So any user who can sign in to Nunchi's Privy app gets admin — run plans,
dispatch agents — on any publicly bound `roko serve` that has not configured those fields.

Fix direction: fail closed. A Privy JWT gets admin only when an allow-list matches
(`privy_allowed_roles`, `privy_workspace_id`, or a new `privy_allowed_user_ids`);
otherwise read scope, with a startup warning naming the settings. Check deployed
instances (Railway) before shipping, since they may rely on today's behaviour.

Deferred by Will on 2026-09-28 ("file it for later"); not changed in the portal push.

2026-09-28: `wpank/roko` is public (checked with `gh repo view`), so this code is already public. `deploy-fly.yml` and `docker-publish.yml` are guarded by the repository variable `ALLOW_PUBLIC_DEPLOY` (b02812036) until this is fixed.

2026-09-29: fixed on branch `fix/privy-no-default-admin`. `roko serve` no longer fills in
`NUNCHI_PRIVY_APP_ID`: Privy JWT auth is off until the operator sets `serve.auth.privy_app_id`.
Without `privy_allowed_roles` or `privy_workspace_id`, a Privy JWT is rejected outright rather
than getting read scope (the direction given for the fix), with a warning at startup and on the
first such request. The verify test was renamed to match.
