+++
id = "find-8599dd"
kind = "finding"
title = "Pre-Cutover Graph Host Lint Enforcement Gate"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/285-precutover-graph-host-lint-gate.md#285 — Pre-Cutover Graph Host Lint Enforcement Gate"
discovered_from = "audit:tmp/backlog/archive/285-precutover-graph-host-lint-gate.md#285 — Pre-Cutover Graph Host Lint Enforcement Gate"
anchors = ["crates/roko-cli/src/lib.rs:14"]
links = { depends_on = [], blocks = [], related = ["gap-1535e7"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "Obsolete: the Graph cutover this gate was meant to precede has landed (#260 Graph default, Runner-v2 event loop deleted in 6b5da8616, `--engine legacy` now only errors) and the manifest/test were never created. The residual risk, crate-wide clippy allows at crates/roko-cli/src/lib.rs:14, is tracked by gap-1535e7."
+++
[blocked] Blocked — workspace Clippy can report green while `roko-cli` blanket lint allows still mask defects in the new Graph host path

Imported without verification from:
- `tmp/backlog/archive/285-precutover-graph-host-lint-gate.md#285 — Pre-Cutover Graph Host Lint Enforcement Gate`

Some cited files are gone: `crates/roko-cli/tests/fixtures/graph_host_lint_manifest.toml`, `crates/roko-cli/tests/graph_host_lint_manifest.rs`, `tmp/engine-audit/RUN-LEDGER.md`, `tmp/engine-audit/SPEC-HARDENING-ADDENDUM.md`.

How to verify: Check: Create and populate the exact manifest and enforcement test above.; Include the existing engine-selector/default-routing symbols that #260 will edit and define the refresh command #260 must run if it adds or moves a selector.; Put scoped… [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: closed as superseded; see [closed].evidence.
