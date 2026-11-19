+++
id = "gap-ca983a"
kind = "gap"
title = "A held attempt's review hold gives no size for the binary files it changes (numstat -/-, 'Binary files differ'), so a person approves a 4.3 MB render blind"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/review"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-ladder#binary-outputs"
discovered_from = "agent:roko-ladder (round v7 binary-outputs test, roko c25301eef; split from gap-0a126f)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::await_review"]
links = { depends_on = ["bug-ba0c40"], blocks = [], related = ["gap-0a126f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_held_attempts_binary_files_say_their_size' crates/ && cargo test -p roko-cli --lib -- a_held_attempts_binary_files_say_their_size"
+++

## Problem

A plan with `[meta] approval = "per_task"` holds each verified attempt for a person's review
(`.roko/state/review-holds/<plan>/<task>.json`). For a binary file, the hold's `numstat` reads
`-	-	artifacts/audio/init.wav`, and its `patch` reads "Binary files a/… and b/… differ". It gives no size.

In round v7's binary-outputs test (roko c25301eef), a re-render of Undertow's 4.32 MB demo WAV and a 1.3 MB review
image showed exactly that: the reviewer approves without knowing what landed.

## Why it matters

gap-0a126f gave the run diff route the sizes (`bytes_before`, `bytes_after`). The review hold is the other place a
person decides on a change before it lands, and the Hermes plugin's review card reads it.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::await_review`: builds the hold JSON from
  `attempt_review_diff` (`crates/roko-cli/src/orchestrator/worktree/acceptance.rs`).

## Plan

1. For each numstat line that reads `-` and `-`, add `{path, bytes_before, bytes_after}` to the hold under
   `binary_files`:
   - the size at the base commit (`git cat-file -s <base>:<path>`, or null when the base lacks it);
   - the size in the checkout (or null when the file was deleted).
2. A test with a held attempt that changes a binary file.

## Done when

A held attempt's review hold names each binary file it changes, with its bytes before and after. Checked by the test
in `[[verify]]`.

## Notes

- Waits for bug-ba0c40 (claimed by work-batch 1602), which is changing `attempt_workspace.rs` and `acceptance.rs`.
- Split from gap-0a126f, which covers the run diff route.
