# S-gate-3: Integration gate (Rung 6) implementation

## Task
Implement `crates/roko-gate/src/integration_gate.rs`. Runs integration scenarios (YAML files defining steps + expected outcomes). Returns `GateOutcome`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/29-gate-pipeline-rungs-3-5-6.md` § G-3.

## Exact changes

### Scenario format

```yaml
# tests/integration/<name>.yaml
name: "auth wiring smoke"
steps:
  - run: "cargo test --test auth_smoke -- --nocapture"
    expect_exit: 0
  - run: "rg 'pub fn login' src/auth.rs"
    expect_exit: 0
```

### `crates/roko-gate/src/integration_gate.rs` (new)

```rust
use std::path::Path;
use serde::Deserialize;
use crate::{GateOutcome, Rung};

#[derive(Deserialize)]
pub struct IntegrationScenario {
    pub name: String,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Deserialize)]
pub struct ScenarioStep {
    pub run: String,
    #[serde(default)]
    pub expect_exit: i32,
}

pub async fn run_integration_gate(workdir: &Path) -> GateOutcome {
    let dir = workdir.join("tests").join("integration");
    let alt = workdir.join("integration-tests");
    let dir = if dir.exists() { dir } else if alt.exists() { alt } else {
        return GateOutcome::Skipped {
            rung: Rung::Integration,
            reason: "no integration scenarios directory".into(),
        };
    };

    let scenarios = match load_scenarios(&dir).await {
        Ok(s) => s,
        Err(e) => return GateOutcome::Error {
            rung: Rung::Integration,
            error: format!("load scenarios: {e}"),
        },
    };

    if scenarios.is_empty() {
        return GateOutcome::Skipped {
            rung: Rung::Integration,
            reason: "scenarios directory empty".into(),
        };
    }

    let mut failed = Vec::new();
    for scenario in &scenarios {
        for (i, step) in scenario.steps.iter().enumerate() {
            // Use shell to run the command; respect exit code.
            let out = match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&step.run)
                .current_dir(workdir)
                .output()
                .await
            {
                Ok(o) => o,
                Err(e) => {
                    failed.push(format!("{}/step{i} ({}): spawn error {e}", scenario.name, step.run));
                    continue;
                }
            };
            let exit = out.status.code().unwrap_or(-1);
            if exit != step.expect_exit {
                failed.push(format!(
                    "{}/step{i} ({}): exit {exit} != expected {}",
                    scenario.name, step.run, step.expect_exit
                ));
            }
        }
    }

    if failed.is_empty() {
        GateOutcome::Passed { rung: Rung::Integration }
    } else {
        GateOutcome::Failed {
            rung: Rung::Integration,
            rationale: failed.join("\n"),
        }
    }
}

async fn load_scenarios(dir: &Path) -> std::io::Result<Vec<IntegrationScenario>> {
    let mut out = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(e) = entries.next_entry().await? {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("yaml") { continue; }
        let raw = tokio::fs::read_to_string(&p).await?;
        match serde_yaml::from_str::<IntegrationScenario>(&raw) {
            Ok(s) => out.push(s),
            Err(e) => tracing::warn!(file = %p.display(), error = %e, "skip invalid scenario"),
        }
    }
    Ok(out)
}
```

### Mount

```rust
pub mod integration_gate;
```

### Tests

```rust
#[tokio::test]
async fn integration_gate_skipped_when_no_dir() {
    let dir = tempdir().unwrap();
    let outcome = run_integration_gate(dir.path()).await;
    assert!(matches!(outcome, GateOutcome::Skipped { .. }));
}

#[tokio::test]
async fn integration_gate_passes_when_step_succeeds() {
    let dir = tempdir().unwrap();
    let scen = dir.path().join("tests/integration/smoke.yaml");
    std::fs::create_dir_all(scen.parent().unwrap()).unwrap();
    std::fs::write(&scen, "name: smoke\nsteps:\n  - run: 'true'\n    expect_exit: 0\n").unwrap();
    let outcome = run_integration_gate(dir.path()).await;
    assert!(matches!(outcome, GateOutcome::Passed { .. }));
}
```

## Write Scope
- `crates/roko-gate/src/integration_gate.rs` (new)
- `crates/roko-gate/src/lib.rs`
- `crates/roko-gate/Cargo.toml` (add `serde_yaml` if missing)

## Verify

```bash
ls crates/roko-gate/src/integration_gate.rs

rg 'run_integration_gate' crates/roko-gate/src/
```

## Do NOT

- Do NOT use a custom DSL when YAML is fine.
- Do NOT bundle with S-gate-1/2.
- Do NOT run scenarios in parallel without bounded concurrency.
- Do NOT skip step on empty `run` — that's a malformed YAML, error.
