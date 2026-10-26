+++
id = "gap-0a126f"
kind = "gap"
title = "A try's binary files show no size anywhere: the review hold and the run diff give a 4.3 MB WAV as binary +0/-0 with no bytes, so a person approving Undertow's renders can't tell them from a 200 MB mistake"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/runs"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "7b29eb18a"
source = "round-v7-roko-ladder#binary-outputs"
discovered_from = "agent:roko-ladder (round v7 binary-outputs test, roko c25301eef)"
anchors = ["crates/roko-serve/src/routes/run_result.rs"]
links = { depends_on = [], blocks = [], related = ["bug-ba0c40", "gap-ca983a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_binary_file_in_a_run_diff_says_its_size' crates/ && cargo test -p roko-serve --lib -- a_binary_file_in_a_run_diff_says_its_size"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T15:29:25Z"
commit = "7b29eb18a"
by = "roko-ladder"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T14:40:13Z"
forced = false
evidence = "7b29eb18a: GET /api/runs/{id}/diff gives each binary file bytes_before and bytes_after (one git --literal-pathspecs ls-tree -r -l -z per side, run_result.rs blob_sizes; null on the side it is missing from), since a binary file counts no lines; text files unchanged. The review hold's sizes are split into gap-ca983a (filed here, depends on bug-ba0c40, whose files it would touch). Test a_binary_file_in_a_run_diff_says_its_size (modified WAV 1000->2000 bytes, an added PNG with [ ] and a space in its name, a deleted binary, a text file without sizes): red with the sizes switched off (left (Null, Null), right (1000, 2000)), green now. Checks: roko-serve lib 1070 passed plus its 15 integration targets; clippy -D warnings (roko-serve) and nightly rustfmt clean. Found by the round v7 binary-outputs test (3 PNGs 1.30-1.85 MB, a 4.32 MB WAV through a plan run: binary +0/-0 everywhere)."
+++

## Problem

Round v7's $0 binary-outputs test (roko c25301eef) committed Undertow-sized artifacts through a plan run:
- three review images, `artifacts/ui/editor-{idle,playing,clipping}.png` (1200 x 760; 1.30, 1.64 and 1.85 MB);
- a demo render, `artifacts/audio/init.wav` (48 kHz, 24-bit stereo, 15 s; 4.32 MB).

Every place a person or a host looks at the change shows those 9.1 MB as nothing:
- The review hold of a `[meta] approval = "per_task"` plan (`.roko/state/review-holds/<plan>/<task>.json`): its
  `numstat` reads `-	-	artifacts/audio/init.wav` and its `patch` reads "Binary files a/… and b/… differ". No size.
- `GET /api/runs/{id}/diff` (a serve run that re-rendered the WAV): `binary: true`, `additions: 0`,
  `deletions: 0`, `patch: null`. No size.
- The attempt record (`attempts.jsonl`): nothing about the change's size (`diff_sha256: null`). `max_loc` is only a
  plan-generation limit, so no run-time count covers binaries.

A person approving phase 5's four 4.3 MB WAVs, or a 200 MB render left by mistake, sees the same thing: a binary
file and +0/-0.

## Why it matters

Phases 4 and 5 of Undertow commit binary outputs on purpose (3 PNGs under 2 MB, 4 WAVs of about 4.3 MB, by the
specs' own checks). Each re-render stays in git history: the test's `.git` grew 7.19 MB for the first set, and
about 3.3 MB for each later WAV re-render. A reviewer needs the bytes to judge a binary change. A host (the Hermes
plugin's page) needs them to show it.

Related: bug-ba0c40 (acceptance commits undeclared untracked build output). If its fix only commits declared paths,
the artifacts must be in their tasks' `files`, as they were in this test.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::await_review`: writes the review hold from
  `attempt_review_diff`'s numstat and text patch (`orchestrator/worktree/acceptance.rs`). bug-ba0c40 is changing that
  file, so the sizes are worked out in `await_review`.
- `crates/roko-serve/src/routes/run_result.rs`: the diff route's per-file entries. Binary detection comes from
  numstat's `-` and `-`.

## Plan

1. The diff route's binary entries carry their size before and after (`git cat-file -s <tree>:<path>`), e.g.
   `bytes_before`, `bytes_after`.
2. The review hold gets the same in gap-ca983a. Its files (`attempt_workspace.rs`, `acceptance.rs`) are being
   changed by bug-ba0c40 (claimed by work-batch 1602), so that part waits for it.
3. Optional: the run's events say when a try adds more than a threshold of binary bytes, so a page can flag it.

## Done when

The diff route gives a binary file's bytes before and after. Checked by the test in `[[verify]]`. The review hold's
sizes are gap-ca983a, which depends on bug-ba0c40.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/roko-ladder.md` ("Binary outputs"). The harness is
  `binary_e2e.py` and `binary_followup.py` in the roko-ladder scratch, $0.
- Nothing in the test read binary contents into a prompt, a record or an event (see the findings). This item is
  only about the size being invisible.
