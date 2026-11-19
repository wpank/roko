+++
id = "bug-0e081d"
kind = "bug"
title = "A roko-acp test's learning writes recreate its deleted temp directory: dispatch_failure_after_mark_settles_the_receipt_as_abandoned leaves .tmpXXXX/.roko (learn/, and in a module run episodes.jsonl and learn/cascade-router.json) in TMPDIR"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-acp"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "roko-recovery's TMPDIR probe of roko-acp's lib tests at c4f45d4eb (round v8, 2026-11-19)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-acp/src/bridge_events/tests.rs::dispatch_failure_after_mark_settles_the_receipt_as_abandoned"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-234204"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "d=$(mktemp -d) && TMPDIR=$d cargo test -p roko-acp --lib -- --exact bridge_events::tests::dispatch_failure_after_mark_settles_the_receipt_as_abandoned && [ -z \"$(ls -A \"$d\")\" ]"
+++

## Problem

roko-acp's `bridge_events::tests::dispatch_failure_after_mark_settles_the_receipt_as_abandoned` runs
`handle_session_prompt` with a `tempfile::tempdir()` as its workdir. The prompt's learning writes go on after the test
returns and its temp directory is deleted, and they create it again: `.tmpXXXX/.roko/learn` is left in TMPDIR. When the
whole module runs, `.roko/episodes.jsonl`, `.roko/learn/cascade-router.json` and their lock files are left too.

Seen on 2026-11-19 at main c4f45d4eb, while looking for the source of bug-234204. Each roko-acp test module ran with
an empty TMPDIR of its own, and only `bridge_events::tests` left anything. Within that module only this test did,
which reproduces alone:

```
TMPDIR=$(mktemp -d) target/debug/deps/roko_acp-<hash> --exact \
  bridge_events::tests::dispatch_failure_after_mark_settles_the_receipt_as_abandoned
```

## Why it matters

Goal tooling. Every run of roko-acp's tests leaves a roko workspace in the temp directory, which other tests can find.
It also shows that `handle_session_prompt` returns before its learning writes settle. A test that drops its directory
right away sees them land after the directory is gone.

## Where

- `crates/roko-acp/src/bridge_events/tests.rs::dispatch_failure_after_mark_settles_the_receipt_as_abandoned`.
- What `handle_session_prompt` leaves running: the episode and router writes of a failed dispatch.

## Plan

1. The test waits for the prompt's background writes before its temp directory goes, or gives them nowhere to land.
2. If `handle_session_prompt` can exit while those writes still run, note it for the ACP server's shutdown.

## Done when

The test, run alone with an empty TMPDIR, leaves nothing there, and the `[[verify]]` passes.
