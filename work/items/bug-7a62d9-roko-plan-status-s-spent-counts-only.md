+++
id = "bug-7a62d9"
kind = "bug"
title = "roko plan status's SPENT counts only settled tries: a try under way, whose calls have cost most of the run's spend (its latest call_cost.attempt_cost_usd, serve's live_cost_usd), is left out, so 7.2 read $0.15 against Hermes' $0.84"
status = "open"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 lead: phase 7.2's live terminal frames, the shared readings rule (design system §4.8)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::RunRecords", "crates/roko-cli/src/plan_status_view.rs::PlanStatus"]
links = { depends_on = [], blocks = [], related = ["bug-addd24", "gap-9bfb4e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_reads_like_the_glass' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_reads_like_the_glass"
+++

## Problem

`roko plan status`'s spend counts only settled tries. In round v8's phase 7.2 (Undertow
`p7-2-the-window-as-designed-3`), between 00:53 and 00:59 UTC, it read "$0.15" while Hermes read $0.84. The report
takes the checkpoint's committed spend, which moves only when a try settles. A try under way had already spent most
of the run's money. Its calls report it as they settle (`call_cost.attempt_cost_usd`, the try's spend so far, in the
run's event index), and serve counts that as `live_cost_usd`.

## Why it matters

Goal tooling. The round's shared readings rule (the design system's §4.8, approved by the lead) says SPENT is what
the run's calls have cost so far. Every surface must show the same figure, and the terminal showed a fifth of it.

## Where

`crates/roko-cli/src/commands/diagnose.rs` (the report's spend), `crates/roko-cli/src/plan_status_view.rs` (the
header).

## Plan

1. Red test first: a running plan with $0.15 committed and an open try whose latest call says $0.69 so far. Plan
   status reads $0.84.
2. While the run goes, the report adds each open try's latest `call_cost.attempt_cost_usd` (from the run's event
   index) to the committed spend.

## Done when

The test passes and the `[[verify]]` passes.
