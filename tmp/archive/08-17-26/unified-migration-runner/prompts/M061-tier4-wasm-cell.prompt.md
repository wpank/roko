# M061 — SPI Tier 4: WASM Cell Runtime

## Objective
Implement Tier 4 of the 5-tier SPI: WASM Cell runtime. Define the wit-bindgen ABI for Cells compiled to WASM. Integrate wasmtime for sandboxed WASM execution with fuel metering to prevent infinite loops. This enables third-party Cells to run in a secure sandbox with deterministic resource limits.

## Scope
- Crates: `roko-core`, `roko-runtime`
- Files: `crates/roko-core/src/wasm_abi/` (new directory), `crates/roko-runtime/src/wasm/` (new directory)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.10
- Spec ref: `tmp/unified/14-CONFIG-AND-AUTHORING.md` SS3 (5-Tier SPI), `tmp/unified/20-DEPLOYMENT.md` SS5

## Steps
1. Check for existing WASM-related code:
   ```bash
   grep -rn 'wasm\|wasmtime\|wit\|fuel' crates/ --include='*.rs' | grep -v target | head -15
   grep 'wasmtime\|wit-bindgen' Cargo.toml crates/*/Cargo.toml 2>/dev/null | head -10
   ```

2. Add wasmtime dependency to roko-runtime/Cargo.toml:
   ```toml
   [dependencies]
   wasmtime = { version = "latest-stable", features = ["component-model"] }
   ```

3. Define the Cell ABI in `crates/roko-core/src/wasm_abi/mod.rs`:
   ```rust
   /// Input/output format for WASM Cells.
   /// WASM Cells receive JSON-encoded input and return JSON-encoded output.
   pub struct WasmCellInput {
       pub signal: serde_json::Value,
       pub context: serde_json::Value,
   }

   pub struct WasmCellOutput {
       pub signals: Vec<serde_json::Value>,
       pub metadata: serde_json::Value,
   }
   ```

4. Implement the WASM runtime in `crates/roko-runtime/src/wasm/mod.rs`:
   ```rust
   pub struct WasmCellRuntime {
       engine: wasmtime::Engine,
       fuel_limit: u64,
   }

   impl WasmCellRuntime {
       pub fn new(fuel_limit: u64) -> Result<Self>;
       pub fn execute(&self, wasm_bytes: &[u8], input: WasmCellInput) -> Result<WasmCellOutput>;
   }
   ```

5. Configure fuel metering:
   - Default fuel limit: 1_000_000 (configurable per-Cell)
   - Fuel exhaustion returns `CellError::FuelExhausted` instead of hanging
   - Memory limit: 64MB default (configurable)

6. Implement a wrapper Cell that loads and executes a WASM binary:
   ```rust
   pub struct WasmCell {
       name: String,
       wasm_path: PathBuf,
       runtime: Arc<WasmCellRuntime>,
   }
   ```

7. Write tests:
   - A simple WASM Cell (compile a trivial Rust function to WASM) executes and returns output
   - Fuel limit prevents infinite loops (WASM with infinite loop hits fuel limit)
   - Memory limit prevents excessive allocation

## Verification
```bash
cargo check -p roko-core
cargo check -p roko-runtime
cargo clippy -p roko-runtime --no-deps -- -D warnings
cargo test -p roko-runtime -- wasm
```

## What NOT to do
- Do NOT implement the full wit-bindgen component model in this batch -- start with a simple JSON-in/JSON-out ABI
- Do NOT compile roko itself to WASM here -- that is M094
- Do NOT add WASM Cell discovery/registry -- that is handled by the marketplace (M062)
- Do NOT give WASM Cells direct filesystem or network access -- all I/O goes through the host ABI
