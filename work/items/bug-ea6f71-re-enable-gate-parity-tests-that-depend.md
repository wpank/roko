+++
id = "bug-ea6f71"
kind = "bug"
title = "Re-enable gate parity tests that depend on 'uncommitted gate_adapter.rs' changes"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/gates"]
created = 2026-09-05
updated = 2026-09-28
source = "crates/roko-cli/tests/gate_parity.rs:669"
discovered_from = "audit:crates/roko-cli/tests/gate_parity.rs:669"
anchors = ["crates/roko-cli/tests/gate_parity.rs::graph_cell_matches_fixture_expectations", "crates/roko-cli/tests/gate_parity.rs::runner_and_graph_verdicts_converge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Two gate parity tests (graph cell vs fixtures; runner vs graph verdict convergence) are ignored citing uncommitted gate_adapter.rs changes from the engine-convergence branch, although convergence has landed (#260/#276). Runner/Graph gate verdict parity is unproven.

Imported without verification from:
- `crates/roko-cli/tests/gate_parity.rs:669`
- `crates/roko-cli/tests/gate_parity.rs:745`

How to verify: cargo test -p roko-cli --test gate_parity -- --ignored; locate gate_adapter.rs on main.
