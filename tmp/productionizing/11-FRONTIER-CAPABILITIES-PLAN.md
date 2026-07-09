# Implementation Plan: Frontier Capabilities (from R3)

> Source: `docs/v2-depth/RESEARCH-PROMPT-3.md`
> Scope: Wire existing code + implement missing pieces for self-bootstrapping, metacognition,
> emergent economics, cross-system composition, temporal reasoning, agents-as-infrastructure,
> math discovery, and self-play/curriculum capabilities.

## What Already Exists (DO NOT REBUILD)

Before implementing ANYTHING, search the codebase. 31,000+ LOC of frontier capability code
already exists across these crates:

| Area | LOC | Key Files | Status |
|------|-----|-----------|--------|
| Self-bootstrapping | ~3,500 | `roko-learn/src/{curriculum,skill_library,error_pattern_store,playbook,causal}.rs` | Mostly wired |
| Metacognition | ~4,200 | `roko-daimon/src/{somatic_ta,goals,life_review}.rs`, `roko-agent/src/introspection.rs` | Wired |
| Emergent economics | ~2,800 | `roko-chain/src/{marketplace,reputation_registry,agent_registry,korai_token,trace_rank}.rs` | Built, unwired |
| Cross-system composition | ~5,000 | `roko-agent/src/{composition,multi_pool}.rs`, `roko-serve/`, `orchestrate.rs` | Wired |
| Temporal reasoning | ~2,000 | `roko-neuro/src/temporal.rs`, `roko-learn/src/{latency,kalman,prediction}.rs` | Wired |
| Agents-as-infrastructure | ~3,500 | `roko-agent/src/{lifecycle,dispatch_resolver,provider,task_runner}.rs`, `roko-agent-server/` | Wired |
| Math discovery | ~4,000 | `roko-primitives/src/{hdc,codebook,tda,manifold,sheaf,tropical}.rs`, `roko-learn/src/causal.rs` | Partially wired |
| Self-play/curriculum | ~6,500 | `roko-dreams/src/{imagination,rehearsal,threat,cycle,replay}.rs`, `roko-learn/src/{bandits,active_inference,adversarial}.rs` | Mostly wired |

---

## Anti-Patterns (READ FIRST)

1. **DO NOT rebuild what exists.** Run `grep -rn 'StructName\|function_name' crates/ --include='*.rs' | grep -v target/` before writing ANY new code.
2. **DO NOT create new crates.** All frontier work fits in existing crates. If you think you need a new crate, you're wrong — search harder.
3. **DO NOT wire chain code into runtime.** Chain integration is Phase 2+. Do not add blockchain dependencies to the hot path.
4. **DO NOT add `unwrap()` or `expect()` in any new code.** Use `?` operator or `anyhow::Context`. See `07-ANTI-PATTERNS.md` §2.
5. **DO NOT silently swallow errors.** Every `let _ = ...` needs a `tracing::warn!`. See `07-ANTI-PATTERNS.md` §3.
6. **DO NOT hardcode model names.** Always read from config. See `07-ANTI-PATTERNS.md` §5.
7. **DO NOT skip pre-commit checks.** `cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace` before every commit.
8. **Wire, don't build.** If existing code just needs to be called from orchestrate.rs, do that instead of building something new.

---

## Tasks

### F1: Wire ADAS (Autocatalytic Design) into Curriculum Scheduler

**Priority**: High
**Scope**: ~100 LOC changes across 2 files
**Dependencies**: None
**Blocks**: F2

**Context**: `crates/roko-learn/src/adas.rs` implements Autocatalytic Design of AI Systems (LEARN-08) — an optimization loop that discovers better agent configurations. `crates/roko-learn/src/curriculum.rs` has `CurriculumScheduler` with 4 modes (EasyFirst, HardFirst, Interleaved, Adaptive). ADAS is built but never called.

**What to do**:
1. Find ADAS entry point:
   ```bash
   grep -rn 'pub fn\|pub async fn' crates/roko-learn/src/adas.rs --include='*.rs'
   ```
2. Find CurriculumScheduler integration point:
   ```bash
   grep -rn 'CurriculumScheduler\|curriculum' crates/roko-learn/src/lib.rs crates/roko-cli/src/orchestrate.rs
   ```
3. Add ADAS as a 5th curriculum mode or as a meta-optimizer that wraps Adaptive mode.
4. Wire ADAS output into the curriculum scheduler's difficulty model updates.
5. Ensure ADAS persists its state to `.roko/learn/adas.json` (follow the pattern in `cascade_router.rs` for JSON persistence).

**What NOT to do**:
- Don't create a new crate for this.
- Don't modify the existing 4 curriculum modes — add alongside them.
- Don't add runtime dependencies on external services.

**Verification**:
```bash
# Must compile
cargo build -p roko-learn

# Must pass tests
cargo test -p roko-learn

# Verify ADAS is reachable from CLI
grep -rn 'adas\|Adas\|ADAS' crates/roko-cli/src/ --include='*.rs'

# Verify persistence file is created after a run
ls -la .roko/learn/adas.json
```

---

### F2: Wire Research Pipeline into Orchestrator

**Priority**: High
**Scope**: ~150 LOC across 3 files
**Dependencies**: None
**Blocks**: None

**Context**: `crates/roko-learn/src/research_pipeline.rs` implements a Paper → Claim → Trial → Ledger pipeline for systematically testing research claims. It's built but has no orchestrator hook — no CLI command or dispatch path invokes it.

**What to do**:
1. Read the research pipeline's public API:
   ```bash
   grep -rn 'pub fn\|pub async fn\|pub struct' crates/roko-learn/src/research_pipeline.rs
   ```
2. Check if it's re-exported from the crate:
   ```bash
   grep -rn 'research_pipeline' crates/roko-learn/src/lib.rs
   ```
3. Add a `research trial` subcommand to `roko-cli` that invokes the pipeline, OR wire it as a post-plan hook in `orchestrate.rs` that runs after new playbooks are extracted.
4. The pipeline should:
   - Accept a claim (from playbook or manual input)
   - Generate a trial (task to test the claim)
   - Execute the trial via the existing plan runner
   - Record result in the ledger
5. Persist ledger to `.roko/learn/research-ledger.jsonl`.

**What NOT to do**:
- Don't duplicate the plan runner — use the existing `PlanRunner` to execute trials.
- Don't add new LLM calls — the trial execution should use existing agent dispatch.
- Don't skip serialization tests for the ledger format.

**Verification**:
```bash
cargo build -p roko-learn
cargo test -p roko-learn
# If CLI subcommand added:
cargo run -p roko-cli -- research --help | grep -i trial
```

---

### F3: Wire Tropical Algebra and Sheaf Math into Routing

**Priority**: Medium
**Scope**: ~200 LOC across 3 files
**Dependencies**: None
**Blocks**: None

**Context**: `crates/roko-primitives/src/tropical.rs` implements tropical (max-plus) algebra with `TropicalF64`, polynomials, and tropical attention. `crates/roko-primitives/src/sheaf.rs` implements cellular sheaves with coboundary operators, Laplacian, and inconsistency scoring. Both are built and tested but not called from the orchestrator or routing layer.

**What to do**:
1. Read the tropical algebra API:
   ```bash
   grep -rn 'pub fn\|pub struct' crates/roko-primitives/src/tropical.rs
   ```
2. Read the sheaf API:
   ```bash
   grep -rn 'pub fn\|pub struct' crates/roko-primitives/src/sheaf.rs
   ```
3. Identify where in the routing pipeline these could add value:
   - **Tropical attention**: Could replace/augment the attention bidder weights in `orchestrate.rs` context composition. Tropical semiring gives max-plus semantics (pick the best, not average).
   - **Sheaf inconsistency**: Could detect when different routing signals disagree (cascade router vs role config vs task hint). High inconsistency = escalate to human or use most conservative model.
4. Wire tropical attention into `AttentionBidder` or `ContextComposer` in orchestrate.rs.
5. Wire sheaf inconsistency into the routing decision logging in `cascade_router.rs`.

**What NOT to do**:
- Don't replace existing routing — add as supplementary signal.
- Don't make this a hard dependency — if tropical/sheaf computation fails, fall through to existing behavior.
- Don't add new allocations in the hot path — these should be computed once per routing decision, not per token.

**Verification**:
```bash
cargo build --workspace
cargo test -p roko-primitives
cargo test -p roko-learn
# Check that routing logs include inconsistency scores:
grep -rn 'inconsistency\|sheaf\|tropical' crates/roko-learn/src/routing_log.rs
```

---

### F4: Wire Phase 2 Daimon Stubs (Fatigue, Contagion, Behavioral State)

**Priority**: Medium
**Scope**: ~150 LOC across 2 files
**Dependencies**: None
**Blocks**: None

**Context**: `crates/roko-daimon/src/phase2_stubs.rs` has fatigue detection, behavioral state tracking, contagion tracking, and contrarian tracking — all built as stubs, never called from the runtime.

**What to do**:
1. Read the stubs:
   ```bash
   grep -rn 'pub fn\|pub struct' crates/roko-daimon/src/phase2_stubs.rs
   ```
2. Find where DaimonState is loaded in orchestrate.rs:
   ```bash
   grep -rn 'DaimonState\|daimon' crates/roko-cli/src/orchestrate.rs
   ```
3. Wire fatigue detection into the per-task loop: after each task, update fatigue state. If fatigue exceeds threshold, insert a rest period (delay before next dispatch) or switch to a simpler model.
4. Wire behavioral state tracking into episode logging: record the agent's behavioral phase (Camel/Lion/Child from Nietzsche's framework in `mortality.rs`) per episode.
5. Contagion tracking: record when one agent's failure mode spreads to others sharing the same model/prompt. Flag in routing decisions.

**What NOT to do**:
- Don't make fatigue blocking — it should be advisory (logged + metric), not prevent task dispatch.
- Don't add sleep/delay calls in the hot path — schedule rest periods as metadata on next dispatch.
- Don't implement contagion prevention (that's a different task) — just detect and log.

**Verification**:
```bash
cargo build -p roko-daimon
cargo test -p roko-daimon
# Verify fatigue state is updated per-task:
grep -rn 'fatigue' crates/roko-cli/src/orchestrate.rs
# Verify behavioral state appears in episodes:
grep -rn 'behavioral_state\|behavioral_phase' crates/roko-learn/src/episode_logger.rs
```

---

### F5: Add Open-Endedness Mechanics (Novelty Search)

**Priority**: Low
**Scope**: ~300 LOC new code in existing crate
**Dependencies**: F1 (ADAS wiring)
**Blocks**: None

**Context**: The codebase has no explicit novelty search or environment randomization. The curriculum scheduler adapts difficulty but doesn't seek novel task types. For true open-endedness, agents need to discover new problem categories they haven't seen before.

**What to do**:
1. Add a `novelty_search.rs` module to `crates/roko-learn/src/`:
   - Define `NoveltyArchive`: stores HDC fingerprints of all previously solved task types.
   - `is_novel(fingerprint: &HdcVector) -> f64`: returns novelty score (1.0 - max_similarity to archive).
   - `add_to_archive(fingerprint: HdcVector)`: insert after successful completion.
2. Wire into curriculum scheduler's Adaptive mode:
   - When selecting next task, boost priority of novel tasks (novelty_score > 0.7).
   - Don't ONLY select novel tasks — balance exploitation vs exploration (use UCB1 from existing bandits.rs).
3. Use existing `hdc_fingerprint.rs` to generate fingerprints — DO NOT create a new fingerprinting system.
4. Persist archive to `.roko/learn/novelty-archive.bin` (binary serialization of HDC vectors for speed).

**What NOT to do**:
- Don't create a new HDC implementation — use `roko-primitives/src/hdc.rs`.
- Don't create a new bandit — use `roko-learn/src/bandits.rs`.
- Don't make novelty search mandatory — it should be opt-in via config (`learning.novelty_search = true`).
- Don't store raw task descriptions in the archive — only HDC fingerprints (for size and privacy).

**Verification**:
```bash
cargo build -p roko-learn
cargo test -p roko-learn

# Unit test: create 10 fingerprints, verify novelty scores decrease as archive fills
# Unit test: verify archive persists and reloads correctly
# Integration: after a plan run, check that novelty archive grows
ls -la .roko/learn/novelty-archive.bin
```

---

### F6: Wire Dynamic Agent Spawning Based on Task Requirements

**Priority**: Medium
**Scope**: ~200 LOC across 3 files
**Dependencies**: None
**Blocks**: None

**Context**: The orchestrator currently dispatches one agent per task sequentially within a DAG wave. There's no mechanism to spawn multiple agents for a single task (e.g., 3 agents for parallel subtask execution). `crates/roko-agent/src/multi_pool.rs` has `MultiAgentPool` with warm-reuse, but it's not used for dynamic per-task parallelism.

**What to do**:
1. Read MultiAgentPool:
   ```bash
   grep -rn 'pub fn\|pub async fn\|pub struct' crates/roko-agent/src/multi_pool.rs
   ```
2. Read the orchestrator's task dispatch:
   ```bash
   grep -rn 'dispatch_agent_with\|execute_task' crates/roko-cli/src/orchestrate.rs | head -20
   ```
3. Add a `parallel_subtasks` field to task definitions (in `roko-core`'s task types):
   - If `parallel_subtasks > 1`, the orchestrator splits the task prompt into N sub-prompts and dispatches N agents from the pool.
   - Results are merged using the existing `MergeStrategy` from `composition.rs` (Concatenate, Aggregate, Vote, BestOfN).
4. Wire the merge strategy selection into the task config or use defaults:
   - Code tasks: BestOfN (gate selects the passing result)
   - Research tasks: Concatenate (combine findings)
   - Review tasks: Vote (majority consensus)

**What NOT to do**:
- Don't change the DAG execution order — parallel subtasks happen WITHIN a single DAG node.
- Don't spawn unbounded agents — cap at `agent.max_parallel` from config (default: 3).
- Don't duplicate the agent pool — use the existing MultiAgentPool.
- Don't change the gate pipeline — each subtask result goes through gates independently, then merge.

**Verification**:
```bash
cargo build --workspace
cargo test --workspace
# Verify the field exists:
grep -rn 'parallel_subtasks' crates/roko-core/src/ --include='*.rs'
# Verify pool usage:
grep -rn 'MultiAgentPool' crates/roko-cli/src/orchestrate.rs
```

---

### F7: Wire Collusion Detection into Marketplace

**Priority**: Low (Phase 2 prep)
**Scope**: ~80 LOC across 2 files
**Dependencies**: None
**Blocks**: None

**Context**: `crates/roko-chain/src/collusion.rs` has assignment graph clique analysis for detecting collusion rings among agents in the marketplace. It's built and tested but never called from the marketplace or orchestrator.

**What to do**:
1. Read the collusion detection API:
   ```bash
   grep -rn 'pub fn\|pub struct' crates/roko-chain/src/collusion.rs
   ```
2. Read where marketplace assigns agents:
   ```bash
   grep -rn 'assign\|hire\|dispatch' crates/roko-chain/src/marketplace.rs
   ```
3. Add a post-assignment check: after an agent is assigned a job, run collusion detection over recent assignment history. If a clique is detected, log a warning and apply the reputation penalty from `reputation_registry.rs` (–50% feedback weight for 30 days).
4. This should be a background check (async), not a blocking pre-assignment gate.

**What NOT to do**:
- Don't block assignments on collusion checks — detection is async + advisory.
- Don't integrate with the blockchain — this is an in-memory check for now (Phase 2 will anchor to chain).
- Don't modify the reputation registry's penalty table — use existing `CollisionFeedbackDilution`.

**Verification**:
```bash
cargo build -p roko-chain
cargo test -p roko-chain
# Verify collusion check is called:
grep -rn 'collusion' crates/roko-chain/src/marketplace.rs
```

---

## Execution Order

```
INDEPENDENT (can run in parallel):
  F1 (ADAS wiring)
  F2 (Research pipeline)
  F3 (Tropical/sheaf math)
  F4 (Daimon phase 2 stubs)
  F6 (Dynamic agent spawning)
  F7 (Collusion detection)

DEPENDS ON F1:
  F5 (Novelty search) — needs ADAS wired first
```

## Checklist

- [ ] F1: ADAS wired into CurriculumScheduler
- [ ] F2: Research pipeline callable from CLI or orchestrator hook
- [ ] F3: Tropical attention and sheaf inconsistency in routing
- [ ] F4: Fatigue, contagion, behavioral state tracked per-task
- [ ] F5: Novelty archive with HDC fingerprint-based discovery
- [ ] F6: Multi-agent parallel subtask dispatch via MultiAgentPool
- [ ] F7: Collusion detection wired into marketplace assignments
- [ ] All tasks pass: `cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings && cargo test --workspace`

## Operations baseline

Ship frontier changes only on top of a hardened deploy baseline: see **09-OPERATIONS-RUNBOOK.md** (health URLs, Railway vs `roko serve`, volumes) after **P1–P15** from **10-IMPLEMENTATION-PLAN.md**.
