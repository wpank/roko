+++
id = "bug-330679"
kind = "bug"
title = "Cross-Crate Utility Duplication (parse_duration, constant_time_eq, truncate_to_budget)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/102-cross-crate-code-duplication.md#102 — Cross-Crate Utility Duplication (parse_duration, constant_time_eq…"
discovered_from = "audit:tmp/backlog/archive/102-cross-crate-code-duplication.md#102 — Cross-Crate Utility Duplication (parse_duration, constant_time_eq…"
anchors = ["crates/roko-core/", "crates/roko-runtime/", "crates/roko-cli/", "crates/roko-compose/", "crates/roko-serve/", "crates/roko-plugin/", "apps/agent-relay/", "apps/mirage-rs/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
maintainability; each duplicate has diverged in behavior, making bug fixes error-prone. Several utility functions are independently implemented multiple times across crates. Each copy has slightly different behavior: different supported units, different return types, and different error handling…

Imported without verification from:
- `tmp/backlog/archive/102-cross-crate-code-duplication.md#102 — Cross-Crate Utility Duplication (parse_duration, constant_time_eq…`

How to verify: Check: A single `roko_core::duration::parse_duration` function exists that supports all units:; All six local `parse_duration*` implementations in main-tree crates are deleted and; A single `roko_core::constant_time_eq` function exists that uses… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 6 |]
