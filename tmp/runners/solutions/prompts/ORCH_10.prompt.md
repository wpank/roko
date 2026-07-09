# ORCH_10: Define EffectDriver Service Traits for orchestrate.rs Features

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-10`](../ISSUE-TRACKER.md#orch-10)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.10
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

orchestrate.rs implements features that belong in dedicated service traits, not a 22K-line monolith. The feature extraction plan (ORCH-002) calls for six service trait families. This task defines the traits; subsequent tasks implement them.

The traits should follow the existing pattern in `foundation.rs`: `ModelCaller`, `PromptAssembler`, `FeedbackSink`, `GateRunner`, `AffectPolicy` -- all `Send + Sync + 'static` with async methods returning `roko_core::Result<T>`.

The highest-impact features to extract (from the PLAN priority analysis):
1. Knowledge routing (`build_knowledge_routing_advice()` in orchestrate.rs)
2. Episode recording (`EpisodeLogger` in roko-learn)
3. Playbook queries (`PlaybookStore` in roko-learn)
4. Error pattern queries (`ErrorPatternStore` in roko-learn)

## Exact Changes

1. Add to `crates/roko-core/src/foundation.rs`:
   ```rust
   /// Knowledge routing service -- queries durable knowledge store for task context.
   #[async_trait]
   pub trait KnowledgeRouter: Send + Sync {
       async fn route(&self, task_description: &str, role: &str) -> Result<Vec<String>>;
   }

   /// Episode recording service -- records agent turns and gate results.
   #[async_trait]
   pub trait EpisodeRecorder: Send + Sync {
       async fn record_turn(&self, run_id: &str, role: &str, model: &str, tokens: u64, cost: f64) -> Result<()>;
       async fn record_gate(&self, run_id: &str, gate_name: &str, passed: bool) -> Result<()>;
       async fn finalize(&self, run_id: &str, succeeded: bool) -> Result<()>;
   }

   /// Error pattern query service.
   #[async_trait]
   pub trait ErrorPatternQuery: Send + Sync {
       async fn match_error(&self, gate_output: &str) -> Result<Option<String>>;
   }
   ```
2. Add optional service fields to `EffectServices`:
   ```rust
   pub knowledge_router: Option<Arc<dyn KnowledgeRouter>>,
   pub episode_recorder: Option<Arc<dyn EpisodeRecorder>>,
   pub error_pattern_query: Option<Arc<dyn ErrorPatternQuery>>,
   ```
3. Update all call sites constructing `EffectServices` to pass `None` for the new fields.

## Design Guidance

Keep traits minimal. Each trait should have 2-4 methods maximum. Use `Option<Arc<dyn Trait>>` so the EffectDriver degrades gracefully when a service is not available. Do NOT try to port the implementations in this task -- just define the contracts.

## Write Scope

- `crates/roko-core/src/foundation.rs`
- `crates/roko-runtime/src/effect_driver.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Traits defined in `foundation.rs` with documented contracts
- [ ] `EffectServices` has optional fields for each new trait
- [ ] All existing code compiles with `None` for new fields

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Traits defined in `foundation.rs` with documented contracts
- `EffectServices` has optional fields for each new trait
- All existing code compiles with `None` for new fields
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
