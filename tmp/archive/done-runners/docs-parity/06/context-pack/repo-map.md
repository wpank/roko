# Repo Map — Shared Neuro Context

## Workspace Reality

- workspace members: **36**
- total Rust LOC: **322,088**
- event bus event types: **2** (`PlanRevision`, `PrdPublished`)
- `roko-serve`: **200+ routes**
- TUI: **58K LOC**
- `roko-learn`: **42 modules**

## PU06 Paths

| Path | Why it matters |
|------|----------------|
| `crates/roko-core/src/engram.rs` | top-priority HDC-on-Engram follow-up |
| `crates/roko-core/src/traits.rs` | `Substrate` still lacks `query_similar()` |
| `crates/roko-primitives/src/hdc.rs` | real HDC implementation |
| `crates/roko-neuro/src/` | shipped neuro subsystem |
| `crates/roko-cli/src/main.rs` | neuro CLI is still query / stats / gc only |
| `docs/06-neuro/` | source docs being corrected |
| `tmp/docs-parity/06/` | parity refresh bundle |

## Working Rule

Use the codebase numbers above to keep the docs grounded. If a concept has zero
lines of code, label it as deferred or target-state.

Dreams-side transfer logic is only adjacent evidence. It does not prove that the
general doc-08 resonance system ships today.
