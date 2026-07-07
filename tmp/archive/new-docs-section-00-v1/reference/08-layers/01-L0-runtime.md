# L0 — Runtime Layer

> Platform abstraction, async executor, and memory allocator.

**Status**: Shipping
**Crate**: `roko-runtime`
**Depends on**: Nothing within Roko (depends only on OS / embedded platform / WASM)
**Used by**: All other layers
**Last reviewed**: 2026-04-19

---

## TL;DR

`roko-runtime` is the foundation of the Roko stack. It abstracts the execution
platform (native Linux/macOS, WASM, embedded no-std) and provides the async executor
and allocator that all higher layers assume. No other Roko crate knows anything about
the platform; they only know about `roko-runtime`'s public API.

---

## Responsibilities

| Responsibility | Description |
|---|---|
| Async executor | Tokio-based multi-threaded executor for native; single-threaded for WASM |
| Allocator | Global allocator; wrappable with instrumented allocator for profiling |
| Platform I/O | File system, network sockets, timers — abstracted behind traits |
| Signal handling | SIGTERM / SIGINT handling; graceful shutdown protocol |
| Feature flags | Compile-time feature flags that gate platform-specific code |

---

## Platform Targets

| Target | Status | Notes |
|---|---|---|
| Native (Linux x86-64) | Shipping | Primary target; full feature set |
| Native (macOS arm64) | Shipping | Development target |
| WASM (wasm32-wasi) | Built | Smaller binary; no native threads; I/O via WASI |
| Embedded (no-std) | Specified | Requires custom allocator; deferred |

---

## Public API Surface

`roko-runtime` exports:

```rust
// source: crates/roko-runtime/src/lib.rs
pub use executor::RokoRuntime;      // the main runtime struct
pub use io::PlatformIo;             // abstract I/O trait
pub use alloc::RokoAllocator;       // the global allocator wrapper
pub use signal::ShutdownSignal;     // graceful shutdown coordination
```

Higher layers do not use `tokio::spawn` directly — they use
`RokoRuntime::spawn_task()` to ensure tasks are tracked and gracefully cancelled
during shutdown.

---

## Shutdown Protocol

When `ShutdownSignal::trigger()` is called (e.g., on SIGTERM), the runtime:
1. Stops accepting new tasks.
2. Sends cancellation to all tracked tasks.
3. Waits up to `shutdown_timeout` (default 30 s) for tasks to complete.
4. Force-kills remaining tasks.
5. Flushes the substrate write buffer.
6. Exits.

This guarantees that in-flight PERSIST writes complete before the process exits.

---

## See also

- [L1 Framework](02-L1-framework.md) — built directly on L0
- [Dependency Rules](06-dependency-rules.md) — L0 may not import from L1+
- [Operations / deployment](../../operations/deployment/README.md) — platform targets
