+++
id = "gap-987510"
kind = "gap"
title = "Research paper appendices C and D: align the audit protocol and metrics with S05, S09 and the ViabilityBench analysis"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "5a9e07a26"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wave-1 reports)"
anchors = ["tmp/cybernetic-harness/paper/sections/C-audit-protocol.md", "tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/C-audit-protocol.md tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Appendices C and D (tmp, wk-rp-appCD) aligned with S09 v1.1 and §5: D.11/D.12 use S09's decided Holm mapping and name the plan-level slice RQ3 (outcomes PL, no S09 budget line yet); metric definitions match §5; budget lines BL0-BL11; provisional defaults D28-D31, D33, D34, D9, D10, D13, D5/D6 named where used (claims CD.19, CD.21, CC.1, CC.21 carry '(default)'). Old Table 9.1 dropped: its M1-M10 mapping is inline in §9; the verbatim table is archived at tmp/cybernetic-harness/paper/archive/table-9.1-removed-from-09-limitations.md. paperlint --budget 1.2 --check-identifiers clean on C and D; §5.6's stale D.11 TODO removed; CLAIMS-EVIDENCE regenerated (417 claims, --check up to date)."
+++

## Problem

Appendices C (the audit protocol) and D (metrics and statistics) predate the v1.1 specs' final wording and the
2026-09-29 decisions: the plan-level slice, and pass^k with its seed count. Old Table 9.1, cut from §9, is saved at
`/private/tmp/claude-501/-Users-will-dev-nunchi-roko-roko/7622b882-2d61-4b7f-83a7-1e4212db277e/scratchpad/table-9.1-removed-from-09-limitations.md`
and needs a home, if any appendix wants it.

## Why it matters

These appendices are the analysis plan the pre-registration will point to. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/C-audit-protocol.md` and `tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md`. Sources: `tmp/cybernetic-harness/specs/S05-*.md` and
`S09-*.md`, and §5 (gap-ac4ce8).

## Current state

Both are written as designed.

## Plan

1. **Align** the metric definitions (verified success, cost per verified success, pass^k, false greens, makespan for
   the plan-level slice) with S09 v1.1 and §5.
2. **Table 9.1:** place it, or record why it was dropped.
3. **Trim:** keep both appendices within 1.2× budget.

## Done when

- [ ] The definitions are consistent with §5 and S09.
- [ ] The `[[verify]]` command passes.

## Notes

- These files are untracked, so edit them in place in the main checkout. Edit only this item's anchored files.
- Keep every `[[RESULT …]]` slot, and use the marker grammar and the `observational` claim type in
  `tmp/cybernetic-harness/paper/00-README.md`.
- The working title is "A Cybernetic Harness for Spec'd Agent Work: Frontier Models Plan, Cheap Models Execute"; the
  thesis is golden path plus cybernetics. Use the status tags in `docs/whitepaper/data/mechanisms.toml`.
