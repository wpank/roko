+++
id = "find-34a4b5"
kind = "finding"
title = "16 learning/routing closures wired into deleted Runner-v2 event_loop.rs"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "learning"
subsystem = ["roko-learn"]
created = 2026-09-06
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops"
discovered_from = "audit:tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:1538", "crates/roko-cli/src/knowledge_helpers.rs::apply_neuro_gate_hints", "crates/roko-cli/src/knowledge_helpers.rs::apply_neuro_gate_hints_persist", "crates/roko-cli/src/runtime_feedback/episodes.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs::update_bidders_with_cost", "crates/roko-cli/src/dispatch/factory.rs", "crates/roko-learn/src/cascade_router.rs::select_tier_with_active_inference", "crates/roko-learn/src/efficiency.rs::PromptEfficiencyScore", "crates/roko-learn/src/tool_metrics_store.rs", "crates/roko-learn/src/tool_recommendation.rs", "crates/roko-learn/src/hindsight.rs::HindsightRelabeler", "crates/roko-compose/src/attention.rs::ModelAttentionCurves"]
links = { depends_on = [], blocks = [], related = ["gap-5fb9a7", "reg-ff6e1a", "reg-c7ecf6", "q-1faa0c", "find-4b4344"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'knowledge_ids: vec!\\[\\],' crates/roko-cli/src/graph_task_dispatch.rs && { ! grep -q 'fn apply_neuro_gate_hints' crates/roko-cli/src/knowledge_helpers.rs || grep -rn 'apply_neuro_gate_hints' crates/roko-cli/src --include='*.rs' | grep -v 'knowledge_helpers.rs' | grep -q .; }"
+++

## Problem

The cybernetic audit checklist (`30-master-checklist.md`) marked 16 learning and routing feedback closures done on
2026-09-06. Every one of them was wired into `crates/roko-cli/src/runner/event_loop.rs`, and the Runner-v2 event
loop was deleted the same day. Graph plan runs (`roko plan run`) are now the only plan executor, and most of these
closures do not happen on that path. Code exists, but it has no caller, or the Graph path passes empty data into it.

Status of each closure on the Graph path, checked 2026-09-29 at HEAD `a17d9d766` (static reading):

| ID | Closure (checklist wording) | State at HEAD | Tracked by |
|---|---|---|---|
| P0-01 | Section outcome bandit ranks prompt sections | Partial. `dispatch/prompt_builder.rs` reads `SectionEffect` weights, but plan runs never update them. Graph dispatch writes efficiency events straight to JSONL (`graph_task_dispatch.rs` ~1639) and skips `roko_learn::runtime_feedback`, which is where section effectiveness is updated (`runtime_feedback/mod.rs:837`). `FeedbackService` is used by `roko run`, `roko do`, chat and serve, not by plan runs. `SectionOutcomeStore` (`section-outcomes.jsonl`) has no writer. There is no bandit. | gap-b0ebae, gap-ee03d6, gap-6c006d (all parked) |
| P0-02 | Persist EFE `BeliefState` across runs | Lost. No `efe-belief.json` anywhere. `CascadeRouter::select_tier_with_active_inference` (`cascade_router.rs:562`) has no callers. | gap-b0ebae, spec-9ba7f0 (parked) |
| P0-04 | Playbook hits feed `playbook_hit_rate` | Lost. `CompoundingMetric` no longer exists, and `playbook_hit_rate` is only computed in `roko-learn/src/aggregate.rs`. Graph dispatch sends `playbook_ids: vec![]` (`graph_task_dispatch.rs:1539`). | reg-3f5969 |
| P0-07 | Episodes record `knowledge_ids_injected` | Lost on the Graph path. `EpisodeSink` copies the ids (`runtime_feedback/episodes.rs:101`), but Graph dispatch sends `knowledge_ids: vec![]` (`graph_task_dispatch.rs:1538`), although `dispatch_plan.prompt.diagnostics.knowledge_ids` is available (`:3567`). | this item |
| P0-09 | HDC fingerprint from the real prompt and outcome | Survived. `attach_episode_hdc_fingerprint` calls `fingerprint_episode(&prompt, &outcome_text)` (`episodes.rs:168-190`), and Graph passes `prompt_text: Some(system_prompt)`. | none needed |
| P0-13 | `score_prompt_efficiency()` grade on efficiency events | Lost. The function no longer exists, and `PromptEfficiencyScore` (`roko-learn/src/efficiency.rs:473`) is never built. | none: file one |
| P1-09 | `apply_neuro_hints()` on the adaptive thresholds at start | Lost. `apply_neuro_gate_hints` (`knowledge_helpers.rs:427`) and `apply_neuro_gate_hints_persist` (`:490`) have no callers. | this item |
| P1-19 | Cost attribution feeds the prompt bidders | Lost. `update_bidders_with_cost` (`dispatch/prompt_builder.rs:1571`) has no callers. | gap-4f8af2 (parked) |
| P1-20 | `ModelAttentionCurves` drive placement | Lost. The type exists only in `roko-compose/src/attention.rs`. `prompt_builder.rs:1760` calls `dynamic_placement(&mut sections, task_query)`, which takes no model curve. No `attention-curves.json` exists. | spec-e924b1 (parked) |
| P1-22 | Format bandit select and update per dispatch | Lost. `ToolFactory` holds a static `ProfileBandit::with_static_profiles()` (`dispatch/factory.rs:241`). The `format_bandit()` accessor has no callers, and nothing selects, updates or persists. | none: file one |
| P2-15 | Tier progression events | Lost on the Graph path. | reg-06ae9f |
| P3-17 | Affect-driven reward shaping for the router | Not present. No reward-shaping code exists. Affect only enters routing selection through `daimon_policy.affect_confidence` (`cascade_router.rs:2671`). | none: file one |
| P3-32 | `HindsightRelabeler` after gate failures | Lost. It exists only in `roko-learn/src/hindsight.rs`. | gap-5fb9a7 |
| P4-04 | Automatic prompt experiments | Lost. There is no `auto_experiments` config or code, and Graph dispatch passes `prompt_experiment: None` (`graph_task_dispatch.rs:3539`). | gap-fdd27f |
| P4-17 | Persistent tool metrics (`tool-metrics.jsonl`) | Lost. `roko-learn/src/tool_metrics_store.rs` and `roko-fs` `JsonlMetricsSink` have no production users. | none: file one |
| P4-19 | Tool recommendations from efficiency history | Lost. `roko-learn/src/tool_recommendation.rs` has no callers. | none: file one |

## Why it matters

- Goal `learning` ("Learning loops on the Graph path"): these are the feedback paths that stopped when Runner-v2 was
  deleted. The checklist still says "done", so anyone reading the audit believes the loops are closed.
- Related items: `find-4b4344` (the matching finding for 9 gate and threshold closures), `reg-c7ecf6`, `reg-ff6e1a`,
  `q-1faa0c`, plus the per-closure items in the table.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: the Graph per-task dispatch. The `FeedbackEvent::TaskCompleted` with
  empty `knowledge_ids` and `playbook_ids` is built at lines 1529-1543. Direct efficiency JSONL writes are at
  ~1591-1650. Gate thresholds are loaded with `GateThresholds::load_or_default` at ~2218.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (around line 935): the Graph feedback facade. Its sinks are
  `EpisodeSink`, `RoutingObservationSink`, `DreamConsolidationSink` and `DaimonPersistenceSink`.
- `crates/roko-cli/src/runtime_feedback/episodes.rs`: the episode sink (knowledge ids and HDC fingerprint).
- `crates/roko-cli/src/knowledge_helpers.rs`: `apply_neuro_gate_hints` (line 427, `AdaptiveThresholds`) and
  `apply_neuro_gate_hints_persist` (line 490, the persist-layer thresholds, via `runner/persist.rs:378`).
- The other homes of lost code are listed in the table.

## Current state

- The table above is the current state. Since 2026-09-06 no commit has re-attached any of the lost closures to the
  Graph path.
- `section_bandit.rs`, `score_prompt_efficiency` and `CompoundingMetric`, all cited by the checklist, no longer
  exist.
- The current `[[verify]]` is broken. It is a bare `for s in ...` with no `do`/`done`, so it is a syntax error that
  can never pass.

## Plan

This finding is an umbrella. Resolve it by giving every lost closure a home, and fix the two small ones here.

1. Re-wire P0-07. In `graph_task_dispatch.rs`, pass `dispatch_plan.prompt.diagnostics.knowledge_ids.clone()` as
   `knowledge_ids` in `FeedbackEvent::TaskCompleted`, in place of `vec![]` at line 1538. Add or extend a test showing
   that the episode written through `EpisodeSink` carries the ids. Leave `playbook_ids` to `reg-3f5969`.
2. Re-wire or delete P1-09. Either call `knowledge_helpers::apply_neuro_gate_hints_persist` on the thresholds the
   Graph path loads (around `graph_task_dispatch.rs:2218`, or once per plan at start), or delete both helpers and
   `GateThresholds::apply_neuro_hints` as dead. The Graph dispatcher holds no `KnowledgeStore` today, so open one once
   per plan with `roko_neuro::KnowledgeStore::for_workdir(workdir)`, as `serve_runtime.rs:747` does. Recommended: re-wire. It is a few lines, and it is the only way knowledge feeds the gate thresholds.
   Coordinate with `find-4b4344` and `reg-c7ecf6`, which touch the same thresholds.
3. File one item for each closure marked "none: file one" (P0-13, P1-22, P3-17, P4-17, P4-19). Use
   `python3 tools/work.py new --kind gap ...` with `goal = "learning"`, anchors from the table, and a guarded verify.
   For each one the choice is re-wire or delete the module. Deleting is acceptable for P0-13, P4-17 and P4-19 if no
   one wants them. Write that down in the new item.
4. In each new item, and in the items already listed, add `find-34a4b5` to `links.related`.

## Done when

- Graph episodes carry the injected knowledge ids.
- `apply_neuro_gate_hints` is called on the Graph path, or it is deleted.
- Every "Lost" or "Not present" row has an open item, or is re-wired or deleted.
- Verify:
  `! grep -q 'knowledge_ids: vec!\[\],' crates/roko-cli/src/graph_task_dispatch.rs && { ! grep -q 'fn apply_neuro_gate_hints' crates/roko-cli/src/knowledge_helpers.rs || grep -rn 'apply_neuro_gate_hints' crates/roko-cli/src --include='*.rs' | grep -v 'knowledge_helpers.rs' | grep -q .; }`

## Notes

- Everything here is static reading. Before re-wiring, confirm the behaviour with a real `roko plan run` and inspect
  `.roko/episodes.jsonl` and `.roko/learn/`, as CLAUDE.md rule 3 says.
- Do not resurrect `runner/event_loop.rs` or copy its code. Attach to the Graph feedback facade
  (`runtime_feedback::FeedbackFacade` sinks) or to `graph_task_dispatch.rs`.
- `graph_task_dispatch.rs` is large and changes often. Keep each re-wire a small, separate commit.
- Do not edit the closed checklist in `tmp/archive/` (it is not in the repo). This item and its children are the
  record now.

## Original notes

Checked done 2026-09-06 with Files in runner/event_loop.rs, same period Runner-v2 was deleted: P0-01,P0-02,P0-04,P0-07,P0-09,P0-13,P1-09,P1-19,P1-20,P1-22,P2-15,P3-17,P3-32,P4-04,P4-17,P4-19. Wiring may not have survived Graph cutover.

Imported without verification from:
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code`

A source claims this was fixed; confirm against current code before closing.

Some cited files are gone: `section_bandit.rs/section_effect.rs`.

How to verify: For each ID, grep the named symbol for call sites under graph_execution/ or roko-graph (not runner/); check .roko/learn artifacts update after a Graph plan run.

Verified 2026-09-28: symbol scan of non-test code. Survived on live paths: section_effect (dispatch/prompt_builder.rs, roko-learn feedback_service.rs), FormatBandit (dispatch/factory.rs), active-inference BeliefState (roko-learn cascade_router.rs), apply_neuro_hints (knowledge_helpers.rs:481/544). Lost or unwired: HindsightRelabeler (only hindsight.rs; see gap-5fb9a7), ModelAttentionCurves (only roko-compose attention.rs), no efe-belief.json persistence anywhere, score_prompt_efficiency and CompoundingMetric no longer exist, and playbook_hit_rate is only computed in roko-learn aggregate.rs. Each lost closure needs its own item or a re-wire.

Checked 2026-09-29: Partly fixed in 33e107da1: knowledge, hindsight, prompt experiments and playbook credit are live on the Graph path. Still missing there: ModelAttentionCurves, EfeRouter and playbook_hit_rate, and apply_neuro_gate_hints still has no caller (the verify's second check).
