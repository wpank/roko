# Event Log and Episodes

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 13.
> Preserves and updates content from v1 `01-orchestration/10-event-log.md`.

---

## Overview

The execution system produces three complementary logs:

1. **Activity recording** -- per-node JSONL for deterministic replay
2. **Episode log** -- agent turns and gate results for learning
3. **Efficiency events** -- per-turn telemetry for cost optimization

Together these logs provide full observability into execution: what
happened (activity recording), what was learned (episodes), and what it
cost (efficiency events). Each serves a different purpose and can be
enabled or disabled independently.

**Source:** `crates/roko-graph/src/replay.rs`,
`crates/roko-cli/src/runner/episode_logger.rs`

---

## Activity Recording (Replay Log)

Every Activity node's output is appended to a JSONL file immediately
after execution. This is the replay log used for crash recovery -- it
is infrastructure, not learning.

### RecordEntry

```rust
pub struct RecordEntry {
    pub graph_id: String,
    pub run_id: String,
    pub node_id: String,
    pub tick: u64,
    pub signals: Vec<Signal>,
}
```

### Recording behavior

- The file is flushed after every write. Partial runs are recoverable
  because each JSONL line is self-contained.
- Only Activity nodes are recorded. Workflow nodes are re-derived from
  inputs on resume, saving disk I/O.
- The file is opened in append mode for resumed runs (`create()`).
- Fresh runs truncate the file (`create_fresh()`).

### Replay behavior

On resume, the `ActivityReplayer` loads all recorded entries and creates
a lookup map:

```rust
pub struct ActivityReplayer {
    entries: HashMap<String, Vec<Signal>>,
    replayed: HashSet<String>,
}
```

When the engine encounters an Activity node with a recorded output, it
substitutes the recording. The `replayed` set tracks which entries have
been consumed to detect duplicate or orphaned recordings.

### Storage path

Activity recordings live alongside their graph checkpoint:

```
.roko/state/graph/<run-id>/
    snapshot.json          -- GraphSnapshotV2
    activities/
        main.jsonl         -- Activity recording
```

---

## Episode Log

The episode log (`.roko/episodes.jsonl`) records agent turns and gate
results. Each entry captures the context needed for the learning subsystem
to improve future execution. This is a learning log, not an infrastructure
log.

### Episode entry structure

An episode entry includes:

| Field | Type | Purpose |
|---|---|---|
| `plan_id` | `String` | Owning plan |
| `task_id` | `String` | Specific task |
| `role` | `String` | Agent role (implementer, auditor, etc.) |
| `provider` | `String` | Provider used (anthropic, openai, etc.) |
| `model` | `String` | Model ID |
| `prompt_tokens` | `u64` | Input token count |
| `completion_tokens` | `u64` | Output token count |
| `cost_usd` | `f64` | Cost in USD |
| `duration_ms` | `u64` | Wall-clock duration |
| `gate_passed` | `Option<bool>` | Gate verdict (if applicable) |
| `verdict` | `Option<String>` | Verdict summary |
| `hdc_fingerprint` | `Option<String>` | HDC vector fingerprint |
| `iteration` | `u32` | Retry iteration |
| `timestamp_ms` | `i64` | When the episode occurred |

### HDC fingerprint

Each episode includes an HDC (Hyperdimensional Computing) fingerprint --
a high-dimensional binary vector that encodes the episode's semantic
content. These fingerprints enable:

- **Similarity-based episode retrieval**: when enriching a new task, the
  system retrieves episodes with similar fingerprints.
- **Clustering**: related episodes can be grouped for pattern discovery.
- **Fast lookup**: approximate nearest-neighbor search in the knowledge
  store uses HDC distance.

The fingerprint is computed from the task description, role, file paths,
and domain using the HDC encoding from `roko-primitives`.

### Learning feedback

Episodes feed into multiple learning subsystems:

| Consumer | What it learns | Persistence |
|---|---|---|
| `CascadeRouter` | Model selection (LinUCB bandit) | `.roko/learn/cascade-router.json` |
| `AdaptiveThresholds` | Per-gate-rung pass rates (EMA) | `.roko/learn/gate-thresholds.json` |
| `PlaybookStore` | When/then patterns | `.roko/learn/playbooks.jsonl` |
| `SkillLibrary` | Reusable task patterns | `.roko/learn/skills.jsonl` |
| `KnowledgeStore` | Durable knowledge entries | `.roko/neuro/` |

The cascade router uses gate outcomes as reward signals. Successful
dispatches (gate passed) reward the model arm; failures penalize it.
Over time, the router learns which models are effective for which task
types, reducing cost by routing simple tasks to cheaper models.

---

## Efficiency Events

Per-turn efficiency telemetry is written to `.roko/learn/efficiency.jsonl`.
This provides fine-grained cost and performance data.

### AgentEfficiencyEvent

```rust
pub struct AgentEfficiencyEvent {
    pub plan_id: String,
    pub task_id: String,
    pub role: String,
    pub model: String,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub gate_passed: bool,
}
```

### Uses

Efficiency events drive:

- **Budget optimization**: identifying which models are cost-effective for
  which task types.
- **Cascade routing**: the router's reward function incorporates cost
  as a penalty term, preferring cheaper models when they perform equally.
- **PERT estimation**: historical task durations grouped by complexity
  tier provide three-point estimates for critical path analysis.
- **Cost reporting**: `roko show costs` aggregates efficiency events by
  plan, task, model, and role.
- **Threshold adaptation**: the adaptive gate thresholds use efficiency
  data to track pass rates per rung over time.

---

## Log Rotation

All three logs use bounded JSONL with generation rotation:

- `.roko/episodes.jsonl` is rotated when it exceeds the configured size
  limit (default: 10 MB).
- `.roko/learn/efficiency.jsonl` follows the same rotation policy.
- Activity recordings are scoped to a run ID and do not rotate.

Rotation creates numbered generations (`.jsonl.1`, `.jsonl.2`, etc.)
up to a configured maximum. Old generations are deleted. This prevents
unbounded disk usage during long-running development sessions.

---

## Relationship Between Logs

```
Activity Recording           Episode Log              Efficiency Events
(replay infrastructure)      (learning feedback)      (cost optimization)

+------------------+         +------------------+     +------------------+
| RecordEntry      |         | Episode          |     | EfficiencyEvent  |
| - graph_id       |         | - plan_id        |     | - plan_id        |
| - run_id         |         | - task_id        |     | - task_id        |
| - node_id        |         | - role, model    |     | - model          |
| - tick           |         | - cost, duration |     | - tokens, cost   |
| - signals        |         | - gate_passed    |     | - gate_passed    |
+------------------+         | - hdc_fingerprint|     +------------------+
                              +------------------+

Written by:                  Written by:              Written by:
  GraphEngine                  Controller               Controller

Used by:                     Used by:                  Used by:
  ActivityReplayer             CascadeRouter             Budget optimizer
  (crash recovery)             PlaybookStore             Cost reporting
                               KnowledgeStore            Cascade router
```

The activity recording is an infrastructure concern (crash recovery).
Episodes and efficiency events are learning concerns (improving future
execution). They are independent: you can disable learning without
affecting replay, and vice versa.

The key distinction: activity recordings capture full Signal outputs
(which may be large) for deterministic replay. Episodes and efficiency
events capture metadata (tokens, cost, duration, verdict) for analysis.
This means the activity log grows proportionally to output size, while
episodes and efficiency events grow proportionally to dispatch count.

---

## Inspection Commands

```bash
# Show recent episodes
cargo run -p roko-cli -- learn episodes

# Show efficiency data
cargo run -p roko-cli -- learn efficiency

# Show episode count and knowledge stats
cargo run -p roko-cli -- learn knowledge-stats

# Show cost breakdown
cargo run -p roko-cli -- show costs

# Inspect raw episode log
jq -c '{plan: .plan_id, task: .task_id, model: .model, passed: .gate_passed}' \
  .roko/episodes.jsonl | tail -10

# Inspect efficiency events
jq -c '{task: .task_id, model: .model, cost: .cost_usd, ms: .duration_ms}' \
  .roko/learn/efficiency.jsonl | tail -10
```
