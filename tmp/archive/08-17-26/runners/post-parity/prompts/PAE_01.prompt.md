# PAE_01: Consolidate 3 gate dispatch paths into GateService

## Task
Wire all gate invocations through the existing `GateService` instead of 3 separate dispatch paths with different capabilities.

## Runner Context
Runner PAE (Gate Pipeline Completeness), batch 1 of 4. No dependencies.

## Problem
GP-1 anti-pattern: "Three gate paths, three behaviors." Gates are invoked from 3 different locations with different capabilities:
1. `roko run` (run.rs:2933-2980) — 4 hardcoded gates, no rung selection, no LLM judge
2. ACP runner (runner.rs:1645-1690) — 3 gates with adaptive thresholds, skip logic
3. orchestrate.rs (L16333-17178) — full 7-rung pipeline with rung dispatch (dead code)

There is already a `GateService` in `roko-gate/src/gate_service.rs` that was created to unify these. Wire it into paths 1 and 2.

## Current Code

**GateService** — `crates/roko-gate/src/gate_service.rs:191-250`:
Has `StubJudgeGate` returning "Skipped" for LLM judge. Intercepts `judge`/`llm-judge` requests.

**Path 1** — `crates/roko-cli/src/run.rs:2933-2980` (behind `legacy-orchestrate`):
```rust
fn run_gate() { /* iterates config.gates, calls Verify per GateConfig variant */ }
```

**Path 2** — `crates/roko-acp/src/runner.rs:1645-1690`:
```rust
fn run_gates() { /* hardcoded CompileGate/TestGate/ClippyGate, uses AdaptiveThresholds */ }
```

**Path 3** — orchestrate.rs (dead code, behind feature gate). Most complete.

## Exact Changes

### Step 1: Verify GateService API

Read `gate_service.rs` and confirm it supports:
- Accepting a list of gate names/configs
- Running gates in sequence
- Returning per-gate verdicts
- Adaptive threshold support

If it's missing adaptive thresholds, add:
```rust
impl GateService {
    pub fn with_adaptive_thresholds(mut self, thresholds: AdaptiveThresholds) -> Self { ... }
}
```

### Step 2: Replace run.rs gate invocation

```rust
// BEFORE (run.rs:2933-2980):
fn run_gate(&self, config: &GateConfig, workdir: &Path) -> Verdict { ... }

// AFTER:
let gate_service = GateService::new(workdir)
    .with_gates(config.gates.iter().map(|g| g.name.clone()).collect());
let verdicts = gate_service.run_all().await?;
let all_passed = verdicts.iter().all(|v| v.passed);
```

### Step 3: Replace ACP runner gate invocation

```rust
// BEFORE (runner.rs:1645-1690):
fn run_gates(&self) -> Result<bool> {
    // hardcoded CompileGate/TestGate/ClippyGate
}

// AFTER:
let gate_service = GateService::new(&self.workdir)
    .with_adaptive_thresholds(self.thresholds.clone())
    .with_gates(vec!["compile", "test", "clippy"]);
let verdicts = gate_service.run_all().await?;
```

### Step 4: Ensure GateService supports the union of features

GateService must support:
- From path 1: configurable gate list from roko.toml
- From path 2: adaptive thresholds with EMA, skip logic for high-pass-rate gates
- From path 3: rung-based dispatch (for future activation)

## Write Scope
- `crates/roko-gate/src/gate_service.rs` (extend if needed for thresholds/config)
- `crates/roko-cli/src/run.rs` (replace gate invocation)
- `crates/roko-acp/src/runner.rs` (replace gate invocation)

## Read-Only Context
- `crates/roko-gate/src/rung_dispatch.rs` (rung system, stub verdicts)
- `crates/roko-gate/src/lib.rs` (Gate trait, Verdict)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- All gate invocations route through GateService
- GateService supports adaptive thresholds (from ACP path)
- GateService supports configurable gate list (from run.rs path)
- Existing gate behavior unchanged for both paths
- GateService verdicts include per-gate pass/fail and timing

## Do NOT
- Wire rungs 3-6 in this prompt (that's PAE_02)
- Change the Gate trait or Verdict struct
- Remove adaptive threshold support from ACP
