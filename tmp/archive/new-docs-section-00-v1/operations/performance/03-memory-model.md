# Memory Model

> How Roko allocates memory, which data structures use arenas and pools, and what
> determines the RSS of a running Roko instance.

**Status**: Shipping
**Crate**: `roko-core`, `roko-orchestrator`, `roko-agent`
**Depends on**: [00-overview.md](00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko's steady-state RSS is ~50–150 MB per process, excluding agent subprocesses and
the workspace symbol index. Each agent subprocess (spawned for a task) is ~100–200 MB.
The hot path uses arena allocators and avoids per-Engram heap allocation.

---

## Process Memory Breakdown

A running `roko plan run` instance with 4 concurrent agents:

| Component | Typical RSS | Notes |
|-----------|------------|-------|
| `roko-orchestrator` binary | ~30 MB | Static code + Tokio runtime |
| Workspace symbol index | 10–200 MB | Depends on workspace size; mmap'd |
| Substrate write buffer | < 1 MB | 64 KB per JSONL file; pooled |
| Episode buffer (pre-flush) | < 5 MB | Batch of up to 100 episode records |
| HDC index (in-memory, 100K vectors) | ~130 MB | 10,240 bits × 100K = ~128 MB |
| EventBus channel buffers | < 5 MB | Bounded channel capacity |
| Per-agent process (×4) | 100–200 MB each | Each `roko-agent` subprocess |
| **Total (4 agents, 100K HDC)** | **~700 MB** | **Estimate for typical project** |

---

## Arena Allocators

The hot path uses a **tick arena allocator** in `roko-core` for short-lived allocations
within a single agent turn:

```
<!-- source: crates/roko-core/src/arena.rs -->
```

The tick arena allocates from a pre-reserved region (default: 1 MB) and resets entirely
at the end of each agent turn. This means:

- **No per-Engram `malloc` calls** in steady state (Engram fields are allocated in the arena).
- **No per-Engram `free` calls** — the entire arena is released at turn boundary.
- **Cache-friendly** — all data for a single turn is contiguous.

Objects that must outlive a turn (persisted Engrams, playbook rules) are moved from the
arena into the heap-allocated Substrate write buffer before the arena is reset.

---

## HDC Vector Memory

Each HDC fingerprint is a 10,240-bit (1,280-byte) binary vector. Memory usage at various
index sizes:

| Index entries | Raw memory | With metadata (4× overhead) |
|--------------|-----------|---------------------------|
| 10,000 | 12.8 MB | ~52 MB |
| 100,000 | 128 MB | ~512 MB |
| 1,000,000 | 1.28 GB | ~5.1 GB |

For deployments with > 100K indexed entries, consider sharding the HDC index across
multiple processes or using the planned LanceDB backend (which pages vectors to disk).

The HDC index is loaded lazily — it is not read into memory until the first similarity
search query. After the first query, it is kept warm in memory for the lifetime of the
process.

---

## Symbol Index Memory

The workspace symbol index (`roko-index`) uses memory-mapped files (`memmap2`) for
the cached snapshot and an in-memory Salsa incremental computation graph for live
symbol resolution.

Memory usage scales with workspace size:

| Workspace size | Symbol index RSS | Notes |
|---------------|----------------|-------|
| Small (< 10K LOC, < 20 crates) | 5–15 MB | |
| Medium (50K–200K LOC, 20–50 crates) | 50–200 MB | Typical self-hosting scenario |
| Large (> 500K LOC, > 100 crates) | 200 MB–1 GB | Consider index sharding |

The symbol index is shared across all concurrent agents (via an `Arc<SymbolIndex>`). It
is not duplicated per agent.

---

## Agent Subprocess Memory

Each agent task spawns a subprocess (`roko-agent`). The subprocess:

- Loads its own copy of the LLM backend library.
- Holds its context window in memory (up to `agent.max_turns × response_tokens`).
- Runs all MCP server subprocesses as children.

Typical RSS per agent subprocess: 100–200 MB at peak (during context assembly). After
the task completes and the subprocess exits, this memory is returned to the OS.

With 8 concurrent agents: `8 × 150 MB ≈ 1.2 GB` of agent subprocess memory. Add the
orchestrator process (~300 MB with HDC index) for a total of ~1.5 GB RSS. This is
within the resource limit for most server deployments.

---

## Reducing Memory Usage

**For memory-constrained environments:**

1. Limit agent concurrency: `roko plan run --concurrency 2 plans/`
2. Reduce HDC index size: set a lower `substrate.max_size_gb` to trigger more aggressive GC.
3. Disable in-memory HDC index: not yet configurable — planned.
4. Use the `memory` Substrate backend for ephemeral runs (no index built).

**For maximum HDC search speed**, allocate more RAM and keep the full index in memory.

---

## See Also

- [04-numerical-stability.md](04-numerical-stability.md) — f32 arithmetic for Score and decay
- [05-hot-paths.md](05-hot-paths.md) — which operations must not allocate
- [10-resource-limits.md](10-resource-limits.md) — configurable memory caps

## Open Questions

- A configurable maximum HDC index size (with LRU eviction to disk) is planned but not yet implemented.
- The arena allocator is not yet exposed in the public API; third-party crates cannot use it.
