+++
id = "bug-3f40d2"
kind = "bug"
title = "A retry's diagnosis can name a file that does not exist: after T5's failed check in the 2026-10-19 golden-path run 2, the cheap model's diagnosis blamed crates/alpha/src/initials.rs line 42, and the next prompt led with it"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-19
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "gap-f30b8e golden-path rerun review, live run 2 (2026-10-19)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs::GateFeedback"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_diagnosis_naming_a_missing_file_is_not_led_with' crates/roko-cli/src && cargo test -p roko-cli --lib a_diagnosis_naming_a_missing_file_is_not_led_with"
+++

## Problem

After a failed verify step, roko asks a cheap helper model for a short diagnosis, and the next attempt's prompt
leads with it (`## Diagnosis`). In the 2026-10-19 golden-path live run 2 (`graph-golden-path-7f133a7f-…`,
`4535b3914`), T5's first attempt failed its accept test (`initials` upper-cased ASCII only: "éZ", "ßO"), and the
diagnosis left for the retry blamed `crates/alpha/src/initials.rs` line 42, a file that does not exist: the
attempt had patched only `crates/alpha/src/lib.rs` (`.roko/events.jsonl:450`, `tool_audit.jsonl:143` in the
run's workspace). The retry passed anyway, from the raw test output.

## Why it matters

The diagnosis is the one part of the retry feedback that is always rendered, and it comes first. A diagnosis
that names a file outside the task's `files`, or one that does not exist, sends a cheap model to the wrong
place, and it costs a helper call per failed check (8 a run on 2026-10-18).

## Where

- The diagnosis call after a failed verify: `crates/roko-cli/src/graph_task_dispatch/verification.rs`
  (`enriched_diagnosis`, from `roko_learn::error_enrichment::enrich_error_digest` on the run's cheap agent),
  given the raw failure text and the task's title, and nothing of the attempt's tree.
- Where it lands: `GateFeedback::with_diagnosis` (`crates/roko-cli/src/dispatch/prompt_builder.rs`).

## Plan

1. Ground the diagnosis: check that each path it names exists in the attempt's tree (or is in the task's
   `files`), and drop or mark a diagnosis that names a missing file.
2. A test with a scripted helper that names a missing file: the next prompt does not lead with it.

## Done when

- [ ] The `[[verify]]` test passes.

Checked 2026-10-27: 2026-10-26 sweep: still open. Four commits touched verification.rs since filing (best-tree judging, hand-off continuation, retry ranking) but none added grounding to enrich_error_digest/normalize_diagnosis.
