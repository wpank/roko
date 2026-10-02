+++
id = "gap-8b8051"
kind = "gap"
title = "Make PRD Commands Honor One Execution and Validation Contract"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/backlog/archive/303-prd-cli-consistency-and-integrity.md#303 — Make PRD Commands Honor One Execution and Validation Contract"
discovered_from = "audit:tmp/backlog/archive/303-prd-cli-consistency-and-integrity.md#303 — Make PRD Commands Honor One Execution and Validation Contract"
anchors = ["commands/prd.rs", "prd.rs", "tasks.toml", ".roko/prd/consolidation/<timestamp>.md", "model_selection::resolve_effective_model_key", "ArtifactKind", "ValidationIssue", "ArtifactValidationReport"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:20Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded: the PRD commands were removed on 2026-10-02 (merge bfd36512f)."
+++
[blocked] Blocked on #262, #280, and #283 — The PRD command family is functional but each subcommand applies a different contract:

Imported without verification from:
- `tmp/backlog/archive/303-prd-cli-consistency-and-integrity.md#303 — Make PRD Commands Honor One Execution and Validation Contract`

Some cited files are gone: `.roko/prd/consolidation/<timestamp>.md`.

How to verify: Check whether the gap described in tmp/backlog/archive/303-prd-cli-consistency-and-integrity.md still exists at the anchored paths. [evidence: own status: Blocked on #262, #280, and #283]

Triage note 2026-09-28: #303 appears largely landed despite its archived Blocked status - crates/roko-cli/src/commands/prd.rs exists (the import warning was wrong) and defines ArtifactKind/ValidationIssue/ArtifactValidationReport (prd.rs:50-63), and resolve_effective_model_key is used at commands/prd.rs:377/700/822/862; the remaining acceptance bullets (--json/--quiet, draft-edit validation, consolidation artifact, status-count parsing) were not checked.
