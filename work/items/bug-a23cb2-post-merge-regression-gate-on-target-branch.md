+++
id = "bug-a23cb2"
kind = "bug"
title = "Post-Merge Regression Gate on Target Branch"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/194-post-merge-regression.md#194 — Post-Merge Regression Gate on Target Branch"
discovered_from = "audit:tmp/backlog/archive/194-post-merge-regression.md#194 — Post-Merge Regression Gate on Target Branch"
anchors = ["merge.rs:121", "merge.rs:343", "merge.rs:350", "crates/roko-cli/src/runner/merge.rs", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-core/src/config/schema.rs", "PlanMerger", "backlog #404", "RUNG_MERGE"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
merged plans can silently break compilation when cross-plan changes conflict on the target branch; currently only the merge branch itself is checked. `PlanMerger` in `crates/roko-cli/src/runner/merge.rs` already runs a post-merge regression gate via the `RegressionGate` trait (default…

Imported without verification from:
- `tmp/backlog/archive/194-post-merge-regression.md#194 — Post-Merge Regression Gate on Target Branch`
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-07: Implement batch branch`
- `tmp/archive/plan-audit-2026-09-23/12-merge-batch-branch.md`
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: After a successful merge, `cargo check --workspace` runs against the target branch; If the target-branch check fails, the merge is marked as failed; A regression event is emitted with diagnostic output from the failed check [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 5 |] / grep for batch branch creation (roko/batch). / grep RUNG_MERGE usages.

Merged 3 mined candidates: m1-048, m3-121, m3-149.
