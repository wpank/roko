+++
id = "bug-955c41"
kind = "bug"
title = "API Key Comparison Not Timing-Safe in Auth Middleware"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-serve"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/96-serve-api-key-timing-safe.md#96 — API Key Comparison Not Timing-Safe in Auth Middleware"
discovered_from = "audit:tmp/backlog/archive/96-serve-api-key-timing-safe.md#96 — API Key Comparison Not Timing-Safe in Auth Middleware"
anchors = ["crates/roko-serve/src/routes/middleware.rs::authenticate_api_key", "crates/roko-serve/src/routes/middleware.rs::constant_time_eq", "crates/roko-serve/src/routes/team.rs:318", "crates/roko-serve/src/routes/webhooks.rs:666", "crates/roko-plugin/src/registry.rs:878"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rn 'token == auth.api_key' crates/roko-serve/src"

[[verify]]
command = "grep -q 'core::hint::black_box(diff) == 0' crates/roko-serve/src/routes/middleware.rs"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed in committed code; these files are clean in the working tree. middleware.rs:389 compares keys with constant_time_eq. The canonical pub(crate) constant_time_eq with core::hint::black_box is at middleware.rs:1574. team.rs:25/318 and webhooks.rs:29/666/696 import that shared helper. Residual hardening nit, not the issue in the title: crates/roko-plugin/src/registry.rs:878 keeps its own fold-based helper without black_box. Both grep verify commands pass; cargo tests were not run in this triage."
+++
[partial] PARTIALLY DONE (2026-08-23). Canonical `constant_time_eq` added to middleware.rs; `token == auth.api_key`… — Security: the legacy single-key auth path uses `==` on strings, which leaks timing information that can be exploited to recover the key character-by-character. The HTTP control…

Imported without verification from:
- `tmp/backlog/archive/96-serve-api-key-timing-safe.md#96 — API Key Comparison Not Timing-Safe in Auth Middleware`

How to verify: Check: The comparison at `middleware.rs` line 389 (`token == auth.api_key`) is replaced with `constant_time_eq(token.as_bytes(), auth.api_key.as_bytes())`.; There is exactly one canonical `constant_time_eq` implementation with… [evidence: own status: PARTIALLY DONE (2026-08-23). Canonical `constant_time_eq` added to middleware.rs; `token == auth.api_key`…; 00-INDEX historical claim: Plan…]

Verified 2026-09-28: fixed (middleware.rs:389 and :1574). See [closed].
