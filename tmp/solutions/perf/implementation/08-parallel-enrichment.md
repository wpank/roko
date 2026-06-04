# 08 — Parallel Enrichment Phases (B07) — with caveats

> Bottleneck: the enrichment pipeline runs steps sequentially.
>
> **CAVEAT.** The bottleneck doc (`BOTTLENECK-ANALYSIS.md` §B07) and
> playbook (`OPTIMIZATION-PLAYBOOK.md` §8) describe steps as
> "independent" and propose `tokio::join!`. **This is wrong for the
> 13-step plan-enrichment pipeline** in
> `crates/roko-compose/src/enrichment/`, where later steps consume
> earlier outputs as input files. Naively parallelising those steps
> produces empty/incoherent prompts.
>
> What can be parallelised is a smaller set of *prompt-time enrichment
> queries* that the assembler runs to gather context for a single
> dispatch. This plan addresses the safe wins and explicitly warns off
> the unsafe ones.
>
> Effort: ≈3 h. Risk: low (only safe parallelisation).

---

## Goal & success criteria

After this change:

1. Independent prompt-context lookups in the assembler/orchestrator are
   issued via `tokio::join!`, not awaited serially.
2. The 13-step plan enrichment pipeline (`EnrichmentPipeline::run_steps`)
   stays sequential, with a comment documenting why.
3. (Optional) A future-work issue is filed describing how to build a
   dependency DAG over enrichment steps if true parallelism is ever
   wanted.

Done when:

- `tokio::join!` is used at the verified-independent join points
  catalogued in this plan.
- Macro-benchmark on standard workflow shows ≥80 ms improvement vs the
  plan-07 baseline.
- The enrichment pipeline retains its sequential semantics
  (regression-tested via existing `run_steps_executes_only_requested_steps_in_explicit_order`).

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B07.
- Live evidence the plan-enrichment steps are NOT independent
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

  The doc-comment two lines up explicitly says:

  > Steps are ordered by dependency: earlier steps produce artifacts
  > consumed by later steps.

  And `crates/roko-compose/src/enrichment/inputs.rs` shows each
  `EnrichStep` reading the output files of preceding steps. Parallel
  execution would race on those files.

- Genuine parallelism wins live in the **prompt-assembly enrichment**
  inside the orchestrator's per-dispatch path:
  - file-intel context (workdir scan / cached conventions),
  - knowledge store query,
  - recent execution history (episodes log read),
  - playbook query,
  - research-store fetch (if enabled).

  These are independent reads and safe to overlap.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-compose/src/enrichment/pipeline.rs` | The 13-step pipeline. **Read the doc comment carefully.** |
| `crates/roko-compose/src/enrichment/step.rs` (`ALL_ORDERED`) | Confirms dependency ordering. |
| `crates/roko-compose/src/enrichment/inputs.rs` | Shows each step's input files (the dependency graph). |
| `crates/roko-cli/src/orchestrate.rs` | `enrich_task_context_with_search` and friends — the per-dispatch enrichment that *is* parallelisable. |
| `crates/roko-compose/src/prompt_assembly_service.rs` | `assemble()` calls multiple stores in series; primary parallelisation target. |

---

## Code-level plan

### Step 1 — Inventory the per-dispatch enrichment calls

Inside `crates/roko-cli/src/orchestrate.rs`, find the
`enrich_task_context_with_search` (line ≈2343) and any other
`enrich_*` helpers used during dispatch. List each independent IO
operation; mark its inputs and outputs.

A typical pre-state:

```rust
let file_intel = enrich_file_intel(workdir, task).await?;
let knowledge = enrich_knowledge(&knowledge_store, task).await?;
let recent_eps = enrich_recent_episodes(&episodes_path).await?;
let playbook = enrich_playbook(&playbook_store, task).await?;
let research = enrich_research(&research_store, task).await.ok();
```

If you find code that resembles the above, this is your edit target.
If you do not (the code may already be partially parallel), file it
under the "Files to inspect" list and move on — do not invent
parallelism that wasn't sequential before.

### Step 2 — Convert sequential awaits to `tokio::join!`

```rust
let (file_intel, knowledge, recent_eps, playbook, research) = tokio::join!(
    enrich_file_intel(workdir, task),
    enrich_knowledge(&knowledge_store, task),
    enrich_recent_episodes(&episodes_path),
    enrich_playbook(&playbook_store, task),
    enrich_research(&research_store, task),
);
let file_intel = file_intel?;
let knowledge = knowledge?;
let recent_eps = recent_eps?;
let playbook = playbook?;
let research = research.ok();   // optional
```

> **Use `tokio::join!`, NOT `futures::join_all`** for fixed-arity
> joins. `join_all` allocates a Vec; `join!` is a macro that produces a
> tuple with no allocation.
>
> For dynamic arity (e.g., looping over an arbitrary list of
> independent enrichers), `futures::future::join_all` is correct.

### Step 3 — Verify error semantics

Sequential awaits short-circuit on the first error. `tokio::join!` runs
all branches to completion and returns a tuple of `Result`s. If your
code previously bailed on the first failed enrichment, decide whether
that semantic still applies (usually: no — enrichment is best-effort
and a failed enricher should degrade context gracefully, not abort the
dispatch). Document the new semantic in a code comment.

If you really need bail-on-first-error, use `tokio::try_join!` instead
(also tuple-returning, also macro-based, no allocation).

### Step 4 — Document the plan-enrichment ordering invariant

In `crates/roko-compose/src/enrichment/pipeline.rs::run_steps`, add a
doc-comment block:

```rust
/// Run a selected subset of enrichment steps in the order provided.
///
/// **Sequential by design.** Later steps in `ALL_ORDERED` consume
/// artifacts from earlier ones. Naive `tokio::join!` parallelisation
/// races on the output files and produces empty/inconsistent prompts.
/// A safe parallel implementation would require a static dependency
/// DAG (see TODO/future-work issue). Until then, do not parallelise.
pub async fn run_steps(&self, plan_base: &str, steps: &[EnrichStep]) -> Vec<StepOutcome> {
    // ... unchanged body ...
}
```

### Step 5 — File a follow-up issue (optional)

If the team wants real parallel enrichment later, the right design is a
**static dependency DAG** with a topological sort that yields parallel
"layers". Each layer can be `tokio::join!`-ed; layers run in sequence.
This is a multi-day refactor (data structures, validators,
integration). Do not attempt it inside this plan; file a tracking
issue.

---

## Step-by-step execution

1. `git checkout -b perf/08-parallel-enrichment-safe`.
2. Locate the per-dispatch enrichment call site in `orchestrate.rs`
   (Step 1).
3. Replace serial awaits with `tokio::join!` (Step 2).
4. Verify error semantics (Step 3).
5. Add the safety comment to `run_steps` (Step 4).
6. (Optional) File a follow-up issue (Step 5).
7. Macro-benchmark on standard workflow.
8. Open PR `perf(orchestrator): parallelise per-dispatch enrichment IO
   (B07-safe)`.

---

## Anti-patterns / things NOT to do

- **Do NOT parallelise `EnrichmentPipeline::run_steps`.** This is the
  big one. The doc comment, the dependency-input file, and the
  ordering test (`run_steps_executes_only_requested_steps_in_explicit_order`)
  all encode the sequential constraint. Breaking it will surface as
  empty `tasks.toml`/`brief.md`/etc. with confusing error messages
  hours later.
- **Do NOT parallelise IO that touches the same file.** Two writes to
  `efficiency.jsonl` racing through `tokio::join!` corrupt the JSONL
  log silently (writes are not atomic at the OS level for >1 page).
- **Do NOT use `tokio::spawn`** for the join-able futures. Spawn moves
  to a different task; for short, dispatch-time IO you keep the
  current task and avoid the scheduler hop. `tokio::join!` is a
  cooperative concurrency primitive, not a parallelism one.
- **Do NOT join futures with disparate timeouts.** A 30 s research
  fetch joined with a 10 ms file read makes the whole dispatch wait 30
  s. Wrap long-tail enrichers in `tokio::time::timeout(short_dur, ...)`
  before joining.
- **Do NOT join futures whose side effects are interdependent** (e.g.,
  one writes to a cache the other reads). Even if they "feel"
  independent, the order matters and `tokio::join!` does not guarantee
  one.
- **Do NOT wrap `tokio::join!` results in a custom enum** to hide the
  tuple. The tuple is part of the API surface; a wrapper just adds
  indirection without value.
- **Do NOT increase enrichment scope** ("while we're here, let's also
  enrich X") without re-evaluating dependencies. Adding a new enricher
  to a `tokio::join!` is fine only after you confirm it is independent.

---

## Test plan

| Level | Test | Where |
|---|---|---|
| Unit | `tokio::join!` produces same result tuple as serial awaits (asserts equivalence) | new test next to the changed function |
| Unit | `run_steps_executes_only_requested_steps_in_explicit_order` still passes | existing test in `enrichment/pipeline.rs` |
| Integration | `roko run --workflow standard "fix x"` produces identical prompt content before/after (modulo timestamps) | manual diff or snapshot test |
| Macro-bench | Standard-workflow wall-time improvement ≥80 ms | manual `/usr/bin/time -l` |

---

## Rollback plan

- `git revert` the parallelisation commit; the file reverts to serial
  awaits with no observable behaviour change beyond timing.
- The doc-comment on `run_steps` is informational — no rollback needed.

---

## Status check (acceptance)

- [ ] At least one set of dispatch-time enrichers is now joined via
      `tokio::join!`.
- [ ] `EnrichmentPipeline::run_steps` is unchanged and the safety
      comment is in place.
- [ ] Existing ordering test still green.
- [ ] Macro-benchmark improvement ≥80 ms recorded.
