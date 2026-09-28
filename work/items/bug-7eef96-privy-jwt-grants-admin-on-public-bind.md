+++
id = "bug-7eef96"
kind = "bug"
title = "Privy JWT grants admin to any Nunchi Privy user on a publicly bound roko serve"
status = "open"
triage = "verified"
severity = "p0"
subsystem = ["roko-serve/auth"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md#security-finding"
discovered_from = "session:roko-55 (tmp/cybernetic-harness/research/F-fix-inventory.md), verified by session roko-b6"
anchors = ["crates/roko-serve/src/lib.rs::build_app_state", "crates/roko-serve/src/routes/middleware.rs::try_privy_jwt"]
doc = ""
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = "grep -n 'No role filter configured' crates/roko-serve/src/routes/middleware.rs"
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
