# Failure Classification and Reflection-to-Playbook Pipeline

> Covers gaps #13 (Failure Classification) and #15 (Reflection -> Playbooks).
> Completed for structured gate failure classification and retry metadata in implementation pass 2026-04-26. Full reflection-to-playbook automation remains in the learning backlog.

## Problem Statement

The failure-retry loop has three disconnected pieces that each handle failure classification independently, with no structured mapping from failure kind to retry policy, and no mechanism to convert repeated failures into durable playbook rules.

### What exists today

**1. Gate-level classification exists but is not used for retry decisions.**

`roko-gate/src/compile_errors.rs` has a full classification stack:

- `FailureClass` enum: 11 variants including `SyntaxError`, `ImportError`, `TypeError`, `MissingDependencyOrFeature`, `BorrowOrLifetime`, `TestExpectationFailure`, `ExternalEnvironment`, `UnsafeStubOrPassBehavior`, `PromptContextInsufficiency`, `RoleToolPermission`, `ArchitecturalConflictRequiresReplan`, `Unknown`
- `GateFailureAction` enum: `Retry`, `NeedsReplan`, `Blocked`, `NeedsHuman`
- `GateFailureClassification` struct with `primary`, `classes`, `compile_errors`, `recommended_action`, `replan_candidate`, `blocking_findings`
- `classify_gate_failure()` produces a full classification from raw gate output
- `render_failure_classification()` serializes it to JSON for the error digest

This is called in `TestGate::verify()` and `CompileGate::verify()`, and the classification is attached to the `Verdict` via `with_error_digest()`. But the classification reaches the orchestrate.rs gate-completion handler only as a rendered string in the verdict's `error_digest` field. The structured `GateFailureClassification` with its `recommended_action` and `FailureClass` variants is lost. The orchestrator's retry decision is binary: pass or fail. When fail, it increments `gate_failure_count` and either emits `GateFailed` or `Fatal` based on the raw count exceeding `retry_budget`.

**2. Mori's `FailureKind` is not ported.** Mori (`/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/executor.rs`) has a `FailureKind` enum with 11 variants and per-kind cooldown logic (`retry_cooldown_secs`). Roko's `ExecutorEvent::GateFailed` is a unit variant with no payload -- there is no failure kind attached. The executor treats all gate failures identically.

**3. The replan pipeline uses `GateFailureAction` but only at one decision point.** The `may_attempt_gate_failure_replan()` method in `orchestrate.rs` calls a helper that inspects the verdicts and classifies the failure into `GateFailureAction`. If the action is `Retry`, it short-circuits. If `Blocked` or `NeedsHuman`, it stops. Only `NeedsReplan` flows through to `build_gate_failure_plan_revision()`. But this logic does not vary the retry strategy (cooldown, prompt augmentation, context enrichment) based on the specific `FailureClass`.

**4. There is no reflection-to-playbook pipeline.** When a gate fails:
- The error digest is extracted (`error_patterns.rs::extract_error_digest`)
- The classification is rendered and attached to the verdict
- The `learned_error_signature()` in `learning_helpers.rs` extracts a short signature from the last gate failure
- But none of this is persisted as a structured reflection, and no mechanism converts repeated reflections with the same error type into a playbook rule

The `PlaybookStore` in `roko-learn/src/playbook.rs` is a fully functional store with `save`, `load`, `query`, `record_outcome`, and relevance ranking. But playbooks are only created manually or by the tier progression's D3 compilation. There is no automatic creation from gate failure patterns.

### Why it matters

Without per-kind retry policy:
- A type error (permanent, needs different code) gets the same 2-second retry as a test flake (transient, just needs re-run)
- An OOM crash (resource, abort) gets the same treatment as a missing import (permanent, needs prompt enrichment)
- A broken verify script (structural, needs regeneration) gets the same treatment as a timeout (transient)

Without reflection-to-playbook:
- The same "missing import for renamed type" error causes 3 failures across 3 different plans over 3 different weeks, and each time the agent rediscovers the fix from scratch
- Knowledge is ephemeral: the error signature is logged but never consulted on the next occurrence

## Ideal Design

### 1. `FailureKind` Enum (Task-Level)

This sits above the gate-level `FailureClass` and `GateFailureAction`. It classifies the task-level failure mode for retry policy decisions.

```rust
// crates/roko-core/src/failure.rs (new file)

/// Task-level failure classification for retry policy decisions.
///
/// Maps from gate-level `FailureClass` and execution context into
/// a coarser category that determines retry strategy, cooldown,
/// and whether the failure should produce a reflection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// Transient: test flake, timeout, network blip, race condition.
    /// Strategy: retry with same prompt after short cooldown (2s).
    Transient,

    /// Permanent: type error, missing import, wrong API, logic bug.
    /// Strategy: retry with full prompt augmented by error digest.
    /// The agent needs different code, not a re-run.
    Permanent,

    /// Resource: OOM, disk full, process limit, ulimit.
    /// Strategy: abort the task. No retry will help.
    Resource,

    /// Structural: verify script broken, impossible gate conditions,
    /// acceptance contract impossible to satisfy.
    /// Strategy: regenerate the verify script, then retry.
    Structural,
}

impl FailureKind {
    /// Whether this failure kind should be retried at all.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient | Self::Permanent | Self::Structural)
    }

    /// Cooldown before retry, in seconds.
    pub const fn retry_cooldown_secs(&self) -> u64 {
        match self {
            Self::Transient => 2,
            Self::Permanent => 0, // no cooldown, but prompt changes
            Self::Resource => 0,  // not retried
            Self::Structural => 5,
        }
    }

    /// Whether the retry prompt should include the error digest.
    pub const fn needs_error_digest(&self) -> bool {
        matches!(self, Self::Permanent | Self::Structural)
    }

    /// Whether a reflection should be generated from this failure.
    pub const fn generates_reflection(&self) -> bool {
        matches!(self, Self::Permanent | Self::Structural)
    }

    /// Whether this failure should trigger verify script regeneration.
    pub const fn needs_verify_regen(&self) -> bool {
        matches!(self, Self::Structural)
    }
}
```

### 2. Classification Bridge

Map from gate-level classification to task-level `FailureKind`:

```rust
// crates/roko-core/src/failure.rs

use roko_gate::compile_errors::{FailureClass, GateFailureAction, GateFailureClassification};

impl FailureKind {
    /// Classify a gate failure into a task-level failure kind.
    ///
    /// Uses the structured classification when available, falling back
    /// to heuristic detection from raw output.
    pub fn from_gate_classification(classification: &GateFailureClassification) -> Self {
        // First: check recommended action for clear signals
        match classification.recommended_action {
            GateFailureAction::Blocked => return Self::Resource,
            GateFailureAction::NeedsHuman => return Self::Permanent,
            _ => {}
        }

        // Second: map from primary failure class
        match classification.primary {
            // Transient failures: re-run may succeed
            FailureClass::ExternalEnvironment => Self::Transient,
            FailureClass::TestExpectationFailure
                if is_likely_flake(&classification.raw_excerpt) =>
            {
                Self::Transient
            }

            // Permanent failures: agent needs different code
            FailureClass::SyntaxError
            | FailureClass::ImportError
            | FailureClass::TypeError
            | FailureClass::MissingDependencyOrFeature
            | FailureClass::BorrowOrLifetime
            | FailureClass::TestExpectationFailure
            | FailureClass::PromptContextInsufficiency
            | FailureClass::RoleToolPermission => Self::Permanent,

            // Structural: the gate itself is broken
            FailureClass::UnsafeStubOrPassBehavior
            | FailureClass::ArchitecturalConflictRequiresReplan => Self::Structural,

            FailureClass::Unknown => Self::from_raw_output(&classification.raw_excerpt),
        }
    }

    /// Fallback classification from raw output when structured classification
    /// is unavailable.
    pub fn from_raw_output(output: &str) -> Self {
        let lower = output.to_ascii_lowercase();

        // Resource signals
        if lower.contains("out of memory")
            || lower.contains("oom")
            || lower.contains("no space left")
            || lower.contains("disk full")
            || lower.contains("too many open files")
            || lower.contains("cannot allocate memory")
        {
            return Self::Resource;
        }

        // Transient signals
        if lower.contains("timed out")
            || lower.contains("timeout")
            || lower.contains("connection refused")
            || lower.contains("connection reset")
            || lower.contains("flaky")
            || lower.contains("intermittent")
        {
            return Self::Transient;
        }

        // Structural signals
        if lower.contains("verify script")
            || lower.contains("acceptance contract")
            || lower.contains("impossible")
        {
            return Self::Structural;
        }

        // Default to permanent: agent needs to change something
        Self::Permanent
    }
}

fn is_likely_flake(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    lower.contains("flaky")
        || lower.contains("intermittent")
        || lower.contains("race condition")
        || lower.contains("timed out")
        || (lower.contains("1 failed") && lower.contains("passed"))
}
```

### 3. Error Digest Extraction (Already Exists, Wire Through)

`error_patterns.rs::extract_error_digest()` already does exactly what we need:
- Scans output for error signature lines
- Caps at 10 unique errors
- Truncates each at 200 chars
- Dedupes by first line

The existing implementation is correct. The gap is that it's only called via `render_failure_classification()` on the gate side. The orchestrator needs to re-parse the classification from the verdict's error_digest field, or (better) the structured `GateFailureClassification` needs to survive through the verdict into the orchestrator.

### 4. Structured Reflection

```rust
// crates/roko-learn/src/reflection.rs (new file)

/// A structured reflection extracted from a gate failure.
///
/// Reflections are the intermediate form between raw gate output and
/// durable playbook rules. They capture the error type, affected files,
/// and the fix strategy that should be attempted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reflection {
    /// Unique identifier.
    pub id: String,
    /// Normalized error type (from FailureClass or error code).
    /// Used as the grouping key for playbook auto-creation.
    pub error_type: String,
    /// File patterns affected by this failure (e.g., "crates/roko-core/src/*.rs").
    pub file_patterns: Vec<String>,
    /// Recommended fix strategy derived from the failure classification.
    pub fix_strategy: String,
    /// Compact error digest (from extract_error_digest).
    pub error_digest: String,
    /// Task ID that produced this reflection.
    pub task_id: String,
    /// Plan ID that produced this reflection.
    pub plan_id: String,
    /// The FailureKind that was classified.
    pub failure_kind: FailureKind,
    /// Timestamp.
    pub created_at: DateTime<Utc>,
}

/// Persistent store for reflections, backed by JSONL.
pub struct ReflectionStore {
    path: PathBuf,
    write_gate: Arc<Mutex<()>>,
}

impl ReflectionStore {
    pub fn new(path: impl Into<PathBuf>) -> Self { ... }
    pub fn for_roko_dir(roko_dir: impl AsRef<Path>) -> Self {
        Self::new(roko_dir.as_ref().join("learn").join("reflections.jsonl"))
    }

    /// Append a reflection.
    pub fn add(&self, reflection: Reflection) -> Result<()> { ... }

    /// Load all reflections.
    pub fn read_all(&self) -> Result<Vec<Reflection>> { ... }

    /// Count reflections grouped by error_type.
    pub fn count_by_error_type(&self) -> Result<HashMap<String, usize>> { ... }

    /// Find reflections matching the given error_type.
    pub fn find_by_error_type(&self, error_type: &str) -> Result<Vec<Reflection>> { ... }
}
```

### 5. Reflection Extraction

```rust
// crates/roko-learn/src/reflection.rs

impl Reflection {
    /// Extract a reflection from a gate failure classification and task context.
    pub fn from_gate_failure(
        classification: &GateFailureClassification,
        failure_kind: &FailureKind,
        plan_id: &str,
        task_id: &str,
        task_files: &[String],
    ) -> Self {
        let error_type = normalize_error_type(&classification.primary);
        let file_patterns = derive_file_patterns(task_files, &classification.compile_errors);
        let fix_strategy = derive_fix_strategy(&classification.primary, classification);
        let error_digest = roko_gate::extract_error_digest(&classification.raw_excerpt);

        Self {
            id: format!("refl-{plan_id}-{task_id}-{}", short_hash(&error_type)),
            error_type,
            file_patterns,
            fix_strategy,
            error_digest,
            task_id: task_id.to_string(),
            plan_id: plan_id.to_string(),
            failure_kind: failure_kind.clone(),
            created_at: Utc::now(),
        }
    }
}

fn normalize_error_type(class: &FailureClass) -> String {
    match class {
        FailureClass::SyntaxError => "syntax_error".to_string(),
        FailureClass::ImportError => "import_error".to_string(),
        FailureClass::TypeError => "type_error".to_string(),
        FailureClass::MissingDependencyOrFeature => "missing_dep".to_string(),
        FailureClass::BorrowOrLifetime => "borrow_lifetime".to_string(),
        FailureClass::TestExpectationFailure => "test_expectation".to_string(),
        FailureClass::ExternalEnvironment => "external_env".to_string(),
        FailureClass::UnsafeStubOrPassBehavior => "unsafe_stub".to_string(),
        FailureClass::PromptContextInsufficiency => "prompt_insufficient".to_string(),
        FailureClass::RoleToolPermission => "permission".to_string(),
        FailureClass::ArchitecturalConflictRequiresReplan => "arch_conflict".to_string(),
        FailureClass::Unknown => "unknown".to_string(),
    }
}

fn derive_file_patterns(
    task_files: &[String],
    compile_errors: &[CompileError],
) -> Vec<String> {
    // Prefer compile error file paths (specific)
    let mut patterns: Vec<String> = compile_errors
        .iter()
        .filter_map(|e| e.file.as_ref())
        .map(|f| f.to_string())
        .collect();

    // Fall back to task file list
    if patterns.is_empty() {
        patterns = task_files.to_vec();
    }

    // Deduplicate
    patterns.sort();
    patterns.dedup();
    patterns.truncate(10);
    patterns
}

fn derive_fix_strategy(class: &FailureClass, classification: &GateFailureClassification) -> String {
    // Use compiler suggestions when available
    let suggestions: Vec<&str> = classification
        .compile_errors
        .iter()
        .filter_map(|e| e.suggestion.as_deref())
        .collect();

    if !suggestions.is_empty() {
        return format!(
            "Apply compiler suggestions: {}",
            suggestions.join("; ")
        );
    }

    // Fall back to class-based strategy
    match class {
        FailureClass::ImportError => "Check renamed types/modules; search for the correct import path".to_string(),
        FailureClass::TypeError => "Review function signatures; check type parameter bounds".to_string(),
        FailureClass::MissingDependencyOrFeature => "Add missing dependency to Cargo.toml or enable required feature".to_string(),
        FailureClass::BorrowOrLifetime => "Restructure ownership; consider Clone, Arc, or lifetime annotations".to_string(),
        FailureClass::TestExpectationFailure => "Update test expectations to match actual behavior".to_string(),
        FailureClass::SyntaxError => "Fix syntax errors; check for missing braces/semicolons".to_string(),
        FailureClass::UnsafeStubOrPassBehavior => "Replace stub/no-op with real implementation".to_string(),
        FailureClass::PromptContextInsufficiency => "Enrich prompt with additional context about the task".to_string(),
        _ => "Review error output and adjust implementation".to_string(),
    }
}
```

### 6. Playbook Auto-Creation

```rust
// crates/roko-learn/src/reflection.rs

/// Check if any error_type has accumulated enough reflections to
/// warrant automatic playbook creation.
///
/// Threshold: 3 reflections with the same error_type.
/// Created playbook starts with confidence 0.5.
pub async fn maybe_create_playbooks(
    reflection_store: &ReflectionStore,
    playbook_store: &PlaybookStore,
    threshold: usize, // default 3
) -> Result<Vec<String>> {
    let counts = reflection_store.count_by_error_type()?;
    let mut created = Vec::new();

    for (error_type, count) in &counts {
        if *count < threshold {
            continue;
        }

        let playbook_id = format!("auto-{error_type}");

        // Skip if playbook already exists
        if playbook_store.load(&playbook_id).await?.is_some() {
            continue;
        }

        let reflections = reflection_store.find_by_error_type(error_type)?;
        let playbook = build_playbook_from_reflections(&playbook_id, error_type, &reflections);
        playbook_store.save(&playbook).await?;
        created.push(playbook_id);
    }

    Ok(created)
}

fn build_playbook_from_reflections(
    playbook_id: &str,
    error_type: &str,
    reflections: &[Reflection],
) -> Playbook {
    // Collect the most common fix strategies
    let mut strategy_counts: HashMap<&str, usize> = HashMap::new();
    for refl in reflections {
        *strategy_counts.entry(refl.fix_strategy.as_str()).or_default() += 1;
    }
    let mut strategies: Vec<_> = strategy_counts.into_iter().collect();
    strategies.sort_by(|a, b| b.1.cmp(&a.1));

    // Collect affected file patterns
    let mut all_patterns: Vec<String> = reflections
        .iter()
        .flat_map(|r| r.file_patterns.iter().cloned())
        .collect();
    all_patterns.sort();
    all_patterns.dedup();

    let goal = format!(
        "Fix {error_type} errors in: {}",
        all_patterns.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
    );

    let mut pb = Playbook::new(playbook_id, &goal);
    pb.name = format!("Auto: fix {error_type}");

    // Build steps from the most common strategies
    for (idx, (strategy, _count)) in strategies.iter().take(3).enumerate() {
        pb.steps.push(PlaybookStep::new(
            idx as u32,
            *strategy,
            if idx == 0 { "analyze_error" } else { "edit_file" },
            vec!["compile_ok".into()],
        ));
    }

    // Final verification step
    pb.steps.push(PlaybookStep::new(
        pb.steps.len() as u32,
        "Run verification gates",
        "run_command",
        vec!["gate_passed".into()],
    ));

    pb
}
```

### 7. Per-Kind Retry Policy Wiring

The retry policy is applied in the orchestrator's gate completion handler:

```rust
// In orchestrate.rs gate completion handler (conceptual)

// Before: binary pass/fail
let event = if passed {
    ExecutorEvent::GatePassed
} else if gate_failure_count > retry_budget {
    ExecutorEvent::Fatal(failure_reason)
} else {
    ExecutorEvent::GateFailed
};

// After: per-kind retry policy
let classification = parse_failure_classification_from_verdict(&verdict);
let failure_kind = FailureKind::from_gate_classification(&classification);

let event = if passed {
    ExecutorEvent::GatePassed
} else {
    match failure_kind {
        FailureKind::Resource => {
            // Abort immediately -- no retry will help
            tracing::warn!(plan_id, task_id, "resource failure — aborting");
            ExecutorEvent::Fatal("resource failure (OOM/disk)".to_string())
        }
        FailureKind::Structural => {
            // Regenerate verify script, then retry
            tracing::info!(plan_id, task_id, "structural failure — regenerating verify");
            // ... trigger verify regen ...
            ExecutorEvent::GateFailed
        }
        FailureKind::Transient => {
            // Short cooldown, retry with same prompt
            tokio::time::sleep(Duration::from_secs(2)).await;
            ExecutorEvent::GateFailed
        }
        FailureKind::Permanent => {
            if gate_failure_count > retry_budget {
                ExecutorEvent::Fatal(failure_reason)
            } else {
                // Retry with error digest injected into prompt
                // (handled by prompt builder on next dispatch)
                ExecutorEvent::GateFailed
            }
        }
    }
};
```

### Data Flow

```
Gate Failure
    |
    +---> [1] Extract GateFailureClassification
    |         (already done in TestGate/CompileGate/ClippyGate)
    |
    +---> [2] Classify into FailureKind
    |         Transient? Permanent? Resource? Structural?
    |
    +---> [3] Apply Per-Kind Retry Policy
    |         Transient: 2s cooldown, retry same prompt
    |         Permanent: no cooldown, retry with error digest
    |         Resource: abort immediately
    |         Structural: regen verify, then retry
    |
    +---> [4] Generate Reflection (if Permanent or Structural)
    |         Extract { error_type, file_patterns, fix_strategy }
    |         Persist to .roko/learn/reflections.jsonl
    |
    +---> [5] Check Playbook Auto-Creation
              If 3+ reflections with same error_type:
                Create playbook with confidence 0.5
                Persist to .roko/learn/playbooks/auto-{error_type}.json
```

## Implementation Plan

### Step 1: Add `FailureKind` to `roko-core`

**File**: `crates/roko-core/src/failure.rs` (new file)

- Define `FailureKind` enum with 4 variants: `Transient`, `Permanent`, `Resource`, `Structural`
- Implement `is_retryable()`, `retry_cooldown_secs()`, `needs_error_digest()`, `generates_reflection()`, `needs_verify_regen()`
- Implement `from_raw_output()` for fallback classification

**File**: `crates/roko-core/src/lib.rs`

- Add `pub mod failure;`
- Re-export `FailureKind`

### Step 2: Add classification bridge

**File**: `crates/roko-core/src/failure.rs`

- Implement `from_gate_classification()` that maps `GateFailureClassification` -> `FailureKind`
- Add `is_likely_flake()` helper

Note: This requires `roko-core` to depend on `roko-gate` for the classification types. If that creates a circular dependency, move the bridge to a separate function in `roko-cli` that takes both types. The `FailureClass` and `GateFailureClassification` types could also be moved to `roko-core` to avoid the dependency -- they are pure data types with no gate runtime logic.

### Step 3: Add `Reflection` and `ReflectionStore` to `roko-learn`

**File**: `crates/roko-learn/src/reflection.rs` (new file)

- Define `Reflection` struct with fields: `id`, `error_type`, `file_patterns`, `fix_strategy`, `error_digest`, `task_id`, `plan_id`, `failure_kind`, `created_at`
- Define `ReflectionStore` with JSONL backing (same pattern as `PlaybookStore` but append-only like `KnowledgeStore`)
- Implement `add()`, `read_all()`, `count_by_error_type()`, `find_by_error_type()`
- Implement `Reflection::from_gate_failure()` constructor

**File**: `crates/roko-learn/src/lib.rs`

- Add `pub mod reflection;`
- Re-export `Reflection`, `ReflectionStore`

### Step 4: Add playbook auto-creation logic

**File**: `crates/roko-learn/src/reflection.rs`

- Add `maybe_create_playbooks()` async function
- Add `build_playbook_from_reflections()` helper
- Default threshold: 3 reflections with same `error_type`
- Created playbook starts with confidence 0.5 (no success/failure count yet)

### Step 5: Wire `FailureKind` into gate completion handler (runner path)

**File**: `crates/roko-cli/src/runner/event_loop.rs`

In the gate completion branch (around line 243-258):

```rust
// Current: binary fail
match executor.apply_event(&completion.plan_id, &ExecutorEvent::GateFailed) { ... }

// New: classify, then decide
let failure_kind = classify_completion_failure(&completion);
match failure_kind {
    FailureKind::Resource => {
        let _ = executor.apply_event(
            &completion.plan_id,
            &ExecutorEvent::Fatal("resource exhaustion".into()),
        );
    }
    FailureKind::Transient | FailureKind::Permanent | FailureKind::Structural => {
        match executor.apply_event(&completion.plan_id, &ExecutorEvent::GateFailed) { ... }
    }
}
```

Add a helper:

```rust
fn classify_completion_failure(completion: &GateCompletion) -> FailureKind {
    // Try to parse the structured classification from verdict error_digest
    for verdict in &completion.verdicts {
        if !verdict.passed {
            if let Some(digest) = &verdict.error_digest {
                if let Ok(classification) = serde_json::from_str::<GateFailureClassification>(digest) {
                    return FailureKind::from_gate_classification(&classification);
                }
            }
            // Fallback to raw output
            return FailureKind::from_raw_output(&verdict.reason);
        }
    }
    FailureKind::Permanent // conservative default
}
```

### Step 6: Wire reflection generation into gate completion handler

**File**: `crates/roko-cli/src/runner/event_loop.rs` (or extracted to a helper module)

After classifying the failure and before applying the executor event:

```rust
if failure_kind.generates_reflection() {
    let reflection = Reflection::from_gate_failure(
        &classification,
        &failure_kind,
        &completion.plan_id,
        &completion.task_id,
        &task_files,
    );
    let reflection_store = ReflectionStore::for_roko_dir(&paths.roko_dir);
    if let Err(e) = reflection_store.add(reflection) {
        warn!(err = %e, "failed to persist reflection");
    }

    // Check if we should auto-create a playbook
    let playbook_store = PlaybookStore::new(paths.roko_dir.join("learn").join("playbooks"));
    match maybe_create_playbooks(&reflection_store, &playbook_store, 3).await {
        Ok(created) if !created.is_empty() => {
            info!(playbooks = ?created, "auto-created playbooks from reflections");
        }
        Err(e) => warn!(err = %e, "playbook auto-creation failed"),
        _ => {}
    }
}
```

### Step 7: Wire `FailureKind` into orchestrate.rs (legacy path)

**File**: `crates/roko-cli/src/orchestrate.rs`

In the gate completion handling section (around line 7990-8000), replace the binary decision with the classified path. This mirrors Step 5 but for the orchestrate.rs event loop.

### Step 8: Enrich retry prompts with error digest for `Permanent` failures

**File**: `crates/roko-cli/src/orchestrate.rs` (prompt building)

When `failure_kind.needs_error_digest()` is true and the task is being retried:

```rust
// In the prompt builder for retry attempts:
if let Some(error_digest) = &tracker.last_error_digest {
    prompt_sections.push(format!(
        "## Previous Failure\n\nThe last attempt failed with:\n```\n{error_digest}\n```\n\nFix the root cause before re-attempting."
    ));
}
```

This is partially implemented already via `learned_error_signature()` and `with_task_failure_context()`. The new design makes it explicit and conditional on `FailureKind::Permanent`.

## Verification

### Unit tests

1. **FailureKind::from_gate_classification**: Test each `FailureClass` maps to the expected `FailureKind`. Test that `ExternalEnvironment` -> `Transient`, `ImportError` -> `Permanent`, `UnsafeStubOrPassBehavior` -> `Structural`.

2. **FailureKind::from_raw_output**: Test OOM detection ("out of memory" -> Resource), timeout ("timed out" -> Transient), default ("unknown error" -> Permanent).

3. **is_likely_flake**: Test that "1 failed, 47 passed" is a flake. Test that "47 failed" is not.

4. **Reflection::from_gate_failure**: Construct a `GateFailureClassification` with 2 compile errors. Verify the reflection captures both file patterns and the correct error_type.

5. **ReflectionStore roundtrip**: Add 5 reflections with 3 having the same error_type. Verify `count_by_error_type()` returns 3. Verify `find_by_error_type()` returns the correct 3.

6. **maybe_create_playbooks**: Add 3 reflections with error_type "import_error". Call `maybe_create_playbooks()` with threshold 3. Verify a playbook named "auto-import_error" was created with steps derived from the reflections' fix strategies.

7. **Idempotency**: Call `maybe_create_playbooks()` twice with the same reflections. Verify the playbook is only created once (second call skips because it already exists).

8. **Per-kind retry policy**: Verify `Transient.retry_cooldown_secs() == 2`, `Resource.is_retryable() == false`, `Permanent.needs_error_digest() == true`, `Structural.needs_verify_regen() == true`.

### Integration test

```bash
# 1. Set up a plan with a task that will fail with a known error type
# (e.g., intentionally wrong import in a test file)
cargo run -p roko-cli -- plan run plans/test-failure-retry/

# 2. After run, verify:
#    - .roko/learn/reflections.jsonl contains reflection entries
#    - Each reflection has error_type, file_patterns, fix_strategy
#    - If 3+ same error_type: .roko/learn/playbooks/auto-<type>.json exists

# 3. Verify retry behavior:
#    - Resource failures (OOM): task is marked Fatal, not retried
#    - Transient failures: task is retried after ~2s
#    - Permanent failures: task is retried with error digest in prompt
```

### CLI verification

```bash
# Check reflections
cat .roko/learn/reflections.jsonl | jq '.error_type' | sort | uniq -c
# Should show grouped reflections by type

# Check auto-created playbooks
ls .roko/learn/playbooks/auto-*.json
# Should show playbooks for error types with 3+ reflections

# Check playbook contents
cat .roko/learn/playbooks/auto-import_error.json | jq '.steps[].description'
# Should show fix strategies derived from reflections
```

## Rating: 9.5/10

**Strengths**: The design bridges the existing gap between gate-level classification (which already exists in `roko-gate`) and retry-level policy (which doesn't exist). It reuses `FailureClass`, `GateFailureClassification`, `extract_error_digest`, and `PlaybookStore` -- all of which are built and tested. The `Reflection` struct is a minimal new type that captures exactly the information needed for playbook auto-creation. The 3-reflection threshold for playbook creation is conservative enough to avoid junk playbooks but responsive enough to learn from recurring patterns.

**Residual risk**: The classification bridge assumes the `GateFailureClassification` can be round-tripped through the verdict's `error_digest` field (which currently contains `render_failure_classification()` output -- pretty JSON). If the JSON parse fails, the system falls back to `from_raw_output()` which is less precise. A cleaner approach would be to add a `classification: Option<GateFailureClassification>` field to the verdict struct, but that requires a change to `roko-core`'s `Verdict` type. The fallback is adequate for the initial wiring; the structured field can be added in a follow-up. The rating reflects this as an acceptable compromise.

## Implementation Packet

This work turns gate failures into structured retry decisions and durable playbooks.

### Required Context

- `crates/roko-gate/src/compile_errors.rs`
- `crates/roko-gate/src/gate_pipeline.rs`
- `crates/roko-orchestrator/src/executor/state_machine.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-learn/src/playbook.rs`
- `crates/roko-learn/src/error_pattern_store.rs`
- `docs/04-verification/03-gate-pipeline.md`
- `docs/05-learning/01-playbook-system.md`
- `tmp/unified/04-EXECUTION.md`

### Target Files

- [ ] Add or update `crates/roko-cli/src/runner/failure.rs`.
- [ ] Add structured failure payload to runner gate completion.
- [ ] Update retry decision path in runner.
- [ ] Update playbook recording path.

### Checklist

- [ ] Preserve structured `GateFailureClassification` as data instead of only `error_digest` text.
- [ ] Map each `FailureClass` to retry, replan, blocked, or human-required action.
- [ ] Add cooldown/backoff per failure kind.
- [ ] Include failure kind and retry action in prompt retry context.
- [ ] Record reflection after every failed attempt.
- [ ] Create or update a playbook after recurring similar failures.
- [ ] Stop retrying when repeated failure class indicates architectural conflict.
- [ ] Emit projection events explaining retry reason.

### Acceptance Criteria

- [ ] Compile syntax error triggers auto-fix retry.
- [ ] Missing dependency triggers retry or replan according to configured policy.
- [ ] Architectural conflict triggers replan/block rather than blind retry.
- [ ] Three similar reflections create a playbook candidate.
- [ ] Retry prompt contains structured failure data, not only raw gate output.
