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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "gaps-md#impact-analysis-uses-conservative-syntax-and-cargo-graph-evidence----partial"
anchors = ["crates/roko-cli/src/runner/impact_analysis.rs::analyze", "crates/roko-cli/src/runner/impact_analysis.rs::try_symbol_oracle", "crates/roko-cli/src/commands/impact.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn index_references_select_targets' crates/roko-cli/ && cargo test -p roko-cli --lib runner::impact_analysis::tests::index_references_select_targets"
+++

Focused verification maps diffs to Cargo targets and honours required features. It widens shared modules and compiles bounded reverse dependents for public, re-export, trait and serde edits. Pre-dispatch prompts and plan preflight flag likely scope omissions. The analysis is still text- and Cargo-graph based (`crates/roko-cli/src/runner/impact_analysis.rs`, used by `commands/impact.rs`, `runner/cargo_command.rs` and `runner/gate_dispatch.rs`), not a semantic `roko-index` reference query. It can miss macro-generated public APIs, non-Rust schema consumers and symbol-level call sites, which then need a full verification lane or operator review.

Fix: back impact scoping with `roko-index` symbol references, falling back to the current analysis when the index is stale.

Rechecked 2026-09-29 at d9e79e9d8. A roko-index reference query already exists: try_symbol_oracle in crates/roko-cli/src/runner/impact_analysis.rs, since 72e0a76b8. It only labels the report (index_referenced_symbols, confidence = High) and never narrows or widens report.targets. It falls back only when WorkspaceIndex::load fails, not when the index is stale. The fix should build on this function: use its references to select targets, and add a staleness check.

## Notes

2026-10-01 (wk-gates): blocked on a decision; no code changed.
- Staleness is not the gap. `try_symbol_oracle` calls `WorkspaceIndex::load` (roko-index `workspace.rs:521`), which
  parses the live tree on every call (`collect_source_files`), so its references are never stale. What it costs is a
  full parse per focused analysis, and what it lacks is completeness.
- Using its references to select targets means pruning reverse dependents that reference none of the public symbols
  defined in the changed files. That is unsound in cases the index cannot see: a removed or changed trait impl (a
  consumer uses the trait and the type, both defined elsewhere), glob re-exports, and macro-generated items. Pruning
  then drops a dependent that breaks, and focused verification passes it. That goes against the fail-closed rule.
- Decision for Will: may focused mode prune dependents behind guards (no trait impls, `pub use` re-exports or macros
  in the changed files, every changed file parsed, no macro or schema flags), or does the oracle stay advisory (the
  confidence label only)?
- Next step if pruning is approved: have `try_symbol_oracle` return the referencing dependents, prune
  `report.reverse_dependents` and their `Package` targets under those guards, log what was pruned, and add
  `index_references_select_targets` with a temp workspace of two crates.
