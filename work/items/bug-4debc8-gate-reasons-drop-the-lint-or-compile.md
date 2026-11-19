+++
id = "bug-4debc8"
kind = "bug"
title = "Gate reasons drop the lint or compile error when cargo colors its output (CARGO_TERM_COLOR=always, as CI sets it): diagnostic lines start with escape sequences, so the summary falls back to cargo's 'Checking …' line and the classification misses the diagnostic"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-gate"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead: release PR #89's Test + Clippy run 44589959537 (roko-gate tests/rungs.rs:352)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-gate/src/clippy_gate.rs", "crates/roko-gate/src/compile.rs", "crates/roko-gate/src/verify_chain_gate.rs::strip_ansi"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-eeadbb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn output_text' crates/roko-gate/src/output_text.rs && cargo test -p roko-gate --test rungs -- colors_its_output"
+++

## Problem

roko-gate's gates summarize a failed command's output into the verdict's reason. The clippy gate
(`summarize_lint_issues`) and the compile gate (`summarize_errors`) take the first lines that start with `error:`,
`error[` or `warning:`, and otherwise fall back to the first non-empty line. The failure classification
(`compile_errors`) reads the same output line by line.

When cargo colors its output, every diagnostic line starts with an escape sequence instead
(`ESC[1mESC[91merrorESC[0mESC[1m: unused variable: ...`), so none of them matches. Cargo colors it under
`CARGO_TERM_COLOR=always`, which CI sets for every job and some people export in their shells, or under
`--color always`. The clippy gate's reason then becomes cargo's first status line, "Checking fixture v0.1.0 (...)",
with its escapes: the lint a person needs is gone. The verdict's detail, which retry feedback shows the agent, keeps
the escapes too.

Seen on 2026-11-19 in release PR #89's Test + Clippy on Linux (run 44589959537).
`crates/roko-gate/tests/rungs.rs::clippy_rung_passes_and_rejects_lint_noise` failed at line 352: the reason contained
neither "unused" nor "warning". It isn't Linux: on macOS, the test fails the same way with `CARGO_TERM_COLOR=always`
and passes without it. CI never got to it before, because `cargo test --workspace` stopped at earlier failing test
binaries.

## Why it matters

Goal core. A lint or compile failure's reason is what the operator, the retry prompt and the failure classification
read. Under a common cargo setting the reason drops the lint and keeps a progress line, and the classification no
longer sees the diagnostic.

## Where

- `crates/roko-gate/src/clippy_gate.rs`, `compile.rs`, `test_gate.rs` and the other gates that capture a command's
  output: `generated_test_gate.rs`, `property_test_gate.rs`, `integration_gate.rs`, `benchmark_gate.rs`,
  `generated.rs` and `shell.rs`.
- `crates/roko-gate/src/verify_chain_gate.rs::strip_ansi`, the existing stripper.

## Plan

1. One helper turns a command's captured bytes into plain text, with the escape sequences stripped. The gates use it
   where they capture output, so reasons, details and classifications all read plain text. A shell step's live lines
   stay as they come.
2. Red first, deterministically: the clippy and compile rungs run with `--color always`. Today the clippy reason is
   cargo's colored "Checking" line.

## Done when

A lint or compile failure's reason carries its first diagnostic whatever cargo's color setting is, and the
`[[verify]]` passes.
