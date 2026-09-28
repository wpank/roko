+++
id = "bug-5b897b"
kind = "bug"
title = "DOCS-06 A4: `knowledge sync --direction` unvalidated; can corrupt version vectors"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/knowledge-sync"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A4. `knowledge sync` can corrupt version vectors"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A4. `knowledge sync` can corrupt version vectors"
anchors = ["crates/roko-cli/src/main.rs:1283", "crates/roko-cli/src/commands/knowledge.rs:761"]
links = { depends_on = [], blocks = [], related = ["gap-d6b2eb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "direction: KnowledgeSyncDirection" crates/roko-cli/src/main.rs'

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "--direction is a clap value_enum (KnowledgeSyncDirection: send|receive|both, default both) at crates/roko-cli/src/main.rs:1281-1283, so invalid values are rejected at parse time before any store access; commands/knowledge.rs:761-765 maps the enum to send/receive flags (committed). Version-vector regression tests remain tracked by gap-d6b2eb"
+++
Invalid --direction values are not validated and can corrupt knowledge sync version-vector state (decision: add input validation). 09-03 claims vector updates made conditional; 09-15 still lists it open.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A4. `knowledge sync` can corrupt version vectors`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#6. CLI Audit (`tmp/cli-audit/`, 34 files)`
- `tmp/cli-audit/SUMMARY.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: Run `roko knowledge sync <peer> --direction bogus` against a temp store. / Find the cli-audit detail; review version-vector merge in knowledge sync.

Merged 2 mined candidates: m4-034, m5-142.

Verified 2026-09-28: fixed - main.rs:1283 `direction: KnowledgeSyncDirection` (value_enum).
