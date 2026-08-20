# State Persistence Comparison: Mori vs Roko

## 1. Directory Structure

### Mori: `.mori/`

```
.mori/
  config.toml                    # Runtime configuration (models, routing, agents)
  config.toml.example
  queue.toml                     # Milestone/batch queue definitions
  costs.db                       # SQLite cost database (22 MB)
  index.db                       # SQLite code index (376 MB)
  mcp-config.json                # MCP server configuration
  artifacts/                     # (empty)
  cache/
    cargo-target/                # Dedicated build cache
    fastembed/                   # Embedding model cache
    ort/                         # ONNX runtime cache
  manual-repair/                 # Manual recovery scripts
  memory/
    episodes.jsonl               # Episode log (13 MB, 6579 episodes)
    playbook.toml                # Learned when/then rules (232 KB)
    efficiency.json              # Aggregate efficiency statistics
    efficiency-history.jsonl     # Efficiency snapshots over time (1.3 MB)
    dependencies.toml            # Inter-plan dependency graph (547 KB)
    fixtures.toml                # Test fixture definitions (235 KB)
    prompt-logs/                 # Per-task prompt dumps (299 entries)
    context-packs/               # Per-plan compressed context bundles (7375 entries)
    refresh-state.json           # Playbook/efficiency refresh watermark
  plans/                         # 171 plan directories
    <plan-name>/
      tasks.toml                 # Task definitions
      plan.md                    # Full plan document
      brief.md                   # Plan brief
      prd-extract.md             # Extracted PRD content
      decomposition.md           # Task decomposition
      research.md                # Pre-research
      rubric.md                  # Evaluation rubric
      review-tasks.toml          # Review agent tasks
      scribe-tasks.toml          # Scribe agent tasks
      verify-tasks.toml          # Verification tasks
      integration.md             # Integration notes
      dependency-manifest.toml   # Plan-local dependencies
      fixture-manifest.toml      # Plan-local fixtures
      reviews/                   # Review output
      testing-backlog.md         # Deferred test items
  runs/
    status.json                  # Live run status (version, plan, phase, PID)
    task-state.json              # Task-level crash recovery (64 KB)
    events.jsonl                 # Run event log (4.3 MB)
    run.pid                      # Process lock file
    mori.log                     # Runtime log
    audit-results.json           # Structural audit
    costs/                       # Cost summaries
    recovery/                    # Per-plan recovery snapshots (53 plans)
    recovery-backups/            # Backup recovery data
    status-archive/              # Rotated status snapshots (keep 10)
    events-archive/              # Rotated event logs (12 archives)
    task-state-archive/          # Rotated task state backups (14 archives)
    fixtures/                    # Runtime fixture state
    babysitter*.{log,jsonl,txt}  # Hang-detection babysitter state
    output/                      # Agent output capture
  runtime/
    agent-pids.json              # Live agent PIDs
    codex-home/                  # Codex agent home directory
  tools/                         # Custom tool definitions
```

### Roko: `.roko/`

```
.roko/
  GAPS.md                        # Canonical gap tracker (60 KB)
  INDEX.md                       # Directory index
  VERSION                        # Schema version marker
  engrams.jsonl                  # Signal log (133 KB) + .lock
  episodes.jsonl                 # Episode log (268 KB) + .lock
  events.jsonl                   # Runner event log (2.7 MB) + .lock
  gate-verdicts.jsonl            # Gate outcomes (14 KB) + .lock
  mcp-auto.json                  # Auto-discovered MCP config
  chat_history                   # Chat REPL history
  roko.log.YYYY-MM-DD           # Date-rotated runtime logs
  runner-stderr.log              # Captured agent stderr
  workspaces.json                # Workspace registry
  acp.log                       # ACP server log
  bench/
    index.jsonl                  # Benchmark index
    runs/                        # Benchmark run results
    suites/                      # Benchmark suite definitions (TOML)
  cache/                         # (empty; runtime cache)
  cold/
    YYYY-MM.jsonl                # Monthly cold archive partitions
    index.json                   # Cold archive index
  config/                        # (empty; uses roko.toml at workspace root)
  daimon/                        # Affect engine state
  dreams/
    counterfactuals.jsonl        # Counterfactual reasoning log (144 KB)
    cross-episode/               # Cross-episode dream consolidation
    dream-<timestamp>.json       # Dream cycle outputs (396 KB, 56 KB)
    journal.jsonl                # Dream journal
    staging-buffer.json          # Pre-dream staging
  graphs/                        # Graph execution state
  immune/                        # Safety immune system controls
    agent-controls.json.lock     # Agent control lock
  jobs/                          # Marketplace job state
  learn/
    cascade-router.json          # Model routing bandit state (70 KB)
    gate-thresholds.json         # Adaptive gate EMA thresholds
    efficiency.jsonl             # Per-turn efficiency events (61 KB)
    efficiency-summaries.jsonl   # Aggregate efficiency snapshots
    costs.jsonl                  # Cost ledger
    compounding.jsonl            # Compounding rewards
    provider-health.json         # Provider circuit-breaker state
    provider-model-outcomes.jsonl# Provider/model outcome history
    section-outcomes.jsonl       # Prompt section effectiveness (920 KB)
    knowledge-candidates.jsonl   # Knowledge admission candidates
    knowledge-feedback.jsonl     # Knowledge feedback signals
    knowledge-scores.json        # Knowledge tier scores
    knowledge-seeds.jsonl        # Knowledge seed ingestion
    experiments.json             # A/B experiment state
    experiment-winners.json      # Settled experiment winners
    attention-bidders.json       # Context attention bidder state
    dream-routing-advice.json    # Dream-informed routing
    local-rewards.json           # Local reward tracking
    latency-stats.json           # Latency statistics
    skills.json                  # Skill/competence model
    gateway.jsonl                # Inference gateway log
    wal.jsonl                    # Write-ahead log
    playbooks/                   # Per-playbook JSON files (14 entries)
  memory/                        # Legacy learning data (pre-unification)
    cascade-router.json
    episodes.jsonl.v2-legacy
    costs.jsonl
    ...
  metrics/                       # Periodic telemetry samples
  neuro/
    knowledge.jsonl              # Durable knowledge store (373 KB)
    knowledge-confirmations.jsonl# Knowledge confirmation evidence (1.2 MB)
    knowledge-candidates.jsonl   # Knowledge admission candidates
    knowledge-admission-decisions.jsonl
  notes/                         # Operator notes
  plans/                         # Plan definitions (separate from .roko/state)
  prd/                           # PRD lifecycle documents
  research/                      # Research artifacts
  sessions/                      # Session tracking
  state/
    state-snapshot.json          # Unified checksummed snapshot (86 KB)
    state-snapshot.json.*.bak    # Timestamped backup snapshots (~20 backups)
    run-ledger.jsonl             # Typed run ledger (6.2 MB)
    executor.json.bak.*          # Legacy executor backups
    daimon.json                  # Daimon affect state snapshot
    server-state.json            # Server/agent registry state
    dashboard-gen.json           # Dashboard generation counter
  subscriptions/                 # Event subscription state
  task-outputs/                  # Agent output capture
  templates/                     # Prompt templates
  traces/                        # Execution trace spans
  worktrees/                     # Worktree tracking
  workspaces/                    # Per-workspace isolation (66 entries)
  runtime/
    agent-pids.json              # Live agent PIDs
```

## 2. Core State Files

### 2.1 Run Status

**Mori** -- `runs/status.json` (JSON, atomic write):

```json
{
  "version": 2,
  "run_id": "20260819-100352",
  "batch_id": "current",
  "plans_total": 48,
  "plans_completed": 36,
  "plans_remaining": 12,
  "current_plan": "40-styx-architecture",
  "current_phase": "gating",
  "current_iteration": 2,
  "started_at": "2026-08-19T10:03:52.693907+00:00",
  "last_activity": "2026-08-19T10:27:10.920957+00:00",
  "pid": 38775,
  "anvil_pid": 0,
  "hang_threshold_seconds": 600
}
```

**Roko** -- No direct equivalent. Run status is embedded within the unified
`state/state-snapshot.json`. The TUI/server read state from the snapshot
or the typed `events.jsonl` stream. The `state/dashboard-gen.json` tracks
the generation counter for reactive dashboard invalidation.

**Difference**: Mori has a dedicated lightweight status file optimized for
external readers (shell scripts, babysitter). Roko embeds status within its
unified snapshot, requiring JSON parsing of the larger blob.

### 2.2 Task-Level Crash Recovery

**Mori** -- `runs/task-state.json` (JSON, atomic write, 64 KB):

```rust
pub struct TaskStateFile {
    pub version: u32,
    pub run_id: String,
    pub batch_branch: String,
    pub completed_tasks: Vec<String>,        // "plan:task" format
    pub in_flight: HashMap<String, String>,  // "plan:task" -> instance_id
    pub completed_plans: Vec<String>,
    pub total_tokens: TokenCount,
    pub plan_iterations: HashMap<String, u32>,
    pub merge_queue: Vec<String>,
    pub plans_since_refactor: usize,
    pub plans_since_integration_test: usize,
    pub active_worktrees: HashMap<String, String>,
    pub plan_phases: HashMap<String, String>,
    pub merge_in_progress: Option<MergeCheckpoint>,
    pub review_feedback: HashMap<String, Vec<String>>,
    pub correction_factor: Option<f64>,
    pub task_failure_counts: HashMap<String, u32>,
    pub skipped_tasks: Vec<String>,
}
```

Flat key format: `"plan:task"` strings. No fingerprinting or drift detection.

**Roko** -- `state/state-snapshot.json` contains `run_state_json` (embedded):

```rust
pub struct RunStateSnapshot {
    pub schema_version: u32,
    pub run_id: String,
    pub started_at_ms: u64,
    pub timestamp_ms: u64,
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_cost_usd: f64,
    pub total_agent_calls: usize,
    pub plan_costs: HashMap<String, f64>,
    pub task_usage: HashMap<String, TaskUsage>,
    pub accounted_usage_attempts: Vec<String>,
    pub completed_tasks: HashMap<String, Vec<String>>,
    pub failed_tasks: HashMap<String, Vec<String>>,
    pub skipped_tasks: HashMap<String, HashMap<String, SkippedReason>>,
    pub lifecycle: Option<RunnerLifecycleProjection>,
    pub snapshot_fail_streak: u32,
    pub fingerprints: Vec<TaskDefFingerprint>,
    // ... cascade_router_json, conductor state, replan ledger
}
```

Rich per-task `TaskUsage` with cost breakdown. HashMap-of-Vec structure
(plan -> task_ids) instead of flat "plan:task" strings. Includes typed
`SkippedReason` and `TaskDefFingerprint` for drift detection.

**Difference**: Roko adds task definition fingerprinting (SHA-256 of task
content) for strict resume validation. Mori trusts plan:task string identity.
Roko separates failed/completed/skipped into distinct typed maps; Mori keeps
one completed list and a failure count map.

### 2.3 Unified Snapshot (Roko only)

Roko wraps four state groups into one checksummed envelope:

```rust
pub struct StateSnapshot {
    pub version: u32,               // STATE_SNAPSHOT_VERSION = 2
    pub timestamp_ms: u64,
    pub executor_json: String,      // Opaque executor projection
    pub orchestrator_json: String,  // Merge queue, plan states
    pub run_state_json: String,     // Cost/token/completed-task counters
    pub gate_thresholds_json: String,// Adaptive gate EMA state
    pub checksum: String,           // SHA-256 over domain-separated fields
}
```

Written by a dedicated async `SnapshotWriter` thread with a bounded channel.
If multiple snapshots queue up, intermediate ones are skipped (latest-wins).
Serialization is budget-capped at 16 MB (`MAX_DURABLE_RUNNER_PROJECTION_BYTES`).
Backup files are timestamped: `state-snapshot.json.<epoch_ms>.bak`.

Mori has no equivalent. Its state is spread across `status.json`,
`task-state.json`, and the events log, with no integrity checksum.

## 3. Episode Logging

### Mori: `.mori/memory/episodes.jsonl`

```json
{
  "id": "07-terminal-protocol-views:T7:2026-03-22T21:20:38.448652+00:00",
  "timestamp": "2026-03-22T21:20:38.448654+00:00",
  "plan_id": "07-terminal-protocol-views",
  "task_id": "T7",
  "role": "implementer",
  "model": "claude-sonnet-4-6",
  "files_changed": [],
  "input_tokens": 14,
  "output_tokens": 1950,
  "cost_usd": 0.339,
  "gate_passed": true,
  "iterations": 1,
  "duration_secs": 85,
  "error_signature": null,
  "reflection": null
}
```

Fields: id, timestamp, plan_id, task_id, role, model, provider, files_changed,
input_tokens, output_tokens, cost_usd, gate_passed, iterations, duration_secs,
error_signature, reflection. Flat structure, one line per task completion.

### Roko: `.roko/episodes.jsonl`

```json
{
  "kind": "",
  "id": "ep_04224a20f71d0985",
  "timestamp": "2026-07-11T08:15:21.319702Z",
  "agent_id": "E01-T01",
  "task_id": "E01-T01",
  "input_signal_hash": "",
  "output_signal_hash": "",
  "episode_id": "ep_04224a20f71d0985",
  "agent_template": "",
  "model": "o3-mini",
  "backend": "codex-cli",
  "trigger_kind": "",
  "trigger_signal_hash": "",
  "started_at": "2026-07-11T08:15:21.319702Z",
  "completed_at": "2026-07-11T08:15:21.319702Z",
  "duration_secs": 33.869,
  "gate_verdicts": [],
  "usage": {
    "input_tokens": 0,
    "output_tokens": 0,
    "cache_read_tokens": 0,
    "cache_write_tokens": 0,
    "cost_usd": 0.0,
    "cost_usd_without_cache": 0.0,
    "wall_ms": 33869
  },
  "success": false,
  "turns": 0,
  "tokens_used": 0,
  "external_actions": [],
  "failure_reason": null,
  "reflection": null,
  "reasoning_summary": null,
  "hdc_fingerprint": "base64...",
  "emotional_tag": null,
  "headline": false,
  "extra": { "plan_id": "E01-execution-engine" }
}
```

Significantly richer record: separate `usage` object with cache token
breakdown, HDC (Hyperdimensional Computing) fingerprint per episode,
signal hashes for linking to the engram DAG, backend/template/trigger
metadata, emotional tags, and structured gate verdicts.

**Difference**: Roko episodes are ~10x more fields, include HDC fingerprints
for similarity search, and connect episodes to the Signal DAG via hash links.
Mori episodes are flat and self-contained.

## 4. Learning Data Persistence

### Mori

| File | Format | Purpose |
|---|---|---|
| `memory/playbook.toml` | TOML array | Learned when/then rules with trigger tags/files, context text, confidence, validated count |
| `memory/efficiency.json` | JSON | Aggregate per-provider/model/route efficiency statistics |
| `memory/efficiency-history.jsonl` | JSONL | Timestamped efficiency snapshots (top provider/model/route per snapshot) |
| `memory/dependencies.toml` | TOML | Inter-plan crate/fixture dependencies with downstream refs |
| `memory/fixtures.toml` | TOML | Test fixture definitions |
| `memory/context-packs/` | Per-plan dirs | Pre-assembled context bundles for agent invocation |
| `memory/prompt-logs/` | Per-task files | Logged prompt content for analysis |
| `memory/refresh-state.json` | JSON | Watermark: last episode count when playbook/efficiency were refreshed |

Single `playbook.toml` with all rules inline. Learning loop runs on a
configurable episode-count watermark.

### Roko

| File | Format | Purpose |
|---|---|---|
| `learn/playbooks/` | Per-playbook JSON | Individual playbook files (dream-derived, hand-authored, compile-check patterns) |
| `learn/cascade-router.json` | JSON | Multi-armed bandit model routing state (70 KB; per-model-slug history) |
| `learn/gate-thresholds.json` | JSON | Per-rung EMA pass rates for adaptive gate thresholds |
| `learn/efficiency.jsonl` | JSONL | Per-turn efficiency events with full cost/token/timing |
| `learn/efficiency-summaries.jsonl` | JSONL | Periodic aggregate summaries |
| `learn/costs.jsonl` | JSONL | Cost ledger entries |
| `learn/section-outcomes.jsonl` | JSONL | Per-prompt-section outcome tracking (920 KB) |
| `learn/provider-health.json` | JSON | Per-provider circuit-breaker state (consecutive failures, windows) |
| `learn/provider-model-outcomes.jsonl` | JSONL | Provider/model outcome telemetry |
| `learn/experiments.json` | JSON | Active A/B experiment assignments |
| `learn/experiment-winners.json` | JSON | Settled experiment results |
| `learn/knowledge-*.jsonl` | JSONL | Knowledge admission, feedback, seeds, scores |
| `learn/attention-bidders.json` | JSON | Context attention bidder weights |
| `learn/dream-routing-advice.json` | JSON | Dream-cycle-informed routing advice |
| `learn/local-rewards.json` | JSON | Local reward accumulation |
| `learn/skills.json` | JSON | Skill/competence model state |
| `learn/wal.jsonl` | JSONL | Write-ahead log for crash-safe learning updates |

**Difference**: Mori concentrates learning in one `playbook.toml` and one
`efficiency.json`. Roko decomposes learning into ~20 specialized files with
separate concerns (routing, gating, prompt sections, knowledge tiers,
experiments, provider health). Roko's playbook rules are individual JSON
files in a directory rather than one monolithic TOML. Roko also persists
dream-derived routing advice and A/B experiment state, which Mori lacks.

## 5. Signal / Engram Log (Roko only)

`.roko/engrams.jsonl` -- the core Signal log. Each entry is a full `Engram`:

```json
{
  "id": "f9fde5ae...",
  "kind": "gate_verdict",
  "body": {
    "format": "json",
    "data": { "duration_ms": 112652, "gate": "gate-pipeline:default", "passed": true, ... }
  },
  "created_at_ms": 1785941413804,
  "decay": { "kind": "none" },
  "provenance": {
    "author": "runner/gate",
    "trust": 1.0,
    "taint": { "kind": "clean" },
    "taint_level": "Public"
  },
  "score": { "confidence": 0.5, "novelty": 0.0, "utility": 0.0, ... },
  "lineage": [],
  "tags": { "pulse_seq": "0", "pulse_topic": "gate.verdict.emitted" },
  "balance": 1.0,
  "status": "working",
  "access_count": 0,
  "demurrage_paid": 0.0
}
```

Mori has no equivalent. Gate verdicts, events, and learning signals are
separate files. Roko's Signal log is a unified append-only DAG with
content-addressed IDs, provenance tracking, trust/taint metadata, scoring,
decay policies, and demurrage (economic primitives for signal lifecycle).

## 6. Cost Tracking

**Mori**: SQLite database (`costs.db`, 22 MB) + JSON summary (`runs/costs/summary.json`).
The summary had a floating-point overflow bug producing e+58 values.

**Roko**: JSONL append log (`learn/costs.jsonl`) + per-plan `plan_costs` and
per-task `task_usage` in the run-state snapshot. No SQLite dependency. Cost
is tracked per-turn in `efficiency.jsonl` with input/output/cache token
breakdowns. The unified snapshot embeds `total_cost_usd` and `total_tokens_in/out`.

**Difference**: Mori uses SQLite for durable cost storage; Roko uses JSONL
files and JSON snapshots. Roko avoids the SQLite dependency at the cost of
slower aggregate queries. Roko has per-turn granularity with cache-aware cost
calculation; Mori tracks per-task aggregates.

## 7. Resume Mechanism

### Mori

1. On startup, `PersistenceManager::prepare_new_run()` checks for `run.pid`
   and `status.json`:
   - If `task-state.json` exists, treat as resumable regardless of events
   - If events exist but run is not marked complete, treat as resumable
   - Otherwise archive stale artifacts
2. `cleanup_stale_artifacts()` removes `.tmp`, `.corrupt` files and prunes
   dead git worktrees
3. `check_stale_pid()` detects stale processes, kills them (SIGTERM then
   SIGKILL after 300ms), and also `pkill`s leftover codex processes
4. `load_task_state()` reads the `TaskStateFile` which has flat completed
   task lists and in-flight tracking
5. Resume trusts plan:task string identity -- no task content fingerprinting
6. The executor tests verify resume from various phases (gating, reviewing,
   autofixing) by redispatching the appropriate actions

On clean exit, `cleanup_pid(preserve_resume_state)` either:
- Preserves `task-state.json` + events for future resume
- Removes them for a clean slate

### Roko

1. `prepare_resume()` in `runner/resume.rs`:
   - Loads unified `state-snapshot.json` (falls back to legacy `run-state.json`)
   - Validates schema version compatibility
   - Detects stale snapshots (no plan overlap with current run) and ignores them
   - Computes `TaskDefFingerprint` (SHA-256 of task content) for every task
   - Compares fingerprints against the snapshot's stored fingerprints
   - Reports `DriftedTask` entries for re-queuing (does not abort the run)
   - Supports `--force-resume` to skip drift validation
2. JSONL recovery runs on `episodes.jsonl`, `events.jsonl`, `efficiency.jsonl`:
   - Detects truncated trailing lines from crash
   - Drops invalid JSON lines
   - Reports recovery outcome
3. Cascade router and conductor circuit-breaker state are restored from the
   snapshot's embedded JSON
4. The `SnapshotWriter` creates timestamped `.bak` files before overwriting

**Difference**: Roko has strict task-content fingerprinting that detects when
a task definition changed between runs, preventing stale-state bugs. Mori
trusts task identity strings alone. Roko also recovers corrupted JSONL files
(truncated/invalid lines) as part of resume, while Mori does not inspect
event log integrity.

## 8. Event Logging

### Mori: `runs/events.jsonl`

```json
{"ts":"2026-03-28T21:12:05.101060+00:00","event":"task_start","plan":"70a-terminal-hearth-mind","task":"T1","instance":"implementer:70a-terminal-hearth-mind"}
```

Events: `task_start`, `task_done`, `plan_gates_passed`, `plan_merged`, etc.
Simple flat records with timestamp, event type, plan, optional task/instance.
Rotated on run completion into `events-archive/`.

### Roko: `.roko/events.jsonl`

```json
{"agent_pid":66182,"attempt":6,"event":{"message":"+  return;"},"plan_id":"SH02-isolation-recovery","run_id":"run-1783937473752","task_id":"SH02-T05","timestamp":"2026-07-13T18:24:05.420305+00:00","timestamp_ms":1783967045420,"type":"agent.error"}
```

Richer structure: typed `type` field (agent.error, agent.output, task.start,
task.complete, gate.verdict, etc.), agent PID, attempt number, run ID,
millisecond timestamps. Also has a separate typed `state/run-ledger.jsonl`:

```json
{"data":{"plan_id":"...","task_id":"...","timestamp_ms":...},"kind":"task_started","ts":"2026-05-05T17:40:00.910646+00:00"}
```

And a separate `gate-verdicts.jsonl`:

```json
{"duration_ms":113240,"gate_kind":"Gate","kind":"GateVerdict","passed":true,"plan_id":"demo-hello","rung":2,"task_id":"DEMO-T01","timestamp":"2026-08-05T14:50:13.812581+00:00"}
```

**Difference**: Mori has one event stream. Roko has three: a general event
log, a typed run ledger, and a gate verdict log. Roko's events carry more
metadata (PID, attempt, run_id) and are consumed by the TUI, SSE, and server.

## 9. Cold Storage / Archival

**Mori**: Rotates `events.jsonl` and `status.json` into timestamped archive
directories (`events-archive/`, `status-archive/`, `task-state-archive/`).
Keep-latest-10 rotation policy. No cold archival of signals.

**Roko**: Dedicated cold storage in `.roko/cold/`:
- Monthly JSONL partitions (`2026-05.jsonl`, `2026-08.jsonl`)
- Index file (`index.json`) tracking partition boundaries
- Configurable server timer archives aged signals before pruning hot substrate
- Dream consolidation in `.roko/dreams/` produces counterfactuals, cross-episode
  analysis, and staging buffers

**Difference**: Roko has an explicit hot-to-cold signal archival pipeline.
Mori only rotates operational logs; it has no concept of cold signal storage.

## 10. Knowledge Persistence (Roko only)

`.roko/neuro/` -- durable knowledge store:

| File | Purpose |
|---|---|
| `knowledge.jsonl` | Durable knowledge entries (373 KB) |
| `knowledge-confirmations.jsonl` | Confirmation evidence (1.2 MB) |
| `knowledge-candidates.jsonl` | Candidate entries awaiting admission |
| `knowledge-admission-decisions.jsonl` | Admission decisions |

Knowledge follows a tiered progression: Transient -> Working -> Durable.
Gate-backed runner ingestion records confirmation/context evidence and
evaluates tier progression. Connected to the dream consolidation subsystem.

Mori has no equivalent knowledge store. Its learning is limited to the
playbook (when/then rules) and efficiency statistics.

## 11. Affect/Daimon State (Roko only)

`.roko/state/daimon.json` -- persisted PAD (Pleasure-Arousal-Dominance)
affect state with ALMA (A Layered Model of Affect) temporal decomposition:

```json
{
  "state": {
    "pad": { "pleasure": -0.003, "arousal": 0.0, "dominance": 0.01 },
    "confidence": 0.648,
    "behavioral_state": "exploring",
    "alma": {
      "emotion": { ... },
      "mood": { ... },
      "temperament": { ... },
      "tau_emotion": 0.1, "tau_mood": 0.5, "tau_temperament": 0.9
    }
  },
  "somatic_landscape": { "markers": [] },
  "strategy_space": { "domain": "coding", "dimensions": ["complexity", "risk", ...] },
  "crate_confidence_map": {},
  "contrarian_tracker": { ... },
  "error_patterns": { ... },
  "fatigue_detector": { ... },
  "behavioral_tracker": { ... }
}
```

Mori has no affect model. Task routing is driven by config-level complexity
bands and learned playbook rules.

## 12. Summary Table

| Aspect | Mori | Roko |
|---|---|---|
| **State format** | Separate JSON files (status, task-state, events) | Unified checksummed StateSnapshot |
| **Write strategy** | Atomic (write .tmp, rename) | Atomic + async SnapshotWriter thread; budget-capped at 16 MB |
| **Integrity** | No checksums | SHA-256 checksum over domain-separated fields |
| **Resume validation** | Trust plan:task identity strings | SHA-256 task-definition fingerprinting with drift detection |
| **JSONL recovery** | None | Automatic truncated/invalid line repair |
| **Episode format** | 12 flat fields | ~30 fields + HDC fingerprint + signal hashes |
| **Cost storage** | SQLite (costs.db) | JSONL + embedded JSON in snapshot |
| **Learning files** | 2 main (playbook.toml + efficiency.json) | ~20 specialized files (routing, gating, experiments, knowledge, etc.) |
| **Playbook format** | Monolithic TOML | Individual JSON files in a directory |
| **Signal log** | None | Append-only engrams.jsonl with provenance/trust/decay |
| **Knowledge store** | None | Tiered neuro/ with admission, confirmation, progression |
| **Affect state** | None | PAD/ALMA daimon with somatic markers |
| **Cold archival** | Log rotation only | Monthly partitioned cold store with configurable server archival |
| **Event streams** | 1 (events.jsonl) | 3 (events.jsonl, run-ledger.jsonl, gate-verdicts.jsonl) |
| **Backup strategy** | status-archive/ (keep 10), task-state-archive/ | Timestamped .bak files alongside primary snapshot |
| **PID management** | run.pid + stale PID kill (SIGTERM/SIGKILL) | agent-pids.json via ProcessSupervisor |
| **DB dependencies** | SQLite (costs.db 22 MB + index.db 376 MB) | None (all JSONL/JSON) |
| **Config format** | .mori/config.toml (flat keys) | roko.toml at workspace root (hierarchical TOML with profiles) |
| **Queue definition** | queue.toml with milestone arrays | CLI --plan-dir with DAG-ordered execution |
