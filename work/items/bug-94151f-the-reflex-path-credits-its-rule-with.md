+++
id = "bug-94151f"
kind = "bug"
title = "The reflex path credits its rule with a gate pass before any gate runs"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-learn/reflex_store"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "584abd414"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e2"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #2; §4 Remove)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-learn/src/reflex_store.rs::ReflexStore::record_gate_pass_for"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn reflex_match_records_no_gate_pass_before_verify' crates/roko-cli/src/ && cargo test -p roko-cli --lib reflex_match_records_no_gate_pass_before_verify"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Reflex path no longer credits a gate pass before any gate runs: the early record_gate_pass_for is removed and the T0 reflex path runs only with the new [learning] t0_reflexes flag (default false); test reflex_match_records_no_gate_pass_before_verify (9a4d8db5c, merged 328123077). Crediting after verify waits for gap-96f7ed (filed by wk-filer2). Batch 2 gate (work/rust-batch-2; crates tree identical to MAIN after the merges): cargo check --workspace --tests clean; nightly rustfmt clean; clippy -p roko-cli -p roko-graph -p roko-execution -p roko-core -p roko-acp -p roko-learn --no-deps -D warnings clean; lib tests roko-cli 3060 passed (8 threads; two timing tests flaked only under full parallel load and pass alone), roko-graph 460, roko-execution 252, roko-core 1913, roko-acp 196, roko-learn 1166."
+++

## Problem

When a T0 reflex rule matches, `GraphTaskDispatcher::dispatch` does four things:

1. It skips the model call.
2. It settles the budget at $0.
3. It returns the rule's cached output stamped `TaskGateVerdict::Unverified`.
4. It immediately calls `reflex_store.record_gate_pass_for(rule_id)` (`graph_task_dispatch.rs:3408`).

The comment above that call says gate feedback should be posted "after we run the verify steps", but the pass is
recorded before any verify step has run. A rule therefore gains a gate pass every time it matches, whether or not its
output would pass.

## Why it matters

Reflex rules are promoted and demoted from these counts, so a rule whose output is wrong can look reliable. That is a
false green inside a learning loop. tldr/05 lists this as P0 #2 ("remove the reflex bypass") and again in §4 (remove).
It is part of epic spec-e9d7ec (honest verdicts end to end).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: the "T0 reflex check" block in `GraphTaskDispatcher::dispatch`, about
  lines 3342–3411. The pass is recorded at line 3408.
- `crates/roko-learn/src/reflex_store.rs:353`: `ReflexStore::record_gate_pass_for`.
- The store is attached by `GraphTaskDispatcher::with_reflex_store` (`graph_task_dispatch.rs:1351`).

## Current state

Checked at `41c7ffbd6`. The task's own verdict is already honest: the reflex output is stamped `Unverified` (since
`725f21e05`), so plan success is not affected. Only the reflex store's statistics are wrong.

## Plan

1. **Now, and the default (tldr/05 §3–4, "park behind flags"):** put the reflex path behind a config flag that is off
   by default for plan dispatch, so no rule matches and nothing is credited.
2. **If reflexes are kept:** record the outcome only after the task's verify steps settle. A pass calls
   `record_gate_pass_for`; a failure demotes the rule. This uses the settled attempt outcome from epic E4 (S01.P0-1),
   so it can only follow E4.

## Done when

- [ ] No gate pass is recorded for a reflex match before the task's verify steps have run.
- [ ] A test named `reflex_match_records_no_gate_pass_before_verify` covers a matching rule whose task then fails its
      verify step.
- [ ] The `[[verify]]` command passes.

## Notes

- `graph_task_dispatch.rs` is a hot file. Start only after the portal session's branches have merged and the
  dispatch-file split (plan item E15.4) has landed.
- Do not change the `Unverified` stamp.
- 2026-09-29: Implemented on `work/bug-94151f` at `6cfc96d05`; cargo verification deferred to the batch check.
  Plan step 1: the reflex path runs only with the new `[learning] t0_reflexes` flag, off by default, and the
  premature `record_gate_pass_for` call is gone, so a rule that fires records no gate outcome (a reflex serves
  only tasks without verify steps, whose output stays `Unverified`). Step 2 (credit after verify) still waits for
  E4. `cargo check -p roko-cli --lib --tests` passed with no warnings; the test was not run.
