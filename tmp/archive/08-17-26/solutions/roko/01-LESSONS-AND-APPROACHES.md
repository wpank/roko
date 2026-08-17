# Lessons, Approaches, and Hard-Won Insights

> Compiled from 6 runner configurations, 500+ parallel batches, 3 architectural
> revisions, and 2 dogfood sessions. Every lesson here cost real time.

---

## 1. Solution Approaches Tried (Chronological)

Seven distinct architectures were proposed and evaluated for fixing roko's dispatch/execution.
The progression itself is instructive: the correct solution was the cheapest one.

### Solution A -- Surgical Batches (~35-40h)

Patch individual bugs independently across 7 batches. Low risk but treated symptoms, not root
causes. Example: fixing the `--mcp-config` flag in one path while 3 other paths still ignored it.
Did not fix the fundamental "dispatch is a thin pipe" problem.

**Why it failed:** Each patch created a new inconsistency. Fixing model routing in `dispatch_direct.rs`
while `run.rs` had its own hardcoded model strings meant the same user intent could hit different
models depending on the entry point.

### Solution B -- Architectural Redesign (~55-65h)

Build one `InferenceGateway` struct as the single dispatch path. Session-scoped service owning
HTTP client, model config, conversation history, system prompt, tool registry, cost tracking,
and streaming. Medium risk, one abstraction resolves ~25 individual issues.

**Why it was abandoned:** Designing the gateway took longer than understanding the actual problem.
The gateway was designed to replace something nobody had measured.

### Solution C -- Phased Migration (~45-55h)

Grow existing `dispatch_direct.rs` into an InferenceGateway incrementally. Fastest time-to-visible-
improvement but risks accumulating intermediate scaffolding.

**Why it was abandoned:** The "incremental" part was a lie. `dispatch_direct.rs` was already 800 lines
of mixed concerns. Growing it further added complexity without removing any.

### Solution 1 -- Service Triad (~70-90h)

Three independently deployable services: InferenceGateway, PromptAssemblyService, SessionService.
Addresses all 6 subsystem audits but "massively over-scoped for 'make chat work'."

**Diagnostic value:** Forced clear thinking about service boundaries. The PromptAssemblyService
concept eventually became `ModelCallService` in `crates/roko-agent/src/model_call_service.rs`.

### Solution 2 -- Cell/Graph Engine (~120-150h)

Full unified specification implementation. Everything expressed as Cells in Graphs. Required 120+
hours before any visible result.

**Why it was shelved:** The unified spec was elegant but could never ship incrementally. "Show me
a working chat session" was the requirement; "show me a beautiful type system" was not.

### Solution 3 -- Hybrid Engine (~80-100h)

Thin Cell/Graph engine (~2K LOC) with existing crates wrapped as Cells. ~90% of Solution 2's
quality at ~65% of cost. Still builds infrastructure before product.

**Why it was shelved:** Same fundamental error as Solution 2 -- building frameworks before features.

### Solution ACTUAL -- Wire 4 CLI Flags (~25-30h) [WINNER]

**The breakthrough:** Mori works because it passes `--append-system-prompt`, `--tools`,
`--mcp-config`, and `--resume` to the Claude subprocess. Roko's `dispatch_direct.rs` passes
none of these. The delta is 4 CLI flags, not a new architecture.

All infrastructure already existed but was never called from the chat path:
- System prompt builder at `crates/roko-compose/src/system_prompt_builder.rs` -- built, never wired
- Tool registry at `crates/roko-std/src/tool/` -- built, never passed to Claude
- MCP config at `crates/roko-acp/src/config.rs` -- loaded, never forwarded
- Resume at `crates/roko-agent/src/session.rs` -- implemented, never used

### FINAL-SOLUTION -- ChatAgentSession + Execution Contract Repair

Refined Solution ACTUAL after demo and E2E dogfood. Added prerequisite M0-0 (Execution Contract
Repair) plus PRD/plan artifact grounding. Routes through existing `ClaudeCliAgent` at
`crates/roko-agent/src/claude_cli_agent.rs`.

> **The meta-insight**: The correct solution was the cheapest one. Understanding the actual
> problem (reading Mori's 300 lines of dispatch code) was more valuable than any amount of
> architectural planning.

---

## 2. Runner Approaches (Parallel Code Generation)

Six runner configurations were developed and executed. Each taught different lessons
about parallelism, isolation, and LLM coordination.

| Runner | Batches | Model | Key Innovation | Lesson |
|--------|---------|-------|----------------|--------|
| **Architecture** | 16 batches, 5 phases | -- | First structured parallel code gen | Phase boundaries prevent cross-batch interference |
| **Converge** | 87 batches, 12 tracks | -- | Massive scope runtime convergence | Too many concurrent changes break merge |
| **Converge-Followup** | 32 batches, 6 waves | -- | Fixes from converge | Smaller focused runs more effective |
| **Mega-Parity** | 168 batches, 27 waves | gpt-5.4-mini | 40 parallel Codex sessions, 6 hours | No-build mode + wave gates = 10x throughput |
| **Parallel Template** | Reusable infra | -- | DAG scheduler, anti-pattern checks, wave gates, cherry-pick | Infrastructure for all subsequent runs |
| **Post-Parity** | 131 batches, 28 runners (A-Z, AA, AB) | -- | High-impact wiring focused | Alphabetized runners with scope isolation |

### Mega-Parity Deep Dive

The mega-parity runner was the largest single coordinated code generation run:
- **195 batches** executed across 27 waves
- **40 parallel Codex sessions** at peak
- **gpt-5.4-mini** for all generation (cheaper, faster than full gpt-5.4)
- **~6 hours total** wall clock
- **No compilation during batches** -- deferred to wave gates

Key operational decisions:
1. Context packs limited to ~4000 tokens (5 files)
2. Each batch got cumulative context about what other batches changed
3. Wave gates ran `cargo check` + `cargo clippy` after groups completed
4. Cherry-pick with monitoring, not automatic merge-back
5. `.result` files on disk as sole coordination mechanism

---

## 3. What Worked -- Operational Innovations

### 3.1 Parallel Worktree Model

Each batch gets its own git worktree. Full isolation, no lock contention, deterministic base
(all batches in a wave start from same commit), worktrees survive for inspection.

**Implementation detail:** `git worktree add tmp/runners/<run-id>/<batch> -b <batch>-run` creates
each worktree. The batch agent works in its worktree. After completion, changes are cherry-picked
to the integration branch. The worktree is never deleted automatically.

**Why this matters for roko:** The `WorktreeManager` in
`crates/roko-orchestrator/src/worktree.rs` implements this exact pattern for plan execution.
It already formats branch names via `format_branch_name()` and tracks health via `WorktreeHealth`.

### 3.2 Disabling Builds During Batch Runs

Context file telling agents "Do NOT run any compilation commands" reduced batch times from
**15-40 minutes to 1-5 minutes** (10-100x speedup). Agent cooperates rather than being tricked
with a fake binary.

**The key insight:** LLM agents want to verify their work. If you let them, they'll run the full
test suite on every change. This is wasteful when you're going to run a comprehensive gate at the
wave boundary anyway. The context pack includes an explicit instruction:

```
IMPORTANT: Do NOT run `cargo build`, `cargo check`, `cargo test`, `cargo clippy`,
or any other compilation command. The runner will verify your changes at the wave
gate. Focus only on writing correct code.
```

**Compliance rate:** ~95%. The 5% that ignored this instruction ran 30-minute batches instead
of 3-minute ones, sometimes exhausting disk space with incremental compilation artifacts.

### 3.3 Wave Gates Instead of Per-Batch Verification

Compiling after a group of changes caught real errors (cross-batch type mismatches) without false
positives from mid-wave partial states.

**How it works:** Batches in a wave are independent -- they don't touch the same files. After all
batches in a wave complete, changes are merged serially into the integration branch, then a single
`cargo check` + `cargo clippy` runs against the combined result. This catches:
- Type signature mismatches between batches
- Import conflicts when two batches add the same dependency
- Name collisions from parallel development

**Connection to ACP:** The `PipelineState` in `crates/roko-acp/src/pipeline.rs` has a Gating phase
that maps directly to this pattern -- but it runs gates per-agent rather than per-wave. Extending
ACP to support wave-level gating would unlock the same 10x improvement for multi-agent workflows.

### 3.4 The `--continue` Pattern

Resume from disk state, not memory state. Simple `.result` files on disk as coordination. Any
process can read/write. Manual intervention at any time. Kill and restart freely.

**File format:** Each batch produces a `.result` file:
```json
{"status": "done", "elapsed_ms": 12345, "commit": "abc123", "files_changed": 3}
```

The runner checks for existing `.result` files on startup and skips completed batches. This means
you can kill the runner, fix a problem manually, create the `.result` file yourself, and restart.
The runner picks up where it left off.

**Parallel in roko:** `crates/roko-runtime/src/pipeline_state.rs` implements executor snapshots
via `WorkflowConfig`, persisted to `.roko/state/executor.json`. The `--resume` flag reads this
state and continues.

### 3.5 Anti-Pattern Checks (grep-based)

Fast no-cargo checks catching common LLM mistakes. Milliseconds per batch.

| # | Pattern | What it catches | False positive rate |
|---|---------|----------------|-------------------|
| AP-1 | "Just Shell Out To Claude" | Raw `Command::new("claude")` | Low |
| AP-2 | Inline Prompt Strings | Hardcoded prompt text | Low |
| AP-3 | Build Another Runtime | New event loops | Low |
| AP-4 | Features in Wrong Layer | Decision logic in effect driver | Medium |
| AP-5 | Hardcoded Role Behavior | if/else branches for roles | Low |
| AP-6 | Feedback as Afterthought | Missing episode recording | Low |
| AP-7 | Copy-Paste Between Runtimes | Duplicate implementations | Low |
| AP-8 | Prefixing Unused `_vars` | Signature errors | Medium |
| AP-9 | Bolting Multi-Task onto Single-Task | Mixed state machines | Low |
| AP-10 | Hardcoded localhost | Config defaults | High (2-3% false positive) |

AP-10 was the worst offender for false positives -- legitimate config defaults like
`bind_address = "127.0.0.1"` triggered the check. Needed per-batch exemptions.

### 3.6 Auto-Pick Cherry-Picker

Background process watching for completed work and integrating into working branch. "Conveyor belt"
pattern: agents produce, picker integrates, you review.

**How it works:** A watcher process polls `.result` files. When a batch completes, it:
1. Reads the commit hash from the `.result` file
2. Cherry-picks that commit onto the integration branch
3. If the cherry-pick has conflicts, marks it as needing manual resolution
4. Updates a summary file with pick status

**Conflict rate:** ~30% in large runs with shared files, mostly auto-resolved.

### 3.7 Cumulative Context Sections

Telling each batch what other batches changed in shared files dramatically reduced merge conflicts.
Each batch's context pack includes a section like:

```
## Changes by other batches in this wave:
- BATCH-03 modified `crates/roko-core/src/types.rs`: Added `TaskDomain` enum variant `Chain`
- BATCH-07 modified `crates/roko-core/src/types.rs`: Added `Display` impl for `TaskTier`
```

This reduced merge conflicts by ~40% compared to runs without cumulative context.

### 3.8 "WIRE, Don't Build" Principle

**The most important insight in the entire project.** Roko had 177K LOC of infrastructure.
The fix for interactive chat was 4 CLI flags. Before building anything new, check if existing
code just needs to be called.

**Concrete example:** The `SystemPromptBuilder` at
`crates/roko-compose/src/system_prompt_builder.rs` had 9 layers of prompt assembly, rich templates
in `crates/roko-compose/src/templates/`, and comprehensive tests. Zero callers from any live
code path. The entire builder was "built but never connected." Wiring it required changing
~20 lines in `orchestrate.rs`.

---

## 4. What Didn't Work -- Anti-Patterns and Failures

### 4.1 Over-Engineering Before Understanding the Problem

Solutions A through 3 proposed increasingly complex architectures before anyone read Mori's
dispatch code and discovered the difference was 4 CLI flags.

**Cost:** ~3 weeks of design work across 5 solution proposals. The actual fix took 2 days.

**Lesson:** Read the reference implementation before designing a replacement. "Why does Mori work?"
is a faster question than "How should we architect our replacement?"

### 4.2 Per-Batch Compilation

15-40 minutes per batch when code writing was 1-5 minutes. 10x overhead.

**Root cause:** The Rust compiler's incremental compilation creates ~2GB of target/ artifacts per
worktree. With 40 parallel worktrees, that's 80GB. Machines ran out of disk. Even when space was
available, cargo's build lock serialized compilation across worktrees sharing a workspace.

### 4.3 Anti-Pattern False Positives

AP-10 (hardcoded localhost) caught legitimate config defaults. Blanket regex too crude. Needed
per-batch exemptions. 2-3% of batches wasted on retries from false positive failures.

### 4.4 Agents Ignoring Instructions

~5% of agents ignored "do not run cargo" and ran builds anyway. 30-minute batches instead
of 3-minute. Root cause: the instruction was in a context file, not the system prompt. Moving
it to the first line of the prompt reduced non-compliance to <1%.

### 4.5 Trying to Fix Everything at Once

The 87-batch converge runner was harder to manage than 3 focused 3-8 batch runs. Problems
cascaded: batch 12 broke an interface that batches 13-25 depended on. In a focused run, you'd
catch this in wave 2. In a mega-run, you don't see it until 40 batches have built on the broken
interface.

### 4.6 Auto Merge-Back

Automated merge-back of completed batches into the source branch caused problems when conflicts
needed human judgment. Failed merges left weird state (detached HEAD, partially applied patches).
Manual cherry-pick with monitoring was more reliable.

### 4.7 Generated Plans Without Validation

Plans proposed greenfield crates duplicating existing functionality. No mechanism to check overlap
with existing code. Example: a plan proposed creating `roko-exec` for process management when
`crates/roko-runtime/src/effect_driver.rs` already had `ProcessSupervisor`.

### 4.8 Trusting Subprocess Exit Codes as Success

Claude CLI exiting 0 does not mean the artifact is correct. Must split process outcome from
artifact outcome.

**Example:** Agent asked to implement `ModelCallService.estimate_cost()`. Claude exits 0.
Output contains `pub fn estimate_cost(&self) -> f64 { 0.0 }`. Exit code says success.
Artifact says "I gave up."

**Fix in roko:** The `PipelineState` in `crates/roko-acp/src/pipeline.rs` has separate
`AgentCompleted` vs `GatesPassed` events. Agent completion triggers gates, not commit.

### 4.9 Memory Leaks in Long-Running Orchestration

The dogfood session revealed 9.5-11.5GB RSS after 17 minutes. Root causes:
- Unbounded `Vec<RuntimeEvent>` growing without flush
- Enrichment artifacts (parsed ASTs, research results) held for entire run
- History context for multi-turn agents never compacted

**Fix:** Runner v2 in `crates/roko-runtime/src/workflow_engine.rs` flushes per-task and compacts
history. The ACP runner in `crates/roko-acp/src/runner.rs` caps history at
`MAX_HISTORY_ASSISTANT_BYTES = 10_240`.

### 4.10 Config Schema Divergence

`roko init` writes `[[gate]]` TOML arrays. `roko plan run` reads `[gates]` table. Two different
schemas for the same concept. `roko init`-generated gates are invisible to plan execution.

---

## 5. Speed, Cost, and Reliability Data

### Speed

| Metric | Value | Notes |
|--------|-------|-------|
| Batch time with builds | 15-40 minutes | Dominated by `cargo check` |
| Batch time without builds | 1-5 minutes | Pure code generation |
| Wave gate (cargo check) | 3-8 minutes | Depends on change scope |
| Cherry-pick (manual) | ~30 seconds each | Including conflict check |
| **195 batches total (no-build mode)** | **~6 hours** | Peak 40 parallel sessions |
| 195 batches if per-batch builds enabled | ~50+ hours | Theoretical estimate |
| Post-run cleanup (compile + clippy + tests) | ~30 minutes | Full workspace validation |
| Optimal parallelism (MacBook Pro) | 15-40 concurrent sessions | Bottleneck: memory |

### Cost

| Component | Cost | Notes |
|-----------|------|-------|
| Context packs (195 batches, ~4K tokens each) | ~$0.12 | Negligible |
| gpt-5.4-mini per batch (~2K output tokens) | ~$0.10-0.50 total | Very cheap |
| Wasted cost from build-ignoring agents (5%) | ~10x per-batch | 5% of agents |
| False positive AP failures | 2-3% batches | Retry cost |
| Total mega-parity run cost | ~$1.50-3.00 | 195 batches, sub-$3 |

### Reliability

| Metric | Value |
|--------|-------|
| First-try batch success rate (no-build, well-prompted) | ~95% |
| Agent instruction non-compliance | 5% (drops to <1% with prompt placement fix) |
| Anti-pattern false positive rate | 2-3% |
| Merge conflict rate in large runs | ~30% (mostly auto-resolved) |
| Complete run success (all batches) | ~85% |
| Wave gate pass rate | ~90% first try |

---

## 6. Architecture and Design Lessons

### Fundamental Principles

1. **"Built but never connected" is the project's pattern.** Before building anything, search for
   existing implementations. `grep -rn 'FunctionName' crates/ --include='*.rs'` is mandatory.

2. **177K LOC vs 300 LOC.** Mori's working agent dispatch is 300 LOC. Roko has 177K LOC of
   infrastructure. Sophistication without connection is worthless.

3. **Diagnostic progression is essential.** Individual bugs -> systemic root causes -> grand
   architectures -> actually reading reference code -> realization that wiring is the answer.

4. **Two outcomes, not one.** Process outcome (exit 0) != artifact outcome (valid plan).
   Only emit positive learning when both pass.

5. **Context-root contracts matter.** Wrong working directory = confidently wrong work.
   The `current_dir` field on `ClaudeCliAgent` must match the worktree, not the workspace root.

6. **Isolation is non-negotiable.** Agents in separate worktrees. No shared mutable state.
   `WorktreeManager::create()` in `crates/roko-orchestrator/src/worktree.rs` enforces this.

7. **Context handoff is the hard problem.** Telling agent B what agent A changed is more important
   than telling agent B what to do. The cumulative context sections proved this.

8. **Result files are coordination.** Not messages, not shared memory, not databases. Simple files
   on disk. The `--continue` pattern scales to any failure mode.

9. **Never delete branches or worktrees automatically.** Disk is cheap; lost work is expensive.
   This is also a hard project rule -- see MEMORY.md.

10. **Manual intervention is a feature.** System must make it easy to read status, unblock, kill,
    restart. The `.result` files + `--continue` flag enable this.

### Operational Principles

11. **Defer compilation to wave boundaries.** 10-100x cost savings. The per-batch compilation
    overhead dominates everything else.

12. **Zero is not unknown.** Unparseable usage data should store `None`, not zero. Zero means
    "free." The `UsageObservation` struct in `crates/roko-agent/src/usage.rs` gets this right
    with `Option<u64>` fields.

13. **Cost events should not be duplicated.** One cost event per agent attempt. The
    `ModelCallService` in `crates/roko-agent/src/model_call_service.rs` uses `request_seq:
    AtomicU64` to ensure unique event IDs.

14. **Learning must be gated on artifact quality.** Not just subprocess success. The
    `EpisodeLogger` at `crates/roko-learn/src/episode_logger.rs` should only record positive
    observations when gates pass.

15. **Prompt placement matters for compliance.** Critical instructions in the system prompt
    get 99% compliance. Same instructions in a context file get 95%. Same instructions in the
    middle of the prompt get 85%.

16. **Model selection affects batch cost more than batch count.** gpt-5.4-mini at $0.01/batch
    vs gpt-5.4 at $0.30/batch. For 195 batches, that's $2 vs $60. Use the cheapest model that
    produces correct code for the task complexity.

---

## 7. The Three Runtime Problem

The fundamental architectural issue: three runtimes that don't share code.

| Runtime | File | LOC | Status | Architecture |
|---------|------|-----|--------|-------------|
| ACP pipeline | `crates/roko-acp/src/runner.rs` | ~1,200 | Active (Zed) | Pure state machine + effect driver |
| Runner v2 | `crates/roko-runtime/src/workflow_engine.rs` | ~3,000 | Active (CLI) | Event-driven tokio select! |
| Orchestrate.rs | `crates/roko-cli/src/orchestrate.rs` | 22,522 | Dead (no callers) | Batch monolith |

**The most sophisticated features are only wired in the dead path.** The live paths record
nothing durable. The system cannot learn from 99% of its actual runs.

### Feature Matrix

| Feature | ACP (runner.rs) | Runner v2 | orchestrate.rs |
|---------|----------------|-----------|---------------|
| Episode recording | Yes | No | Yes |
| Cost tracking | Yes | No | Yes |
| CascadeRouter updates | Yes | No | Yes |
| Adaptive thresholds | Yes (EMA) | No | Yes (full SPC) |
| Prompt experiments | No | No | Yes |
| Playbook queries | Yes | No | Yes |
| Knowledge injection | Yes | No | Yes |
| Feedback recording | Yes | No | Yes |
| Streaming output | Yes | Yes | No (batch) |
| Multi-task plans | No | Yes | Yes |
| DAG execution | No | Yes | Yes |
| Worktree isolation | No | Yes | Yes |
| Resume/checkpoint | No | Yes | Yes |
| Memory management | Yes (capped) | Yes (per-task) | No (leaks) |

### The Learning Loop Problem

| Component | Persistence File | Live Callers |
|-----------|-----------------|-------------|
| Episode logger | `.roko/episodes.jsonl` | ACP only |
| CascadeRouter | `.roko/learn/cascade-router.json` | ACP only |
| Efficiency events | `.roko/learn/efficiency.jsonl` | ACP only |
| Prompt experiments | `.roko/learn/experiments.json` | Zero |
| Playbook store | `.roko/learn/playbooks/*.json` | ACP (read only) |
| Conductor bandit | `.roko/learn/conductor.json` | Zero |
| Cost tracking | `.roko/learn/costs.jsonl` | ACP only |
| Knowledge routing | Neuro store queries | ACP only |
| Adaptive thresholds | `.roko/learn/gate-thresholds.json` | ACP (basic EMA only) |

The ACP runner has wired most learning components. Runner v2 has wired none.
orchestrate.rs has the most complete wiring but is never called.

---

## 8. Innovations Worth Preserving

### 8.1 Pure State Machine + Effect Driver (ACP)

The `PipelineState::step()` function in `crates/roko-acp/src/pipeline.rs` is a pure function:
`(state, event) -> (new_state, action)`. No I/O, no side effects, fully testable with 10 unit
tests covering all transitions. The runner at `crates/roko-acp/src/runner.rs` performs the
actual side effects.

This pattern should be the foundation for all execution in roko.

### 8.2 Knowledge-Informed Dispatch

The `query_dispatch_knowledge()` function in `crates/roko-acp/src/knowledge.rs` queries both
the neuro store and the playbook store before every dispatch, then injects results as prompt
context. This means agents get smarter over time as the knowledge store grows.

### 8.3 ModelCallService as Single Dispatch Path

`crates/roko-agent/src/model_call_service.rs` wraps provider dispatch with model resolution,
cost tracking, event emission, feedback recording, L1 caching, budget enforcement, convergence
detection, and thinking caps. Every model call should go through this service.

### 8.4 Convergence Detection

The `convergence` field on `ModelCallService` detects when an agent is producing near-identical
outputs repeatedly, indicating it is stuck. This prevents burning budget on unproductive loops.

### 8.5 Agent Composition Operators

`crates/roko-agent/src/composition.rs` provides pipeline, parallel, conditional, and
mixture-of-agents patterns. The `SkillSelector` routes tasks by category, complexity, reasoning
level, speed priority, and quality profile to different agent branches.

---

## 9. Concrete Next Steps

### 9.1 Converge Runner v2 and ACP Learning

Runner v2 needs to call the same learning hooks that ACP calls. Specifically:
- `EpisodeLogger::log_episode()` after each task
- `CascadeRouter::observe()` with routing outcomes
- `AdaptiveThresholds::observe()` with gate outcomes

**Files to modify:**
- `crates/roko-runtime/src/workflow_engine.rs` -- add episode logging
- `crates/roko-runtime/src/effect_driver.rs` -- add cascade router updates

### 9.2 Wave-Level Gates for Multi-Agent Runs

The ACP pipeline runs gates per-agent. For multi-task plans, deferred wave-level gating
would be dramatically cheaper. The `PipelineState` could be extended with a `WaveGating`
phase that accumulates agent completions and gates the batch.

**Files to modify:**
- `crates/roko-acp/src/pipeline.rs` -- add wave-level phase
- `crates/roko-acp/src/runner.rs` -- implement wave gate execution

### 9.3 Kill the God Object

`orchestrate.rs` at 22K+ lines must be decomposed. The `PlanRunner` struct has 80+ fields.
Every dogfood fix touches it. Recommended decomposition:
- Extract gate execution into `crates/roko-gate/src/gate_executor.rs`
- Extract model routing into `crates/roko-learn/src/routing_executor.rs`
- Extract worktree management (already partially at `crates/roko-orchestrator/src/worktree.rs`)
- Extract context assembly into `crates/roko-compose/src/context_assembler.rs`
- Keep only action dispatch and state management in orchestrate.rs

### 9.4 Context Pack Standards

Standardize the 5-file context pack format used by the mega-parity runner for all agent dispatch:
1. Agent behavior rules (role-specific)
2. Protocol/API reference
3. Architecture overview
4. Type reference (relevant types only)
5. Existing patterns to follow

The `PromptAssemblyService` at `crates/roko-compose/src/prompt_assembly_service.rs` should
generate these packs from the 9-layer builder output.
