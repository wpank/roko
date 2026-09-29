+++
id = "gap-bc0640"
kind = "gap"
title = "Only one vb run can use a secret file at a time, and S09's run schedule doesn't account for it"
status = "open"
triage = "verified"
last_verified = 2026-09-29
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver", "cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix2's report on gap-308373)"
anchors = ["benchmarks/viabilitybench/driver/secret.py::tripwire", "tmp/cybernetic-harness/specs/S09-experiments.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-308373"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qi 'per secret file' tmp/cybernetic-harness/specs/S09-experiments.md"
+++

## Problem

The tripwire (gap-308373) holds an exclusive lock on `<file>.lock` next to each secret file it arms (`secret.py`:39-40). A second `vb run` on the same file stops with "is armed already: one vb run per file at a time" (:500). S09 plans runs that go on at the same time (for example H5-A0 "runs live and concurrently", and several experiment lines in the same weeks), but never mentions this limit.

## Why it matters

Pilot benchmark (epic spec-567e52): a schedule that starts two `vb run`s on one secret file fails at start, or pushes people to copy or disable the secret, which breaks the tripwire's guarantee.

## Where

`secret.tripwire`, and S09's schedule and budget sections (untracked, `tmp/`).

## Plan

Pick one:

- **(a)** S09 runs `vb run`s one at a time per secret file, and says so;
- **(b)** give each concurrent run its own secret file. That also changes the hidden cases, so the runs must not share instances.
- **(c)** make the tripwire support several runs on one file, for example by holding the lock only while a run's agents are active, or by giving each run its own armed copy.

Record the choice in S09.

## Done when

- [x] S09's schedule states how runs share secret files, and the driver matches it.
- [x] The `[[verify]]` command passes. It checks that S09 states the rule; change it if option (c) makes the rule unnecessary.

## Notes

- **Done 2026-09-29 (wk-bench-fix2): option (a), with (b) for runs that must go in parallel.** S09 v1.4 (tmp, edited in
  place) states the rule in §4.1 and clarifies H5's "concurrently":
  - One `vb run` at a time per secret file, and per key file.
  - "Interleaved" and "concurrently" mean the same window and the same `harness_sha`.
  - Parallel runs each get their own secret file, their own key-file copy and their own share of the instances. That
    instance-to-secret assignment is fixed for the campaign, by fingerprint, in the E5 runbook, so every arm and seed
    faces the same truth suite on a task. Order-dependent streams (H7) are never split.
  - Option (c) is ruled out: one run's census window would fall inside another run's agent window.
- **Driver and docs.** The README states the same rule and the parallel recipe, and `secret.tripwire`'s lock refusal
  now points to it.
- **Also caught:** parallel runs need separate key files as well, because the tripwire holds the key file at mode 000
  for a whole run (bug-979a06).
- **Verify.** It greps `tmp/…/S09-experiments.md`, which exists only in MAIN, so it was run from MAIN's root and, for
  the close, through a temporary untracked symlink in the worktree.
