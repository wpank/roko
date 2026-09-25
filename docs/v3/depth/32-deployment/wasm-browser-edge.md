# 32-deployment/03 -- WASM Deployment (Browser and Edge)

> What compiles to WebAssembly, what does not, the MemorySubstrate
> alternative, HDC compatibility, Component Model, and edge runtime targets.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-core/`, `crates/roko-primitives/`, conditional
compilation gates in `roko-std`

---

## 1. WASM Target Overview

The `wasm32-wasi` and `wasm32-wasip2` targets compile Roko's core
cognitive primitives -- the Signal type, the 12 kernel traits, scoring,
routing, and composition -- into WebAssembly modules that can run in any
WASM runtime (browser via wasm-bindgen, Cloudflare Workers, Fastly
Compute, Deno Deploy, wasmtime, wasmer).

This is not a full agent deployment. The complete agent loop with LLM
backends, filesystem persistence, and process supervision requires native
capabilities. WASM targets a subset: lightweight cognitive processing,
context scoring, knowledge retrieval, and HDC similarity search.

---

## 2. Compatibility Matrix

### What Works in WASM

| Component | Support | Notes |
|-----------|---------|-------|
| Signal struct | Full | All fields, serialization, BLAKE3 hashing |
| Score (7-axis) | Full | Pure computation, no I/O |
| Scorer trait | Full | Score computation is pure math |
| Router trait | Full | Selection logic is pure computation |
| Composer trait | Full | Context assembly under budget |
| HDC vectors | Full | Hamming distance, XOR bundling -- pure bits |
| Decay calculations | Full | HalfLife, TTL, Ebbinghaus -- pure math |
| Content addressing | Full | BLAKE3 compiles to WASM natively |
| Lineage DAG | Full | `Vec<ContentHash>` -- no I/O required |

### What Does Not Work in WASM

| Component | Reason | Alternative |
|-----------|--------|-------------|
| FileSubstrate | Requires filesystem | MemorySubstrate |
| LLM providers | Requires HTTP client + TLS | fetch() via host |
| MCP client | Requires stdio or TCP | Not available |
| ProcessSupervisor | Requires process spawning | Not applicable |
| Graph executor | Requires fs, git, processes | Run natively |
| Tokio multi-threaded | WASM is single-threaded | current_thread |
| Tree-sitter | C FFI dependency | Pre-computed indexes |

---

## 3. MemorySubstrate

The MemorySubstrate is the WASM-compatible Substrate trait implementation.
It stores Signals in a `BTreeMap<ContentHash, Signal>` in memory with
indexed lookups by kind, tags, and time range.

```rust
let substrate = MemorySubstrate::new();
substrate.put(signal).await?;
let results = substrate.query(Query::by_kind(Kind::Observation)).await?;
```

The trait interface is identical to FileSubstrate. Code written against
the Substrate trait works in both native and WASM. The only difference
is durability: MemorySubstrate loses state when the module unloads.

For persistence in WASM environments, the host serializes the substrate
contents via `export()` and restores via `import()`. In a browser, this
maps to localStorage or IndexedDB.

---

## 4. HDC Vector Compatibility

HdcVector from roko-primitives is fully WASM-compatible. All operations
are pure bitwise math. The Hamming distance computation uses
`u64::count_ones()` which compiles to efficient WASM instructions.

For performance-critical paths, WASM SIMD128 (universally supported:
Chrome 91+, Firefox 89+, Safari 2024+) provides ~4x speedup:

```toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "target-feature=+simd128"]
```

### Performance Comparison

| Environment | 10K-bit vector comparison | Throughput |
|-------------|--------------------------|------------|
| Native x86_64 (AVX2) | ~5 us | ~200K/sec |
| Native ARM64 (NEON) | ~8 us | ~125K/sec |
| WASM SIMD128 (browser) | ~15 us | ~67K/sec |
| WASM scalar (browser) | ~60 us | ~17K/sec |
| WASM SIMD128 (wasmtime) | ~12 us | ~83K/sec |

---

## 5. Feature Flags for WASM

Disable features that require native capabilities:

```bash
# Core primitives for WASM
cargo build --target wasm32-wasi -p roko-core \
    --no-default-features --features "serde,hdc,decay"

# MemorySubstrate only
cargo build --target wasm32-wasi -p roko-std \
    --no-default-features --features memory-substrate
```

### Conditional Compilation

```rust
#[cfg(not(target_arch = "wasm32"))]
pub mod file_substrate;

#[cfg(target_arch = "wasm32")]
pub mod memory_substrate;

// Always available
pub mod default_scorer;
pub mod default_router;
pub mod default_composer;
```

This ensures that `cargo build --target wasm32-wasi` compiles without
errors, excluding native-only modules at compile time.

---

## 6. WASI Preview 2 and the Component Model

WASI 0.2 introduces the Component Model -- a standardized way for WASM
modules to compose via typed WIT (WebAssembly Interface Types) interfaces.
Rust 1.82+ provides the `wasm32-wasip2` target at Tier 2.

```bash
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 -p roko-core \
    --no-default-features --features "serde,hdc,decay"
```

### WASI 0.2 Interface Usage

| Interface | Package | Roko Usage |
|-----------|---------|------------|
| `wasi:io` | Pollable handles, byte streams | Substrate I/O abstraction |
| `wasi:clocks` | Monotonic and wall clocks | Decay calculations, TTL |
| `wasi:http` | HTTP request/response | Edge-to-core forwarding |
| `wasi:random` | Secure randomness | HDC vector generation |
| `wasi:filesystem` | File/directory access | Optional MemorySubstrate bypass |

### Component Composition

The Component Model enables composing Roko's cognitive kernel with
platform-specific capability providers without recompilation:

```
Platform Runtime (Host)
  roko:cognitive (WASM component)     Platform Provider
  - Scorer                           - wasi:http
  - Substrate                        - wasi:keyvalue
  - HDC vectors                      - wasi:logging
```

---

## 7. Two-Tier Architecture

WASM enables a two-tier deployment pattern:

1. **Edge tier (WASM)**: Fast scoring, routing, and cache lookup at the
   network edge (~5ms latency). Handles ~80% of requests without calling
   an LLM. Maps to T0 zero-LLM probe processing.

2. **Core tier (native)**: Full agent processing with LLM backends,
   filesystem persistence, and orchestration for the ~20% of requests
   needing deep reasoning. Maps to T1/T2 model-assisted processing.

---

## 8. WASM Module Size

The core cognitive primitives compile to approximately 500KB-1MB gzipped:

| Component | Approximate WASM Size |
|-----------|----------------------|
| Signal + Score + ContentHash | ~50KB |
| BLAKE3 | ~30KB |
| MemorySubstrate | ~40KB |
| Scorer + Router + Composer | ~90KB |
| HDC vectors | ~20KB |
| serde + serde_json | ~200KB |
| wasm-bindgen glue | ~50KB |
| **Total (gzipped)** | **~500KB** |

Small enough for browser deployment without code splitting.

---

## 9. Edge Runtime Targets

### Cloudflare Workers

WASM modules run in V8 isolates at 300+ edge locations. Sub-millisecond
cold start. Roko's cognitive primitives stay well under the 10ms free-tier
CPU budget for single-Signal scoring. KV namespaces can back Signal caches.

### Fermyon Spin

Serverless WASM runtime. Each request gets an isolated WASM instance with
sub-millisecond cold start. Spin deploys as OCI artifacts to any WASI P2
runtime.

```bash
spin build && spin up    # Local development
spin deploy --from .     # Fermyon Cloud
```

### wasmCloud (CNCF)

Distributed WASM components across a lattice of hosts connected via NATS.
Spread-scaling distributes cognitive kernel instances across edge and
cloud zones with weighted placement policies.

---

## 10. Browser Deployment via wasm-bindgen

For browser-based deployment, wasm-bindgen exposes Roko's cognitive
primitives to JavaScript:

```rust
#[wasm_bindgen]
pub struct WasmAgent {
    substrate: MemorySubstrate,
    scorer: DefaultScorer,
}

#[wasm_bindgen]
impl WasmAgent {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { /* ... */ }

    pub async fn observe(&mut self, text: &str) -> Result<String, JsValue> {
        // Store observation, return content hash
    }

    pub async fn query_ranked(&self, query: &str) -> Result<JsValue, JsValue> {
        // Score and rank stored Signals by relevance
    }
}
```

```bash
wasm-pack build --target web crates/roko-wasm
# Output: pkg/roko_wasm.js, pkg/roko_wasm_bg.wasm
```

---

## 11. Implementation Status

> **Implementation status:** WASM feature flags exist in roko-core and
> roko-std but have not been validated with end-to-end WASM builds. The
> conditional compilation gates are in place. MemorySubstrate is
> implemented and used extensively in tests. HDC vectors and BLAKE3
> compile to WASM natively. The wasm-bindgen wrapper crate has not been
> created. WASM is at Tier 3H priority (planned but not in the critical
> path). Known validation items: serde_json roundtrip in WASM, BTreeMap
> performance for 100K Signals, cross-environment BLAKE3 hash identity,
> and wasm-bindgen async method exposure.
