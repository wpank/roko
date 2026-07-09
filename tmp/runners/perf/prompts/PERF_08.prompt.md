# PERF_08: Parallel per-dispatch enrichment (B07) — with caveats

## Task

Replace serial `await` chains for **independent** per-dispatch
enrichment IO with `tokio::join!`. **EXPLICITLY do not parallelise**
the 13-step `EnrichmentPipeline::run_steps` (its steps consume each
other's output files). Add a doc comment to that function that locks
the sequential invariant.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_08](../ISSUE-TRACKER.md#perf_08)
- Plan: `tmp/solutions/perf/implementation/08-parallel-enrichment.md`
- Bottleneck: B07 (BOTTLENECK-ANALYSIS.md §B07)
- Performance contract: **C-8** (parallel dispatch-time enrichment;
  sequential plan enrichment)
- Priority: P2
- Effort: ≈3 h
- Depends on: none
- Wave: 1

## Problem

The `BOTTLENECK-ANALYSIS.md` §B07 entry recommends parallelising
enrichment with `tokio::join!`. **That recommendation is correct ONLY
for per-dispatch enrichment**, not for the 13-step plan-enrichment
pipeline.

Verified evidence the plan-enrichment steps are NOT independent
(`crates/roko-compose/src/enrichment/pipeline.rs:397-403`):

```rust
pub async fn run_steps(&self, plan_base: &str, steps: &[EnrichStep]) -> Vec<StepOutcome> {
    let mut outcomes = Vec::with_capacity(steps.len());
    for &step in steps {
        outcomes.push(self.run_step(step, plan_base).await);
    }
    outcomes
}
```

…and the doc-comment two lines up:

> Steps are ordered by dependency: earlier steps produce artifacts
> consumed by later steps.

`crates/roko-compose/src/enrichment/inputs.rs` shows each step reading
the output files of preceding steps. Naive `tokio::join!` would race on
those files and produce empty/incoherent prompts.

The genuine wins live in **per-dispatch enrichment** in the
orchestrator: file intel, knowledge query, recent-episodes lookup,
playbook query, research-store fetch — all independent reads.

## Exact Changes

### Step 1 — Locate the per-dispatch enrichment join sites

```bash
rg -n 'enrich_' crates/roko-cli/src/orchestrate.rs
rg -n '\.await' crates/roko-cli/src/orchestrate.rs | rg -B 1 -A 1 'enrich'
```

Look for `async fn enrich_task_context_with_search` (~line 2343) or any
function that contains a sequence like:

```rust
let file_intel = enrich_file_intel(workdir, task).await?;
let knowledge = enrich_knowledge(&knowledge_store, task).await?;
let recent_eps = enrich_recent_episodes(&episodes_path).await?;
let playbook = enrich_playbook(&playbook_store, task).await?;
let research = enrich_research(&research_store, task).await.ok();
```

If you find the pattern (with any subset of those enrichers), proceed
to Step 2. If you do NOT find such a pattern, the orchestrator may
already be parallelising these calls (or not have them at all). In
that case, document your finding in the commit body:

```text
audit: searched orchestrate.rs for serial dispatch-time enricher
awaits; found none. The IO inside enrich_task_context_with_search is
already parallel (or not applicable to this codebase snapshot).
Marking the per-dispatch parallelisation as N/A; only the sequential
guard on EnrichmentPipeline::run_steps was applied.
```

…and skip Step 2.

### Step 2 — Convert serial awaits to `tokio::join!`

```rust
// BEFORE:
let file_intel = enrich_file_intel(workdir, task).await?;
let knowledge = enrich_knowledge(&knowledge_store, task).await?;
let recent_eps = enrich_recent_episodes(&episodes_path).await?;
let playbook = enrich_playbook(&playbook_store, task).await?;
let research = enrich_research(&research_store, task).await.ok();
```

```rust
// AFTER:
//
// Per-dispatch enrichment: independent reads against different stores.
// `tokio::join!` runs all branches concurrently on the current task.
//
// Long-tail enrichers (research store, network-backed) are wrapped in
// tokio::time::timeout so a slow research backend does not gate the
// whole dispatch on its tail latency.
let (file_intel, knowledge, recent_eps, playbook, research) = tokio::join!(
    enrich_file_intel(workdir, task),
    enrich_knowledge(&knowledge_store, task),
    enrich_recent_episodes(&episodes_path),
    enrich_playbook(&playbook_store, task),
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        enrich_research(&research_store, task),
    ),
);
let file_intel = file_intel?;
let knowledge = knowledge?;
let recent_eps = recent_eps?;
let playbook = playbook?;
let research = research.ok().and_then(Result::ok);
```

> **Why `tokio::join!` and NOT `futures::join_all`?** Fixed-arity joins
> use the macro (no allocation). Reserve `join_all` for runtime-dynamic
> arities (e.g., looping over an arbitrary list of enrichers).

> **Error semantics.** Sequential awaits short-circuit on the first
> error; `tokio::join!` runs every branch to completion. For
> best-effort enrichers (research, playbook), this is a feature — a
> failed lookup degrades context rather than aborting dispatch. If you
> previously bailed on the first failed enrichment, change the
> downstream consumers to handle missing context gracefully (typically
> they already do; the enrichment is best-effort by design).

### Step 3 — Add the safety comment to `EnrichmentPipeline::run_steps`

`crates/roko-compose/src/enrichment/pipeline.rs:397`. Replace the
current doc comment with:

```rust
/// Run a selected subset of enrichment steps in the order provided.
///
/// **Sequential by design.** Later steps in `ALL_ORDERED` consume
/// artifacts (files written under the plan directory) from earlier
/// ones — see `crates/roko-compose/src/enrichment/inputs.rs` for the
/// per-step input lists. Naive `tokio::join!` parallelisation races
/// on the output files and produces empty/inconsistent prompts hours
/// later in the pipeline.
///
/// A safe parallel implementation would require a static dependency
/// DAG (file → producing-step map, then topological layering with
/// `tokio::join!` per layer). That is a multi-day refactor; see the
/// follow-up issue. Until then, **do not parallelise this loop**.
///
/// Continues past failures — each step's outcome is collected.
/// Returns the list of outcomes for the requested steps only.
pub async fn run_steps(&self, plan_base: &str, steps: &[EnrichStep]) -> Vec<StepOutcome> {
    let mut outcomes = Vec::with_capacity(steps.len());
    for &step in steps {
        outcomes.push(self.run_step(step, plan_base).await);
    }
    outcomes
}
```

The function body is **unchanged**. Only the doc comment changes.

### Step 4 — Add a verification test (per-dispatch parallelism)

If Step 2 applied, add a test in the orchestrator's test module
verifying the join's wall-clock benefit (or its concurrent execution).
Example pattern:

```rust
#[tokio::test]
async fn dispatch_enrichment_runs_concurrently() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    // Mock enrichers that sleep; if executed serially the test would
    // take ~5 * 30ms. Concurrently it takes ~30-40ms.
    let counter = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));

    async fn slow(c: Arc<AtomicUsize>, max: Arc<AtomicUsize>) -> Result<(), ()> {
        let now = c.fetch_add(1, Ordering::Relaxed) + 1;
        let mut cur = max.load(Ordering::Relaxed);
        while now > cur {
            match max.compare_exchange(cur, now, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => cur = actual,
            }
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
        c.fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }

    let start = Instant::now();
    let _ = tokio::join!(
        slow(counter.clone(), max_in_flight.clone()),
        slow(counter.clone(), max_in_flight.clone()),
        slow(counter.clone(), max_in_flight.clone()),
        slow(counter.clone(), max_in_flight.clone()),
        slow(counter.clone(), max_in_flight.clone()),
    );
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(120),
        "expected concurrent execution; took {:?}", elapsed);
    assert!(max_in_flight.load(Ordering::Relaxed) >= 2,
        "expected ≥2 in-flight at peak");
}
```

Place this test in a sensible spot near the changed code. (For
orchestrator changes, the existing `#[cfg(test)]` blocks live near the
bottom of `orchestrate.rs`.)

### Step 5 — Confirm the existing pipeline ordering test still passes

```bash
rg -n 'run_steps_executes_only_requested_steps_in_explicit_order' crates/roko-compose/src/
# Expected: one test in pipeline.rs
```

The doc-comment change in Step 3 must not affect the test.

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-compose/src/enrichment/pipeline.rs`

## Read-Only Context

- `crates/roko-compose/src/enrichment/step.rs` (`ALL_ORDERED`)
- `crates/roko-compose/src/enrichment/inputs.rs` (per-step input map)
- `tmp/solutions/perf/implementation/08-parallel-enrichment.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-ENRICH-1/2/3)

## Acceptance Criteria

- [ ] At least one dispatch-time enricher join site uses `tokio::join!` (or the audit-only outcome from Step 1 is documented).
- [ ] Error semantics preserved or explicitly documented (best-effort vs short-circuit).
- [ ] Long-running enrichers wrapped in `tokio::time::timeout(...)`.
- [ ] `EnrichmentPipeline::run_steps` body is **unchanged**.
- [ ] Doc comment block added to `EnrichmentPipeline::run_steps` explaining the sequential invariant ("Sequential by design ...").
- [ ] Existing test `run_steps_executes_only_requested_steps_in_explicit_order` still green.

## Verify

```bash
# Confirm the sequential pipeline still has its for-loop:
rg -nU --multiline 'pub async fn run_steps.*?for &step in steps' \
   crates/roko-compose/src/enrichment/pipeline.rs
# Expected: still matches.

# Confirm new safety comment:
rg -n 'Sequential by design' crates/roko-compose/src/enrichment/pipeline.rs
# Expected: 1 match.

# Confirm at least one tokio::join! in orchestrate.rs near enrichment:
rg -nU --multiline 'tokio::join!' crates/roko-cli/src/orchestrate.rs
```

## Do NOT

- Do NOT parallelise `EnrichmentPipeline::run_steps` (AP-ENRICH-1).
  This is the big one. The doc comment, the dependency-input file, and
  the existing ordering test all encode the sequential constraint.
  Breaking it surfaces as empty `tasks.toml` / `brief.md` hours later.
- Do NOT parallelise IO that touches the same file. Two writes to
  `efficiency.jsonl` racing through `tokio::join!` corrupt the JSONL
  log silently (writes are not atomic at the OS level for >1 page).
- Do NOT use `tokio::spawn` for the join-able futures (AP-ENRICH-3).
  Spawn moves to a different task; for short, dispatch-time IO you
  keep the current task and avoid the scheduler hop.
- Do NOT join futures with disparate timeouts (AP-ENRICH-2). Wrap
  long-tail enrichers in `tokio::time::timeout(short_dur, ...)` first.
- Do NOT join futures whose side effects are interdependent (e.g., one
  writes to a cache the other reads). Even if "feels" independent, the
  order matters.
- Do NOT use `futures::future::join_all` for fixed arity (AP-ASYNC-5).
  `tokio::join!` is allocation-free.
- Do NOT extend enrichment scope ("while we're here, let's also enrich
  X") in this batch. Adding a new enricher is fine only after you
  confirm independence; do it as a follow-up.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_08 done <commit-sha>
```
