+++
id = "gap-618548"
kind = "gap"
title = "M4's audit verdicts never reach M1's E3: audited and audit_false_green are set only in a test fixture"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/audit", "roko-learn/homeostasis", "roko-cli/runtime_feedback"]
created = 2026-10-15
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "8b725b2c9"
source = "status-quo-2026-10-15#finding-12"
anchors = ["crates/roko-cli/src/audit/labels.rs::record_label", "crates/roko-cli/src/audit/labels.rs::VsLearner", "crates/roko-learn/src/homeostasis/resolution.rs::TaskResolution", "crates/roko-learn/src/homeostasis/ev.rs", "crates/roko-cli/src/runtime_feedback/homeostasis.rs::HomeostasisSink"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = ["gap-cad57a"], blocks = [], related = ["gap-d76b1b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn audit_verdict_reaches_m1_false_green' crates/roko-cli/ && cargo test -p roko-cli audit_verdict_reaches_m1_false_green"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T01:20:36Z"
commit = "8b725b2c9"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-16T19:19:29Z"
forced = false
evidence = "8b725b2c9 (work/gap-618548, branched from main at ce67e0cd2): a run's audited VS labels teach every learner through labels::VsLearners (a fan-out with no short circuit; the self-model's risk still tilts the lottery), and open_attempt opens the lottery with the self-model and M1's sink (impl VsLearner for HomeostasisSink: VS 0 is a false green). Labels land after the pass resolved, so they reach E3 through Controller::on_audit, keyed by the pass's settlement id and counted once; ControllerState::audits (serde default) keeps E3's last 30 audits across runs and a loaded controller starts E3 from them; no episode or band changes. HomeostasisSink::audit_label counts only an attempt that passed (noted as it settles, else read from its run's attempts.jsonl for a label that lands first or in a later run), skips holdout and all-off passes and green non-passes, and re-saves the state for a label that lands after the run ended; HomeostasisSink::false_green and `roko learn homeostasis status` report E3. Verify `cargo test -p roko-cli audit_verdict_reaches_m1_false_green` passed on the committed tree (1 passed; all 76 roko-cli test binaries clean). Also passed: `cargo test -p roko-cli -- a_planted audit_ homeostasis vs_label self_model` (58 passed, among them a_planted_false_green_reaches_m1_e3_on_a_run, a real dispatch whose planted cfg!(test) pass the run's audit worker labels as gaming and M1's E3 then holds as one false green, learn_homeostasis_status_reports_e3_from_saved_audits, and the unchanged M3 path audit_labels_correct_the_self_model_with_ipw_weights); `cargo test -p roko-learn --lib homeostasis::` (40 passed, audits_reach_e3_when_their_labels_land_and_carry_across_runs among them); clippy -p roko-learn -p roko-cli --no-deps -D warnings and nightly fmt --check clean. E3 stays unmeasured on default runs until audits are on (gap-d76b1b)."
+++

## Problem

M1's E3 (false-green rate, the Beta(1, 9) posterior over the last 30 audits, `crates/roko-learn/src/homeostasis/
ev.rs:10`, `:43`) never receives an audit. `TaskResolution.audited` and `audit_false_green`
(`crates/roko-learn/src/homeostasis/resolution.rs:101-105`) are set only by a test fixture (`ev.rs:973-974`); both folds
write `false`/`None` (`resolution.rs:276-277`, `:473-474`). M4's verdicts reach M3 through
`crates/roko-cli/src/audit/labels.rs::record_label` and the `VsLearner` trait (implemented only for
`SelfModelRuntime`), but nothing passes them to M1. M4 receives M1's boosts (`graph_task_dispatch/audit_select.rs`) and
sends nothing back.

## Why it matters

Goal `cybernetic` (S06 §4.2, §4.10; paper 9-36 "the audits are the false-green variable's only sensor", D5). NEW-FINDINGS
finding 12; A09 §3 gap 2. Without E3, M1 regulates pass rate, cost and latency with no Goodhart guard: a move that raises
the gate pass rate by letting false greens through looks like a win. This must be in place before audits go on
(gap-d76b1b) and long before M1 `on`. Stage 3 of the epic.

## Where

- `crates/roko-cli/src/audit/labels.rs::record_label` and the `VsLearner` trait (one learner today).
- `crates/roko-learn/src/homeostasis/resolution.rs::TaskResolution` (`audited`, `audit_false_green`).
- `crates/roko-learn/src/homeostasis/ev.rs` (E3) and the controller's saved state.
- `crates/roko-cli/src/runtime_feedback/homeostasis.rs::HomeostasisSink`.
- `crates/roko-cli/src/graph_task_dispatch/audit_select.rs` (the M1 to M4 direction, already wired).

## Current state

M4's worker, labels and ledger exist (slice 71); audits are off by default (gap-d76b1b). A VS label can arrive after the
run that produced the pass has ended (the audit worker closes windows later).

## Plan

1. Let `record_label` notify every learner, not one (a slice, or a fan-out `VsLearner`), and implement the learner for
   M1: a label for a pass that stood marks that chain's resolution `audited`, with `audit_false_green = !vs`.
2. Labels that arrive after the run: append them to M1's state (by settlement id) so the next run's controller folds
   them into E3's window; never rewrite closed episodes.
3. Tests `audit_verdict_reaches_m1_false_green` (a VS label of false on a passed chain moves E3's posterior and marks it
   measured) and a late-label case.

## Done when

The `[[verify]]` command passes, and `roko learn homeostasis status` shows E3 `measured` on a workspace with audit
labels.

## Notes

Depends on gap-cad57a (M1 must run). The M3 path through `VsLearner` must keep working unchanged.
