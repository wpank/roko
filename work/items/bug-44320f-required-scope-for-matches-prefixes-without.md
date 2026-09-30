+++
id = "bug-44320f"
kind = "bug"
title = "required_scope_for matches prefixes without segment boundaries, so POST /api/relay-tokens gets the /relay scope"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/middleware"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/routes/middleware.rs::required_scope_for"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn scope_prefixes_match_whole_path_segments' crates/roko-serve/src/ && cargo test -p roko-serve --lib scope_prefixes_match_whole_path_segments"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 1c7442b58. Scope prefixes match whole path segments; /api/relay-tokens needs admin. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

`required_scope_for` (`routes/middleware.rs:1284`) picks a mutating route's scope with `path.starts_with(entry.prefix)`, with no segment boundary. A prefix entry for `/relay` (or `/api/relay`) therefore also matches `/api/relay-tokens`, and `POST /api/relay-tokens` gets the relay scope (`agent:write`) instead of its own.

## Why it matters

Release blockers: a scope table that matches by string prefix can grant or require the wrong scope for any route whose name extends another's. p2.

## Where

The prefix matching in `required_scope_for`.

## Plan

1. Match a prefix only at a segment boundary: the path equals the prefix, or continues with `/`.
2. Give `/api/relay-tokens` its own entry.
3. Add `scope_prefixes_match_whole_path_segments`.

## Done when

- [ ] Every route gets the scope of its own table entry.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-af1020` at `e82d934d7`; cargo verification deferred to the batch check. Scope-table and extension prefixes match only at a path-segment boundary (`route_permissions::path_has_segment_prefix`, shared with the RBAC table). Three routes relied on the loose match and got explicit entries: `/api/relay-tokens` -> `admin` (issuing needs `token:issue`, admin-only, so no working caller loses access), `/api/prds` -> `plan:write` and `/api/runs` -> `write`, the scopes they had before. A simulation over every registered route shows no other change. Test: `middleware::tests::scope_prefixes_match_whole_path_segments`; the route coverage lists gain `/api/relay-tokens`.
