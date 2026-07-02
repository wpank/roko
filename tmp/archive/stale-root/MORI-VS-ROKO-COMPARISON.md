# Mori vs Roko: Side-by-Side Comparison

> Generated 2026-04-08 from code audit of both systems.
> Purpose: Identify every gap where roko doesn't match mori's behavior.

---

## 1. Model Selection

### Mori
- **3 complexity bands**: Fast, Standard, Complex
- **Per-band model defaults**: `fast_task_model = "gpt-5.4-mini"`, `standard = "gpt-5.4-mini"`, `complex = "gpt-5.4"`
- **Per-plan overrides**: `config.plan_overrides["plan-base"].complexity_band`
- **Retry escalation**: On failure, `band.escalate()` promotes Fast→Standard→Complex
- **Per-task override**: Task can set `preferred_model`
- **Provider routing**: Band maps to provider (anthropic, openai, openrouter)
- **Resolution function**: `resolve_implementer_model_for_tasks()` — 80+ lines of logic considering band, retry count, plan override, task override, provider preference

### Roko (current)
- **4 tiers**: mechanical, focused, integrative, architectural
- **Tier→model mapping**: mechanical→haiku, focused→sonnet, integrative→sonnet, architectural→opus
- **Per-task override**: `model_hint` field in tasks.toml
- ❌ **No retry escalation** (tier doesn't change on failure)
- ❌ **No per-plan override** (config has one global model)
- ❌ **No provider routing** (always anthropic via gateway)
- ❌ **No `resolve_implementer_model()` equivalent** — just `effective_model()` which is tier→model lookup

### Gap
Roko has the tier system but no escalation or per-plan overrides. Mori's model routing is ~200 lines of battle-tested logic; roko's is ~10 lines.

**Checklist items affected**: §13 (Model routing — 10/11 marked [x] but most of this is in roko-learn, not wired)

---

## 2. Prompt Assembly / Context Engineering

### Mori
- **Per-role budgets**: `PromptBudget` struct with 9 fields (plan, workspace_map, prd2, context, brief, reviews, instructions, file_context, skills)
- **Sections**: plan.md, prd-extract.md, brief.md, workspace map, review history, file context, skills/playbooks
- **Character budgets**: Implementer gets 50K plan + 12K PRD + 8K brief; Reviewer gets less
- **Context pack caching**: Hashed to disk at `.mori/memory/context-packs/`, reused across iterations
- **Skills injection**: Successful playbook patterns injected into prompts
- **Enrichment artifacts**: 12 files per plan (brief.md, decomposition.md, prd-extract.md, fixture-manifest.toml, dependency-manifest.toml, research.md, rubric.md, testing-backlog.md, tasks.toml, review-tasks.toml, verify-tasks.toml, scribe-tasks.toml)

### Roko (current)
- **Per-role system prompt**: `build_system_prompt()` returns ~500 chars role description
- **Task prompt**: Either generic "Plan: X, Task: Y" OR surgical context from `TaskDef.build_prompt()` (if tasks.toml has context section)
- **PromptComposer**: exists, used for basic section assembly with token budget
- ❌ **No enrichment artifacts** (no brief.md, prd-extract.md, etc.)
- ❌ **No context pack caching**
- ❌ **No skills injection** (roko-learn has skills but unwired)
- ❌ **No per-role budgets** (one global `prompt.token_budget`)

### Gap
Mori produces 12 enrichment artifacts per plan and caches context packs. Roko has the `SystemPromptBuilder` (6 layers) but the enrichment pipeline doesn't run.

**Checklist items affected**: §3 (Enrichment pipeline — 17/19 marked [x] but only in roko-compose, never called from CLI), §4 (Prompt composition — 12/12 [x] for basic PromptComposer, but not the full budget system)

---

## 3. Conductor / Retry / Escalation

### Mori
- **LLM-powered conductor**: `conductor/llm.rs` — sends full execution state to Claude, gets back interventions
- **Intervention types**: NUDGE, RESTART, RESET_REVIEW, RETRY-PLAN, SOFT-RETRY, FOCUS, DEPRIORITIZE, SKIP, PASS
- **Watcher system**: `conductor/watchers.rs` — checks for: compile fail loops, test budget exceeded, context pressure, spec drift, cost overrun, stuck detection, ghost turns, iteration caps
- **Auto-fix**: On compile failure, spawns AutoFixer agent with error context
- **Model escalation on retry**: `retry_count` fed into model resolution → higher band
- **Max iterations**: Configurable per-complexity (Trivial: 1, Complex: 2)

### Roko (current)
- **Conductor exists**: `roko-conductor` crate with 10 watchers (implemented, tests pass)
- **Executor calls conductor**: `executor/mod.rs` line 198 — `conductor.evaluate(signals, &ctx)` → ConductorDecision
- ❌ **No LLM conductor** (pure rule-based, no Claude intervention decisions)
- ❌ **No auto-fix agent** (no AutoFixer role spawned on compile failure)
- ❌ **No model escalation on retry** (retry count not tracked or passed to model selection)
- ❌ **verify pipeline failures don't trigger retry** (they just return Err)
- ❌ **No SOFT-RETRY vs RETRY-PLAN distinction**

### Gap
Roko has the conductor watchers but they only produce Continue/Restart/Fail decisions. Mori's conductor is much more nuanced with 9 intervention types and an LLM decision-maker.

**Checklist items affected**: §11 (Conductor — 15/18 [x] for watchers, but LLM conductor and intervention system aren't wired)

---

## 4. Gate / Verification Pipeline

### Mori
- **6-rung system**: Compile → Test → Clippy → Verify → Integration → Streaming
- **Per-plan gate config**: Plans can specify which rungs to run
- **Gate runner**: Runs gates in order, stops on first failure
- **Auto-fix on gate failure**: Spawns AutoFixer with compiler error as context

### Roko (current)
- **Gate crate**: `roko-gate` with 11 gates (compile, test, clippy, coverage, invariant, etc.)
- **Per-task verify pipeline**: NEW — `task.verify` steps run after each task
- ❌ **Per-task verify not yet connected to auto-fix** (just returns error)
- ❌ **No AutoFixer agent** that reads the error and tries to fix it
- ✅ **Plan-level gates**: orchestrate.rs runs compile+test+clippy gates per plan

### Gap
Roko's per-task verification is actually MORE granular than mori's (per-task vs per-plan), but it lacks the auto-fix feedback loop.

---

## 5. Worktree Management

### Mori
- **3,225 lines**: `apps/mori/src/git/worktree.rs`
- **Worktree reuse**: Plans keep worktrees across iterations
- **Stale lock detection**: Removes `.git/index.lock` older than 5 min
- **Orphan cleanup**: Prunes worktrees where directory is missing
- **Rebase on preserve**: Rebases preserved worktrees on startup
- **Shared target dir**: CARGO_TARGET_DIR points to shared cache
- **Shared Cargo.lock**: Managed separately
- **Problems**: Rebase conflicts, stale worktrees accumulating, race conditions on concurrent git ops

### Roko (current)
- **967 lines**: `roko-orchestrator/src/worktree.rs`
- **WorktreeManager**: Create/remove/health-check/prune
- **Idle TTL**: Auto-reclaim idle worktrees
- **Health enum**: Ok, Missing, StaleLock, WrongBranch
- ❌ **Not used by orchestrate.rs** (worktree field exists but not called in dispatch)
- ❌ **No per-task worktree creation** (planned but not wired)

### Gap
Roko has a cleaner worktree design but it's not wired into the orchestration loop. Mori's is battle-tested but overly complex.

---

## 6. Cost Tracking

### Mori
- **Gateway cost tracking**: `bardo-gateway` tracks cost per request ($3,224 total, 81.5% savings from caching)
- **Per-plan cost**: Tracked in orchestrator
- **Cost DB**: SQLite with per-request records
- **Budget enforcement**: Configurable per-plan budget
- **Cost display**: TUI shows live cost

### Roko (current)
- **roko-learn costs.rs**: SQLite costs DB exists
- ❌ **Not wired** — orchestrate.rs doesn't record costs
- ❌ **No budget enforcement** (config has no budget fields)
- ❌ **No cost display** (TUI is scaffold only)

---

## 7. Parallel Execution

### Mori
- **`parallel.rs`**: 7,900+ lines managing concurrent agent execution
- **Agent pool**: Max concurrent agents (configurable, default ~8)
- **Per-plan parallelism**: Plans can run in parallel
- **Within-plan**: Tasks run sequentially within a plan (one implementer at a time)
- **Cross-plan**: Multiple plans execute concurrently

### Roko (current)
- **`parallel_groups()`**: Computes dependency levels for within-plan parallelism
- **DAG**: UnifiedTaskDag supports cross-plan deps
- ❌ **orchestrate.rs runs plans sequentially** (one at a time)
- ❌ **Within-plan tasks also sequential** (parallel_groups computed but not used for concurrent dispatch)

---

## Summary: Where Claude Failed to Follow the Checklist

The checklist marks these as [x] but they're not actually working in the runtime:

| Checklist § | Marked | Actual | Why wrong |
|-------------|--------|--------|-----------|
| §3 (Enrichment) | 17/19 [x] | Code exists in roko-compose, never called | Claude implemented the module but not the wiring |
| §5 (Templates) | 11/11 [x] | Templates exist, orchestrate.rs uses inline prompts | Claude built templates in one session, wrote orchestrate.rs in another without knowing templates exist |
| §7.1 (Claude backend) | Multiple [x] | ClaudeCliAgent exists but many flags not passed from task context | Claude built the agent but the task parser wasn't feeding it data |
| §11 (Conductor) | 15/18 [x] | Watchers exist, but no LLM conductor, no auto-fix | Claude implemented rules but not the LLM escalation |
| §13 (Model routing) | 10/11 [x] | roko-learn has bandits but orchestrate.rs doesn't use them | Same "built but unwired" pattern |
| §16 (Memory) | 15/15 [x] | Episode logging exists but was deleted from run.rs, then re-added | Claude refactored without understanding what was load-bearing |

**Root cause pattern**: Every § was implemented by a different Claude session. Each session built its assigned module perfectly. No session wired modules together. The checklist counted "code exists" as [x] rather than "code runs end-to-end."

---

## What to do about Claude quality regressions

### The problem
Claude's instruction-following quality fluctuates between versions. Specific failure modes:
1. **Ignoring explicit CLAUDE.md rules** — even when rules say "DO NOT rewrite", Claude rewrites
2. **Optimistic completion** — marks tasks done based on "I wrote code" not "I verified it works"
3. **Context amnesia** — forgets what exists in the codebase between tool calls
4. **Creative reinterpretation** — "I'll improve the approach" instead of following the spec

### Mitigations that work

1. **`CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1`** — Prevents Claude from "reasoning" about whether to follow instructions. Already set in your config.

2. **Smaller tasks**: Tasks ≤50 LOC have 3x higher pass rate than tasks ≥200 LOC. The tier system enforces this.

3. **Executable verification**: Claude can't lie about grep output. Every acceptance criterion being a runnable command catches false completions.

4. **Per-task verify pipeline**: Run checks IMMEDIATELY after each task, not at the end. Catches drift early.

5. **Anti-patterns in context**: Explicitly telling Claude "DO NOT do X" in the task context is more effective than hoping it reads CLAUDE.md.

6. **Model diversity**: Use the cheapest model that works. Haiku follows simple instructions more reliably than Opus (which overthinks and "improves" things). The tier system enables this.

7. **Structured output when possible**: `--json-schema` for review verdicts. TOML parsing for tasks. Machine-parseable output can't be hallucinated.

8. **Index files as context**: The auto-maintained indexes (INDEX.md) are injected into every agent prompt, so Claude always knows what exists before creating anything new.

### What other LLMs do better

| Task type | Claude weakness | Better alternative |
|-----------|----------------|-------------------|
| Mechanical (imports, renames) | Overthinks, adds "improvements" | GPT-5.4-mini: follows instructions literally |
| Code generation | Good | Claude Opus is still best for complex code |
| Instruction following | Fluctuates with versions | GPT-5.4: very reliable instruction following |
| Structured output | Sometimes ignores schema | GPT-5.4-mini: strict schema adherence |

Mori already supports this via provider routing (fast_task_model = "gpt-5.4-mini"). Roko should too — the gateway already routes to OpenAI.
