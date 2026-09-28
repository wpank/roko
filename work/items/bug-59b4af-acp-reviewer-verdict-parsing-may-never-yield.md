+++
id = "bug-59b4af"
kind = "bug"
title = "ACP reviewer verdict parsing may never yield a revise outcome"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp/review"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/runner.rs::parse_review_output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The ACP reviewer's answer goes through `parse_review_output` (`crates/roko-acp/src/runner.rs:1401`); the pipeline has `ReviewRevise` steps (`:1477`, `:1594`).
A local audit found the reviewer prompt and the parser disagree on the verdict format, so a "revise" answer becomes a parse failure instead of a revise round.
Confirm with a unit test that feeds a revise-shaped answer; fix by aligning prompt and parser on one verdict grammar (approve / revise with findings / reject) and testing each outcome.
