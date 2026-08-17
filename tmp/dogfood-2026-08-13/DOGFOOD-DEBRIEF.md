# Dogfood Debrief: 2026-08-13

> End-to-end test of roko's self-hosting workflow running against its own codebase.

## Executive Summary

**Roko's full PRD→plan→execute workflow does NOT work end-to-end.** The pipeline
breaks at `plan run` due to a cascade of UX issues, configuration conflicts, and
a scheduler deadlock. Getting from `roko init` to a completed task required 5
manual interventions and ultimately still failed at the gate phase.

The agent dispatch works — an LLM was invoked, wrote code, and returned
successfully. But the runner never progressed past `AgentCompleted` to run gates
or advance to the next task. The run timed out after 600 seconds of scheduler
stall.

## Test Scenario

**Chosen task:** Wire plugin discovery at runtime (from E30-T09 in MASTER-EXECUTION-CHECKLIST)

**Generated plan:** 2 tasks — T1: add `plugins_dir()` to `RokoLayout`, T2: wire
into extension loader + add test.

## Timeline

| Step | Command | Time | Outcome |
|---|---|---|---|
| 0 | `roko init --profile rust` | 0.2s | ✓ Generated roko.toml |
| 1 | `roko prd idea "Wire plugin discovery..."` | 0.1s | ✓ Idea captured |
| 2 | `roko prd draft new "plugin-discovery-runtime"` | 3m 54s | ✓ Generated 22KB PRD |
| 3 | `roko prd list` | 0.04s | ✓ Draft visible |
| 4 | `roko prd plan plugin-discovery-runtime` | 2m 35s | ✓ Generated 2-task plan |
| 5 | `roko plan run plans/plugin-discovery-runtime` | instant | ✗ **Ambiguous model slug** |
| 5a | (fix model config, retry) | instant | ✗ **ANTHROPIC_API_KEY required** |
| 5b | (fix model key, retry) | instant | ✗ **Still ambiguous slug** |
| 5c | (override global key, retry) | instant | ✗ **Stale executor snapshot** |
| 5d | `roko plan run ... --fresh` | 12s | ✗ **core.fsmonitor git error** |
| 5e | (unset fsmonitor, retry with --fresh) | 10m | ✗ **Scheduler deadlock after agent success** |

**Total wall time:** ~18 minutes of commands + troubleshooting
**Furthest progress:** Agent completed T1 (wrote code), but gates never ran

## Blocking Issues Found

### 1. CRITICAL: `roko init` generates config that conflicts with global config

**Symptom:** `error: ambiguous model slug 'claude-sonnet-4-6' is defined by keys: claude-sonnet, claude-sonnet-4-6`

**Root cause:** `roko init --profile rust` generates `[models.claude-sonnet-4-6]` with
`slug = "claude-sonnet-4-6"`. The user's global config (`~/.roko/config.toml`) already
has `[models.claude-sonnet]` with `slug = "claude-sonnet-4-6"`. After merge, two keys
produce the same slug, and the validation rejects this.

**Why it's broken:** Local config should override global for same-slug models, or the
init template should check for existing global models before generating model sections.

**Fix options:**
1. `roko init` should read global config and skip generating model entries that would conflict
2. The slug ambiguity check should prefer local over global (last-writer-wins)
3. Init should not generate model entries at all when a global config already has the model

**Files:** `crates/roko-core/src/config/loader.rs:400-407` (ambiguity check),
`crates/roko-cli/src/commands/init.rs` (template generation)

### 2. CRITICAL: No way to override model provider for built-in slugs

**Symptom:** User wants `claude_cli` provider but global config uses `anthropic_api`.
Cannot define a local model with the same slug. Changing the key name still triggers
slug ambiguity.

**Root cause:** The model validation treats any two model configs with the same slug as
an error, even when the intent is to override the provider. There's no "local overrides
global" semantics for model profiles.

**Fix:** When merging global config into local, same-key entries should use the local
version. Same-slug entries from different keys should also prefer local over global.

### 3. CRITICAL: Scheduler deadlock after agent completion

**Symptom:** Agent completes T1 at 12:52:29. Runner sits idle for 10 minutes doing
nothing. Times out at 13:01:55 with `SchedulerNoProgress`.

**Root cause:** The state `plugin-discovery-runtime:T1:1:AgentCompleted` is set but the
event loop doesn't advance to the gate phase. The `AgentCompleted` event was received
(log shows agent turn completed) but the scheduler loop that processes completions and
dispatches gates appears stuck.

**Impact:** Even when everything else is fixed, roko cannot complete any plan because
agent work is never validated or committed.

**Needs investigation:** The event loop's `tokio::select!` at line 2018 of
`event_loop.rs` — the branch handling agent completions may have a missing state
transition, or the gate dispatch path has an early-exit condition that blocks progress.

### 4. HIGH: Stale executor snapshot blocks fresh runs

**Symptom:** `error: resume validation failed: plan 'demo-hello' is in snapshot but not in the current run`

**Root cause:** `roko prd plan` internally generates a demo plan that creates a
state-snapshot.json. When the user later runs `roko plan run` on a different plan, the
snapshot validation rejects because it has state for plans not in the current run.

**Fix:** Either `prd plan` should not write to the main executor state, or `plan run`
should auto-detect when the snapshot has zero overlap with the current plans and start
fresh without requiring `--fresh`.

**Workaround:** `--fresh` flag works but users shouldn't need to know this.

### 5. HIGH: `core.fsmonitor` breaks worktree checkout

**Symptom:** `error: worktree unavailable ... unsafe git execution policy: unsupported checkout extension 'core.fsmonitor'`

**Root cause:** Git's security policy treats `core.fsmonitor` as an unsafe extension in
worktree operations. Many developers have this enabled for performance (VS Code sets it).

**Fix options:**
1. The worktree manager should handle this gracefully — either temporarily unset the
   config for the worktree, or catch the error and suggest the fix
2. Better: pass `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.fsmonitor GIT_CONFIG_VALUE_0=false`
   as env vars to the worktree git commands

**Files:** `crates/roko-orchestrator/src/worktree.rs`

### 6. MEDIUM: Cascade router ignores configured model

**Symptom:** Config says `default_model = "claude-sonnet"` but cascade router selected
`kimi-k2-5` for the task.

**Root cause:** The cascade router has learned from prior sessions (persisted in
`.roko/learn/cascade-router.json`) and routes to kimi-k2.5 based on historical
performance. The user's `default_model` is treated as a suggestion, not a mandate.

**Not necessarily a bug:** This is the learning system working as designed. But there's
no clear way to override it. The UX should show why a different model was selected.

### 7. LOW: Unknown config keys produce warnings

**Symptom:** `WARN roko config: unknown key will be ignored; check for typos or schema change key=github / key=resources`

**Root cause:** The `roko init` template generates config sections (`[github]`,
`[resources]`) that the loader doesn't recognize. These sections do get parsed by some
code paths but the unified loader warns about them.

## Performance Observations

| Operation | Time | Notes |
|---|---|---|
| Release binary build | 14 min | Cold build; dominated by alloy deps |
| `roko init` | 0.2s | Fast |
| `roko prd idea` | 0.1s | Fast |
| `roko prd draft` | 3m 54s | 234s agent time, 2s context scan |
| `roko prd list` | 0.04s | Fast |
| `roko prd plan` | 2m 35s | 88s agent time, 1s context scan, 66s post-processing |
| Cargo cache warm | 0.4-23s | Varies based on incremental state |
| Agent factory init | <1ms | Fast |
| Agent dispatch latency | 2.6s | Time from task ready to agent command issued |
| Agent T1 execution | 34s | kimi-k2.5 via moonshot, 81K tokens |

**Bottlenecks:**
1. **Release build (14m):** Dominated by alloy full-feature dependency tree. Most chain
   modules are unused at runtime.
2. **PRD draft (3m 54s):** Almost entirely LLM wall time. Context scan (2s) is fine.
3. **Plan generation post-processing (66s):** Includes a "regeneration" step that
   re-generates the tasks.toml with enhanced fields — takes a full extra agent call.
4. **Agent dispatch latency (2.6s):** Worktree checkout, daimon state computation,
   model selection, prompt assembly. Acceptable for production, could be optimized.

## Comparison with Mori/Bardo

| Factor | Mori | Roko | Impact |
|---|---|---|---|
| First-run success | Would complete e2e | Fails at 5 points | **Critical** |
| Effective --bare mode | Yes (92% token reduction) | Flag removed from Claude CLI | ~30K extra tokens/turn |
| Config complexity | Single config, no global merge | Global + local merge with conflicts | Init → run friction |
| Executor state | Per-run, ephemeral | Persisted across runs, conflicts | Stale state blocks |
| Workspace pre-warm | sccache probing + background warm | `cargo check` once | Similar, roko is fine |
| Prompt assembly | Cached context packs | Fresh 9-layer build per dispatch | Roko more capable, slightly slower |
| Executor architecture | Pure-state DAG (4.6K lines) | God object (19.8K lines) | Harder to debug |
| Model routing | Fixed or env-selected | Cascade router with learning | More capable but opaque |
| Git worktree handling | Direct checkout | Via orchestrator with safety checks | fsmonitor breaks roko |

### Why Mori Works Better in Practice

1. **Simplicity over capability:** Mori has one binary, one config, one executor. No
   global/local config merge. No model slug ambiguity. No persistent state to conflict.

2. **Bare mode:** The single biggest performance difference. Mori's `--bare` flag
   (removed from newer Claude CLI) eliminated 30K tokens of overhead per turn. Roko has
   no equivalent.

3. **Battle-tested plans:** Mori's 171 plans were iteratively refined over months. Each
   has dense acceptance criteria and checkpoint scripts. Roko's generated plans are newer
   and less specified.

4. **Tight error handling:** Mori handles the git worktree edge cases because they were
   encountered and fixed during real production use. Roko's worktree manager is more
   sophisticated but hasn't been exercised enough to handle real-world git configs.

### What Roko Does Better (When Working)

1. **Model routing:** CascadeRouter persists learning across sessions — Mori resets on restart
2. **Knowledge store:** Neuro module with tier progression, distillation — Mori has nothing equivalent
3. **HTTP control plane:** 85 routes for external integration
4. **Adaptive gates:** EMA-based threshold tuning — Mori uses fixed thresholds
5. **Daimon affect engine:** Somatic markers modulate dispatch — unique to roko

## Recommendations (Priority Order)

### P0: Fix the scheduler deadlock
The agent completes but the runner never runs gates. This blocks all plan execution.
Start by adding debug logging around the `AgentCompleted` → gate dispatch state
transition in `event_loop.rs`.

### P0: Fix config/model conflict on init
`roko init` must produce a config that works with `roko plan run` without manual
intervention. Check for global config conflicts, skip redundant model entries, or make
the slug ambiguity check support local-overrides-global semantics.

### P1: Handle core.fsmonitor in worktree operations
Either strip it from worktree git env or catch the error with a clear diagnostic.

### P1: Auto-detect stale executor snapshots
When the snapshot has zero plan overlap with the current run, start fresh automatically
instead of requiring `--fresh` flag.

### P2: Find --bare equivalent or implement it in roko
The `--bare` flag's 92% token reduction was the single biggest performance optimization
in mori. Without it, every roko agent turn carries 30K+ extra tokens.

### P2: Add model selection override/explanation
When cascade router overrides the configured model, log/show why and provide a
`--force-model` flag.

### P3: Feature-gate alloy in default build
Making `roko-chain` optional behind a `chain` feature flag would cut 3-5 minutes from
cold release builds.

## Files Modified During Dogfood

| File | Change | Why |
|---|---|---|
| `roko.toml` | Removed/modified model section | Fix slug ambiguity |
| `.roko/prd/drafts/plugin-discovery-runtime.md` | Generated by prd draft | PRD content |
| `plans/plugin-discovery-runtime/tasks.toml` | Generated by prd plan | Plan content |
| `plans/plugin-discovery-runtime/plan.md` | Generated by prd plan | Plan summary |
| `.gitconfig` (local) | Unset core.fsmonitor | Fix worktree error |

## Raw Logs

All command outputs captured in `tmp/dogfood-2026-08-13/`:
- `00-init.log` through `03-prd-list.log`: PRD workflow (all succeeded)
- `04-prd-plan.log`: Plan generation (succeeded)
- `05-plan-run.log` through `08-plan-run-retry3.log`: Config/state failures
- `09-plan-run-fresh.log`: fsmonitor failure
- `10-plan-run-debug.log`: Debug logging showing fsmonitor error
- `11-plan-run-final.log`: Agent succeeded then scheduler deadlocked
