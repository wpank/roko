# ⚠️ SUPERSEDED — See [`MASTER-PLAN.md`](MASTER-PLAN.md)
>
> This document has been replaced by `MASTER-PLAN.md`. Sections 1-11 below are absorbed
> into MASTER-PLAN.md Tier 1. This file is retained for historical reference only.

---

# Roko Remaining Work — Checklist

> **Read `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` first.**
>
> ~~This file contains ALL remaining work from the 2026-04-08 session.~~
> Work through sections in order. Each section is one focused session.
> After each section: `cargo test --workspace --exclude roko-demo` must pass.
>
> **THE RULE: Do NOT simplify. Do NOT skip items. Do NOT stub with println.
> Every handler must do real work. Verify each item before marking done.**

---

## Section 1: Wire run_task_plans into the 14-phase executor

**Scope**: Replace the manual task loop in `run_task_plans()` with the existing executor state machine.

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/executor/state_machine.rs` — transition table
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/executor/mod.rs` — tick() and apply_event()
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` — current run_task_plans() at the method that calls `find_plan_dirs`

**Mori reference**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pipeline.rs` — phase-driven execution loop
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs` — agent dispatch per phase

Checklist:
- [x] `run_task_plans()` calls `self.executor.tick()` in a loop instead of manually iterating tasks
- [x] Each `ExecutorAction` from tick() is dispatched to the appropriate handler
- [x] `apply_event()` is called after each handler to transition the phase
- [x] Loop continues until all plans are terminal (Complete, Failed, or Skipped)
- [x] The `dispatch_agent` function still reads tasks.toml for surgical context and model selection
- [x] `run_all()` and `run_task_plans()` are unified into one method (not two code paths)

Verify:
```bash
grep -c 'self.executor.tick()' crates/roko-cli/src/orchestrate.rs  # >= 1
grep -c 'self.executor.apply_event' crates/roko-cli/src/orchestrate.rs  # >= 5
grep -c 'run_task_plans' crates/roko-cli/src/orchestrate.rs  # should still exist but use executor
cargo test -p roko-cli --lib
```

---

## Section 2: Context attribution feedback loop

**Scope**: The ContextProvider (P12, already implemented) assembles tiered context. This section closes the feedback loop: track which context sections agents actually use, and feed that back into future context assembly.

**What already exists**:
- `crates/roko-compose/src/context_provider.rs` — tiered context (Surgical/Focused/Full), symbol resolver, task brief
- `crates/roko-compose/src/symbol_resolver.rs` — grep-based signature extraction
- `crates/roko-compose/src/task_brief.rs` — per-task What/Why/How brief
- `crates/roko-learn/src/efficiency.rs` — per-section token attribution (built, not wired)

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs` — `ContextSource` enum, `ResolvedContext`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` — `EfficiencyEvent`, section attribution
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` — LinUCB bandit (learns model routing)

Checklist:
- [x] After agent returns output, scan output for references to each injected context section (file paths, symbol names, keywords from brief)
- [x] Record per-section `was_referenced: bool` — computed in dispatch_agent_with, logged to context-attribution.jsonl
- [ ] Maintain a rolling average per `(task_tier, context_source_type)` — e.g. "focused tasks referencing PlanBrief 15% of the time"
- [ ] When `ContextProvider.resolve()` runs, check rolling averages: if a source type has <10% reference rate for this tier, demote its priority from Normal to Low (making it droppable under budget pressure)
- [ ] Log context attribution decisions: `[context] plan_brief: included (ref_rate=0.42)` / `[context] research: dropped (ref_rate=0.03)`
- [x] Store attribution data in `.roko/context-attribution.jsonl` (append-only, one line per task)

Verify:
```bash
grep -c 'was_referenced\|attribution\|ref_rate\|context.*feedback' crates/roko-cli/src/orchestrate.rs  # >= 3
grep -c 'context-attribution' crates/roko-cli/src/orchestrate.rs  # >= 1
cargo test -p roko-cli --lib
```

---

## Section 3: AutoFix phase handler

**Scope**: When gates fail, spawn an AutoFixer agent with the error context.

**Read first**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs:6141-6282` — mori auto-fix dispatch
- `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs:2238` — `auto_fix_risky_dirty_files`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/executor/state_machine.rs` — `AutoFixing` phase transitions

Checklist:
- [x] When executor emits `SpawnAgent(AutoFixer)`, handler reads the last gate failure output
- [x] Handler builds a fix prompt: error message + original task context + files changed
- [x] Handler selects model: Haiku for compile errors, Sonnet for test failures
- [x] Handler dispatches the AutoFixer agent
- [x] After agent runs, handler runs the gate pipeline again (via AutoFixDone → Gating state machine transition)
- [x] If gates pass, emit `AutoFixDone` → executor goes back to Gating → passes → Reviewing
- [x] If gates fail again and max auto-fix iterations (5) exceeded, emit `Fatal`
- [x] Max auto-fix iterations read from executor constant `MAX_AUTO_FIX_ITERATIONS` (currently 5)

Verify:
```bash
grep -c 'AutoFix\|auto_fix\|AutoFixer' crates/roko-cli/src/orchestrate.rs  # >= 5
cargo test -p roko-cli --lib
```

---

## Section 4: Review phase handler

**Scope**: After gates pass, spawn Auditor/Reviewer agents to inspect the diff.

**Read first**:
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pipeline.rs:462-510` — review flow
- `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2497-2510` — Reviewer tool allowlist
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/reviewer.rs` — review template (EXISTS, use it)

Checklist:
- [x] When executor emits `SpawnAgent(Auditor)`, handler generates a diff of plan changes
- [x] Handler builds review prompt using the existing `ReviewerTemplate` from roko-compose (do NOT write new prompt)
- [x] Reviewer gets Read-only tools: `Read,Glob,Grep,Bash` (no Edit/Write)
- [x] Handler parses review verdict (approve/reject with reasons)
- [x] On approve: emit `ReviewApproved` → DocRevision
- [x] On reject: emit `ReviewRejected` → back to Implementing (with review feedback as context)
- [x] Review feedback injected into task context as `prior_failures` for the retry

Verify:
```bash
grep -c 'Review\|Auditor\|ReviewApproved\|ReviewRejected' crates/roko-cli/src/orchestrate.rs  # >= 6
grep -c 'ReviewerTemplate\|reviewer' crates/roko-cli/src/orchestrate.rs  # >= 1 (uses existing template)
cargo test -p roko-cli --lib
```

---

## Section 5: DocRevision and Merge phase handlers

**Scope**: After review approval, update docs and merge.

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/templates/scribe.rs` — scribe template (EXISTS, use it)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/worktree.rs` — WorktreeManager (EXISTS, use it)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pipeline.rs` — commit/merge flow

Checklist:
- [x] DocRevision handler spawns Scribe agent using existing `ScribeTemplate`
- [x] Scribe gets Read+Write tools to update docs
- [x] After scribe, emit `DocRevisionDone`
- [x] Merge handler uses `WorktreeManager` from `roko-orchestrator/src/worktree.rs` to merge the branch
- [x] On merge success, emit `MergeSucceeded` → Complete
- [x] On merge conflict, emit `MergeFailed` → Failed (with conflict details)

Verify:
```bash
grep -c 'DocRevision\|Scribe\|MergeBranch\|MergeSucceeded' crates/roko-cli/src/orchestrate.rs  # >= 6
grep -c 'WorktreeManager\|worktree' crates/roko-cli/src/orchestrate.rs  # >= 3
cargo test -p roko-cli --lib
```

---

## Section 6: Worktree per task + parallel execution

**Scope**: Each concurrent task gets its own worktree. Tasks within a dependency level run in parallel.

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/worktree.rs` — WorktreeManager (EXISTS)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs` — `parallel_groups()` (EXISTS)
- `/Users/will/dev/uniswap/bardo/apps/mori/src/git/worktree.rs` — mori worktree (reference only, don't copy the complexity)

Checklist:
- [ ] Before dispatching a task, acquire a worktree from WorktreeManager
- [ ] Agent runs in the worktree directory (not repo root)
- [ ] After task completes (pass or fail), release the worktree
- [ ] Worktrees are ephemeral: created per task, deleted after use
- [ ] Tasks within a parallel group dispatch concurrently using `tokio::spawn` + `futures::future::join_all`
- [ ] Concurrency capped at `tasks_file.meta.max_parallel`
- [ ] Stale locks (`.git/index.lock` > 60s) cleaned before worktree operations
- [ ] `CARGO_TARGET_DIR` set to shared target dir (not per-worktree)

Verify:
```bash
grep -c 'acquire\|release\|worktree.*create\|worktree.*remove' crates/roko-cli/src/orchestrate.rs  # >= 4
grep -c 'tokio::spawn\|join_all\|JoinSet' crates/roko-cli/src/orchestrate.rs  # >= 2
cargo test -p roko-cli --lib
```

---

## Section 7: Conductor checks between phases

**Scope**: After every phase transition, run conductor watchers and potentially intervene.

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/watchers/` — 10 watchers (ALL exist and tested)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/diagnosis.rs` — error pattern matching
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/watchers.rs:32` — `check()` method
- `/Users/will/dev/uniswap/bardo/apps/mori/src/conductor/llm.rs` — LLM conductor

Checklist:
- [ ] After every `apply_event()` call, run conductor evaluation
- [ ] Import and instantiate the 10 watchers from roko-conductor
- [ ] If conductor returns `ConductorDecision::Restart`, reset the plan to Implementing
- [ ] If conductor returns `ConductorDecision::Fail`, transition to Failed
- [ ] Conductor context includes: gate results, iteration count, cost so far, context window pressure
- [ ] Log conductor decisions to the event log

Verify:
```bash
grep -c 'conductor\|Watcher\|ConductorDecision\|evaluate' crates/roko-cli/src/orchestrate.rs  # >= 5
cargo test -p roko-cli --lib
```

---

## Section 8: Cost budget enforcement

**Scope**: Track cost per task/plan and stop before overspending.

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs` — `BudgetConfig` (EXISTS, parsed)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs.rs` — costs DB (EXISTS)
- `/Users/will/dev/uniswap/bardo/apps/bardo-gateway/src/cost_db.rs` — gateway cost tracking

Checklist:
- [ ] After each agent dispatch, record cost (input_tokens, output_tokens, cost_usd) to costs DB
- [ ] Before dispatching an agent, check `config.budget.max_task_usd` — abort if exceeded
- [ ] Track cumulative plan cost — abort plan if `config.budget.max_plan_usd` exceeded
- [ ] Warn at `config.budget.warn_at_percent` threshold
- [ ] Cost data recorded in episode log alongside wall_ms

Verify:
```bash
grep -c 'budget\|max_plan_usd\|max_task_usd\|cost_usd\|warn_at' crates/roko-cli/src/orchestrate.rs  # >= 4
cargo test -p roko-cli --lib
```

---

## Section 9: Cybernetic self-learning loop

**Scope**: Close the learning loop — model routing bandits, skill extraction, context caching, and failure-driven re-planning.

**What already exists**:
- `crates/roko-learn/src/model_router.rs` — LinUCB contextual bandit (17-dim context vector, cold-start + confidence + UCB stages)
- `crates/roko-learn/src/cascade_router.rs` — 3-stage cascade (static → confidence → UCB)
- `crates/roko-learn/src/efficiency.rs` — per-turn efficiency events (section tokens, tool usage, cost)
- `crates/roko-learn/src/episode_logger.rs` — append-only episode log (wired)
- `crates/roko-learn/src/skill_library.rs` — skill extraction + retrieval (built, not wired)
- `crates/roko-learn/src/playbook.rs` — playbook store (built, not wired)
- `crates/roko-compose/src/context_provider.rs` — tiered context assembly with source tracking

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` — bandit `observe()` and `select_model()` API
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/skill_library.rs` — `extract_skill()` and `query()` API
- `/Users/will/dev/uniswap/bardo/tmp/mori-agents/19-practical-self-learning.md` — learning guide

### 9a: Model routing bandit

Checklist:
- [ ] After task success, call `bandit.observe(context_vec, model_idx, reward)` with reward = `pass_rate * 0.5 + (1 - norm_cost) * 0.3 + (1 - norm_duration) * 0.2`
- [ ] After task failure, call `bandit.observe(context_vec, model_idx, 0.0)`
- [ ] Context vector built from: task tier (one-hot), complexity scalar, iteration count, agent role hash, crate familiarity, prior failure flag
- [ ] Before model selection, call `cascade_router.select(context_vec)` — use its recommendation instead of static tier_models when bandit has >50 observations
- [ ] Fall back to `TaskDef.effective_model()` when bandit is cold-starting
- [ ] Bandit state persisted to `.roko/bandit-state.json` between runs

### 9b: Skill extraction + injection

Checklist:
- [ ] On task success: extract a "skill" (the context pack + prompt + model + gate results that worked)
- [ ] Store skill in skill library keyed by: task files, task tier, symbols referenced
- [ ] Before building context for a new task, query skill library: "what worked for similar tasks?"
- [ ] If a matching skill is found, inject its successful patterns as a Low-priority context section
- [ ] Cap skill injection at 1K tokens (it's a hint, not a manual)

### 9c: Failure-driven re-planning

Checklist:
- [ ] When a task fails after max retries AND auto-fix, record the failure pattern (error type, files, model)
- [ ] When `roko prd plan` generates a new plan, query failure patterns: "has this crate/file combination failed before?"
- [ ] If yes, inject the failure context into plan generation prompt so it generates different task decomposition
- [ ] Track a per-crate "familiarity score" (success_count / total_count) — use as a bandit feature

Verify:
```bash
grep -c 'bandit\|observe\|select_model\|cascade_router' crates/roko-cli/src/orchestrate.rs  # >= 3
grep -c 'skill\|skill_library\|extract_skill' crates/roko-cli/src/orchestrate.rs  # >= 2
grep -c 'familiarity\|failure_pattern\|re.plan' crates/roko-cli/src/orchestrate.rs  # >= 2
cargo test -p roko-cli --lib
```

---

## Section 10: Cross-plan dependencies + plan-level ordering

**Scope**: Plans with `depends_on` in frontmatter wait for their dependencies.

**Read first**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/dag.rs` — `UnifiedTaskDag` with plan deps (EXISTS)
- Plan frontmatter: `depends_on: ["W01-wire-system-prompts"]`

Checklist:
- [ ] When loading plans, read `depends_on` from each plan's frontmatter
- [ ] Build a plan-level DAG (not just task-level)
- [ ] Plans whose dependencies haven't completed yet stay in `Queued` phase
- [ ] When a plan completes, check if any queued plans' dependencies are now satisfied
- [ ] When adding a new plan, scan all existing plans and update any that reference it

Verify:
```bash
grep -c 'depends_on\|plan_deps\|plan.*dag\|plan.*depend' crates/roko-cli/src/orchestrate.rs  # >= 3
cargo test -p roko-cli --lib
```

---

## Section 11: Regenerate existing plans with full format

**Scope**: P06 and W01 have old-format tasks.toml (no tier/context/verify). Regenerate them.

Checklist:
- [ ] Run `roko plan generate --from-file plans/P06-process-management/plan.md` to regenerate P06
- [ ] Verify P06/tasks.toml now has `tier`, `model_hint`, `context.read_files`, and `verify` steps
- [ ] Run `roko plan generate --from-file plans/W01-wire-system-prompts/plan.md` to regenerate W01
- [ ] Verify W01/tasks.toml has the same fields
- [ ] Run `roko plan run plans/W01-wire-system-prompts/` and verify it executes through all phases

Verify:
```bash
grep -c 'tier\|model_hint\|read_files\|verify' plans/P06-process-management/tasks.toml  # >= 10
grep -c 'tier\|model_hint\|read_files\|verify' plans/W01-wire-system-prompts/tasks.toml  # >= 10
```
