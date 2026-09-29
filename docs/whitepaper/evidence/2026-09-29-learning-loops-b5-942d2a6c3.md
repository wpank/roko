# The 16 Runner-v2 learning loops, re-checked at `942d2a6c3`

- **What:** a re-check of the 16 learning loops that Runner-v2 had, on the Graph path at `942d2a6c3` (2026-09-29,
  after gap-8f6206 merged in `74eaf5c8f`). Written by gap-4471d1 with research note B5's method and vocabulary: each
  row's anchors read at `942d2a6c3` with `git grep`, and `git log 98ee1418f..942d2a6c3` on its files. Code was read
  only; nothing was built or run.
- **Earlier states:** the loop list, the Runner-v2 behaviour, the statuses at `d9e79e9d8` and the first re-check at
  `98ee1418f` are in `learning-loops-B5.md`. Rows keep B5's numbers and names.
- **Result:** 2 WIRED, 5 PARTIAL, 7 ORPHANED, 1 BROKEN and 1 BUILT-UNWIRED, the same tags as at `98ee1418f` (0/8/7/0/1
  at `d9e79e9d8`). The merges since `98ee1418f` changed the evidence behind rows 1, 5, 7, 9, 10, 12 and 14 but moved no
  tag.
- **The main change:** since `04c1da262` (gap-8f6206, bug-c34782) every learner reads only the settled verdict's
  learning label: 1 for a pass, 0 for a failure of the agent's work, and none for an unverified, force-accepted,
  provider-failed or harness-failed attempt, which updates no learner.
- **Source:** this file is the record. The tldr note B5 (`tmp/cybernetic-harness/tldr/research/B5-learning-loops-v2-vs-graph.md`,
  gitignored) carries only the new counts.

| # | Loop | Status at `942d2a6c3` | Evidence at `942d2a6c3`, and what changed since `98ee1418f` |
|---|---|---|---|
| 1 | Playbook credit | WIRED@942d2a6c3 | W07 credits the playbooks a prompt injected with the attempt's learning label, and an attempt without one credits none (`graph_task_dispatch/feedback.rs`; `04c1da262`), so an unverified attempt no longer counts as a pass. A floor still puts three playbooks into every prompt |
| 2 | Post-gate lessons → retry prompt | PARTIAL@942d2a6c3 | Unchanged: on a gate failure with `replan_on_gate_failure` set, `graph_task_dispatch/verification.rs` still writes a lesson to `PostGateReflectionStore`. Its only reader is `roko learn` (`commands/learn.rs`); `graph_task_dispatch/retry_feedback.rs` does not read it |
| 3 | Rung EMA → retry budget | WIRED@942d2a6c3 | Unchanged: `graph_task_dispatch/retry_budget.rs` takes `suggested_max_retries(rung)` for tasks that author no `max_retries` |
| 4 | Discovered error patterns → dispatch | ORPHANED@942d2a6c3 | Unchanged: the plan runner loads `error-patterns.json` through `with_error_patterns_from_disk`, but its only writer, `ErrorPatternSink`, is built only by `build_settler` (`graph_execution/feedback.rs`), which only tests call |
| 5 | Router learns from outcomes | PARTIAL@942d2a6c3 | The routing sink learns only from the learning label (`04c1da262`; bug-c34782 closed), router-chosen failures update LinUCB like successes (`51ed5719c`; bug-8da8ba closed), and concurrent saves merge through a journaled WAL (`7769a5ae4`, `c48c6a2ad`). Still partial: after a provider failover the router credits the substitute model as its own pick (bug-35379d, open) |
| 6 | Calibration, verdict history, cost-spike anomaly | ORPHANED@942d2a6c3 | Unchanged: `run_learning_subscriber` is called only by its own tests |
| 7 | Knowledge injection, write-back and tiers | BROKEN@942d2a6c3 | Write-back now takes passes only (`04c1da262`). Unchanged: the plan-run prompt cache still loads the store with an empty query, so no plan-run prompt carries knowledge (bug-86117a, open; `dispatch/prompt_cache.rs` has no commit since `98ee1418f`). Knowledge-aware routing is still test-only |
| 8 | Section outcomes → section choice | ORPHANED@942d2a6c3 | Unchanged: the section-effectiveness registry is read at prompt assembly, and the prompt builder's `record_outcome` has only a test caller |
| 9 | Prompt experiments | PARTIAL@942d2a6c3 | Settlement follows the learning label, and an attempt without one abandons its treatments (`04c1da262`). Unchanged: arms are assigned by UCB1 score and the winner is declared by a chi-squared test, which adaptive assignment invalidates (`roko-learn/src/prompt_experiment.rs` has no commit since `98ee1418f`) |
| 10 | Affect → dispatch | PARTIAL@942d2a6c3 | The affect state appraises only labelled outcomes (`04c1da262`), and the routing context still shifts the tier when the state is Struggling. `modulate_dispatch` is implemented in `roko-daimon` but has no caller |
| 11 | Gate-failure cross-cut cascade | ORPHANED@942d2a6c3 | Unchanged: only `arbitrate_cross_cut_routing_bias` survives, as a routing bias |
| 12 | Offline consolidation after plans (B5's CL-24) | ORPHANED@942d2a6c3 | Unchanged: no production code emits `FeedbackEvent::PlanCompleted`, so the plan-completion sink never runs (q-6b7cca, open). Since `1f312c78c` consolidation on completion is also off by default, and since `c482329be` so is the ACP episode-count trigger |
| 13 | Conductor → interventions | ORPHANED@942d2a6c3 | Unchanged: every call to `evaluate_full` is in tests (`runner/conductor_adapter.rs`, after its `#[cfg(test)]`) |
| 14 | T0 reflex promotion | ORPHANED@942d2a6c3 | The reader no longer credits a rule with a gate pass that no gate gave, and it consults rules only when `[learning] t0_reflexes` is on, which it is not by default (`6cfc96d05`; bug-94151f closed). `try_promote` still has only test callers, so nothing writes rules |
| 15 | Gate-failure replan | PARTIAL@942d2a6c3 | Unchanged: a retry carries the raw gate feedback (`graph_task_dispatch/retry_feedback.rs`); no replan function exists, and `replan_candidate` is always false |
| 16 | Similar-episode recall | BUILT-UNWIRED@942d2a6c3 | Unchanged: fingerprints are written, and `query_similar_episodes` is called only by its own test |

Paths are under `crates/roko-cli/src/` unless they name a crate. `graph_task_dispatch.rs` was split into the
`graph_task_dispatch/` modules in `a729fb911` without a change in behaviour, so B5's GTD line anchors no longer apply.
