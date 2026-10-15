+++
id = "bug-17213a"
kind = "bug"
title = "A Privy JWT with a role outside the allow-list gets read scope instead of being rejected (D18, backlog 9340)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/auth"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-3"
anchors = ["crates/roko-serve/src/routes/middleware.rs::try_privy_jwt", "crates/roko-serve/src/routes/middleware.rs::privy_jwt_scope_follows_the_role_allow_list"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-47e0de", "gap-00e882", "bug-7eef96"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'downgrading to read scope' crates/roko-serve/src/routes/middleware.rs && grep -rqw 'fn privy_jwt_role_mismatch_is_rejected' crates/roko-serve/src/ && cargo test -p roko-serve --lib privy_jwt_role_mismatch_is_rejected"
+++

## Problem

A Privy JWT whose `role` is not in `serve.auth.privy_allowed_roles` still authenticates, with `read` scope.
`try_privy_jwt` (`crates/roko-serve/src/routes/middleware.rs:469`) grants `admin` to a listed role and, for a missing or
unlisted one, logs "Privy JWT role not in allowed list — downgrading to read scope" and returns `read` (:503-518; the doc
comment at :465-468 says the same). `read` covers every GET, including `/api/config` and secret names
(`required_scope_for`). The test `privy_jwt_scope_follows_the_role_allow_list` (:2746) asserts the downgrade.

## Why it matters

Goal `release` (S11.T5, G0b). Will's decision 3 (2026-10-15) accepted 9304's default, D18: reject a Privy JWT on a role
or user-id mismatch. Any operator who sets a role allow-list today gives read access to every other member of the Privy
app. Backlog task 9340 (`tmp/backlog/2026-10-02-complete-and-wire/9340-privy-role-mismatch-is-rejected-not-
downgraded.md`) sits inside package PK87 (gap-00e882), which is held on the showcase decisions 9302/9304; this item
takes 9340 out so it is not blocked by the showcase.

## Where

- `crates/roko-serve/src/routes/middleware.rs::try_privy_jwt`, the role branch, and its doc comment.
- Tests in the same file: `privy_jwt_scope_follows_the_role_allow_list`, helpers `privy_auth`, `privy_test_jwt`,
  `privy_middleware_status`.
- `docs/v3/24-AUTH.md` §7.2 if it describes the downgrade.

## Current state

Unchanged since 9340 was written (`2c3ea9f73`). A matching `privy_workspace_id` with no roles still grants `admin` (the
"workspace is the allow-list" case, intended). bug-7eef96 made Privy off unless `privy_app_id` is set and fail closed
without an allow-list.

## Plan

1. Return `None` on a missing or unlisted role, with a `warn!` naming `sub` and the role.
2. Rewrite `privy_jwt_scope_follows_the_role_allow_list` to expect 401 for the unlisted role, and add
   `privy_jwt_role_mismatch_is_rejected` (a real ES256 token through the middleware, as the existing tests do).
3. Update the doc comment's step 4 and the auth doc.

## Done when

A role mismatch returns 401 on a protected route, and the `[[verify]]` command passes.

## Notes

- gap-47e0de (a `privy_allowed_user_ids` allow-list) edits the same function: run one, then the other. Under D18 its
  user-id mismatch must also reject, not downgrade.
- In showcase mode every Bearer JWT is refused anyway (9326); this matters for non-showcase public deploys.
