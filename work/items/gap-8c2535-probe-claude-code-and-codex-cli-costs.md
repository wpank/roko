+++
id = "gap-8c2535"
kind = "gap"
title = "Probe Claude Code and Codex CLI costs by hand and file the vendor-versus-snapshot table, task 6107"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
hold = "needs live claude -p and codex exec runs on Will's subscriptions: D42 re-confirmed and his approval"
subsystem = ["config/prices"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 6107 (blocked in wave 6, PK47)"
discovered_from = "gap-62e1b9"
anchors = ["config/prices/2026-09-28.toml", "benchmarks/viabilitybench/driver/ledger.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-62e1b9", "gap-154f93"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f config/prices/2026-09-28.probes.md && grep -q 'reasoning_output_tokens' config/prices/2026-09-28.probes.md && grep -q 'costBasis' config/prices/2026-09-28.probes.md"
+++

## Problem

Task 6107 of PK47 (gap-62e1b9, merged in wave 6b) files a vendor-versus-snapshot cost table,
`config/prices/2026-09-28.probes.md`: three fresh `claude -p --output-format json` runs on the subscription and three
`codex exec --json` runs under the ChatGPT login, each re-priced from the snapshot by
`benchmarks/viabilitybench/driver/ledger.py`, with the vendor figure, the snapshot figure, Δ% and `costBasis` per
run. Wave 6 made no live calls, so the table doesn't exist.

## Why it matters

Two cost facts that P1 and M3 rely on have never been observed: whether Claude Code's `total_cost_usd` matches a
snapshot re-price (and what `costBasis` says), and whether Codex's `reasoning_output_tokens` is already inside
`output_tokens` (the snapshot's `reasoning_in_output = true` rests on documentation alone). They settle the pricing
rules in 6105 and 6106 and the frontier-direct cost basis behind H1.

## Where

`config/prices/2026-09-28.toml` (the snapshot under test, immutable), `benchmarks/viabilitybench/driver/ledger.py`
(the re-pricer), and the new table beside the snapshot.

## Current state

6105 and 6106 (per-model Claude re-pricing and Codex snapshot pricing) are merged and tested against fixtures; no
probe has run. gap-154f93's saved fd_claude probe, if it exists, can serve as one of the Claude runs.

## Plan

1. Re-confirm D42 (subscription terms permit scripted runs) and get Will's approval for the six runs.
2. Run the probes as task 6107 says: fresh sessions, never `--resume`; archive the raw JSON outside the repo
   (`$VB_RESULTS`) with account and session ids scrubbed; record the spend in the S09 ledger.
3. Commit the table, with a one-line conclusion on Codex reasoning tokens. If either fact disagrees with the
   snapshot, file a task for a new dated snapshot.

## Done when

- [ ] The `[[verify]]` command passes.
- [ ] S04 §3 cites the table, in the local spec under `tmp/cybernetic-harness/specs/`.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/6107-probe-claude-code-and-codex-cli-costs-by-hand-and-file-a.md`.
- Left PK47's package item at gate 6b (2026-10-03), as 3236 left PK24's (gap-942f88).
