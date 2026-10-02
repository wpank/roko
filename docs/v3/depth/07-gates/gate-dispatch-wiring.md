# Gate Dispatch Wiring

> Depth file for [07-GATES.md](../../07-GATES.md) section 5.
> Source: `crates/roko-cli/src/runner/gate_dispatch.rs` and sub-modules
>
> **Status (2026-09-29, at `7c556bc0a`):** the plan execution event loop this module
> served was Runner-v2's, deleted on 2026-09-06 (`6b5da8616`). Graph runs use two of its
> helpers (`attempt_auto_fix`, `acquire_compile_ownership`); only tests reach `spawn_gate`
> and `run_gate_once`. Graph plan tasks run their authored `verify` commands through
> `ShellGate` instead (see [07-GATES.md](../../07-GATES.md)).

---

## 1. Overview

The `gate_dispatch.rs` module (3,393 lines) in `crates/roko-cli/src/runner/`
is the bridge between the plan execution event loop and the gate
infrastructure in `roko-gate`. It constructs gate payloads, manages compile
coordination, dispatches gate runs as background Tokio tasks, and routes
verdicts back to the runner.

Four sub-modules are extracted for clarity:

| Module | Lines | Responsibility |
|---|---|---|
| `cargo_command.rs` | 737 | Cargo command parsing, fingerprinting, deduplication |
| `gate_input.rs` | 236 | Deterministic worktree fingerprinting |
| `gate_report.rs` | 198 | Output rendering and failure classification |
| `gate_adapter.rs` | 426 | `RunnerProductionGateAdapter` and artifact store |

Together they form approximately 5,000 lines of gate-dispatch wiring.

---

## 2. GateTaskContext

The runner event loop constructs a `GateTaskContext` from the current task
definition before spawning a gate worker. This struct carries everything the
advanced gate rungs need:

```rust
#[derive(Clone, Debug, Default)]
pub struct GateTaskContext {
    pub plan_id: String,
    pub symbols: Vec<String>,           // For Symbol gate (Rung 3)
    pub acceptance: Vec<String>,        // For FactCheck gate (Rung 5)
    pub task_description: Option<String>, // For LlmJudge gate (Rung 6)
    pub task_title: String,             // Fallback when description absent
    pub planned_files: Vec<String>,     // File scope from plan
}
```

Construction from a task definition:

```rust
impl GateTaskContext {
    pub fn from_task_def(
        plan_id: &str,
        task_def: Option<&TaskDef>,
    ) -> Option<Self> {
        let td = task_def?;
        Some(Self {
            plan_id: plan_id.to_string(),
            symbols: td.context.as_ref()
                .map(|ctx| ctx.symbols.clone())
                .unwrap_or_default(),
            acceptance: td.acceptance.clone(),
            task_description: td.description.clone(),
            task_title: td.title.clone(),
            planned_files: td.files.clone(),
        })
    }
}
```

The `symbols` field feeds the `SymbolManifest` for Rung 3. The `acceptance`
field provides assertions for the `FactCheckGate`. The `task_description`
gives the `LlmJudgeGate` context for quality evaluation.

---

## 3. build_rung_execution_inputs

This function constructs real `RungExecutionInputs` for the advanced gate
rungs (Symbol, FactCheck, LlmJudge, GeneratedTest) from the
`GateTaskContext`:

```rust
pub fn build_rung_execution_inputs(
    ctx: &GateTaskContext,
) -> RungExecutionInputs
```

For each rung:

| Rung | Input constructed |
|---|---|
| Symbol (3) | `SymbolManifest` with expectations from `ctx.symbols` |
| FactCheck (5) | Acceptance criteria from `ctx.acceptance` |
| LlmJudge (6) | `JudgePayload` from `ctx.task_description` |
| GeneratedTest (4) | Generated checks from registered test artifacts |

The `SymbolManifest` parsing handles symbol strings in the format
`"pub struct FooBar"` by extracting visibility, kind, and name:

```rust
SymbolExpectation {
    name: "FooBar".into(),
    kind: SymbolKind::Struct,
    visibility: Visibility::Public,
    path: None,
}
```

---

## 4. build_rung_execution_config

Constructs `RungExecutionConfig` with environment variables, timeouts, and
parallelism limits:

```rust
pub fn build_rung_execution_config(
    working_dir: &Path,
    gate_config: &GatesConfig,
) -> RungExecutionConfig
```

Key environment variables injected:

| Variable | Value | Purpose |
|---|---|---|
| `CARGO_BUILD_JOBS` | `cargo_build_jobs()` | Half logical CPUs, prevents exhaustion |
| `RUSTC_WRAPPER` | `sccache` (if available) | Build cache acceleration |
| `CARGO_TARGET_DIR` | Configured or default | Isolate build artifacts |

---

## 5. Sentinel Rung Values

Two sentinel rung values extend the standard 0-6 range for special purposes:

```rust
pub const RUNG_PLAN_VERIFY: u32 = 1000;
pub const RUNG_MERGE: u32 = 1001;
```

**RUNG_PLAN_VERIFY (1000)**: Plan-level verification that runs after all
tasks complete. Validates the entire plan's output rather than individual
task output. Used by `plan run` to perform final workspace checks.

**RUNG_MERGE (1001)**: Post-merge regression gates. After a plan's changes
are merged into the main branch, these gates verify that the merge did not
introduce regressions. Used by the GitHub workflow integration.

---

## 6. Compile Coordination

When multiple agents run gate checks in parallel on the same repository, they
must not run concurrent Cargo builds -- Cargo holds a lockfile on the target
directory and concurrent invocations either deadlock or produce spurious
failures.

The `CompileCoordinatorRegistry` solves this:

```rust
struct CompileCoordinatorRegistry {
    repositories: HashMap<PathBuf, Weak<Semaphore>>,
    workdir_keys: HashMap<PathBuf, PathBuf>,
}
```

Each repository gets a `Semaphore` keyed by its git common directory
(resolving worktrees to their parent repo). Before running any Cargo command,
the gate worker acquires a permit from the semaphore. This serializes builds
per-repository while allowing builds on different repositories to proceed in
parallel.

The `compile_coordinator()` function resolves the common directory via
`git rev-parse --git-common-dir` with a 2-second timeout. If git is
unavailable, it falls back to using the working directory as the key.

### CPU Limiting

```rust
fn cargo_build_jobs() -> String {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    (cpus / 2).max(1).to_string()
}
```

Half the logical CPUs, floored to 1. This prevents CPU exhaustion when
multiple agents run simultaneously. The value is injected as
`CARGO_BUILD_JOBS` in the environment of every Cargo subprocess.

### sccache Detection

```rust
fn sccache_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let path_var = std::env::var("PATH").unwrap_or_default();
        std::env::split_paths(&path_var)
            .any(|dir| dir.join("sccache").is_file()
                     || dir.join("sccache.exe").is_file())
    })
}
```

Detected once and cached. When available, set as `RUSTC_WRAPPER` to
accelerate incremental builds across attempts.

---

## 7. Gate Mode: Full vs. Focused

The `GateMode` enum (from `roko-core` config) controls which gates run:

| Mode | Behavior |
|---|---|
| `Full` | Run the standard rung ladder based on complexity |
| `Focused` | Run only authored verify steps from the task definition |
| `Skip` | Skip gates entirely (development only) |

FAST mode (`ROKO_FAST_MODE=1`) combined with `ROKO_TASK_VERIFY_ONLY=1`
forces `Focused` mode, which runs exactly one authored `verify` command. The
`fast_task_verify_contract_error()` function enforces that FAST tasks define
exactly one verify step:

```rust
fn fast_task_verify_contract_error(
    fast_mode: bool,
    task_verify_only: bool,
    authored_verify_count: usize,
) -> Option<String> {
    (fast_mode && task_verify_only && authored_verify_count != 1).then(|| {
        format!(
            "FAST task-owned verification requires exactly one authored \
             verify step; found {authored_verify_count}"
        )
    })
}
```

---

## 8. Sub-Module: cargo_command

The `cargo_command.rs` module (737 lines) handles Cargo-specific concerns:

| Function | Purpose |
|---|---|
| `canonical_verify_commands()` | Map rung to standard Cargo commands |
| `cargo_command_fingerprint()` | Content-hash a command for deduplication |
| `cargo_command_with_profile()` | Select dev/release profile |
| `deduplicate_verify_steps()` | Remove redundant verify commands |
| `focused_verify_steps()` | Extract task-authored verify steps |
| `scope_authored_verify_steps()` | Validate authored steps against allowed set |
| `targeted_cargo_check()` | Generate focused `cargo check` for changed files |
| `with_targeted_compile_rung()` | Override Rung 0 with targeted check |

Command fingerprinting enables deduplication: if two tasks produce identical
verify commands, the second is skipped and the first's verdict is reused.

---

## 9. Sub-Module: gate_input

The `gate_input.rs` module (236 lines) creates deterministic worktree
fingerprints:

```rust
pub fn gate_input_snapshot(
    workdir: &Path,
) -> Result<InputSnapshot, io::Error>

pub fn accepted_input_snapshot(
    workdir: &Path,
    accepted_files: &[PathBuf],
) -> Result<InputSnapshot, io::Error>

pub fn owned_input_fingerprint_id(
    snapshot: &InputSnapshot,
) -> String
```

An `InputSnapshot` captures the content hashes of all files in scope. Two
snapshots with different fingerprints mean the gate inputs changed. This
enables the runner to skip re-verification when the filesystem state has not
changed since the last gate run.

The `fetch_git_diff()` function retrieves the diff between the current
worktree and a reference commit, feeding the `DiffGate` and impact analysis.

---

## 10. Sub-Module: gate_report

The `gate_report.rs` module (198 lines) renders gate output for the runner:

| Function | Purpose |
|---|---|
| `render_output()` | Format verdict for display |
| `classify_failure_kind()` | Map verdict to `RunnerFailureKind` |
| `filter_preexisting_failures()` | Exclude failures present before agent changes |
| `gate_failure_input()` | Extract actionable failure data for retry |
| `raw_gate_name()` | Strip rung prefix from gate name |

`classify_failure_kind()` maps gate verdicts to structured failure categories
that the event loop uses for escalation and replanning decisions.

---

## 11. Sub-Module: gate_adapter

The `gate_adapter.rs` module (426 lines) bridges the runner to the
`ProductionGateService`:

```rust
pub struct RunnerProductionGateAdapter {
    service: ProductionGateService,
    artifact_store: FsGeneratedArtifactStore,
    verdict_publisher: Option<VerdictPublisher>,
}
```

The adapter implements the trait interface expected by both the Runner-v2
event loop and the Graph engine's `GatePipelineCell`. It holds:

- A `ProductionGateService` that constructs and runs gate pipelines
- An `FsGeneratedArtifactStore` for persisting generated test artifacts to
  the filesystem
- An optional `VerdictPublisher` for broadcasting verdicts as Pulse events

The `default_gate_adapter()` factory function creates a standard adapter
from workspace configuration.

---

## 12. Impact Analysis Integration

Before running gates, the runner performs impact analysis to determine which
files were changed and which rungs are relevant:

```rust
use super::impact_analysis::ImpactReport;
```

The `ImpactReport` identifies changed files, affected crates, and dependency
chains. This feeds:

- `targeted_cargo_check()` -- focus Rung 0 on changed crates only
- `focused_verify_steps()` -- select only relevant verify commands
- `GateTaskContext.planned_files` -- scope the Symbol gate's search

---

## 13. VerdictPublisher Integration

When a `VerdictPublisher` is configured on the gate adapter, every verdict
emitted by the pipeline is broadcast as a `Pulse`:

```rust
pub struct VerdictSummary {
    pub gate: String,
    pub passed: bool,
    pub score: f32,
    pub reason: String,
    pub rung: Option<u32>,
    pub duration_ms: u64,
}
```

The publisher assigns a monotonic sequence number to each pulse, enabling
consumers to detect gaps (missed verdicts). Downstream consumers include
the adaptive threshold system, the efficiency event logger, the dashboard
SSE endpoint, and the telemetry Lens runtime.

---

## 14. GateCompletion Flow

The gate worker reports results back to the runner event loop via a
`GateCompletion` message:

```rust
pub struct GateCompletion {
    pub kind: GateCompletionKind,
    pub verdicts: Vec<GateVerdictSummary>,
    pub effects: Vec<GateEffectRef>,
}
```

`GateCompletionKind` indicates whether the gate run succeeded, failed,
timed out, or was cancelled. The runner event loop matches on this to decide
whether to retry, escalate, or proceed to the next task.

---

## 15. Verification commands

```bash
# Run gate dispatch tests
cargo test -p roko-cli -- gate_dispatch

# Run cargo command sub-module tests
cargo test -p roko-cli -- cargo_command

# Run gate adapter tests
cargo test -p roko-cli -- gate_adapter

# Run gate input fingerprinting tests
cargo test -p roko-cli -- gate_input

# Check the full gate dispatch module compiles
cargo check -p roko-cli
```

---

## 16. Test criteria

| Test | Property |
|---|---|
| `gate_task_context_from_task_def` | All fields extracted from TaskDef |
| `cargo_build_jobs_at_least_one` | Even on single-CPU, returns "1" |
| `sccache_detection_cached` | Second call returns same value without PATH scan |
| `fast_mode_requires_one_verify` | 0 or 2 authored verify steps -> error |
| `focused_mode_overrides_full` | FAST + TASK_VERIFY_ONLY -> GateMode::Focused |
| `compile_coordinator_shares_semaphore` | Two worktrees in same repo -> same semaphore |
| `compile_coordinator_isolates_repos` | Two different repos -> different semaphores |
| `rung_plan_verify_sentinel` | RUNG_PLAN_VERIFY == 1000 |
| `rung_merge_sentinel` | RUNG_MERGE == 1001 |
| `command_fingerprint_deduplicates` | Identical commands -> same fingerprint hash |
| `input_snapshot_detects_changes` | File modification -> different fingerprint |
| `verdict_publisher_increments_sequence` | Each publish -> sequence + 1 |
