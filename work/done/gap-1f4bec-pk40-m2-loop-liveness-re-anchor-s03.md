+++
id = "gap-1f4bec"
kind = "gap"
title = "PK40 M2 loop-liveness: Re-anchor S03 §3 and §7 A1 at HEAD: five census rows have moved (+8 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 40
size = "L"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "730b43d91"
source = "tmp/backlog/2026-10-02-complete-and-wire PK40"
anchors = ["crates/roko-cli/src/commands/learn.rs", "tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md"]
lane = "rust-cold"
parent = "spec-c6e21b"
links = { depends_on = ["gap-46fd19", "gap-5ddf9b", "gap-ac2611"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'retry_budget.rs' tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md && grep -q 'v1.2' tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md"

[[verify]]
command = "python3 -c \"import pathlib,sys; d=pathlib.Path('crates/roko-learn/tests/fixtures/loop_census'); need=['MANIFEST.json','episodes.jsonl','learn/cascade-router.json','learn/attention-bidders.json','learn/section-outcomes.jsonl','learn/retrieval-outcomes.jsonl','learn/efficiency.jsonl','learn/holdout-state.json','learn/experiments.json','learn/gate-thresholds.json']; sys.exit(0 if all((d/n).is_file() for n in need) else 1)\" && ! grep -rqE '\"(query|reasoning_summary|headline|reflection)\"|/Users/' crates/roko-learn/tests/fixtures/loop_census/"

[[verify]]
command = "test -f crates/roko-learn/examples/loop_census.rs && grep -rqw 'fn census_flags_known_dormant_loops' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn census_flags_known_dormant_loops"

[[verify]]
command = "grep -rqw 'fn learn_loops_census_json_matches_library' crates/roko-cli/src/commands/ && cargo test -p roko-cli --lib learn_loops_census_json_matches_library"

[[verify]]
command = "grep -rqw 'fn legacy_rule_matches_prompt_experiment' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn legacy_rule_matches_prompt_experiment && test -f crates/roko-learn/tests/fixtures/legacy_experiment_rule.json"

[[verify]]
command = "grep -rqw 'fn cs_time_uniform_coverage' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn cs_time_uniform_coverage && grep -rqw 'fn srm_evalue_controls_null_and_flags_imbalance' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn srm_evalue_controls_null_and_flags_imbalance"

[[verify]]
command = "test -f crates/roko-learn/tests/fixtures/loop_audit_cs/reference.json && grep -qw 'fn cs_widths_match_python_reference' crates/roko-learn/tests/loop_audit_cs_reference.rs && cargo test -p roko-learn --test loop_audit_cs_reference"

[[verify]]
command = "grep -rqw 'fn exposure_decomposes_into_read_reach_honest_receipt' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn exposure_decomposes_into_read_reach_honest_receipt && grep -rqw 'fn net_influence_subtracts_aa_floor' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn net_influence_subtracts_aa_floor"

[[verify]]
command = "grep -rqw 'fn aipw_unbiased_with_misspecified_model' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn aipw_unbiased_with_misspecified_model && grep -rqw 'fn cuped_reduces_variance_on_synthetic' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn cuped_reduces_variance_on_synthetic && grep -rqw 'fn increments_within_declared_bounds' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn increments_within_declared_bounds"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T02:27:44Z"
commit = "730b43d91"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T01:27:34Z"
forced = false
evidence = "Gate 6a (merged into main as 730b43d91, tree identical to work/backlog-batch-6a apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 7,078 passed (roko-agent, roko-cli, roko-learn), roko-cli bin + golden-path canaries + operator_checkout_clean 442/442, hub_ipc 7/7, roko-learn legacy_rule_live + loop_audit_cs_reference; every [[verify]] passes."
+++

## Problem

This package delivers 9 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK40, slice 51xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 5104 | S | p3 | Re-anchor S03 §3 and §7 A1 at HEAD: five census rows have moved | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5104-re-anchor-s03-current-state-and-a1-at-head.md` |
| 2 | 5105 | S | p2 | Commit a trimmed loop-census fixture cut from the 09-29 .roko snapshot | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5105-trimmed-loop-census-fixture-from-0929-snapshot.md` |
| 3 | 5106 | M | p2 | Report-only loop census (first slice) and the loop_census example | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5106-report-only-loop-census-and-example.md` |
| 4 | 5107 | S | p2 | Add `roko learn loops --census --json` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5107-roko-learn-loops-census-json.md` |
| 5 | 5109 | S | p2 | Keep a faithful copy of the legacy experiment rule for the E2 contrast | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5109-faithful-copy-of-legacy-experiment-rule.md` |
| 6 | 5110 | M | p2 | Confidence sequences with per-step bounds, and the SRM e-value | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5110-confidence-sequences-and-srm-evalue.md` |
| 7 | 5111 | S | p3 | Cross-check the Rust confidence sequences against the Python analysis toolkit | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5111-cross-check-cs-against-python-toolkit.md` |
| 8 | 5112 | M | p2 | Exposure (ε) and net-influence (ι) estimators with the A/A floor | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5112-exposure-and-net-influence-estimators.md` |
| 9 | 5113 | M | p2 | Benefit estimators: IPW difference in means, CUPED and AIPW within declared bounds | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5113-benefit-estimators-ipw-cuped-aipw-bounded.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5100-epic-m2-loop-liveness-audit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/learn.rs`, `crates/roko-learn/examples/loop_census.rs`, `crates/roko-learn/src/loop_audit/census.rs`, `crates/roko-learn/src/loop_audit/cs.rs`, `crates/roko-learn/src/loop_audit/estimators.rs`, `crates/roko-learn/src/loop_audit/exposure.rs`, `crates/roko-learn/src/loop_audit/sim.rs`, `crates/roko-learn/tests/fixtures/legacy_experiment_rule.json`, `crates/roko-learn/tests/fixtures/loop_audit_cs/reference.json`, `crates/roko-learn/tests/fixtures/loop_audit_cs/widths.json`, `crates/roko-learn/tests/fixtures/loop_census/**`, `crates/roko-learn/tests/legacy_rule_live.rs`, `crates/roko-learn/tests/loop_audit_cs_reference.rs`, `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK22 (gap-46fd19), PK27 (gap-5ddf9b), PK34 (gap-ac2611).
- Suggested model: opus.

## Progress

Implemented on `work/gap-1f4bec`; cargo verification deferred to the batch gate.

- 5104: implemented in place in the untracked `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md` (v1.3; S03 was already v1.2 from 2202, so the verify's `v1.2` still matches); no commit
- 5105: implemented at 66259e3a0 (231 KB, row counts exact)
- 5106: implemented at b78915e79 (retired L-holdout and L-rag11 print `retired` with their last declared reason)
- 5107: implemented at f8078481f (handler in the library, `commands/learn_loops.rs`, so `--lib` runs the test)
- 5109: implemented at 5806dd045 (fixture computed by a faithful Python port of today's rule; the gate's two tests confirm it against `LegacyRule` and the live `PromptExperiment`)
- 5110: implemented at 6964e5642 and 0e5ab8141
- 5111: implemented at 5e3f69fe8 (adds the Python side to `analysis/test_toolkit.py`, which passes)
- 5112: implemented at f6ac1ebf0
- 5113: implemented at ef9e78528
