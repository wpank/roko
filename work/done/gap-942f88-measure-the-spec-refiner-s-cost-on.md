+++
id = "gap-942f88"
kind = "gap"
title = "Measure the spec refiner's cost on a live gpt-oss-120b run (within $0.02), task 3236"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/specops"]
created = 2026-10-03
updated = 2026-10-18
last_verified = 2026-10-18
last_verified_rev = "9ba428fc0"
source = "tmp/backlog/2026-10-02-complete-and-wire 3236 (blocked in wave 6, PK24)"
anchors = ["benchmarks/viabilitybench/specops"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'within the $0.02 cap: yes' benchmarks/viabilitybench/reports/refiner-cost/summary.md"

[closed]
at = 2026-10-18
at_ts = "2026-10-18T18:29:38Z"
commit = "9ba428fc0"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-18T17:10:14Z"
forced = false
model = "claude-opus-5-5"
evidence = "9ba428fc0 commits benchmarks/viabilitybench/reports/refiner-cost/ (summary.md, manifest.json, refine.jsonl, ledger.jsonl): one live `vb refine --allow-network --max-cost-usd 0.40` run, refine-3236-20261018b, 2026-10-18 18:24Z, at 8431b7f65 (clean tree), of specops/refine.py with the new live adapter driver/refine_live.py (d77feb626, prompt refine-live-1) on gpt-oss-120b via Cerebras, through the metering proxy: the key read only by the driver's key-file loader and sent only by the proxy, experiment REFINE-COST on the new budget line BL15 ($0.40 cap, decision 5, taken out of BL11's cap 18 -> 17.60 since the caps already sum to the $400 stop). 20 band C/D tasks under sq-3 (10 active plans, 10 speclint fixtures, seed 3236), 40 calls all answered, both rounds kept on every task. Cost a task (ledger): mean $0.0027, median $0.0026, max $0.0050; total $0.0546 billed on BL15, equal to the proxy's meter; summary line `within the $0.02 cap: yes`; the [[verify]] passes (tools/work.py verify). Scores rose 45.3 -> 79.0 mean, but tasks with a hard fail rose 2 -> 11 (HF4 on 10, HF2 on 1) because refine keeps a round that adds a hard fail: filed bug-d0906c. Fixed on the way: Cerebras's Cloudflare refused urllib's User-Agent (bug-7b1b7f, ba12f300e; the day's first run at d77feb626 got 20/20 403s, $0 metered, moved out of the results root once its unknown-cost rows had charged BL15's $0.40) and refused calls priced as unknown cost (8431b7f65); filed bug-2a5566 (the direct loop prices them the same way). Spend: $0.0546 billed in all (the 403 run and the User-Agent probes $0). ~/.config/viabilitybench: listing and sha256 of keys and secret unchanged before and after. Report: tmp/status-quo/2026-10-16-live/small-live-2026-10-18.md."
+++

## Problem

Task 3236 of PK24 (gap-e120a1, merged in 730b43d91) measures what the spec refiner (3235) costs on a real cheap model:
one ledgered run on gpt-oss-120b, with a committed `benchmarks/viabilitybench/reports/refiner-cost/summary.md` that
states whether the refiner stayed within its $0.02 cap. Wave 6 made no live calls, so the report doesn't exist.

## Why it matters

The refiner runs before every cheap execution in the S07 design; its cost per task has to be small and measured.

## Where

`benchmarks/viabilitybench/specops/refine.py` (3235), the cost report under `reports/refiner-cost/`.

## Current state

The refiner, critic and ambiguity probe exist and are tested against a stub model; nothing has run live.

## Plan

1. With Will's spend approval, run the refiner live on gpt-oss-120b as task 3236 says, through the metered proxy so
   the spend is ledgered.
2. Commit the summary with the measured cost and the cap check.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3236-*.md`.

2026-10-15 (Will, 2026-10-15; tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md), decision 5: one live gpt-oss-120b spec-refiner run approved, at most $0.40. Hold lifted.

## Decision (2026-10-18, Will)

Unblocked: the key file ~/.config/viabilitybench/keys was created on 2026-10-18 at Will's request (the four key lines copied from ~/.roko/.env, mode 0600, values never displayed).
