# 29 — Gate Pipeline Rungs 3, 5, 6

The 7-rung gate pipeline today actually runs 3-4 rungs. Rungs 3
(Symbol), 5 (PropertyTest), and 6 (Integration) are gated off by
hardcoded flags or missing implementations. T1-10 (explicit match arms)
and T1-11 (auto-detected scaffolding flags) unblocked the **selection**
of these rungs; they still need real implementations.

Source: subsystem-audits/gate-pipeline/AUDIT.md, doc 36, doc 40.

---

## Today's State

- T1-10: explicit match arms in `selected_gate_steps` for each rung.
- T1-11: `gate_rung_caps` detects scaffolding (`symbols.json`,
  `proptest-regressions/`, `tests/integration/`) so caps reflect actual
  workspace shape.
- Rungs 3, 5, 6 may now be selected, but their gate runners do not exist
  or return empty stubs.

---

## Anti-Patterns

1. **No "skip rung if not in workspace" without recording.** A skipped
   rung is `GateStatus::Skipped { reason }`, not absent.
2. **No gate that ignores its input.** Each rung must consume the
   artifact / state it claims to validate.
3. **No silent pass.** A gate must always return one of `Passed`,
   `Failed`, `Skipped`, `Error` — never "I don't know."
4. **No "this rung returns Passed unless catastrophic failure."** That's
   the "agent loop deceives gates" pattern.
5. **No new dispatch path inside a gate.** Gates use shared services
   (compile via cargo, test via cargo, etc.).

---

## Plan

### [ ] G-1: Symbol gate (Rung 3)

**Purpose**: Validate the symbol manifest produced by tree-sitter
indexing matches the source. Detects undefined references, broken
imports, missing exports.

**Implementation**: `crates/roko-gate/src/symbol_gate.rs` (new)

Use `roko-codeintel`'s symbol graph:

```rust
pub async fn run_symbol_gate(workdir: &Path) -> GateOutcome {
    let manifest_path = workdir.join("symbols.json");
    if !manifest_path.exists() {
        return GateOutcome::Skipped { reason: "no symbol manifest" };
    }
    let manifest = SymbolManifest::load(&manifest_path)?;
    let graph = roko_codeintel::SymbolGraph::scan(workdir)?;
    let drift = manifest.diff(&graph);
    if drift.is_empty() {
        GateOutcome::Passed
    } else {
        GateOutcome::Failed {
            rationale: format!("{} symbols drifted: {}", drift.len(), drift.iter().take(5).join(", ")),
        }
    }
}
```

**Estimated effort**: 6-10 hours (depends on existing `SymbolGraph` API
maturity).

### [ ] G-2: PropertyTest gate (Rung 5)

**Purpose**: Run proptest regressions and assert they all pass. Today
proptest seed corpora exist (`proptest-regressions/`); they need to be
consumed.

**Implementation**: `crates/roko-gate/src/property_test_gate.rs` (new)

```rust
pub async fn run_property_test_gate(workdir: &Path) -> GateOutcome {
    let regressions_dir = workdir.join("proptest-regressions");
    let property_tests_dir = workdir.join("tests").join("property");
    if !regressions_dir.exists() && !property_tests_dir.exists() {
        return GateOutcome::Skipped { reason: "no property tests" };
    }

    // Run cargo test --test property -- --include-ignored --quiet
    let output = tokio::process::Command::new("cargo")
        .args(["test", "--test", "property", "--", "--include-ignored", "--quiet"])
        .current_dir(workdir)
        .output().await?;
    if output.status.success() {
        GateOutcome::Passed
    } else {
        GateOutcome::Failed {
            rationale: String::from_utf8_lossy(&output.stderr).into(),
        }
    }
}
```

**Estimated effort**: 4-6 hours.

### [ ] G-3: Integration gate (Rung 6)

**Purpose**: Run integration scenarios (full workflows or end-to-end
tests).

**Implementation**: `crates/roko-gate/src/integration_gate.rs` (new)

```rust
pub async fn run_integration_gate(workdir: &Path, scenarios: &[IntegrationScenario]) -> GateOutcome {
    // Each scenario is a YAML file under tests/integration/.
    // The scenario lists steps; each step is a roko command + expected outcome.
    let mut failed = vec![];
    for scenario in scenarios {
        match run_scenario(workdir, scenario).await {
            Ok(()) => {}
            Err(e) => failed.push(format!("{}: {}", scenario.name, e)),
        }
    }
    if failed.is_empty() {
        GateOutcome::Passed
    } else {
        GateOutcome::Failed { rationale: failed.join("\n") }
    }
}
```

**Estimated effort**: 8-12 hours.

### [ ] G-4: Wire all three gates into the pipeline

**File**: `crates/roko-cli/src/orchestrate.rs::run_gate_pipeline` (or
equivalent).

The pipeline already has slot for each rung after T1-10. Add the gate
runner calls:

```rust
let symbol_outcome = if caps.has_symbol_manifest {
    Some(run_symbol_gate(workdir).await)
} else { None };

let proptest_outcome = if caps.has_property_tests {
    Some(run_property_test_gate(workdir).await)
} else { None };

let integration_outcome = if caps.has_integration_scenario {
    Some(run_integration_gate(workdir, &scenarios).await)
} else { None };
```

Record each outcome in the run ledger (plan 24).

### [ ] G-5: Regression tests

```rust
#[tokio::test]
async fn symbol_gate_detects_missing_export() {
    let workdir = make_workdir_with_drifted_symbols();
    let outcome = run_symbol_gate(&workdir).await;
    assert!(matches!(outcome, GateOutcome::Failed { .. }));
}

#[tokio::test]
async fn property_gate_skipped_when_no_corpus() {
    let workdir = make_empty_workdir();
    let outcome = run_property_test_gate(&workdir).await;
    assert!(matches!(outcome, GateOutcome::Skipped { .. }));
}
```

---

## Combined Verification

```bash
cargo test -p roko-gate symbol_gate --lib
cargo test -p roko-gate property_test_gate --lib
cargo test -p roko-gate integration_gate --lib

# In a workspace with all scaffolding, all 7 rungs run
cd test-workspace-with-everything
cargo run -p roko-cli -- plan run --plan plans/sample/
# Look for: "rung 3 (Symbol): Passed", "rung 5 (PropertyTest): Passed", etc.
```

---

## Status

- [ ] G-1 — Symbol gate
- [ ] G-2 — PropertyTest gate
- [ ] G-3 — Integration gate
- [ ] G-4 — Wire into pipeline
- [ ] G-5 — Regression tests

**Estimated effort**: 24-40 hours.
