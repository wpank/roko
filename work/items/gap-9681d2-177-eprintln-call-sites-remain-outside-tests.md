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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#batch-2026-08-1213/eprintln-expect"
anchors = ["crates/roko-cli/src/prd.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/runner/output_sink.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq 'print_stderr *= *\"(warn|deny)\"' Cargo.toml && test \"$(git grep -n 'eprintln!' -- 'crates/*.rs' ':!*/tests/*' ':!*tests.rs' ':!*/examples/*' ':!*/benches/*' | wc -l)\" -le 60"
+++

The 2026-08-12/13 conversions (`eprintln!` to `tracing`, `.expect()` to `Result`) were only partial. On 2026-09-28 there are 177 `eprintln!` calls in non-test Rust files. The largest counts are `crates/roko-cli/src/prd.rs` (39), `main.rs` (22) and `runner/output_sink.rs` (13). Some of these are legitimate user-facing stderr; the rest should use `tracing`. Backlog #381 is archived without a status. `.expect()` sites were not re-counted.

Fix: classify each call site as user-facing output or diagnostics, and convert the diagnostics. Add a lint or grep check that blocks new ones.

Re-checked 2026-09-29: unchanged; 174 non-test calls by a stricter count than the original 177, with the same top files. The parked, unverified gap-556ffe (created 2026-09-21, backlog #381, 'Complete eprintln! to tracing Migration') describes the same migration. Mark it duplicate_of this verified item, or fold it in, so only one item stays live.
