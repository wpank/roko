+++
id = "spec-6ac537"
kind = "spec"
title = "Epic: cybernetic core"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "L"
subsystem = ["roko-learn/cascade-router", "roko-cli/runtime-feedback", "roko-cli/graph-dispatch", "roko-serve/state"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e17"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P2 #17–22); workstreams/assessment/W3a-crosswalk-core.md"
anchors = ["crates/roko-learn/src/cascade_router.rs::CascadeRouter::observe_multi_objective", "crates/roko-cli/src/runtime_feedback/routing.rs::RoutingObservationSink", "crates/roko-cli/src/graph_execution/feedback.rs::RoutingSink", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["bug-f68404", "bug-8da8ba", "bug-9c88ac", "bug-012303", "bug-605a8a", "find-0dc1d5", "reg-3f5969", "gap-fdd27f", "gap-644040", "bug-dfb28f", "bug-3ea1f5", "bug-84de98", "bug-7a2630", "bug-8b0d0a"], blocks = [], related = ["spec-b7303f", "gap-96f7ed", "gap-8cb382", "gap-c8e1f1", "dec-e70592", "gap-25065c", "bug-cfe0be"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn failed_override_lowers_success_rate' crates/roko-learn/ && grep -rqw 'fn gate_pass_increments_selected_playbook' crates/roko-cli/src/ && grep -rqw 'fn frozen_learning_run_writes_no_learned_state' crates/roko-cli/src/ && cargo test -p roko-learn failed_override_lowers_success_rate && cargo test -p roko-cli --lib gate_pass_increments_selected_playbook && cargo test -p roko-cli --lib frozen_learning_run_writes_no_learned_state"
+++

## Problem

On the Graph path the learning loops learn from wrong labels, lose what they learn, or cannot be held still:

- **Router labels.** `observe_multi_objective` adds a trial and a success for every observation, so a failed
  `--model` override counts as a success (bug-f68404), and router-chosen failures never reach LinUCB (bug-8da8ba).
  B5 found 270 router observations but a single LinUCB observation.
- **Router state.** Saves are last-writer-wins across processes (bug-9c88ac); serve keeps router instances it never
  saves (bug-012303); a narrower or reordered model list drops or mismatches state (bug-605a8a); Path B observations
  skip the write-ahead log (find-0dc1d5).
- **Failure memory.** Playbook outcomes are credited to a synthetic `task-<id>` (reg-3f5969); prompt experiments are
  never assigned (gap-fdd27f); the error-pattern reader loads a file nothing writes (B5).
- **No frozen mode.** Benchmarks and A/B runs cannot hold learned state fixed (gap-644040).

## Why it matters

Goal `cybernetic` (tldr/05 P2 #17–22). The thesis says feedback loops make Roko improve measurably. That can be tested
only if the loops learn from verified outcomes (S02), if an audit can show each loop's exposure, influence and benefit
(M2, S03), and if a benchmark can switch learning off (the pilot, epic E12). M1, M3 and M4 build on the same records.

## Where

- `crates/roko-learn/src/`: `cascade_router.rs` (`observe_multi_objective`, `record_override_outcome`, `save`,
  `load_or_new`), `cascade/persistence.rs`, `model_router.rs`, `model_call_feedback.rs`.
- `crates/roko-cli/src/`: `runtime_feedback/routing.rs` (`RoutingObservationSink`), `graph_execution/feedback.rs`
  (`RoutingSink`, `PlaybookSink`), `graph_task_dispatch.rs`, `dispatch/prompt_builder.rs`.
- `crates/roko-serve/src/` (`state.rs`, `service_factory.rs`, `dispatch.rs`) and `crates/roko-gateway/src/gateway.rs`.

## Current state

Checked at `41c7ffbd6`. All nine children are open, and a static check confirms each premise:
- `cascade_router.rs:1718-1719` still increments `trials` and `successes` unconditionally, and
  `record_override_outcome` (:1540) forwards to it.
- The Graph `RoutingSink` records router outcomes only through `record_confidence_outcome` (`feedback.rs:391`).
- `cascade_router.rs` has no locked save, and serve's gateway still builds `CascadeRouter::new(gateway_models)`
  (`roko-serve/src/state.rs:1042`).
- `graph_task_dispatch.rs` still has `prompt_experiment: None` at :3666 and :4416 and `playbook_ids: vec![]` at :1634;
  synthetic `task-{}` ids remain at `graph_task_dispatch.rs:1830` and `feedback.rs:469`.

**`ce3bdcbb8` (verify-path learning loops, merged today) fixed none of them.** It keeps retry feedback across resume,
lets adaptive gate thresholds set retry budgets (find-4b4344, P3-15) and names the failing rung. It touched no router,
playbook, prompt-experiment or serve code.

**In flight:** the portal session's `feat/learning-completion-loops` worktree (`roko-wt-learn-a`, uncommitted) rewrites
the playbook-id and prompt-experiment code for reg-3f5969 and gap-fdd27f. Don't start those two until it merges or is
dropped.

**Premise notes (no edits made):**
- bug-8da8ba has no `[[verify]]`; S02.P1-1 names its test `routing_sink_updates_linucb_on_failure`.
- bug-9c88ac, bug-012303 and find-0dc1d5 name tests that don't exist yet, with no `grep … fn` guard, so each verify
  can pass without its test (`check --lint` warns). find-0dc1d5's `grep -qi 'wal'` also matches any word containing
  "wal". Guard them when each is picked up.
- bug-012303 is narrower than its title: its 09-29 note says the unsaved routers are the gateway's and
  `service_factory.rs:245`'s.

## Plan

This is the implementation plan.

1. **Router labels** (bug-f68404 and bug-8da8ba share a root; one change, S02.P1-1): `observe_multi_objective`
   honours the outcome, overrides become an importance weight, and failures reach LinUCB with reward 0 from both
   sinks. The `cascade_router.rs` half is cold and can start now. The `graph_execution/feedback.rs` half is hot:
   land it after the dispatch split (gap-c8e1f1), and read E4's settled verdict (gap-96f7ed) rather than the
   provider's success flag.
2. **Router persistence, cold, serial on `cascade_router.rs`:** bug-605a8a (import arms by slug), bug-9c88ac (locked
   read-merge-write; reuse `roko_fs::with_locked_json_transaction` rather than adding S02's `LockedFold`), then
   find-0dc1d5 (journal Path B before applying).
3. **One router in serve, hot:** bug-012303 (S02.P1-4b), after step 2.
4. **Failure memory, hot:** reg-3f5969 and gap-fdd27f once the learn-a branch settles; then one error-pattern store
   written on verify failure (S02.P1-13, not yet an item).
5. **Frozen learning** (gap-644040), recorded in E4's run manifest (gap-8cb382); needed before the pilot's Roko arm.
6. **After E4 (spec-b7303f) lands:** adopt the M1–M4 items outlined below.

**Outline of the items to adopt once E4 lands.** gap-25065c imports these checklist rows as unverified items with
`parent = "spec-6ac537"`; the E17 owner reviews them rather than writing new files (re-anchor, split to S/M, write
verify commands that read E4's attempt record, set `triage = "verified"`, add them to `depends_on`).

| Stream | Cold, can start early | Hot, after E4 and the dispatch split |
|---|---|---|
| S02 loop re-closure | P1-6 decay, P1-12 thresholds observe-only | P1-2, P1-3, P1-5, P1-7, P1-8, P1-10, P1-13, P1-14, P1-15, then the P1-16 census |
| M2 loop audit (S03) | T1–T10 `loop_audit` library, census first | T11–T15; T16 last |
| M4 deep audits (S05) | tasks 1–3 Python slice (after E12's S08 slice), 4–5, 16 | 6–10, 12–15, 17 |
| M3 self-model (S04) | T01 price loader, T03–T07 offline library and replay | T07b–T14, shadow first |
| M1 controller (S06) | T1–T11 `homeostasis` library and shadow replay | T12–T16 |
| Guarded commit (tldr #21) | S06.T4 safety box, S06.T11 last-known-good | — |

## Done when

- [ ] bug-f68404: Manual --model overrides are always recorded as router successes (existing item)
- [x] bug-8da8ba: Router-chosen failures never update the LinUCB model (existing item)
- [x] bug-9c88ac: roko serve overwrites router state learned by concurrent CLI runs (existing item)
- [x] bug-012303: Serve dispatch paths build CascadeRouter instances whose learning is never saved (existing item)
- [x] bug-605a8a: Loading the router with a narrower model list drops or misaligns other models' state (existing item)
- [x] find-0dc1d5: Path B cascade observations not covered by WAL (write-ahead log) (existing item)
- [x] reg-3f5969: Graph dispatch drops selected playbook IDs; outcomes are recorded under synthetic task IDs (existing item)
- [x] gap-fdd27f: Prompt experiments are never assigned on the Graph execution path (existing item)
- [ ] gap-644040: No way to run with learning frozen: prompts and routing change from run to run (existing item)
- [ ] bug-dfb28f: Graph runs save the cascade router only when the run ends, so a crash loses the run's routing learning
- [ ] bug-3ea1f5: LearningRuntime rewards a failed attempt with up to 0.5 through cost and latency, unlike every other router path
- [x] bug-84de98: LearningRuntime::open replays a running writer's unsaved model-call observations, which that writer later saves again
- [x] bug-7a2630: WAL replay drops entries for models the router doesn't track and then truncates wal.jsonl, and serve never truncates it
- [x] bug-8b0d0a: One gateway call can be observed up to three times on serve's shared cascade router
- [ ] The epic's `[[verify]]` command passes on the merged branch: one named test each for router labels, failure
      memory and frozen learning. Extend it with S02.P1-16's wiring census when the M1–M4 items join.

## Notes

- **Existing children keep goal `learning` and severity p2.** PLAN.md §5 proposes moving them to `cybernetic`
  (not approved).
- **Hot files:** `graph_task_dispatch.rs`, `graph_execution/feedback.rs`, `plan_runner.rs`, `dispatch/`, serve
  `state.rs` and `dispatch.rs`. The hot halves wait for the portal session's branches and gap-c8e1f1.
- **Proposed split of bug-012303** (S02 sizes it L): (a) the gateway's router loads from and saves to
  `cascade-router.json`; (b) one `Arc<CascadeRouter>` in `AppState`, shared by dispatch, feedback and the gateway.
- **Needs Will:** tldr/05 P2 #18 says "park LinUCB", and no decision item covers that (dec-e70592 covers only the
  three LinUCB tuning items). If LinUCB is parked, bug-8da8ba's LinUCB half is moot and its fix shrinks to honest
  confidence counts.
- **Related:** bug-6f1685 (parked) duplicates bug-8da8ba; bug-cfe0be (provider health, goal `core`) needs the same
  locked merge-save as bug-9c88ac.
- **Still open (not accepted on 2026-09-29):** parking LinUCB behind a flag (tldr/05 P2 #18).
