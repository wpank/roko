# Crate–Layer Map

> Which crate sits at which layer — the authoritative assignment table.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## The Map

| Crate | Layer | Description | Status |
|---|---|---|---|
| `roko-runtime` | L0 | Async executor, allocator, platform I/O | Shipping |
| `roko-core` | L1 | Traits + core data types | Shipping |
| `roko-std` | L2 | Default operator implementations | Shipping |
| `roko-agent` | L2 | `loop_tick()`, `Agent`, `AdaptiveClock` | Shipping |
| `roko-compose` | L2 | Multi-agent composition utilities | Shipping |
| `roko-neuro` | L2 | Neuro cross-cut implementation | Shipping |
| `roko-daimon` | L2 | Daimon cross-cut implementation | Built |
| `roko-dreams` | L2 | Dreams cross-cut implementation | Built |
| `roko-substrate-sled` | L2 | Sled-backed Substrate implementation | Shipping |
| `roko-substrate-postgres` | L2 | Postgres-backed Substrate | Built |
| `roko-substrate-memory` | L2 | In-memory Substrate (tests / WASM) | Shipping |
| `roko-hdc` | L2 | HDC fingerprint computation (SIMD) | Shipping |
| `roko-orchestrator` | L3 | Agent lifecycle, `TickContextBuilder` | Shipping |
| `roko-gate` | L3 | Gate pipeline management | Shipping |
| `roko-cli` | L4 | `roko` binary, CLI commands | Shipping |
| `roko-serve` | L4 | REST + WebSocket API server | Shipping |

---

## Cross-Cut Crates

Cross-cut crates (`roko-neuro`, `roko-daimon`, `roko-dreams`) implement L1 traits
and live at L2. They are not their own layer — they are L2 implementations that
participate in the loop as injected trait objects. See
[Cross-Cuts](../09-cross-cuts/README.md).

---

## External (Vendored) Crates

These third-party crates are in the workspace but not subject to layer rules:

| Crate | Source | Purpose |
|---|---|---|
| `sled` (re-exported) | vendored | Embedded key-value store |
| `tokenizers` (re-exported) | vendored | Token counting |

---

## See also

- [Overview](00-overview.md) — the five-layer structure
- [Dependency Rules](06-dependency-rules.md) — how layer assignment is enforced
- [reference/11-crate-map.md](../11-crate-map.md) — the full crate inventory (Cluster D)
