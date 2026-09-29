+++
id = "bug-019f02"
kind = "bug"
title = "speclint and roko_gate::spec_quality ignore [task.accept], so plans that pin acceptance tests lose verify steps and acceptance credit"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-gate/spec_quality", "benchmarks/viabilitybench/speclint"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-appAB's report)"
anchors = ["crates/roko-gate/src/spec_quality.rs", "benchmarks/viabilitybench/speclint/speclint.py"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-d14a43", "gap-46ab3f", "gap-ba4d01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_accept_counts_as_verify_steps_and_acceptance' crates/roko-gate/src/ && cargo test -p roko-gate --lib task_accept_counts_as_verify_steps_and_acceptance"
+++

## Problem

gap-d14a43 added `[task.accept]`: planner-written acceptance tests, pinned by hash and compiled into verify steps at load. The spec-quality linters don't know it:

- the Python `speclint/speclint.py`;
- its Rust port, `roko_gate::spec_quality` (`crates/roko-gate/src/spec_quality.rs`), which credits acceptance only from the `acceptance` field (`acceptance_criteria`, :1034).

Neither file handles an `accept` table. A plan that pins its acceptance tests, as `plans/portal-programme/08f-final-polish` already does, is scored as if those tasks had fewer verify steps and no acceptance criteria.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): the linter penalises exactly the plans that follow the recommended practice, and `plan validate --spec-quality` (gap-46ab3f) would report them as weaker.

## Where

`spec_quality.rs` (verify-step and acceptance scoring) and `speclint.py` (the same rules). The two must stay in parity (`speclint/rust_parity.py`).

## Plan

1. Count each `[task.accept]` test as a verify step, and as acceptance credit, in both linters.
2. Add a parity fixture with an accept plan.
3. Add `task_accept_counts_as_verify_steps_and_acceptance`.

## Done when

- [ ] An accept plan scores at least as well as the same plan with hand-copied tests.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-019f02` at `033d0646f`; cargo verification deferred to the batch check.
- 2026-09-30: an accept entry counts when it names `src`, `dest` and `runner` and has `count > 0` (what the
  loader turns into a step). It scores as a scoped test-class step listed after the task's own steps, and
  as an observable criterion. Fixture `accept-tests`: the accept task scores 52, the same test copied by
  hand 37. Corpus delta: only `08f-final-polish` changes; T06 goes from 51.67 (C) to 66.67 (B). The
  linter id stays `sq-1`.
- Not done: `speclint/dynamic.py` still runs only authored verify steps on the base, so in `--dynamic`
  mode an accept task's SQ06 and HF3 ignore its pinned tests.
