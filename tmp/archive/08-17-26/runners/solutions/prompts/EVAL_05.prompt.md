# EVAL_05: Define `EvalTrace` and JSONL storage

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-05`](../ISSUE-TRACKER.md#eval-05)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.5
- Priority: **P0**
- Effort: 5 hours
- Depends on: `EVAL_01` (source 5.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EvalTrace` is the durable record of every evaluation run. It parallels the existing `Episode` from `crates/roko-learn/src/episode_logger.rs` (agent turn accounting) but captures full evaluation detail: evidence, per-criterion scores, findings, pipeline context, cost. Cross-referenced by `task_id` and timestamp.

## Exact Changes

1. Define `EvalTrace`:
   ```rust
   pub struct EvalTrace {
       pub id: String,
       pub timestamp: DateTime<Utc>,
       pub artifact: ArtifactRef,
       pub profile_id: String,
       pub evidence_phase: Vec<CollectorPhaseRecord>,
       pub criterion_results: Vec<CriterionResult>,
       pub verdict: EvalVerdict,
       pub pipeline_context: PipelineContext,
       pub cost: EvalCost,
       pub duration_ms: u64,
       pub task_id: Option<String>,
       pub plan_id: Option<String>,
   }
   ```
2. Define `PipelineContext` (populated from `AgentEfficiencyEvent` fields at trace emission time):
   ```rust
   pub struct PipelineContext {
       pub model: String,
       pub backend: String,
       pub prompt_variant: Option<String>,
       pub agent_role: String,
       pub generation_cost_usd: f64,
       pub generation_tokens: u64,
   }
   ```
3. Define `EvalCost`:
   ```rust
   pub struct EvalCost {
       pub total_usd: f64,
       pub evidence_usd: f64,
       pub criteria_usd: f64,
       pub judge_usd: f64,
   }
   ```
4. Define `CollectorPhaseRecord`:
   ```rust
   pub struct CollectorPhaseRecord {
       pub collector_name: String,
       pub evidence_kinds: Vec<EvidenceKind>,
       pub duration_ms: u64,
       pub success: bool,
       pub error: Option<String>,
   }
   ```
5. Define `TraceStore` with JSONL persistence at `.roko/eval/traces.jsonl`:
   - `append(trace: &EvalTrace) -> Result<(), EvalError>`: append one line of JSON. Crash-tolerant: write + sync.
   - `recent(limit: usize) -> Result<Vec<EvalTrace>, EvalError>`: read last N traces. Tolerant of malformed trailing lines (skip, do not error).
   - `by_id(id: &str) -> Result<Option<EvalTrace>, EvalError>`: scan for specific trace.
   - `by_task(task_id: &str) -> Result<Vec<EvalTrace>, EvalError>`: filter by task_id.

## Design Guidance

Follow the exact persistence pattern from `EpisodeLogger` at `crates/roko-learn/src/episode_logger.rs`: append-only JSONL, malformed-line tolerance, `parking_lot::Mutex` for concurrent writers. Use `tokio::fs` for async I/O. The `recent()` method reads the entire file and returns the last N entries -- acceptable for MVP (traces accumulate slowly).

## Write Scope

- `crates/roko-eval/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Write/read round-trip test
- [ ] Malformed line tolerance test (corrupt last line, still read preceding lines)
- [ ] `by_task()` filter test

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Write/read round-trip test
- Malformed line tolerance test (corrupt last line, still read preceding lines)
- `by_task()` filter test
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
