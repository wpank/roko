+++
id = "gap-02a66e"
kind = "gap"
title = "Research paper: D.12 and §5.4 pick up S09 v1.3's visible-test condition for the plan-level slice"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:30, wk-bench-slice's report on gap-455aef)"
anchors = ["tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md", "tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-455aef", "gap-204848"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'no visible-check condition' tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md && ! grep -q 'no visible-check condition' tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md"
+++

## Problem

S09 is now v1.3 (gap-455aef). §4.9's verified feature (VF) now requires the base's visible tests, restored from pristine, to pass at the final commit. The census reports `vf` accordingly (gap-204848, merged 248a925d6). The paper still states the old rule: App. D.12 (`D-metrics-statistics.md`, about lines 588–601) says "There is no visible-check condition, because the arms share no visible checks", and §5.4 (`05-evaluation-protocol.md`, about line 122) agrees with it.

## Why it matters

S09 §4.9 says D.12 writes the same definitions as formulas, so the paper and the pre-registration must agree. Epic spec-f8d196.

## Where

The two section files. The source is S09 v1.3 §4.9.

## Current state

Both files state the old rule.

## Plan

1. Add the visible-test factor to VF in D.12, as a formula and in prose.
2. Align §5.4 with it.
3. Keep both files within 1.2× budget.

## Done when

- [ ] D.12 and §5.4 match S09 v1.3 §4.9.
- [ ] The `[[verify]]` command passes.
