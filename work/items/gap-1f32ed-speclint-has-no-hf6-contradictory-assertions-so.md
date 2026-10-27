+++
id = "gap-1f32ed"
kind = "gap"
title = "speclint has no HF6 (contradictory assertions), so the bench's spec.quality records and rust_parity.py would disagree with roko on a task whose tests assert one call to two values"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "M"
subsystem = ["benchmarks/speclint"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-ba2841 (worker, 2026-10-27)"
discovered_from = "gap-ba2841"
anchors = ["benchmarks/viabilitybench/speclint/speclint.py::HARD_FAILS", "crates/roko-gate/src/spec_quality.rs::HARD_FAILS"]
links = { depends_on = [], blocks = [], related = ["gap-ba2841", "bug-fe648b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '\"HF6\"' benchmarks/viabilitybench/speclint/speclint.py && test -d crates/roko-gate/tests/fixtures/speclint/hf6-contradictory-assertions"
+++

## Problem

roko's spec rules have a sixth hard fail since gap-ba2841: HF6, contradictory assertions (a task whose pinned tests
or acceptance criteria assert one call to two values no one result meets). speclint
(`benchmarks/viabilitybench/speclint/speclint.py`), the reference the Rust port is held to, has HF1–HF5 only. For a
task with such a contradiction the two now disagree: roko's `spec.quality` record lists HF6, speclint's does not,
and `rust_parity.py` would report the difference.

## Why it matters

Goal `truth`. The bench's spec-quality records and roko's should agree, or the difference should be a decision. No
plan in the repo, `tmp/` or the mori reference plans triggers HF6 today (checked over about 30,000 tasks at
gap-ba2841), so no published figure changes.

## Where

- `benchmarks/viabilitybench/speclint/speclint.py`: `HARD_FAILS` and the hard-fail checks; its docstring lists the
  linter ids.
- `crates/roko-gate/src/spec_quality.rs`: the module doc notes HF6 as roko's own; `spec_quality/contradiction.rs`
  is what to port.
- `crates/roko-gate/tests/fixtures/speclint/`: the vendored golden fixtures, one per rule and static hard fail.

## Current state

HF6 changes no rule score, band or the linter id (`sq-3`); it only adds a hard fail, and the golden fixtures never
trigger it.

## Plan

Decide first whether HF6 belongs in speclint. If it does:

1. Port the check to speclint (Python's `ast` reads the test code), and decide whether that is a new linter id
   (`sq-4`) with `--linter sq-3` kept for published figures.
2. Add a golden fixture `hf6-contradictory-assertions` in speclint and vendor it under
   `crates/roko-gate/tests/fixtures/speclint/`, and extend the parity test's list of static hard fails.

If it does not, record that decision in speclint's docstring and close this as `wontfix`.

## Done when

speclint reports HF6 for the fixture as roko does, and the `[[verify]]` passes; or the decision not to port is
recorded.
