+++
id = "bug-928add"
kind = "bug"
title = "Permission lookup rewrites non-/api paths, so /relay permission rows never match"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/rbac"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/route_permissions.rs::required_permission_for", "crates/roko-serve/src/routes/route_permissions.rs:183"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn relay_path_requires_agent_spawn' crates/roko-serve/src/routes/route_permissions.rs && cargo test -p roko-serve route_permissions::tests::relay_path_requires_agent_spawn"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in b6906c9f8. /relay paths match their AgentSpawn row per path segment. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++

`required_permission_for` rewrites any path that does not start with `/api/` to `/api{path}` before matching (`route_permissions.rs:198-204`), but the manifest declares the relay row as `/relay` (`:183`, `AgentSpawn`).
`/relay/...` requests are looked up as `/api/relay/...`, miss their row and fall through to the generic rules (per a local audit: reads unrestricted, `ConfigEdit` otherwise), so the declared policy never applies.
Fix: match the raw path first (or declare rows in the rewritten form) and add a test that `/relay` resolves to its declared permission.

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-928add` at `b7aa91535`; cargo verification deferred to the batch check. Paths in a manifest family declared outside `/api` (the root-mounted relay proxy) are matched as they arrive; the match is per path segment so `/relay-tokens` keeps its `TokenIssue` row. Also adds `routes::tests::relay_mutation_uses_declared_agent_spawn_permission` (an `agent:write` key reaches the proxy).
