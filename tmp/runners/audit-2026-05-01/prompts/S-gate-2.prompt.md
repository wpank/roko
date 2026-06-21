# S-gate-2: PropertyTest gate (Rung 5) implementation

## Task
Implement `crates/roko-gate/src/property_test_gate.rs`. Runs proptest regressions and asserts they all pass. Returns `GateOutcome`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/29-gate-pipeline-rungs-3-5-6.md` § G-2.

## Exact changes

### `crates/roko-gate/src/property_test_gate.rs` (new)

```rust
use std::path::Path;
use crate::{GateOutcome, Rung};

pub async fn run_property_test_gate(workdir: &Path) -> GateOutcome {
    let regressions = workdir.join("proptest-regressions");
    let property_tests = workdir.join("tests").join("property");
    if !regressions.exists() && !property_tests.exists() {
        return GateOutcome::Skipped {
            rung: Rung::PropertyTest,
            reason: "no property tests or regressions corpus".into(),
        };
    }

    // Run the property test suite. Convention: `cargo test --test property`.
    let output = match tokio::process::Command::new("cargo")
        .args(["test", "--test", "property", "--", "--include-ignored", "--quiet"])
        .current_dir(workdir)
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => return GateOutcome::Error {
            rung: Rung::PropertyTest,
            error: format!("spawn cargo test: {e}"),
        },
    };

    if output.status.success() {
        GateOutcome::Passed { rung: Rung::PropertyTest }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        GateOutcome::Failed {
            rung: Rung::PropertyTest,
            rationale: stderr.lines().take(20).collect::<Vec<_>>().join("\n"),
        }
    }
}
```

### Mount

`crates/roko-gate/src/lib.rs`:

```rust
pub mod property_test_gate;
```

### Tests

```rust
#[tokio::test]
async fn property_test_gate_skipped_when_absent() {
    let dir = tempdir().unwrap();
    let outcome = run_property_test_gate(dir.path()).await;
    assert!(matches!(outcome, GateOutcome::Skipped { .. }));
}

// A "passes" test would require a real cargo invocation; usually skip in unit tests.
// Add to integration tests if there's a fixture.
```

## Write Scope
- `crates/roko-gate/src/property_test_gate.rs` (new)
- `crates/roko-gate/src/lib.rs`

## Verify

```bash
ls crates/roko-gate/src/property_test_gate.rs

rg 'run_property_test_gate' crates/roko-gate/src/
```

## Do NOT

- Do NOT bundle with S-gate-1/3.
- Do NOT include uncommon test conventions (e.g. `cargo nextest`) — `cargo test --test property` is the contract.
- Do NOT skip if `proptest-regressions` exists but `tests/property` doesn't; still run.
- Do NOT pipe stdout to anything; stderr capture only.
