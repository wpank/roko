# 24 — Runtime Ledger Migration (T5-40 expanded)

The `RunLedger` exists as a typed append-only log of run state. Today,
workflow reports are still inferred by replaying string events; the
ledger captures cancellation events but not gates, artifacts, or
checkpoints. This plan migrates each report-source onto the ledger.

Source: doc 35 § Runtime, ledger, gates, artifacts; doc 41 T5-40; doc 24
(runtime-gate-ledger plan).

---

## Today's State (verified 2026-05-01)

- `RunLedger` skeleton exists in `crates/roko-runtime/src/run_ledger.rs`.
- `run_with_cancel` creates a ledger and records typed cancellation
  requests (R4).
- `CommitOutcome::{Created, NoChanges, Rejected, Failed}` (R1, R2) exist
  with compatibility adapters.
- `GateStatus` enum exists (R5); one runtime rung lookup uses
  `GateRegistry` (R7).
- `ArtifactOutcome` adapter exists for PRD generation (R8).
- **What's missing**: gate verdicts, artifact outcomes, command events,
  resume/checkpoint state are not yet ledger-first.

---

## Anti-Patterns

1. **No new event-replay reports.** New report fields read from typed
   ledger entries.
2. **No string-status sentinels.** Gate skip / pass / fail / error are
   `GateStatus` variants, not strings.
3. **No partial migration with both old and new readers.** When you
   migrate a source onto the ledger, **delete** the legacy reader of
   that source in the same PR (or feature-gate it).
4. **No write-and-forget.** A ledger entry that's written and never read
   is dead code; if the report doesn't consume it, don't add it.
5. **No silent persistence failure.** Ledger writes are
   correctness-critical; surface failures in the run outcome.

---

## Plan

### Slice 1: Gate verdicts

**File**: `crates/roko-cli/src/orchestrate.rs:16800-16940` (gate observation
loop) and `crates/roko-runtime/src/workflow_engine.rs` (report builder).

#### Step 1: Add ledger entry variant

```rust
// crates/roko-runtime/src/run_ledger.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunLedgerEntry {
    // ... existing variants
    Gate {
        rung: Rung,
        status: GateStatus,
        duration_ms: u64,
        attempt: u32,
        rationale: Option<String>,
    },
}

impl RunLedger {
    pub fn record_gate(&mut self, rung: Rung, status: GateStatus, duration: Duration, attempt: u32, rationale: Option<String>) {
        self.append(RunLedgerEntry::Gate {
            rung,
            status,
            duration_ms: duration.as_millis() as u64,
            attempt,
            rationale,
        });
    }
}
```

#### Step 2: Write at the gate observation loop

```rust
// crates/roko-cli/src/orchestrate.rs near the per-rung loop

for recorded in &recorded_verdicts {
    self.adaptive_thresholds.observe(recorded.rung.as_index(), recorded.verdict.passed);
    self.run_ledger.lock().await.record_gate(
        recorded.rung,
        recorded.verdict.status,
        Duration::from_millis(recorded.verdict.duration_ms),
        recorded.attempt,
        recorded.verdict.rationale.clone(),
    );
}
```

#### Step 3: Read from the ledger in the report builder

```rust
// crates/roko-runtime/src/run_ledger.rs

impl RunLedger {
    pub fn gate_verdicts_by_rung(&self) -> HashMap<Rung, GateStatus> {
        let mut latest = HashMap::new();
        for entry in self.entries() {
            if let RunLedgerEntry::Gate { rung, status, .. } = entry {
                latest.insert(*rung, *status);
            }
        }
        latest
    }
}
```

In the report builder (where `RunReport` is assembled), replace the
event-replay-based gate-status fields with `ledger.gate_verdicts_by_rung()`.

#### Step 4: Remove the event-replay path for gates

Find the legacy code that reconstructs gate status from string events:

```bash
rg 'event\.kind\s*==\s*"gate' crates/roko-runtime/src/
rg 'GateCompleted|GatePassed|GateFailed' crates/roko-runtime/src/
```

For each, confirm the report builder no longer uses it (post-migration),
then delete or feature-gate.

#### Tests

```rust
#[test]
fn gate_ledger_round_trips_status() {
    let mut ledger = RunLedger::new();
    ledger.record_gate(Rung::Compile, GateStatus::Passed, Duration::from_secs(2), 1, None);
    ledger.record_gate(Rung::Test, GateStatus::Failed { code: Some(1) }, Duration::from_secs(5), 1,
        Some("test_foo: assertion failed".into()));
    let map = ledger.gate_verdicts_by_rung();
    assert_eq!(map[&Rung::Compile], GateStatus::Passed);
    assert!(matches!(map[&Rung::Test], GateStatus::Failed { .. }));
}
```

#### Verify

```bash
cargo test -p roko-runtime run_ledger --lib
cargo test -p roko-cli orchestrate_gate_ledger --lib
rg 'event\.kind\s*==\s*"gate' crates/roko-runtime/src/   # 0 matches
```

**Estimated effort**: 6-8 hours.

---

### Slice 2: Artifact outcomes

**Files**: `crates/roko-runtime/src/effect_driver.rs`,
`crates/roko-cli/src/orchestrate.rs` (artifact emission paths).

`ArtifactOutcome` already exists for PRD generation (R8). Extend to cover
all artifact emissions.

#### Step 1: Add ledger variant

```rust
RunLedgerEntry::Artifact {
    artifact_id: String,
    kind: ArtifactKind,
    outcome: ArtifactOutcome,    // Created, Updated, Invalid, Missing
    path: Option<PathBuf>,
}
```

#### Step 2: Make artifact validity a workflow outcome

The audit's R8 says artifact validity should be a workflow outcome, not
a side field. Update the workflow engine:

```rust
impl WorkflowEngine {
    fn finalize_outcome(&self, ledger: &RunLedger) -> WorkflowOutcome {
        let invalid_required = ledger.artifacts()
            .filter(|a| a.required && matches!(a.outcome, ArtifactOutcome::Invalid | ArtifactOutcome::Missing))
            .count();
        if invalid_required > 0 {
            return WorkflowOutcome::Failed { reason: format!("{invalid_required} required artifacts invalid") };
        }
        // ... continue with existing logic
    }
}
```

#### Step 3: Remove the side-field tracking

```bash
rg 'artifact_valid|artifacts_passed' crates/roko-runtime/
```

For each, confirm the new ledger-based check covers it; delete the side
field.

#### Tests

```rust
#[test]
fn invalid_required_artifact_blocks_success() {
    let mut ledger = RunLedger::new();
    ledger.record_gate(Rung::Compile, GateStatus::Passed, ...);
    ledger.record_gate(Rung::Test, GateStatus::Passed, ...);
    ledger.record_artifact(Artifact {
        id: "prd-output".into(),
        outcome: ArtifactOutcome::Invalid,
        required: true,
        ..
    });

    let outcome = WorkflowEngine::finalize_outcome(&ledger);
    assert!(matches!(outcome, WorkflowOutcome::Failed { .. }));
}
```

**Estimated effort**: 6-8 hours.

---

### Slice 3: Command events

**File**: `crates/roko-serve/src/command_events.rs` (the typed
`CommandEvent` DTOs from R9), `crates/roko-runtime/src/run_ledger.rs`.

The terminal lifecycle has typed events. Plan 26 migrates the demo
consumer; this slice ensures the events are durably recorded in the
ledger.

#### Step 1: Add ledger variant

```rust
RunLedgerEntry::Command {
    session_id: String,
    event: CommandEvent,
    timestamp_ms: i64,
}
```

#### Step 2: Wire from terminal session

In `crates/roko-serve/src/terminal/...`, after each `CommandEvent` is
emitted to the WebSocket, also append to the ledger (only for sessions
associated with a workflow run).

#### Step 3: Ledger replay for resume

When a session is resumed (Tier 5 R-track), replay the command events to
restore terminal scroll position and re-attach.

**Estimated effort**: 4-6 hours.

---

### Slice 4: Resume / checkpoint

**Files**: `crates/roko-cli/src/runner/resume.rs`,
`crates/roko-runtime/src/run_ledger.rs`.

Today, `--resume .roko/state/executor.json` reads a manually-managed
2.1 MiB executor snapshot. The audit recommends moving this into a typed
ledger checkpoint.

#### Step 1: Add ledger variant

```rust
RunLedgerEntry::Checkpoint {
    state: CheckpointState,
    sequence: u64,
    timestamp_ms: i64,
}

pub struct CheckpointState {
    pub plan: Plan,
    pub completed_tasks: Vec<TaskId>,
    pub current_task: Option<TaskId>,
    pub gate_baselines: HashMap<Rung, AdaptiveThreshold>,
    // Whatever fields executor.json currently carries
}
```

#### Step 2: Periodic checkpoint write

After each task completion, append a `Checkpoint` entry. The latest
checkpoint is the resume point.

#### Step 3: Resume reads ledger

```rust
pub fn resume_from_ledger(path: &Path) -> Result<RunLedger, ResumeError> {
    let ledger = RunLedger::load(path)?;
    // Find the latest Checkpoint entry; that's the resume point.
    let checkpoint = ledger.entries()
        .rev()
        .find_map(|e| match e {
            RunLedgerEntry::Checkpoint { state, .. } => Some(state),
            _ => None,
        })
        .ok_or(ResumeError::NoCheckpoint)?;
    Ok(ledger)
}
```

#### Step 4: Drop `executor.json`

Once the ledger-based path is verified, the legacy
`.roko/state/executor.json` writer is removed. Resume reads the ledger.

#### Tests

```rust
#[test]
fn resume_reconstructs_from_latest_checkpoint() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ledger.jsonl");
    let mut ledger = RunLedger::open(&path).unwrap();
    ledger.record_checkpoint(/* plan, 0 tasks done */);
    ledger.record_task_complete("task-1");
    ledger.record_checkpoint(/* plan, 1 task done */);

    let restored = RunLedger::resume_from_ledger(&path).unwrap();
    let cp = restored.latest_checkpoint().unwrap();
    assert_eq!(cp.completed_tasks.len(), 1);
}
```

**Estimated effort**: 8-12 hours.

---

## Phase 5: Persist fail-closed where correctness depends on it

**Why**: A ledger write failure today is logged and ignored. For
correctness-critical writes (gate verdicts blocking commit, artifact
validity blocking success), failure must surface in the outcome.

#### Implementation

```rust
impl RunLedger {
    pub fn record_gate(&mut self, ...) -> Result<(), LedgerWriteError> {
        let entry = RunLedgerEntry::Gate { ... };
        self.append(entry)?;
        self.fsync()?;        // critical write
        Ok(())
    }
}
```

In the workflow engine, a write failure aborts the run with a typed
`WorkflowOutcome::LedgerFailure`.

For non-critical writes (e.g. command events), keep the
log-and-continue behavior; only critical writes are fail-closed.

**Estimated effort**: 3-4 hours.

---

## Combined Verification

```bash
cargo test -p roko-runtime run_ledger --lib
cargo test --workspace

# Gate, artifact, command, checkpoint variants exist
rg 'RunLedgerEntry::(Gate|Artifact|Command|Checkpoint)' crates/roko-runtime/

# Report reads ledger, not events
rg 'event\.kind\s*==' crates/roko-runtime/src/   # 0 matches in report builder

# Resume reads ledger
rg 'executor\.json' crates/roko-cli/src/runner/   # 0 matches (post-migration)
```

---

## Status

- [ ] Slice 1 — Gate verdicts on ledger
- [ ] Slice 2 — Artifact outcomes as workflow outcome
- [ ] Slice 3 — Command events in ledger
- [ ] Slice 4 — Resume / checkpoint via ledger
- [ ] Phase 5 — Fail-closed persistence

**After completion**: workflow reports are derived from a typed ledger
in one place. The string-event replay path is gone.

**Estimated total effort**: 27-38 hours.
