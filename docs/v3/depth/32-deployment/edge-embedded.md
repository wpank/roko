# 32-deployment/09 -- Edge and Embedded Deployment

> Minimal binary for resource-constrained devices, ~500KB size budget,
> edge-core two-tier architecture, offline sync, and use case patterns.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-core/`, `crates/roko-primitives/`

---

## 1. Edge Deployment Philosophy

Edge deployment targets environments with severe constraints: limited
memory (<64MB), limited storage (<10MB), limited or intermittent network,
and limited CPU. Examples: IoT gateways, network edge nodes, embedded
Linux devices, Raspberry Pi, and industrial controllers.

The design principle: the core cognitive primitives (Signal, Score,
Scorer, Router, Composer) are pure computation with no I/O dependencies.
They run anywhere that has a Rust allocator. I/O-dependent components
(LLM backends, filesystem persistence, process supervision) are optional
and excluded from edge builds.

This maps to the kernel trait system -- each trait is independently
implementable. An edge deployment implements Scorer and Router locally
while delegating Substrate (persistence) and Gate (verification) to a
remote core node.

---

## 2. Minimal Feature Set

Edge builds disable all optional features:

```bash
cargo build --release \
    -p roko-core \
    --no-default-features \
    --features "serde,hdc,decay" \
    --target aarch64-unknown-linux-musl
```

### What Is Included

| Component | Size | Purpose |
|-----------|------|---------|
| Signal + Score | ~15KB | Core type, 7-axis appraisal |
| ContentHash (BLAKE3) | ~30KB | Content addressing |
| Kernel trait defs | ~5KB | 12 trait interfaces |
| DefaultScorer | ~10KB | Score computation |
| DefaultRouter | ~10KB | Selection logic |
| DefaultComposer | ~15KB | Context assembly under budget |
| HdcVector | ~20KB | Hamming similarity, XOR bundling |
| Decay | ~5KB | HalfLife, TTL, Ebbinghaus |
| serde + serde_json | ~200KB | Serialization for IPC |
| Allocator + runtime | ~50KB | Minimal Rust runtime |
| **Total** | **~360KB** | Core cognitive kernel |

### What Is Excluded

| Component | Why Excluded | Alternative |
|-----------|-------------|-------------|
| Tokio async runtime | ~2MB, unnecessary for sync edge | `smol` or bare `poll` |
| LLM providers (reqwest + TLS) | ~5MB, needs network | Proxy through core node |
| ratatui TUI | ~500KB, no display | Not applicable |
| Tree-sitter (code parsing) | ~3MB, C FFI | Pre-computed indexes |
| alloy (Ethereum) | ~10MB, chain-specific | Not needed on edge |
| FileSubstrate | Needs writable fs | MemorySubstrate |
| ProcessSupervisor | Needs process spawning | Not applicable |

---

## 3. Binary Size Budget

Target: ~500KB stripped binary (or WASM module).

```
Core cognitive kernel:     ~360KB (72%)
IPC / networking stub:      ~50KB (10%)
Application logic:          ~90KB (18% -- user's edge agent)
--------------------------------------------
Total:                     ~500KB
```

### Size Optimization Profile

```toml
[profile.edge]
inherits = "release"
opt-level = "z"        # Optimize for size (not speed)
lto = "fat"            # Full LTO for maximum dead code elimination
codegen-units = 1      # Single codegen unit
strip = true           # Strip all symbols
panic = "abort"        # No unwinding tables
```

Build with the edge profile:

```bash
cargo build --profile edge -p roko-core \
    --no-default-features --features "serde,hdc,decay"
```

Additional size reduction strategies:

- Replace `serde_json` with `miniserde` or `nanoserde` (~50KB to ~10KB)
- `blake3` with `no_std` feature (removes threading, saves ~15KB)
- Fixed-size arrays for known tag keys instead of BTreeMap
- `#[cfg(feature = "edge")]` to gate out convenience methods

---

## 4. Use Cases

### Edge Scoring and Pre-Filtering

An edge node receives events (sensor data, log entries, API responses)
and uses the Scorer to determine which are novel enough to forward:

```rust
fn should_forward(event: &str) -> bool {
    let signal = Signal::builder()
        .kind(Kind::Observation)
        .body(Body::Text(event.to_string()))
        .build();
    let score = DefaultScorer::default().score(&signal);
    score.novelty > 0.5 || score.utility > 0.7
}
```

This reduces edge-to-core bandwidth by ~80%, matching the T0 zero-LLM
probe pattern from the dual-process cognition model.

### Local Knowledge Cache

MemorySubstrate holds recently relevant Signals as a local cache, avoiding
round-trips for repeated queries:

```rust
let mut cache = MemorySubstrate::new();
if score.utility > 0.8 {
    cache.put(signal).await?;
}
// Check local cache before forwarding to core
if let Some(cached) = cache.query(Query::similar(&query)).await? {
    return Ok(cached);
}
```

### HDC Similarity Search

HDC vector comparison is ~50ns per 10,000-bit vector on ARM Cortex-A72,
fast enough for real-time classification at the edge:

```rust
let known_patterns: Vec<(String, HdcVector)> = load_from_flash();
let query = HdcVector::encode(incoming_data);

let best = known_patterns.iter()
    .map(|(label, vec)| (label, query.hamming_similarity(vec)))
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
```

### Offline Agent with Periodic Sync

An edge agent operates autonomously when disconnected, accumulating
Signals in MemorySubstrate. When connectivity is restored:

```rust
loop {
    let observation = read_sensor();
    let signal = process(observation);
    local_substrate.put(signal).await?;

    if is_connected() {
        let pending = local_substrate.export().await?;
        send_to_core(pending).await?;
        let updates = receive_from_core().await?;
        for signal in updates {
            local_substrate.put(signal).await?;
        }
    }
    sleep(tick_interval);
}
```

---

## 5. Edge-Core Communication

Edge nodes communicate with core agents using lightweight JSON-RPC over
any available transport (HTTP, MQTT, serial, BLE):

```json
{
    "jsonrpc": "2.0",
    "method": "substrate.put",
    "params": {
        "signal": {
            "id": "blake3:a1b2c3...",
            "kind": "Observation",
            "body": {"text": "sensor reading: 42.5"},
            "score": {"novelty": 0.8, "utility": 0.6}
        }
    },
    "id": 1
}
```

The core responds with knowledge updates:

```json
{
    "jsonrpc": "2.0",
    "method": "substrate.sync",
    "params": {
        "signals": [
            {"id": "blake3:d4e5f6...", "kind": "Heuristic", "body": {"text": "..."}}
        ],
        "since_ms": 1712345678000
    },
    "id": 2
}
```

---

## 6. Relationship to WASM

Edge and WASM deployment share the same core -- the cognitive kernel
compiled without I/O dependencies. The difference is the target:

| Aspect | Edge (native) | WASM |
|--------|--------------|------|
| Target | `aarch64-unknown-linux-musl` | `wasm32-wasi` |
| Format | ELF static binary | `.wasm` module |
| Runtime | Linux kernel | WASM runtime (wasmtime, browser) |
| Performance | Full native speed, SIMD | ~2-5x slower, limited SIMD |
| Size | ~500KB | ~500KB (gzipped) |
| Filesystem | Possible (musl libc) | Not available |
| Networking | Possible (TCP/UDP) | Via host imports only |

For ARM Linux devices, native edge is preferred (better performance, full
system access). For sandboxed environments, WASM. The shared feature
flag system (`--no-default-features --features "serde,hdc,decay"`) ensures
both targets compile the same kernel.

---

## 7. Hardware Considerations

### ARM Cortex-A (Raspberry Pi, Jetson)

Full native performance including NEON SIMD for HDC vectors. The musl
static binary has no runtime dependencies -- copy and run.

### RISC-V

Rust supports `riscv64gc-unknown-linux-gnu` as a Tier 2 target. The
cognitive kernel compiles without architecture-specific code. No SIMD
acceleration is available (scalar fallback).

### Memory-Constrained Environments

For devices with less than 16MB RAM, consider:
- Pre-compute and freeze the Scorer's parameters at build time
- Use a fixed-capacity MemorySubstrate (ring buffer, not growing BTreeMap)
- Disable serde_json in favor of a fixed binary protocol

---

## 8. Implementation Status

> **Implementation status:** Edge deployment is at Tier 3H priority (P3).
> The core cognitive primitives compile with `--no-default-features`
> (tested in CI). MemorySubstrate is used extensively in unit tests.
> The ~500KB binary size target has not been validated with a dedicated
> edge build profile. Implementation needs: create `[profile.edge]` in
> workspace Cargo.toml, validate standalone roko-core compilation without
> Tokio or heavy deps, measure actual binary size, create example edge
> agent, and define the edge-core sync protocol.
