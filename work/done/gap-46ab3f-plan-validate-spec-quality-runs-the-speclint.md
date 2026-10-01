+++
id = "gap-46ab3f"
kind = "gap"
title = "plan validate --spec-quality runs the speclint rules when a plan loads (S07.9)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-gate/spec_quality", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "abc655b5e"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e8"
discovered_from = "tmp/cybernetic-harness/execution/checklist.json (S07.9, with the S07.7 scorer)"
anchors = ["crates/roko-gate/src/spec_quality.rs", "crates/roko-cli/src/commands/plan.rs::cmd_plan_validate", "crates/roko-cli/src/main.rs:1988"]
lane = "rust-hot"
parent = "spec-e57870"
links = { depends_on = ["gap-1cd8d3"], blocks = [], related = ["find-70edcb", "gap-b3fa0a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_quality_matches_speclint_golden_fixtures' crates/roko-gate/src/ && cargo test -p roko-gate --lib spec_quality_matches_speclint_golden_fixtures"

[[verify]]
command = "grep -rqw 'fn spec_quality_flag_reports_scores_and_hard_fails' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate spec_quality_flag_reports_scores_and_hard_fails"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "roko_gate::spec_quality ports speclint sq-1 (SQ01-SQ12, HF1-HF5; SQ06/HF3 unknown in static mode); plan validate --spec-quality prints per-task score/band/rules/hard fails (text and --json), exit 1 only on hard fails under --strict, output unchanged without the flag; golden fixtures vendored with the parity test; rust_parity.py --strict over 132 files and 551 tasks: max score delta 0.00, 0 hard-fail/band/rule/detail differences (d0cad4760 + clippy fix 24be02c53; merged 31bc6ba3d). Batch 7 gate (work/rust-batch-5 tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean after fmt-only 15d3eb5f2; clippy -p roko-cli -p roko-learn -p roko-gate -p roko-agent -p roko-std -p roko-core --no-deps -D warnings clean; lib tests (8 threads) roko-cli 3091, roko-agent 2252 (after test fix f7ad8de76), roko-core 1923, roko-learn 1178, roko-std 221, roko-gate 685 (one pre-existing flaky test, tautology_filter_discards_preexisting_passing_tests, failed once and passed on rerun)."
+++

## Problem

Once gap-1cd8d3 lands, the spec-quality rules exist only as a Python tool. Authors run `roko plan validate`, whose
flags are `--strict`, `--json` and `--dag` (`main.rs:1988`). It has no scoring path, so a plan with vacuous or
grep-only verify steps validates cleanly.

## Why it matters

The spec gate belongs where plans are authored and loaded (S07 §4.3). Only here do find-70edcb's weak-verify
patterns become lint results. tldr/05 P1 #10. Part of epic spec-e57870.

## Where

- **New file:** `crates/roko-gate/src/spec_quality.rs`: the S07.7 scorer (SQS v1: SQ01–SQ12, HF1–HF5, bands,
  lexicon v1, verify classes). It scores a neutral task view, not `TaskDef`. `roko-cli` already depends on
  `roko-gate`, so no new crate edge is needed.
- `crates/roko-cli/src/main.rs:1988`: `PlanCmd::Validate` gains `--spec-quality`.
- `crates/roko-cli/src/commands/plan.rs::cmd_plan_validate` and `plan_validate.rs`: print each task's score, band,
  rule scores and hard fails, and add them to `--json`.
- `crates/roko-cli/tests/plan_validate.rs` (existing): the CLI test.

## Current state

Checked at `41c7ffbd6`: no scorer in Rust and no flag. The checklist entry (AS4, 09-28) says the same.

## Plan

1. Port speclint's static rules (gap-1cd8d3) into `roko_gate::spec_quality`. SQ06 and HF3 stay `unknown` in static
   mode.
2. Parity test `spec_quality_matches_speclint_golden_fixtures`: the same scores (±0.5) and identical hard fails on
   speclint's golden fixtures. If D4 moves the benchmark out of this repo, vendor the fixtures under
   `crates/roko-gate/tests/fixtures/`.
3. Add `--spec-quality` to `plan validate`, in text and JSON. Exit 1 only on hard fails under `--strict`. Output
   without the flag stays byte-identical to today's.
4. Compare against speclint's JSONL over all `plans/**/tasks.toml`, and report the result in the commit message.

## Done when

- [ ] `roko plan validate plans/ --spec-quality --json` matches speclint within ±0.5 points per task, with
      identical hard fails.
- [ ] Without the flag, the output is unchanged.
- [ ] Both `[[verify]]` commands pass.

## Notes

- "When a plan loads" means `plan validate`'s load, which uses the same parser as `plan run`. The runner's load-time
  gate in `graph_execution/plan_runner.rs` (S07.11, advise then enforce) and `--dynamic` (red-on-base, after
  gap-b3fa0a) are later items.
- `main.rs` is a hot file, but the change is one flag.
- If porting the scorer takes more than a day, split S07.7 (the library) into its own item.
- Implemented on `work/gap-46ab3f` at `d0cad4760`; cargo verification deferred to the batch check.
- 2026-09-29: premise re-checked at `7f2796462` (no Rust scorer, no flag). `--spec-quality` scores the files
  `plan validate` lints, so `archive/` and `archived/` plans are skipped as in the rest of validate;
  `benchmarks/viabilitybench/speclint/rust_parity.py` runs `plan validate` on `plans/` and on each archive
  directory and runs speclint on the same files. The Rust record leaves out speclint's `spec_hash`, `spec_origin`,
  `critic`, `ambiguity` and `ts`.
- Batch 6 fixes at `24be02c53`: clippy (`-D warnings`) and nightly rustfmt are clean for roko-gate and roko-cli;
  the 14 `spec_quality` lib tests and the 19 `plan_validate` CLI tests pass, including both verify tests. Plan
  step 4 with that build: `rust_parity.py --strict plans` compared 132 files and 551 tasks, with max |score
  delta| 0.00, 0 hard-fail, band, rule, verify-class or detail differences, and the vendored fixtures in sync. The
  same 551 records match speclint's whole-corpus JSONL exactly.
