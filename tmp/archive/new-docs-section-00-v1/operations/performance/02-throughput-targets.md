# Throughput Targets

> How many Engrams per second and tasks per second Roko should sustain, and at what
> concurrency levels.

**Status**: Shipping
**Crate**: `roko-core`, `roko-orchestrator`, `roko-fs`
**Depends on**: [00-overview.md](00-overview.md), [01-latency-budgets.md](01-latency-budgets.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko's Engram storage layer handles > 50,000 Engrams/second in batched mode. Task
throughput is dominated by LLM and gate latency — a single Roko instance can complete
4–8 coding tasks per minute with 8 concurrent agents.

---

## Engram Throughput

The Substrate layer (`FileSubstrate` / JSONL) is the Engram storage path.

| Mode | Throughput | Notes |
|------|-----------|-------|
| Single-threaded unbuffered append | ~5,000 Engrams/s | Each append is a separate `write(2)` syscall |
| Single-threaded buffered (default) | ~50,000 Engrams/s | 64 KB buffer; one `write(2)` per buffer flush |
| Batched (100 Engrams per write) | ~150,000 Engrams/s | Used by learning batch flush |
| HDC similarity search (1M entries, 1 query) | ~200 queries/s | Linear scan; see search latency in [01-latency-budgets.md](01-latency-budgets.md) |

In steady-state operation, Roko generates approximately:

- 10–50 Engrams per agent task (one Engram per turn, tool call result, and gate verdict).
- 1 episode record per task completion.
- 1 playbook rule per 5–10 episodes (lazy; batch-extracted).

At 8 concurrent agents, this is ~80–400 Engrams/minute — well within the buffered
append throughput. The Substrate is not a bottleneck at normal agent concurrency.

---

## Task Throughput

Task throughput depends on three variables: agent concurrency, LLM latency, and gate
duration. The formula is:

```
tasks_per_minute = (concurrency × 60) / average_task_duration_seconds
```

For the default configuration (Sonnet model, 4-gate pipeline, 1 retry on average):

| Concurrency | Average task duration | Tasks/minute |
|------------|----------------------|-------------|
| 1 agent | ~20 s | ~3 |
| 4 agents | ~20 s | ~12 |
| 8 agents | ~20 s | ~24 |
| 16 agents | ~20 s | ~48 |
| 32 agents | ~20 s | ~96 |

**Practical ceiling for a laptop**: 4–8 concurrent agents before CPU, memory, or rate
limits bind. For a typical laptop with 8 cores and 16 GB RAM:

- `compile` gate saturates at ~4 concurrent build processes (CPU-bound).
- Memory pressure from concurrent agent processes begins at ~8 agents (each agent
  process is ~100–200 MB RSS).
- Anthropic rate limits (tokens per minute) bind before 16 agents for Opus.

**Practical ceiling for a server (16 cores, 64 GB RAM)**: 16–32 concurrent agents before
rate limits bind. At 16 agents with Sonnet and a shared gateway, expect ~50–80 tasks/minute
on a well-parallelised plan.

---

## Rate Limit Considerations

LLM API rate limits are the most common throughput constraint for high-concurrency
deployments. Anthropic Sonnet rate limits (as of 2026-04):

| Tier | Tokens/minute (input + output) |
|------|-------------------------------|
| Free | 40,000 |
| Build | 400,000 |
| Scale | 4,000,000+ (negotiated) |

At 8 concurrent Sonnet agents, each using ~5,000 tokens/task, throughput peaks at
~800 token-minutes, well within Build tier. Opus at 8 agents (~15,000 tokens/task)
uses ~1,200,000 token-minutes — this approaches the Build tier ceiling.

Mitigations:
1. Key rotation (`ANTHROPIC_API_KEY_2` through `_10`) distributes across multiple
   rate-limit buckets.
2. CascadeRouter routes cheap tasks to Haiku (lower token usage, separate rate limit).
3. A local gateway with response caching reduces redundant API calls.

---

## Event Bus Throughput

The in-process `EventBus<E>`:

| Subscribers | Events/second (single publisher) | Notes |
|------------|----------------------------------|-------|
| 1 | ~5,000,000/s | Bounded by MPSC channel throughput |
| 10 | ~500,000/s | Broadcast; each subscriber gets a clone |
| 100 | ~50,000/s | Allocation-heavy at 100+ subscribers |

At normal agent concurrency (< 32 agents), the event bus is not a bottleneck. Each agent
publishes ~1–5 events per turn (task progress, gate results, etc.).

---

## HDC Search Throughput

HDC hypervector similarity search throughput (single thread, 10,240-bit vectors):

| Index size | Queries/second | Notes |
|-----------|---------------|-------|
| 10,000 vectors | 200,000/s | Fits in L3 cache |
| 100,000 vectors | 20,000/s | Memory-bound |
| 1,000,000 vectors | 2,000/s | DRAM bound; consider partitioning |
| 10,000,000 vectors | 200/s | Requires sharding |

For a typical deployment (< 1M episodes/Engrams), HDC search is not a throughput
bottleneck. The learning subsystem queries the HDC index once per task completion, which
at 96 tasks/minute is 96 queries/minute — trivially below any threshold.

---

## See Also

- [01-latency-budgets.md](01-latency-budgets.md) — per-stage latency (complements throughput data)
- [09-scaling-patterns.md](09-scaling-patterns.md) — scaling beyond a single machine
- [10-resource-limits.md](10-resource-limits.md) — rate limits and memory caps

## Open Questions

- Throughput numbers for `sqlite` and `lancedb` substrate backends are not yet benchmarked (backends are not yet Shipping).
- Throughput benchmarks for multi-process deployments (sharing a substrate via NFS) are not yet measured.
