+++
id = "gap-a247a4"
kind = "gap"
title = "Impact analysis is conservative text/Cargo-graph analysis, not a semantic reference query"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/impact-analysis"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#impact-analysis-uses-conservative-syntax-and-cargo-graph-evidence----partial"
anchors = ["crates/roko-cli/src/runner/impact_analysis.rs", "crates/roko-cli/src/commands/impact.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Focused verification maps diffs to Cargo targets and honours required features. It widens shared modules and compiles bounded reverse dependents for public, re-export, trait and serde edits. Pre-dispatch prompts and plan preflight flag likely scope omissions. The analysis is still text- and Cargo-graph based (`crates/roko-cli/src/runner/impact_analysis.rs`, used by `commands/impact.rs`, `runner/cargo_command.rs` and `runner/gate_dispatch.rs`), not a semantic `roko-index` reference query. It can miss macro-generated public APIs, non-Rust schema consumers and symbol-level call sites, which then need a full verification lane or operator review.

Fix: back impact scoping with `roko-index` symbol references, falling back to the current analysis when the index is stale.
