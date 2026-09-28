+++
id = "gap-0f3980"
kind = "gap"
title = "TaskDef Routing Metadata Wiring (24-field gap)"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/403-taskdef-routing-metadata-wiring.md#403 — TaskDef Routing Metadata Wiring (24-field gap)"
discovered_from = "audit:tmp/backlog/archive/403-taskdef-routing-metadata-wiring.md#403 — TaskDef Routing Metadata Wiring (24-field gap)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-cli/src/task_parser.rs::TaskDefSerde", "crates/roko-core/src/task.rs::Task", "crates/roko-cli/src/dispatch/model_routing.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`roko_core::Task` (`crates/roko-core/src/task.rs`) defines 34 fields covering model routing, gate selection, prompt assembly, scheduling, and infrastructure declarations. Every field has a complete type definition, serde derives, unit tests, and doc comments.

Imported without verification from:
- `tmp/backlog/archive/403-taskdef-routing-metadata-wiring.md#403 — TaskDef Routing Metadata Wiring (24-field gap)`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-gate/src/dispatch.rs`.

How to verify: Check: `cargo build --workspace` compiles with all 24 new fields in `TaskDefSerde` and `TaskDef`.; `cargo test -p roko-cli` passes, including task parser roundtrip tests with all new fields.; A `tasks.toml` that uses every new field parses without… [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: roko_core::Task (crates/roko-core/src/task.rs) has 34 fields, crates/roko-cli/src/task_parser.rs TaskDef 27 and TaskDefSerde 26; 25 Task field names are absent from TaskDef, among them preferred_model, preferred_provider, reasoning_level, speed_priority, quality_profile, escalate_on_retry, complexity_band, category, tags, skills, parallel_group, exclusive_files, context_files, test_invariants.
