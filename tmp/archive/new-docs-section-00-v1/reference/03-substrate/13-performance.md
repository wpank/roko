# Performance

> Hot paths, allocation profiles, measured latencies, and target SLAs for `Substrate`.

**Status**: Shipping
**Crate**: `roko-core`, `roko-fs`
**Depends on**: [Backends Overview](./07-backends-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`query_similar` is the hot path — called on every loop tick in the RECALL step. With
`MemorySubstrate` and 10K records at D=10,000 bits, it takes ~1 ms. With `FileSubstrate`
at the same scale, after index load, it is similar. Write (`put`) is fast in memory, ~1–5 ms
on disk.

---

## Target Latency SLAs

<!-- ADDED -->

| Operation | Backend | Scale | Target P99 |
|---|---|---|---|
| `put` | MemorySubstrate | — | < 10 µs |
| `put` | FileSubstrate | — | < 5 ms (with fsync) |
| `put` | FileSubstrate | — | < 0.5 ms (without fsync) |
| `get` | MemorySubstrate | 100K records | < 1 µs |
| `get` | FileSubstrate | 100K records | < 0.5 ms |
| `query` | MemorySubstrate | 100K records | < 5 ms |
| `query_similar` | MemorySubstrate | 10K records, D=10K | < 1 ms |
| `query_similar` | MemorySubstrate | 100K records, D=10K | < 10 ms |
| `prune` | MemorySubstrate | 10K records | < 2 ms |
| `FileSubstrate::open` | — | 100K records | < 500 ms |

---

## Hot Path: `query_similar` on Every Tick

The cognitive loop calls `query_similar` once per RECALL step. At Gamma speed (subsecond
ticks), this is the dominant substrate cost. Key factors:

1. **Record count** — linear in n. At 10K records, 1 ms. At 100K, 10 ms (exceeds Gamma
   budget). Beyond 50K records with sub-ms targets, an LSH index is needed.
2. **Fingerprint dimensionality** — linear in D/64 (we work on 64-bit words). D=10,000 means
   157 64-bit words per record. Reduction to D=4,096 halves the cost.
3. **SIMD availability** — `popcount` and XOR on 64-bit words is auto-vectorised by LLVM.
   On x86 with AVX2, the loop processes 4 words at once.

---

## Allocation Budget

`query_similar` allocates:
- One `Vec<(usize, ContentHash)>` of length n (distances + hashes) — heap allocated.
- The result `Vec<Engram>` — heap allocated, length k.

Total allocation per call: O(n · sizeof(usize + ContentHash)) + O(k · sizeof(Engram)).
At n=10K, k=16, Engram ≈ 512 bytes:
- Distance vec: ~10K × 40 bytes = ~400 KB.
- Result vec: 16 × 512 bytes = ~8 KB.

The distance vec is re-allocated on every call. A future optimisation is to reuse it via a
thread-local buffer or a pre-allocated arena passed to `query_similar`.

---

## `put` Allocation

`put` allocates:
- The fingerprint computation (HDC encode): ~10K bits × token count ÷ 8 bytes.
- The serialized JSON string (FileSubstrate only): O(size of Engram).
- A clone of the `Engram` for the in-memory index.

For typical `Engram`s (100–500 bytes serialised), total allocation per `put` is under 10 KB.

---

## FileSubstrate Startup Cost

`FileSubstrate::open` reads and deserialises every record in the file:
- 100K records × 512 bytes average = ~50 MB read.
- Deserialisation with `serde_json`: ~5 ns per byte, so ~250 ms for 50 MB.
- HDC index build: O(n · D/64) = ~100 ms for 100K records at D=10K.
- Total: ~350–500 ms for 100K records.

For agents that restart frequently, this startup cost matters. Mitigation: use the in-memory
backend or keep the store small via aggressive pruning.

---

## Reducing `query_similar` Cost

In priority order:

1. **Prune aggressively** — fewer records = faster scan.
2. **Lower D** — compile with `HDC_DIMENSIONS=4096` for a 2.5× speedup at some recall cost.
3. **Pre-filter with `SubstrateQuery`** — scan only records matching a `kind` filter first,
   then run HDC over the smaller candidate set. (Not yet implemented; open issue.)
4. **LSH index** — approximate nearest-neighbour search, O(log n). Planned for > 50K records.

---

## See Also

- [Query Similar](./03-query-similar.md)
- [Concurrency Model](./05-concurrency-model.md) — lock contention impact
- [Pruning](./06-pruning.md) — the primary tool for keeping n small

## Open Questions

- What is the right D for production deployments? Should D be runtime-configurable?
- Should there be a benchmark suite in CI to catch performance regressions?
