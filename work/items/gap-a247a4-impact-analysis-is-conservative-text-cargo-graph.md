+++
id = "gap-a247a4"
kind = "gap"
title = "Impact analysis is conservative text/Cargo-graph analysis, not a semantic reference query"
status = "open"
triage = "verified"
severity = "p3"
goal = "features"
subsystem = ["roko-cli/impact-analysis"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#impact-analysis-uses-conservative-syntax-and-cargo-graph-evidence----partial"
anchors = ["crates/roko-cli/src/runner/impact_analysis.rs::analyze", "crates/roko-cli/src/runner/impact_analysis.rs::try_symbol_oracle", "crates/roko-cli/src/commands/impact.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn index_references_select_targets' crates/roko-cli/ && cargo test -p roko-cli --lib runner::impact_analysis::tests::index_references_select_targets"
+++

Focused verification maps diffs to Cargo targets and honours required features. It widens shared modules and compiles bounded reverse dependents for public, re-export, trait and serde edits. Pre-dispatch prompts and plan preflight flag likely scope omissions. The analysis is still text- and Cargo-graph based (`crates/roko-cli/src/runner/impact_analysis.rs`, used by `commands/impact.rs`, `runner/cargo_command.rs` and `runner/gate_dispatch.rs`), not a semantic `roko-index` reference query. It can miss macro-generated public APIs, non-Rust schema consumers and symbol-level call sites, which then need a full verification lane or operator review.

Fix: back impact scoping with `roko-index` symbol references, falling back to the current analysis when the index is stale.

Rechecked 2026-09-29 at d9e79e9d8. A roko-index reference query already exists: try_symbol_oracle in crates/roko-cli/src/runner/impact_analysis.rs, since 72e0a76b8. It only labels the report (index_referenced_symbols, confidence = High) and never narrows or widens report.targets. It falls back only when WorkspaceIndex::load fails, not when the index is stale. The fix should build on this function: use its references to select targets, and add a staleness check.
