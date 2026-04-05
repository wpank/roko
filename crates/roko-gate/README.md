# roko-gate

Concrete `Gate` implementations for Roko. Gates verify signals against ground truth by shelling out to real tools (compilers, test runners, linters).

## Install

```toml
[dependencies]
roko-gate = { path = "../roko-gate" }
roko-core = { path = "../roko-core" }
```

## Gates shipped

| Gate | What it runs | Passes when |
| --- | --- | --- |
| `ShellGate` | Arbitrary command | exit code 0 |
| `CompileGate` | `cargo check` / `npm run build` / `go build` / `python -m py_compile` / `forge build` / `make` | exit code 0 |
| `ClippyGate` | `cargo clippy -- -D warnings` (Rust only) | no warnings + exit 0 |
| `TestGate` | `cargo test` / `npm test` / `go test` / `pytest` / `forge test` | all tests pass |
| `DiffGate` | Analyzes git/working-tree diff | passes/fails based on diff characteristics |

Every gate reads a `GatePayload` from the input signal, runs the subprocess against `payload.working_dir`, and emits a `Verdict`.

## Build systems

`BuildSystem` enum: `Cargo`, `Npm`, `Go`, `Python`, `Forge`, `Make`. Each of `CompileGate`, `ClippyGate`, `TestGate` takes one and picks the appropriate command.

## Example

```rust
use roko_gate::{CompileGate, GatePayload, BuildSystem};
use roko_core::{Body, Context, Gate, Kind, Signal};

let payload = GatePayload::in_dir("/path/to/my-crate").with_label("my-gate");
let input = Signal::builder(Kind::Task)
    .body(Body::from_json(&payload)?)
    .build();

let gate = CompileGate::new(BuildSystem::Cargo).with_timeout_ms(300_000);
let verdict = gate.verify(&input, &Context::now()).await;
if verdict.passed { /* ok */ }
```

## DiffGate

`DiffGate` + `analyze_diff` inspect a diff payload and return `DiffAnalysis` (lines added/removed, files touched, whether binary files changed). Use as a cheap pre-filter before invoking expensive gates on large agent outputs.

## Timeouts

Every gate has `.with_timeout_ms(...)` — defaults are tuned per gate (60s for shell, 10min for compile/test). Gates that exceed the timeout return `Verdict::fail` with a timeout reason.
