+++
id = "gap-9681d2"
kind = "gap"
title = "177 eprintln! call sites remain outside tests; tracing migration incomplete"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["workspace/logging"]
created = 2026-08-13
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#batch-2026-08-1213/eprintln-expect"
anchors = ["crates/roko-cli/src/prd.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/runner/output_sink.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The 2026-08-12/13 conversions (`eprintln!` to `tracing`, `.expect()` to `Result`) were only partial. On 2026-09-28 there are 177 `eprintln!` calls in non-test Rust files. The largest counts are `crates/roko-cli/src/prd.rs` (39), `main.rs` (22) and `runner/output_sink.rs` (13). Some of these are legitimate user-facing stderr; the rest should use `tracing`. Backlog #381 is archived without a status. `.expect()` sites were not re-counted.

Fix: classify each call site as user-facing output or diagnostics, and convert the diagnostics. Add a lint or grep check that blocks new ones.
