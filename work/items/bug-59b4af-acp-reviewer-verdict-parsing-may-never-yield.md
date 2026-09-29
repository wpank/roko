+++
id = "bug-59b4af"
kind = "bug"
title = "ACP reviewer verdict parsing may never yield a revise outcome"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp/review"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/runner.rs::parse_review_output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
commit = "3d0ee4d02"
by = "triage check 2026-09-28"
evidence = "Claim does not hold in current code (no recent fix; grammar has been aligned since acafe0e2e, 2026-04-25, before the audit): the prompt's REVIEW_JSON_SCHEMA (crates/roko-acp/src/runner.rs:1364-1373, status passed|failed|needs_human) matches AgentReviewPayload (crates/roko-gate/src/review_verdict.rs:163-175), whose deserialize_status also accepts approve/revise/reject (:324-331). parse_review_output (runner.rs:1401-1438) maps every non-passed result, including a fail-closed parse error, to (false, findings), and callers emit PipelineEvent::ReviewRevise (runner.rs:1477, :1594), so a revise answer does produce a revise round. No unit test for a revise-shaped answer was checked. (Static check against 3d0ee4d02; tests not re-run.)"
+++

The ACP reviewer's answer goes through `parse_review_output` (`crates/roko-acp/src/runner.rs:1401`); the pipeline has `ReviewRevise` steps (`:1477`, `:1594`).
A local audit found the reviewer prompt and the parser disagree on the verdict format, so a "revise" answer becomes a parse failure instead of a revise round.
Confirm with a unit test that feeds a revise-shaped answer; fix by aligning prompt and parser on one verdict grammar (approve / revise with findings / reject) and testing each outcome.

Checked 2026-09-28: the claim does not hold in current code (see [closed].evidence).
