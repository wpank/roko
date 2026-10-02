+++
id = "dec-39c781"
kind = "decision"
title = "Confirm decisions D28–D36 before the pre-registration lock"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "f37936858"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:13, next paper wave); tmp/cybernetic-harness/DECISIONS.md (D28-D36)"
anchors = ["tmp/cybernetic-harness/DECISIONS.md", "tmp/cybernetic-harness/specs/S09-experiments.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-419298", "q-ab27d3", "gap-ac4ce8"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T15:53:48Z"
commit = "f37936858"
executor = "human"
via = "manual"
forced = false
evidence = "Will confirmed D28-D36 as written on 2026-10-02 (coordinator question round, session roko-90): Holm/Bonferroni multiplicity, exploratory closure tests, BL11 ablation, H5 rho=0.15 tilted with live A0, BLAKE3 config hash, disturbance naming, cross-fitted H4 reference, H3 frontier arm without the API cross-check, spec gate by arm. Unblocks the pre-registration lock (backlog 3345)."
+++

## Problem

Wave 0 of the spec reconciliation (2026-09-28) applied nine decisions to the v1.1 specs as defaults, and
`tmp/cybernetic-harness/DECISIONS.md` marks them **unconfirmed**:

| # | Decision | Default |
|---|---|---|
| D28 | Multiplicity mapping for compound primaries | IUT, Bonferroni and gate rules; H1's level chain runs inside graphical Holm (S09 §4.1) |
| D29 | Closure tests | X1–X4 pre-registered as exploratory $0 replays, outside Holm (S09 §4.4) |
| D30 | T5 leave-one-mechanism-out ablation | Funded as budget line BL11 inside the $390 caps (S09 §4) |
| D31 | H5 primary cell | ρ = 0.15, tilted, Hájek estimator, plus a concurrent live A0 (+$9) (S09 H5, S05) |
| D32 | Config hash | BLAKE3 (`b3:`) over RFC 8785 canonical JSON, with redaction (S01) |
| D33 | Disturbance naming | `flaky_verify`, plus an H6 IUT over the 5 regulable kinds (S06, S09 H6) |
| D34 | H4 reference arm | The lowest-$/VS static arm, cross-fitted (S09 H4) |
| D35 | H3 frontier arm | The `fr_claude` probe; the 20% API cross-check is not funded (S07 §9.7) |
| D36 | Spec gate by arm | `cheap_direct` and `fd_*`: none; `roko_fixed`: off; `roko_full`: on (S07 §9.9) |

The pre-registration lock (S09.E4, criterion SC1) freezes S09 and its analysis, so each of these has to be confirmed or
changed before then.

## Why it matters

A decision changed after the lock turns a confirmatory result into an exploratory one. Several of them set the
statistics of the primary hypotheses (D28, D31, D34) or the spend (D30, D31). Epic spec-567e52.

## Where

`tmp/cybernetic-harness/DECISIONS.md` ("Added in wave 0 … defaults, unconfirmed") and the spec sections in the table.

## Current state

All nine are applied in the v1.1 specs as defaults and are unconfirmed. D37–D40 in the same table concern S11's
showcase deployment and are not part of this lock.

## Plan

1. Will confirms or changes each default. A change is applied to the named spec with a change-log line.
2. Record the answers in `DECISIONS.md`, and settle the related open choices at the same sitting: the plan-level slice
   (gap-419298) and the frontier-lite model (q-ab27d3).

## Done when

- [ ] D28–D36 are each marked confirmed or changed in `DECISIONS.md`, before the pre-registration lock.
- [ ] Any change is reflected in its spec.

## Notes

- No checklist edits: `checklist.json` stays read-only.
