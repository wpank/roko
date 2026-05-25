# M094 — WASM Compilation Target

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/20-deployment/` depth docs. The depth docs specify which crates compile to WASM, feature flag strategy, and progressive enhancement approach.

## Objective
Compile roko-core and selected Cells to WASM via `wasm32-wasi`. Verify that core types (Signal, Pulse, Cell trait) work correctly in a WASM execution context. This enables Cells to be compiled once and run anywhere, including in the browser and in sandboxed WASM runtimes.

## Scope
- Crates: `roko-core` (primary), workspace Cargo.toml
- Files: workspace `Cargo.toml` (add wasm target), `crates/roko-core/Cargo.toml` (feature flags)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.8
- Spec ref: `tmp/unified/20-DEPLOYMENT.md` SS5
- Depth docs: `tmp/unified-depth/20-deployment/` (pending)

## Steps
1. Check current WASM compatibility:
   ```bash
   grep -rn 'wasm\|target.*wasm' Cargo.toml crates/roko-core/Cargo.toml | head -10
   ```

2. Add WASM target to rustup:
   ```bash
   rustup target add wasm32-wasip1
   ```

3. Add feature flags to roko-core for WASM compatibility:
   ```toml
   [features]
   default = ["full"]
   full = ["tokio", "reqwest"]
   wasm = []  # Excludes tokio, reqwest, and other non-WASM deps
   ```

4. Conditionally compile non-WASM-compatible code behind `#[cfg(not(target_arch = "wasm32"))]`.

5. Attempt to compile core types:
   ```bash
   cargo build -p roko-core --target wasm32-wasip1 --no-default-features --features wasm
   ```

6. Verify core types work: write a test Cell that creates a Signal, processes it, and returns output -- compile and run in wasmtime.

7. Document which crates can currently compile to WASM and which cannot (and why).

## Verification
```bash
cargo check -p roko-core --target wasm32-wasip1 --no-default-features --features wasm
# If compilation succeeds:
cargo build -p roko-core --target wasm32-wasip1 --no-default-features --features wasm
```

## What NOT to do
- Do NOT try to compile the entire workspace to WASM -- start with roko-core only
- Do NOT remove async from the codebase -- use feature flags for conditional compilation
- Do NOT proceed without depth docs for feature flag strategy
- Do NOT break native compilation -- WASM support is additive via feature flags
