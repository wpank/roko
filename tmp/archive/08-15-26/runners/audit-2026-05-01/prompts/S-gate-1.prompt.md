# S-gate-1: Symbol gate (Rung 3) implementation

## Task
Implement `crates/roko-gate/src/symbol_gate.rs`: validates the symbol manifest against the live source. Returns `GateOutcome::{Passed, Failed, Skipped, Error}`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/29-gate-pipeline-rungs-3-5-6.md` § G-1.

## Read first

```bash
rg 'pub struct SymbolGraph|pub fn scan' crates/roko-codeintel/src/index.rs
rg 'pub enum GateOutcome|pub enum GateStatus' crates/roko-gate/src/ -n
```

## Exact changes

### `crates/roko-gate/src/symbol_gate.rs` (new)

```rust
//! Rung 3 — Symbol gate.
//!
//! Validates that the persisted symbol manifest (`symbols.json`) matches
//! the symbols actually present in the source. Detects drifted exports,
//! undefined references, missing tree-sitter parses.

use std::path::Path;
use crate::{GateOutcome, Rung};

pub async fn run_symbol_gate(workdir: &Path) -> GateOutcome {
    let manifest_path = workdir.join("symbols.json");
    let alt_path = workdir.join(".roko").join("symbols.json");
    let manifest_path = if manifest_path.exists() {
        manifest_path
    } else if alt_path.exists() {
        alt_path
    } else {
        return GateOutcome::Skipped {
            rung: Rung::Symbol,
            reason: "no symbol manifest".into(),
        };
    };

    let manifest = match roko_codeintel::SymbolManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => return GateOutcome::Error {
            rung: Rung::Symbol,
            error: format!("load manifest: {e}"),
        },
    };

    let live = match roko_codeintel::SymbolGraph::scan(workdir) {
        Ok(g) => g,
        Err(e) => return GateOutcome::Error {
            rung: Rung::Symbol,
            error: format!("scan source: {e}"),
        },
    };

    let drift = manifest.diff(&live);
    if drift.is_empty() {
        GateOutcome::Passed { rung: Rung::Symbol }
    } else {
        GateOutcome::Failed {
            rung: Rung::Symbol,
            rationale: format!(
                "{} symbols drifted: {}{}",
                drift.len(),
                drift.iter().take(5).map(|d| d.symbol.clone()).collect::<Vec<_>>().join(", "),
                if drift.len() > 5 { ", ..." } else { "" },
            ),
        }
    }
}
```

If `SymbolManifest`, `SymbolGraph::scan`, or `manifest.diff` doesn't exist in `roko-codeintel`, this batch is **blocked on roko-codeintel** — log and stop.

### Mount in `lib.rs`

`crates/roko-gate/src/lib.rs`:

```rust
pub mod symbol_gate;
```

### Tests

```rust
#[tokio::test]
async fn symbol_gate_skipped_when_no_manifest() {
    let dir = tempdir().unwrap();
    let outcome = run_symbol_gate(dir.path()).await;
    assert!(matches!(outcome, GateOutcome::Skipped { .. }));
}

#[tokio::test]
async fn symbol_gate_passes_when_manifest_matches() {
    let dir = tempdir().unwrap();
    // Set up a manifest that matches what scan would produce.
    // Hard to do without real tree-sitter; this test may need the real codebase.
}

#[tokio::test]
async fn symbol_gate_fails_when_drifted() {
    // Create a manifest claiming symbols that don't exist; expect Failed.
}
```

## Write Scope
- `crates/roko-gate/src/symbol_gate.rs` (new)
- `crates/roko-gate/src/lib.rs`

## Read-Only Context
- `crates/roko-codeintel/src/index.rs`

## Verify

```bash
ls crates/roko-gate/src/symbol_gate.rs

rg 'run_symbol_gate' crates/roko-gate/src/
```

## Do NOT

- Do NOT bundle with S-gate-2/3.
- Do NOT add `#[allow(...)]` to silence missing-manifest warnings.
- Do NOT let Skipped become a vague catch-all; must include a `reason`.
- Do NOT call this gate from the orchestrator pipeline yet; that's S-gate-4.
