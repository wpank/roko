+++
id = "gap-8f07ae"
kind = "gap"
title = "Claude CLI re-pricing (backlog 6105) has never had its live acceptance: 20 Claude CLI attempts with api_equiv_usd and price_snapshot_id and a median vendor gap of at most 5%"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["roko-agent/claude_cli", "viabilitybench/pilots"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-29"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::price_at_snapshot"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-546e8a", "gap-8c2535", "gap-327242", "gap-ceffb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "set -- work/history/claude-cli-repricing-live-*.md; test -f \"$1\" && grep -Eq 'graph-(run-)?[0-9a-f]+' \"$1\" && grep -q '20 of 20 Claude CLI attempts carry api_equiv_usd and price_snapshot_id' \"$1\" && grep -q 'median vendor gap within 5%' \"$1\""
+++

## Problem

Backlog 6105 (the Claude CLI agent re-prices every model in `modelUsage` at the run's snapshot) was closed on its
unit test alone. Its second Done-when line was never checked and no item carries it: "Live acceptance, run with the
S09 pilot's Claude Code cells: 20 Claude CLI attempts all carry `api_equiv_usd` and `price_snapshot_id`, and the
median |vendor delta| is at most 5%." Status-quo assessment 2026-10-15 at 7b6a5af44, finding 29
(`tmp/status-quo/2026-10-15/backlog/A07-61-m3-self-model.md` row 6105 and lines 82-83). Every Claude CLI attempt on
record ran `claude-sonnet-4-6`, which the snapshot lacks, so none carries `api_equiv_usd` today.

## Why it matters

Goal `proof`: the paper's cost per verified task for Claude CLI arms rests on this re-pricing, and the vendor gap is
the check that roko's snapshot arithmetic matches what Claude Code itself reports.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs`: `priced_at` (about line 1901) and the vendor-gap warning (about line
  1952).
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::price_at_snapshot`: where the agent's figures reach the
  verdict.

## Current state

The code and its test (`claude_cli_agent.rs`, about line 3117) are done. Will's 2026-10-15 decisions 1 and 6 let
Pilot B (gap-327242) run with subscription arms and approve six live CLI cost probes (gap-8c2535).

## Plan

1. When the S09 pilot's Claude Code cells run (or any roko run whose attempts use the Claude CLI agent on a model the
   snapshot lists), collect 20 Claude CLI attempt verdicts.
2. Check that each carries `api_equiv_usd` and `price_snapshot_id`, and compute the median |vendor_usd -
   api_equiv_usd| / vendor_usd.
3. Write `work/history/claude-cli-repricing-live-<date>.md` with the run ids, the 20 rows, the sentence
   "20 of 20 Claude CLI attempts carry api_equiv_usd and price_snapshot_id" and "median vendor gap within 5%" when
   both hold. If they don't, file the defect instead.

## Done when

The `[[verify]]` command passes.

## Notes

No extra spend: it rides on runs Will already approved (Pilot B, the gap-8c2535 probes). If those runs use only
`claude-sonnet-4-6`, gap-ceffb3's new snapshot must land first. Finding 23's item fixes the `cost_source` label on
these verdicts.
