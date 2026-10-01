+++
id = "gap-6e9e14"
kind = "gap"
title = "Research paper appendices A and B: re-pin as-built tags to the merged benchmark and spec-quality code"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "0cccf6be0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (23:10, wk-rp-appCD's report on gap-7ee870)"
anchors = ["tmp/cybernetic-harness/paper/sections/A-benchmark.md", "tmp/cybernetic-harness/paper/sections/B-spec-standard.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-7ee870", "gap-856053"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE '@(4c0326dfc|7bdc61da3)' tmp/cybernetic-harness/paper/sections/A-benchmark.md tmp/cybernetic-harness/paper/sections/B-spec-standard.md && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/A-benchmark.md tmp/cybernetic-harness/paper/sections/B-spec-standard.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Appendices A and B re-pinned in place (tmp, wk-rp-appAB): all 45 status tags at 0cf9bacfe, none left at 4c0326dfc/7bdc61da3, each claim checked against the tree: driver, three built arms, ledger, proxy, secret handling and F1/F4 WIRED on vb run's path; verifier CI, plan-slice fixtures, proxy fault profiles and speclint BUILT-UNWIRED; the Rust spec-quality port PARTIAL (AU3, parity 551/551); red-on-base BUILT-UNWIRED; [task.accept] WIRED (AU7); speclint re-run at 0cf9bacfe (B.8 unchanged except 1,084 verify steps); S09 v1.3/v1.4 reflected. Verify passes; CLAIMS-EVIDENCE 432 claims (--check up to date)."
+++

## Problem

Appendices A (benchmark) and B (spec standard) carry as-built tags pinned at `@4c0326dfc` and `@7bdc61da3`. Those commits predate much that has since merged: the ViabilityBench driver, families F1 and F4, the plan-level slice fixtures, verifier CI, the report and ledger, the metering proxy, both arms, the secret handling, `plan validate --spec-quality` (gap-46ab3f) and the red-on-base checker (gap-b3fa0a). wk-rp-appCD flagged this on 2026-09-29.

## Why it matters

The appendices must describe the artifacts as they exist at the pin they state. Epic spec-f8d196.

## Where

`A-benchmark.md` and `B-spec-standard.md`.

## Current state

The tags describe the tree before the benchmark existed.

## Plan

1. Re-pin the as-built tags at the current working-branch head. Check each claim in the tree.
2. Keep `[[AS-BUILT]]` on anything still designed.
3. Keep both files within budget, and regenerate CLAIMS-EVIDENCE.

## Done when

- [ ] No as-built tag in A or B is pinned before the benchmark merges.
- [ ] The `[[verify]]` command passes.
