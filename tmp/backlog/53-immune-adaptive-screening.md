# 53 — Immune System Adaptive Memory

**Priority**: P2 — the immune pipeline quarantines suspicious output but has no feedback loop; false positives fire indefinitely and novel attack patterns pass indefinitely
**Size**: L (5-7 days)
**Crates**: `crates/roko-agent/` (`roko-agent`), `crates/roko-graph/` (`roko-graph`), `crates/roko-core/` (`roko-core`)
**Depends on**: None

---

## Background

Roko has a five-stage cognitive immune pipeline that screens every output an AI provider produces before that output is returned to the rest of the system. The pipeline stages are: Perception → Assessment → Containment → Validation → Escalation. When an output looks suspicious (high anomaly score), it is quarantined — withheld from the caller and persisted on disk — rather than passed through.

The quarantine enforcement is production-complete and runs on two separate boundaries: (1) every final `AgentResult` from any provider, processed in `crates/roko-agent/src/immune_boundary.rs`, and (2) every result from the host-visible tool dispatcher (file reads, bash, etc.), processed in `crates/roko-agent/src/tool_immune.rs`. Both boundaries use the same fixed-threshold `ImmunePipeline` defined in `crates/roko-core/src/immune.rs`.

The problem is that the pipeline uses static, fixed thresholds: it cannot learn from experience. If a harmless output pattern triggers a false positive, that same pattern will be quarantined forever. If a new attack pattern does not match the existing detectors, it will pass forever. Neither the operator nor the system has any mechanism to feed back "this quarantine was correct" or "this was a false positive" into future decisions.

There is also a gap in visibility: providers may make dangerous internal calls (subprocess execution, network requests) during reasoning that the host never sees. The primary-output boundary captures the final answer but not intermediate tool traces. Finally, the quarantine ledger has no external authentication anchor — a complete rewrite of local `.roko/immune/` files would be undetectable.

---

## Current State

1. The pure five-stage pipeline is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/immune.rs` (1481 lines). The `ImmunePipeline` struct is at line 297 with thresholds `quarantine_threshold` and `critical_threshold`. The `run()` method at line 316 executes all five stages in order.

2. The runtime Graph Cells that host the five pipeline stages are in `/Users/will/dev/nunchi/roko/roko/crates/roko-graph/src/cells/immune.rs`. Cell type constants are defined at lines 26-34. Each Cell accepts exactly one versioned `ImmuneCellState` variant and fails closed if the predecessor is missing or out-of-order.

3. Provider output screening is in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/immune_boundary.rs`. The `ProviderBoundaryRecord` struct at line 82 records the full five-stage decision. The quarantine Store is at the relative path `.roko/immune/quarantine` (constant `QUARANTINE_STORE_RELATIVE_PATH` at line 38).

4. Tool result screening is in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_immune.rs`. The review vault index is at `.roko/immune/quarantine-vault.json` (constant `QUARANTINE_VAULT_RELATIVE_PATH` at line 33). The tool-control ledger is at `.roko/immune/tool-controls.json` (constant `TOOL_CONTROLS_RELATIVE_PATH` at line 35).

5. No `ImmuneMemory` type or `.roko/immune/memory.json` file exists anywhere in the codebase. There is no feedback mechanism at all.

6. The `QuarantineVault` struct (in `roko-core`) stores quarantine entries for operator review but has no "confirm/dismiss" outcome field and no mechanism to communicate decisions back to detector thresholds.

7. The `AnomalyScore` type (in `roko-core/src/immune.rs`, around line 48) carries a `score: f64` and `dimensions: HashMap<String, f64>`. The dimensions represent individual detector signals whose scores are aggregated into the overall score.

---

## Implementation Plan

### Step 1: Define `ImmuneMemory` in `roko-core`

Add a new file `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/immune_memory.rs` with:

```rust
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::ContentHash;

/// Operator feedback on a quarantine decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineFeedback {
    /// Operator confirmed: this quarantine was correct.
    Confirmed,
    /// Operator dismissed: this was a false positive.
    Dismissed,
}

/// One recorded quarantine decision with optional operator feedback.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImmuneMemoryEntry {
    pub target: ContentHash,
    pub anomaly_score: f64,
    pub anomaly_dimensions: HashMap<String, f64>,
    pub quarantined: bool,
    pub recorded_at: DateTime<Utc>,
    pub feedback: Option<QuarantineFeedback>,
    pub feedback_at: Option<DateTime<Utc>>,
}

/// In-memory and on-disk store for immune screening history.
pub struct ImmuneMemory {
    pub entries: Vec<ImmuneMemoryEntry>,
    pub capacity: usize,
}

impl ImmuneMemory {
    pub const DEFAULT_CAPACITY: usize = 1000;
    pub const RELATIVE_PATH: &'static str = ".roko/immune/memory.json";

    pub fn new(capacity: usize) -> Self { ... }
    /// Load from disk; returns empty if the file does not exist.
    pub fn load(path: &Path) -> Result<Self> { ... }
    /// Atomically write to disk (write tmp + rename).
    pub fn save(&self, path: &Path) -> Result<()> { ... }
    /// Record a new quarantine decision.
    pub fn record(&mut self, entry: ImmuneMemoryEntry) { ... }
    /// Apply operator feedback by target hash.
    pub fn apply_feedback(&mut self, target: &ContentHash, feedback: QuarantineFeedback) -> bool { ... }
    /// Score adjustment for a given dimension based on memory.
    /// Returns a multiplier in [0.5, 2.0]: <1.0 reduces sensitivity, >1.0 increases it.
    pub fn score_adjustment(&self, dimension: &str) -> f64 { ... }
}
```

Export `ImmuneMemory`, `ImmuneMemoryEntry`, and `QuarantineFeedback` from `roko-core/src/lib.rs`.

### Step 2: Wire memory recording into the provider boundary

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/immune_boundary.rs`, after each screening decision (whether quarantine or accept):

- Load `ImmuneMemory` from `workspace_root.join(ImmuneMemory::RELATIVE_PATH)` (or start empty if missing).
- Append an `ImmuneMemoryEntry` with the target hash, anomaly score, dimensions, and `quarantined` flag.
- Save atomically back to the same path.

This should happen in the same code path that writes the `ProviderBoundaryRecord` (around line 413 of `immune_boundary.rs`).

### Step 3: Wire memory recording into the tool boundary

Apply the same memory-recording logic in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_immune.rs`, in the function that calls `update_vault` after each tool result screening.

### Step 4: Apply score adjustments during future screenings

In both boundary files, before computing the final `AnomalyScore` that enters the `ImmunePipeline`, load `ImmuneMemory` and for each anomaly dimension, multiply its raw score by `memory.score_adjustment(dimension)`. Hard floor: if the pipeline's final severity is `Critical`, no memory adjustment can reduce it below `Critical` (the adjustment only affects `High` and below).

### Step 5: Add CLI commands for operator feedback

In the existing `roko knowledge` command tree (see `crates/roko-cli/src/commands/`), add two subcommands:

- `roko knowledge immune stats` — prints a summary of `ImmuneMemory`: total entries, quarantine count, confirmed/dismissed counts, and the top 5 most-adjusted dimensions.
- `roko knowledge immune review <target-hash> [--confirm | --dismiss]` — loads `ImmuneMemory`, finds the entry by target hash, applies feedback, and saves.

### Step 6: Add a local audit chain (optional, P3)

After each quarantine write, append a line to `.roko/immune/chain.jsonl` of the form `{"ts": "<iso8601>", "target": "<hash>", "decision": "quarantine"|"accept", "prev": "<sha256-of-prev-line>"}`. This creates a tamper-evident append-only chain within the local filesystem. No external anchor is required for this item.

---

## Acceptance Criteria

1. After any screening decision (quarantine or accept), an entry is appended to `ImmuneMemory` and persisted atomically to `.roko/immune/memory.json`.
2. `roko knowledge immune stats` prints a non-empty summary when at least one agent dispatch has completed.
3. `roko knowledge immune review <hash> --confirm` marks the entry and saves; `--dismiss` marks the entry and reduces future sensitivity for the same anomaly dimensions.
4. Memory adjustments never reduce a `Critical`-severity finding below `Critical` (the hard floor is enforced).
5. All existing tests pass: `cargo test -p roko-agent -p roko-graph -p roko-core`.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## Verification Checklist

- [ ] Run a plan with `roko plan run` and confirm `.roko/immune/memory.json` is created.
- [ ] Confirm the file contains at least one entry per completed agent dispatch.
- [ ] Run `roko knowledge immune stats` and confirm it prints a formatted summary.
- [ ] Manually construct a fake entry, run `roko knowledge immune review <hash> --dismiss`, and confirm the feedback field is set in `memory.json`.
- [ ] Run `cargo test -p roko-agent -p roko-graph -p roko-core` — all tests pass.
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` — clean.

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/immune_memory.rs` | New file: `ImmuneMemory`, `ImmuneMemoryEntry`, `QuarantineFeedback` types with load/save/record/adjust |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs` | Export the new types |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/immune_boundary.rs` | Record and load `ImmuneMemory` on each provider output screening; apply dimension adjustments |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_immune.rs` | Same as above for tool result screening |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/` | Add `roko knowledge immune stats` and `roko knowledge immune review` subcommands |
