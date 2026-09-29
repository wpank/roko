+++
id = "gap-e2d092"
kind = "gap"
title = "Research paper: pick up S09 v1.2 and S01 v1.2 in §5, Appendices A, D and E, and fix S01 §4.5"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "cac54e574"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:03, wk-specs' report on gap-3c430e and gap-419298)"
anchors = ["tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md", "tmp/cybernetic-harness/paper/sections/A-benchmark.md", "tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md", "tmp/cybernetic-harness/paper/sections/E-reproducibility.md", "tmp/cybernetic-harness/specs/S01-instrumentation.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-419298", "gap-3c430e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'BL13' tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md && grep -q 'BL13' tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md && ! grep -q 'no S09 budget line yet' tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md && ! grep -q 'not yet registered in S09' tmp/cybernetic-harness/paper/sections/A-benchmark.md && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md tmp/cybernetic-harness/paper/sections/A-benchmark.md tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md tmp/cybernetic-harness/paper/sections/E-reproducibility.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Paper picks up S09 v1.2 and S01 v1.2 (tmp, wk-rp-appCD): §5.3 T2a names roko_plan and resolves to BL13 + exploratory.PL; §5.7 T2b adds BL13 and caps BL1 160 / BL6 44 / BL7 26 ($353.4 of $390; C5.23, C5.28); App A line 382 and A.7/A.8 add roko_plan on BL13 (CA.17 cites §4.9); App D.12 cites S09 v1.2 §4.9 with its process measures, reporting and NOT RUN rules (EX7 tag fixed to ORPHANED); App E: SHA-256 audit ids, TODO 226 resolved, note 266 trimmed, CE.18 says the specs agree; S01 §4.5/§5.3 route S06's retry budget and verify-depth floor through its harness_policy row. Verify passes (greps plus paperlint clean on 4 files; §5 1.14x)."
+++

## Problem

S09 v1.2 (gap-419298) registers the plan-level slice in §4.9: arm `roko_plan` against `fd_claude`, budget line BL13 ($5 planned, $6 cap) and lock entry `exploratory.PL`. To keep the caps at $390 it lowers BL1 to 160, BL6 to 44 and BL7 to 26. S01 v1.2 (gap-3c430e) makes the token classes disjoint and switches the audit digests to `sha256:`.

The paper hasn't picked these up (wk-specs, 2026-09-29):

- §5.3 T2a still calls the slice's arm "Unregistered", and its TODO should resolve to BL13 and `exploratory.PL`.
- §5.7 T2b needs BL13 and the three new caps. The planned total becomes $353.4 (claim C5.23), and "a line for the slice" leaves "Still open".
- App D.12 says "no S09 budget line yet". It should cite S09 §4.9 and BL13, and D's source list needs §4.9.
- App A line 382 says the slice is "not yet registered in S09".
- In App E, CE.18, the TODO at line 226 and the note at line 266 are resolved, and line 69 needs "audit records use SHA-256".

Also, S01 §4.5 still assigns the `retry_budget` and `verify_depth` decision points to S06, but S06 emits only `harness_policy`.

## Why it matters

The paper and its pre-registration must match the registered design. Epic spec-f8d196.

## Where

The files listed in `anchors`. The sources are S09 §4.9 and S01 v1.2.

## Current state

S09 and S01 are at v1.2; the paper is not.

## Plan

1. Apply each pickup listed above.
2. In S01 §4.5, point the two decision points at what S06 emits, or mark them as S01's own.
3. Keep every `[[RESULT …]]` slot, and keep each file within 1.2× budget.

## Done when

- [ ] Every pickup listed above is applied, and S01 §4.5 agrees with S06.
- [ ] The `[[verify]]` command passes.
