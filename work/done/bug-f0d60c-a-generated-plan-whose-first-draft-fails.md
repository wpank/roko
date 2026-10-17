+++
id = "bug-f0d60c"
kind = "bug"
title = "A generated plan whose first draft fails extraction or validation is rewritten without its source document, from 2,000 characters of the draft and a one-task template, so the source's acceptance checks are lost"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "0b5d222c7"
source = "plans-research, round v6 (tmp/hermes-roko-v6/findings/plans-research.md §3.2 and §5 item 2)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::RETRY_OUTPUT_CHARS"]
links = { depends_on = [], blocks = [], related = ["gap-f8a6ea", "bug-09cc5f", "spec-e57870"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn extraction_retry_keeps_the_source_document' crates/roko-cli/src && cargo test -p roko-cli --lib -- extraction_retry_keeps_the_source_document"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T13:27:28Z"
commit = "340f10bbf"
by = "roko-planner"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Retry prompt is the original request (source included) + the whole rejected reply (up to the planner source budget) + its errors; the one-task template naming crates/roko-core/src/lib.rs is gone; rejected replies kept as .roko/plan-drafts/<slug>/rejected-<n>.md (340f10bbf, 1641a28a7, 3623656d4; rustfmt 987030209). Verify extraction_retry_keeps_the_source_document passes in the stack run (roko-cli lib 3772+ passed, 0 failed). Live (round v6, binary 468e0ba52): 08g with gpt-oss-120b - the first reply held no plan, was kept as rejected-1.md, and the retry with the source wrote 7 tasks that keep 8 of 8 checks (before: 0 of 8 from the template retry)."
+++

## Problem

`roko plan generate --from-file` loses the source document's acceptance checks: in plans-research's three live runs
(`tmp/hermes-roko-v6/findings/plans-research.md` §3.2) it kept 0 of 8, 0 of 8 and 4 of 14 of them.

The main cause is the extraction/validation retry. Every first draft in those runs was rejected for a mechanical
defect. The retry prompt then holds only the error, the first 2,000 characters of the rejected draft
(`RETRY_OUTPUT_CHARS`), and a "MINIMUM REQUIRED STRUCTURE" template: one task with `total = 1` and
`files = ["crates/roko-core/src/lib.rs"]`. **The source document is not in it.** So the planner rebuilt a smaller plan
from the template and dropped the rest. One planner renamed `golem-sonification` to `roko-sonification`. Rejected
drafts are not saved either, so nobody can look at what was lost.

## Why it matters

The pitch is a PRD or task list with verifiable acceptance criteria. A plan that drops the criteria cannot show that
what gets built passes them (goal `golden-path`).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the extraction/validation retry loop and its prompt.
- `crates/roko-cli/src/plan_generate/pipeline.rs::RETRY_OUTPUT_CHARS`.

## Plan

1. Build the retry prompt from the original planner prompt (which carries the source document), then the rejected
   draft in full (bounded generously), then the errors. Ask the planner to fix only what the errors name and to keep
   every task and every verify step.
2. Drop the one-task template, or replace it with structure guidance that names no file.
3. Keep each rejected draft with its error under `.roko/plan-drafts/<slug>/`.

## Done when

`[[verify]]` passes: a retry's prompt holds the source document and the whole rejected draft, never the template's
`crates/roko-core/src/lib.rs`, and the rejected draft is kept.
