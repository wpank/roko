+++
id = "gap-618548"
kind = "gap"
title = "M4's audit verdicts never reach M1's E3: audited and audit_false_green are set only in a test fixture"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/audit", "roko-learn/homeostasis", "roko-cli/runtime_feedback"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-12"
anchors = ["crates/roko-cli/src/audit/labels.rs::record_label", "crates/roko-cli/src/audit/labels.rs::VsLearner", "crates/roko-learn/src/homeostasis/resolution.rs::TaskResolution", "crates/roko-learn/src/homeostasis/ev.rs", "crates/roko-cli/src/runtime_feedback/homeostasis.rs::HomeostasisSink"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = ["gap-cad57a"], blocks = [], related = ["gap-d76b1b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn audit_verdict_reaches_m1_false_green' crates/roko-cli/ && cargo test -p roko-cli audit_verdict_reaches_m1_false_green"
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
