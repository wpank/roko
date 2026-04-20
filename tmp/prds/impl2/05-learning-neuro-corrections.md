# PRD 05: Learning and neuro corrections

**Branch:** `wp-demo`
**Status:** Draft
**Date:** 2026-04-22

---

## Scope

Four concrete gaps in the learning and knowledge subsystems. Each gap has a
source citation, a precise defect statement, and a numbered checklist of
changes to make it correct.

| Gap | Subsystem | Severity |
|-----|-----------|----------|
| A | roko-neuro distillation | High — knowledge accumulation silently broken |
| B | CLAUDE.md status table | Medium — incorrect claims mislead contributors |
| C | Prompt experiments | Medium — A/B system never fires |
| D | roko-index workspace parsing | Low — correctness fine, performance degrades at scale |

---

## Gap A: Neuro distillation silently no-ops without ANTHROPIC_API_KEY

### Defect

`spawn_episode_distillation` at
`crates/roko-neuro/src/episode_completion.rs:25-31` checks for
`ANTHROPIC_API_KEY` and returns `Ok(())` if the variable is absent or empty.
No log line is emitted. The hook is installed at
`crates/roko-cli/src/orchestrate.rs:852-856` and called at lines 4594, 4758,
and 4918. There is also a third `maybe_auto_dream()` call at line 7567 that
triggers dream runs from the heartbeat path. A user running `roko plan run`
without the variable set sees no
indication that every episode completes without distillation, and the
`.roko/neuro/knowledge.jsonl` store never grows.

The distiller itself requires a `ClaudeAgent` constructed from the API key
(`crates/roko-neuro/src/distiller.rs:56-115`). There is no path to use the
agent's already-configured LLM backend for distillation.

### Implementation checklist

**A-1. Emit a warning when the key is absent (episode_completion.rs)**

- [ ] In `distill_episode`, replace the silent `return Ok(())` branch with:
  ```rust
  tracing::warn!(
      "ANTHROPIC_API_KEY not set — episode distillation skipped. \
       Knowledge accumulation is disabled. Set ANTHROPIC_API_KEY to enable it."
  );
  return Ok(());
  ```
  File: `crates/roko-neuro/src/episode_completion.rs`, lines 25-31.

**A-2. Check once at PlanRunner startup (orchestrate.rs)**

- [ ] Add a function `warn_if_distillation_disabled(workdir: &Path)` in
  `crates/roko-cli/src/orchestrate.rs` that checks
  `std::env::var("ANTHROPIC_API_KEY")` exactly once at startup.
- [ ] Call it immediately after each of the three
  `install_episode_distillation_hook` call sites (lines 4594, 4758, 4918), so
  the warning appears once per `PlanRunner::new` rather than once per episode.
- [ ] The check should be a `tracing::warn!` at `info` level visible in the
  TUI status bar. Message: `"ANTHROPIC_API_KEY unset — neuro distillation
  disabled for this run"`.

**A-3. Document the env var requirement (roko.toml and CLAUDE.md)**

- [ ] Add a `[neuro]` section comment to the default `roko.toml` template
  (wherever `roko init` writes it) noting that `ANTHROPIC_API_KEY` is required
  for knowledge accumulation.
- [ ] Update the CLAUDE.md `## Current state` table row for `roko-neuro` to
  read: `Wired — requires ANTHROPIC_API_KEY for distillation` rather than just
  `Wired`.

**A-4. Support the configured LLM backend as a fallback distiller**

- [ ] **NOTE:** The `DistillationBackend` trait already exists at
  `crates/roko-neuro/src/distiller.rs:34-42`, and `Distiller::with_backend()`
  is already at lines 67-71. Less new code is needed than originally implied.
  The work is to add a new impl of the existing trait, not to create the trait.
- [ ] Add a `DistillationBackend` implementation backed by the existing
  `roko-agent` dispatch stack (not just `ClaudeAgent` directly) in
  `crates/roko-neuro/src/distiller.rs`.
- [ ] The new impl should accept any type implementing the existing
  `roko_agent::Agent` trait so that Ollama, OpenAI-compat, or Gemini backends
  can be used.
- [ ] Expose a `Distiller::with_agent(agent: Arc<dyn Agent>) -> Self`
  constructor that calls the existing `with_backend()` internally.
- [ ] In `distill_episode`, after the `ANTHROPIC_API_KEY` check fails, attempt
  to build a `Distiller` from the agent backend resolved via the
  `roko-agent` provider registry; fall back to the warning from A-1 only if
  no backend is available.
- [ ] File changes: `crates/roko-neuro/src/distiller.rs`,
  `crates/roko-neuro/src/episode_completion.rs`.

**A-5. Add integration test for the silent-skip path**

- [ ] Add a unit test in `crates/roko-neuro/src/episode_completion.rs` that
  temporarily unsets `ANTHROPIC_API_KEY`, calls `distill_episode` on a minimal
  `Episode`, and asserts that it returns `Ok(())` without panicking.
- [ ] Add a second test that sets a dummy key and confirms `Distiller::with_claude`
  is instantiated (mock the backend via `Distiller::with_backend`).

---

## Gap B: CLAUDE.md status table corrections

### Defect

The table under `## Current state` in
`/Users/will/dev/nunchi/roko/roko/CLAUDE.md` contains five wrong claims:

| Claim | Actual state | Citation |
|-------|-------------|----------|
| `roko-dreams — Phase 2+` | Wired | `orchestrate.rs:75,6511,7367,7388-7452` |
| `roko-daimon — Phase 2+` | Wired | `orchestrate.rs:48,71-72,3341,4596-4649` |
| `VCG auction in composition — Wired` | Dead code, greedy knapsack runs | `crates/roko-compose/src/prompt.rs` (no `vcg_allocate` call site) |
| `Safety contracts enforcement — Wired` | Only wired in tests and Ollama path; not in Claude CLI dispatch | `crates/roko-agent/src/dispatcher/mod.rs:329` (`authorize_call_with_taint` absent from Claude CLI tool loop) |
| `Knowledge-informed routing — not wired` | Weakly wired | `orchestrate.rs:2616,13255` |

### Implementation checklist

**B-1. Move roko-dreams out of Phase 2+**

- [ ] Change the `roko-dreams` row from `Phase 2+` to `Wired` with note: `DreamRunner called at orchestrate.rs:6511,7367 after 5+ episodes`. **Clarification on default:** the code default for `auto_dream` is `true` (in the struct's `Default` impl), but the `roko init` template sets `auto_dream = false`. Users who run `roko init` will have dreams disabled unless they edit the config. Document both: "code default: true; roko init template: false".
- [ ] Do not change any Rust code — this is a docs-only correction.

**B-2. Move roko-daimon out of Phase 2+**

- [ ] Change the `roko-daimon` row from `Phase 2+` to `Wired` with note: `DaimonState::load_or_new at orchestrate.rs:4596; daimon.query_somatic() at 13493; DaimonPolicy at 3341`.
- [ ] Do not change any Rust code.

**B-3. Correct the VCG auction claim**

- [ ] Change the `VCG auction in composition` row from `Wired` to `Partial — vcg_allocate exists in crates/roko-compose/src/auction.rs:293 and is tested, but the call site in prompt.rs was removed; greedy knapsack runs instead`.
- [ ] Separately, decide whether to reconnect `vcg_allocate` or delete it. This PRD does not mandate the decision, but the table must reflect reality.

**B-4. Correct the safety contracts claim**

- [ ] Change the `Safety contracts enforcement` row from `Wired` to `Partial — SafetyLayer::with_defaults() wired in ExecAgent (crates/roko-agent/src/exec.rs:565,582,593,616) and Gemini native (gemini/native.rs:876); authorize_call_with_taint tested in unit tests (safety/mod.rs:1069-1399) but not invoked in the Claude CLI tool loop at runtime`.
- [ ] Add a follow-up task (separate issue) to wire `SafetyLayer` into the Claude CLI dispatch path.

**B-5. Correct the knowledge-routing claim**

- [ ] Change the `Knowledge-informed routing — not wired` item to `Weakly wired — knowledge_routing_boost() at orchestrate.rs:2616 is called at 13255 but contributes a small fixed offset; full neuro store consultation for model selection is not yet implemented`.

**B-6. Add missing wired items**

- [ ] Add `roko-index + roko-lang-*` row: `Wired — code_context_for_task() at orchestrate.rs:17466 builds WorkspaceIndex per dispatch; Rust, TypeScript, Go providers wired`.
- [ ] Add `roko-dreams` and `roko-daimon` rows to the `| Component | Status | Where |` table (they are currently only listed in the crates table at the bottom, not the state table at the top).

---

## Gap C: Model experiments dead by default

### Defect

**IMPORTANT: There are two separate experiment systems in roko that must not
be conflated:**

1. **`ExperimentStore`** — prompt experiments (A/B testing prompt templates).
   Persists to `.roko/learn/experiments.json`. Used by
   `apply_concluded_experiment_overrides` at orchestrate.rs:4595.

2. **`ModelExperimentStore`** — model A/B experiments (comparing LLM models).
   Persists to `.roko/learn/model-experiments.json`. Used by the assignment
   path at orchestrate.rs:13346-13358.

**This gap targets `ModelExperimentStore` (system 2), not `ExperimentStore`
(system 1).** The file paths, store types, and seeding logic are distinct.

The A/B model experiment infrastructure assigns variants at
`crates/roko-cli/src/orchestrate.rs:13346-13358` and records results at
`9930-9940`. The assignment path calls
`ModelExperimentStore::applicable_experiment` at
`crates/roko-learn/src/model_experiment.rs:266`. But the store is loaded from
`.roko/learn/model-experiments.json`, which is never seeded on a fresh workspace.
`applicable_experiment` returns `None` on every call, so the experiment branch
at line 13346 is never taken and no routing decisions are ever influenced by
the A/B system.

### Implementation checklist

**C-1. Seed a default experiment at the `ModelExperimentStore::load_or_new` call site**

- [ ] **IMPORTANT:** `LearningRuntime` does NOT have a `ModelExperimentStore`
  field. The `ModelExperimentStore` is loaded independently in
  `orchestrate.rs` at line 13340 via `ModelExperimentStore::load_or_new(...)`.
  The seed must happen there, not in `LearningRuntime::new`.
- [ ] After the `ModelExperimentStore::load_or_new(...)` call in
  `orchestrate.rs:13340`, check if the store is empty and call
  `store.register(default_experiment())` if so.
- [ ] `default_experiment()` should return a `ModelExperiment` with:
  - `experiment_id`: `"default-haiku-vs-sonnet"`.
  - `status`: `ExperimentStatus::Running`.
  - `task_category`: `Some("mechanical".to_string())`.
  - `role`: `None` (applies to all roles for mechanical tasks).
  - Two variants: `claude-haiku-3-5` (control) and `claude-sonnet-4-5`
    (treatment), equal weights.
- [ ] The seed should only register if the store has zero experiments, so
  user-defined experiments are not overwritten.

**C-2. Add a `roko experiment` CLI subcommand**

- [ ] Add `experiment` as a top-level subcommand in
  `crates/roko-cli/src/main.rs` (or `lib.rs`).
- [ ] Implement `roko experiment list` — reads `.roko/learn/model-experiments.json`
  (the `ModelExperimentStore`, not the `ExperimentStore`) and prints each
  experiment's id, status, role/category scope, variant names, and sample counts.
- [ ] Implement `roko experiment create --control <model> --treatment <model>
  [--role <role>] [--category <category>]` — registers a new `ModelExperiment`
  and saves to disk.
- [ ] Implement `roko experiment status <id>` — prints variant sample counts,
  costs, gate pass rates, and a simple significance indicator (no external stat
  library required; a Welch t-test on cost or a binomial z-test on gate pass
  rate is sufficient).
- [ ] File: new `crates/roko-cli/src/experiment.rs`.

**C-3. Document experiment lifecycle**

- [ ] Add a short `## Prompt experiments` section to the CLAUDE.md
  `## Self-hosting workflow` block explaining:
  1. Experiments start seeded with `default-haiku-vs-sonnet`.
  2. `roko experiment list` shows current state.
  3. `roko experiment create` registers new experiments.
  4. Variants auto-apply during `roko plan run` at model selection time.
  5. Results persist to `.roko/learn/model-experiments.json` after every dispatched task.

**C-4. Add test for zero-experiment bootstrap**

- [ ] In `crates/roko-learn/src/model_experiment.rs` (or an integration test),
  add a test that constructs a `LearningRuntime` against an empty temp
  directory and asserts that `applicable_experiment("implementer",
  "mechanical")` returns `Some(...)` after initialization.

---

## Gap D: WorkspaceIndex rebuilt from scratch per dispatch

### Defect

`code_context_for_task` at `crates/roko-cli/src/orchestrate.rs:17466` calls
`roko_index::WorkspaceIndex::load(workdir)` which calls
`crates/roko-index/src/workspace.rs:435-440`. `load` calls
`collect_source_files` and parses every source file in the workspace on every
call. On the roko workspace (~177K LOC, ~18 crates) this is a blocking parse
per agent dispatch.

### Implementation checklist

**D-1. Add a cached index field to PlanRunner**

- [ ] Add `workspace_index: Option<roko_index::WorkspaceIndex>` to the
  `PlanRunner` struct (around line 3052 in `orchestrate.rs`).
- [ ] Initialize it as `None` in `PlanRunner::new`.
- [ ] In `code_context_for_task`, change the call from a free function to a
  method `self.code_context_for_task(task_description: &str) -> Vec<String>`.

**D-2. Populate the cache on first use with mtime-based invalidation**

- [ ] On the first call to `self.code_context_for_task`, build the index via
  `WorkspaceIndex::load(&self.workdir)` and store it in `self.workspace_index`.
- [ ] Track the mtime of `self.workdir` (or a sentinel file such as
  `Cargo.lock`) at index build time; store it as `workspace_index_mtime:
  Option<std::time::SystemTime>`.
- [ ] On subsequent calls, stat the sentinel file. If its mtime has advanced,
  rebuild the index; otherwise return cached results.
- [ ] The sentinel file should be `self.workdir.join("Cargo.lock")` as a
  reasonable proxy for source changes in a Rust workspace.

**D-3. Ensure the cache is workspace-scoped, not global**

- [ ] The cache must live on `PlanRunner` and must not use any global `static`
  or `OnceLock`. Each `PlanRunner` instance has its own index because
  `workdir` may differ between instances.
- [ ] Document this invariant in a comment above the field.

**D-4. Handle parse failures gracefully**

- [ ] If `WorkspaceIndex::load` returns an error during the cache-fill phase,
  log at `debug` level (existing behavior) and leave `self.workspace_index` as
  `None`. Do not cache the failure so the next call tries again.
- [ ] Add `workspace_index_mtime: Option<std::time::SystemTime>` to the struct
  only when `workspace_index` is `Some`.

**D-5. Unit test the invalidation logic**

- [ ] Write a test that:
  1. Creates a temp directory with a `Cargo.lock` file.
  2. Calls `code_context_for_task` twice and asserts the index is built only
     once (use a call counter via an `Arc<AtomicUsize>` in a test-only
     `WorkspaceIndex` wrapper, or assert mtime comparison directly).
  3. Advances the `Cargo.lock` mtime by writing to it.
  4. Calls again and asserts the index is rebuilt.

---

## Concrete file touchpoints

| File | Gaps | Changes |
|------|------|---------|
| `crates/roko-neuro/src/episode_completion.rs` | A | A-1, A-4: add `warn!`, add agent-backend fallback |
| `crates/roko-neuro/src/distiller.rs` | A | A-4: add `Distiller::with_agent` constructor and impl |
| `crates/roko-cli/src/orchestrate.rs` | A, D | A-2: `warn_if_distillation_disabled`; D-1,D-2,D-3,D-4: cached index field and invalidation |
| `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` | B | B-1 through B-6: status table corrections |
| `crates/roko-learn/src/runtime_feedback.rs` | C | C-1: seed default experiment at init |
| `crates/roko-learn/src/model_experiment.rs` | C | C-4: bootstrap test |
| `crates/roko-cli/src/main.rs` or `lib.rs` | C | C-2: wire `experiment` subcommand |
| `crates/roko-cli/src/experiment.rs` (new) | C | C-2: `list`, `create`, `status` implementations |
| `crates/roko-index/src/workspace.rs` | D | No changes — caching lives in the caller |

---

## Verification checklist

### Gap A

- [ ] Run `ANTHROPIC_API_KEY= cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -i "distill"` and confirm a `warn` line appears within the first 10 seconds.
- [ ] Run `cargo test -p roko-neuro episode_distillation_skips_without_api_key` and confirm it passes.
- [ ] Run a full plan with a valid `ANTHROPIC_API_KEY` and confirm `.roko/neuro/knowledge.jsonl` gains new entries after task completion.

### Gap B

- [ ] Read `CLAUDE.md` and verify the five corrected rows match the evidence citations listed in B-1 through B-5.
- [ ] Run `grep -n "roko-dreams\|roko-daimon\|VCG auction\|Safety contracts\|Knowledge-informed" CLAUDE.md` and confirm each line reflects the corrected status.

### Gap C

- [ ] Run `cargo run -p roko-cli -- experiment list` on a fresh `.roko/` directory and confirm `default-haiku-vs-sonnet` appears with status `running`.
- [ ] Run `cargo run -p roko-cli -- experiment create --control claude-haiku-3-5 --treatment claude-opus-4-6 --category research` and confirm a new experiment appears in `roko experiment list`.
- [ ] Run a plan that includes a `research` task and confirm the new experiment's sample count increments in `.roko/learn/model-experiments.json`.
- [ ] Run `cargo test -p roko-learn experiment_bootstrap_seeds_default` and confirm it passes.

### Gap D

- [ ] Run `cargo test -p roko-cli workspace_index_cache_invalidates_on_lock_change` and confirm it passes.
- [ ] Run `cargo run -p roko-cli -- plan run plans/ 2>&1 | grep -c "code-context"` before and after D-2 and confirm the count per-task drops from N to 1 (first dispatch) then 0 (subsequent dispatches with unchanged Cargo.lock).

---

## Acceptance criteria

1. **Gap A fully resolved** when: running `roko plan run` without `ANTHROPIC_API_KEY` emits exactly one `WARN` per plan run (not per episode), and running it with the key results in measurable growth of `.roko/neuro/knowledge.jsonl` (the distillation output path).

2. **Gap B fully resolved** when: every row in the CLAUDE.md status table for roko-dreams, roko-daimon, VCG auction, safety contracts, and knowledge-informed routing cites a real file and line number that can be grepped and confirmed.

3. **Gap C fully resolved** when: `roko experiment list` on a fresh workspace shows at least one running experiment, and a subsequent `roko plan run` causes that experiment's `sample_count` to increment by the number of dispatched tasks.

4. **Gap D fully resolved** when: the integration test in D-5 passes, and a profiled run of `roko plan run` shows `WorkspaceIndex::load` called once per `PlanRunner` lifetime rather than once per task dispatch.

---

## Anti-patterns to avoid

- **Do not add `eprintln!` or `println!` for the A-1 warning.** Use `tracing::warn!` so the message routes through the TUI log panel and structured log output.
- **Do not make the D-2 cache global.** Previous mistakes in this codebase introduced global singletons for workspace state that caused cross-test contamination. Cache on the struct.
- **Do not delete `vcg_allocate` as part of this PRD.** The B-3 task only updates the documentation claim. A separate decision is required before removing the function.
- **Do not use `unwrap` in `warn_if_distillation_disabled`.** The env var check is infallible (`std::env::var` returns `Err` only for non-UTF-8 values); handle that with `unwrap_or_default`.
- **Do not seed the default experiment using a `static OnceLock`.** Seeding happens at the `ModelExperimentStore::load_or_new` call site in `orchestrate.rs:13340` so it respects the runtime's configured workdir.

---

## Errata applied

Corrections applied 2026-04-22 based on audit discrepancy report:

1. **CRITICAL: `daimon.query_somatic()` line number corrected.** Changed from
   3503 to 13493 (off by ~10,000 lines in the original).

2. **Wrong experiment store file path corrected.** Changed from
   `.roko/learn/experiments.json` to `.roko/learn/model-experiments.json`.
   The former is the `ExperimentStore` for prompt experiments; the latter is
   the `ModelExperimentStore` for model A/B experiments.

3. **CRITICAL: Gap C-1 seed location corrected.** `LearningRuntime` has no
   `ModelExperimentStore` field. The seed must happen at `orchestrate.rs:13340`
   where `ModelExperimentStore::load_or_new` is called, not in
   `LearningRuntime::new`.

4. **Wrong knowledge path corrected.** Changed from `.roko/learn/knowledge.json`
   to `.roko/neuro/knowledge.jsonl` throughout the document.

5. **`DistillationBackend` trait already exists.** Noted that the trait is at
   `distiller.rs:34-42` with `with_backend()` at 67-71. The work is to add a
   new impl, not create the trait. Less new code needed than implied.

6. **`auto_dream` default clarified.** Code default is `true` (struct Default
   impl), but `roko init` template sets `false`. Both documented.

7. **Two experiment systems clarified.** Added explicit documentation
   distinguishing `ExperimentStore` (prompt experiments,
   `.roko/learn/experiments.json`) from `ModelExperimentStore` (model A/B,
   `.roko/learn/model-experiments.json`). Gap C targets the latter.

8. **Third `maybe_auto_dream()` call added.** Line 7567 (heartbeat path) was
   not mentioned in the original. Now documented alongside the three episode
   distillation hook call sites.
