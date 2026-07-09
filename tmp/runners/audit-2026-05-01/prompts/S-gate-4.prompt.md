# S-gate-4: Wire gates 3/5/6 into orchestrate pipeline

## Task
Make the orchestrate gate pipeline actually call `run_symbol_gate`, `run_property_test_gate`, and `run_integration_gate` when the corresponding scaffolding is present (per T1-11's `gate_rung_caps`).

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-gate-1, S-gate-2, S-gate-3. Wave 4.

## Source plan
`tmp/subsystem-audits/implementation-plans/29-gate-pipeline-rungs-3-5-6.md` § G-4.

## Read first

```bash
rg 'fn run_gate_pipeline|fn execute_gates|gate_rung_caps' crates/roko-cli/src/orchestrate.rs -n
```

T1-11 already detects scaffolding and sets `caps.has_symbol_manifest`, `caps.has_property_tests`, `caps.has_integration_scenario`. T1-10 made each rung's match arm explicit. Now invoke the actual gate runners.

## Exact changes

`crates/roko-cli/src/orchestrate.rs` (or its post-T5-35 split):

In the gate-execution loop, for each rung selected by `select_rungs`, dispatch to the corresponding runner:

```rust
for rung in selected_rungs {
    let outcome = match rung {
        Rung::Compile => run_compile_gate(workdir).await,
        Rung::Test => run_test_gate(workdir).await,
        Rung::Clippy => run_clippy_gate(workdir).await,
        Rung::Symbol => roko_gate::symbol_gate::run_symbol_gate(workdir).await,
        Rung::Diff => run_diff_gate(workdir).await,
        Rung::PropertyTest => roko_gate::property_test_gate::run_property_test_gate(workdir).await,
        Rung::Integration => roko_gate::integration_gate::run_integration_gate(workdir).await,
    };
    record_outcome(rung, outcome, ...);
}
```

If the existing pipeline uses a `Box<dyn GateRunner>` registry pattern (via `GateRegistry`), add the new runners to the registry instead of branching.

```rust
let registry = GateRegistry::default()
    .with(Rung::Symbol, Box::new(SymbolGateRunner))
    .with(Rung::PropertyTest, Box::new(PropertyTestGateRunner))
    .with(Rung::Integration, Box::new(IntegrationGateRunner));
```

Pick whichever pattern matches the current orchestrate code; do not introduce a new dispatch mechanism.

### Record into RunLedger (T5-40a)

After T5-40a lands, the per-rung outcome flows into `ledger.record_gate(rung, status, ...)`. If T5-40a hasn't landed, the existing observation path stays.

## Write Scope
- `crates/roko-cli/src/orchestrate.rs` (or current orchestrate location)

## Read-Only Context
- `crates/roko-gate/src/symbol_gate.rs`
- `crates/roko-gate/src/property_test_gate.rs`
- `crates/roko-gate/src/integration_gate.rs`

## Verify

```bash
rg 'run_symbol_gate|run_property_test_gate|run_integration_gate' crates/roko-cli/src/
# Expect: at least 3 hits

# In a workspace with all scaffolding, all 7 rungs run
# Manual smoke: cd to test workspace, cargo run -p roko-cli -- plan run, check logs
```

## Do NOT

- Do NOT bundle with S-gate-1/2/3.
- Do NOT make rung 3/5/6 mandatory; respect `caps.has_*` flags.
- Do NOT add a new dispatch mechanism if `GateRegistry` already exists.
- Do NOT block the pipeline on `Skipped { reason }` — Skipped is not a failure.
