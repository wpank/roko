# Scaling Patterns

> Horizontal vs vertical scaling for Roko, Substrate sharding for large deployments,
> and Bus scaling when multi-host event distribution is needed.

**Status**: Specified (multi-host patterns) / Shipping (vertical/concurrency scaling)
**Crate**: cross-crate
**Depends on**: [02-throughput-targets.md](02-throughput-targets.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

For most teams (< 50 concurrent agents), vertical scaling (more cores, more RAM on a
single host) is the right approach. Horizontal scaling (multiple Roko hosts sharing a
substrate) is Specified but not yet Shipping.

---

## Vertical Scaling

**What to scale vertically:**

| Resource | When to add more | How |
|----------|-----------------|-----|
| CPU cores | `compile` and `test` gates are CPU-saturated | More cores = more parallel compile jobs |
| RAM | RSS > 80% of available memory | Add RAM or reduce `--concurrency` |
| Disk (IOPS) | `substrate_append_ms` p99 > 5 ms | Move `substrate.data_dir` to an SSD |
| Network bandwidth | `llm_full_response_ms` p99 > 30 s | Higher-bandwidth network to LLM API |

**Tuning concurrency on a given machine:**

```bash
# Measure the optimal concurrency for your machine
for c in 2 4 8 16; do
  echo "=== concurrency $c ==="
  time roko plan run plans/ --concurrency $c --dry-run-gates 2>&1 | tail -5
done
```

Start at `concurrency = n_cores / 2` and increase until you see diminishing returns on
gate throughput or memory pressure.

**Recommended vertical config for a 16-core, 64 GB RAM server:**

```bash
roko plan run plans/ --concurrency 12
```

12 leaves 4 cores for the OS, `sccache`, and other processes.

---

## Horizontal Scaling (Specified)

**Current state**: Horizontal scaling (multiple `roko` processes sharing a substrate)
is not yet Shipping. The Substrate is a single-writer design; multiple writers would
require coordination.

**The planned model:**

```
[Roko Instance A]──┐
[Roko Instance B]──┤──[Shared Substrate (NFS or S3)]
[Roko Instance C]──┘
                   └──[Shared Episode Store]
                   └──[Shared Playbook]
```

Each Roko instance handles its own agent tasks independently. The shared substrate
allows all instances to read each other's Engrams (for context assembly) and the
shared episode store allows all instances to contribute to learning.

**Coordination requirements (planned):**
- A distributed lock on the GC process (only one instance GCs at a time).
- Atomic Engram ID assignment (avoid ID collisions from concurrent writers).
- A merge protocol for concurrent playbook rule extraction.

None of these are implemented yet.

---

## Substrate Sharding (Specified)

For very large Engram stores (> 10M entries), a single JSONL file becomes slow to scan.
The planned sharding model partitions the substrate by Engram kind and time range:

```
substrate/
  engrams/
    2026-01.jsonl
    2026-02.jsonl
    2026-03.jsonl
    ...
  episodes/
    2026.jsonl
```

Queries that specify a time range can skip irrelevant shards. The GC can operate on
individual shards without locking the entire substrate.

**Status: Specified.** No code yet.

---

## Bus Scaling (Specified)

The in-process `EventBus<E>` does not scale across process boundaries. For multi-process
or multi-host deployments, the planned approach is to replace `EventBus<E>` with a
NATS-backed `Bus` (see [operations/configuration/06-bus-config.md](../configuration/06-bus-config.md)).

**NATS topology for a 3-host cluster:**

```
[Roko Host A] ──publish──► [NATS Server] ◄──subscribe── [Roko Host B]
[Roko Host C] ──subscribe──► [NATS Server]
```

All three hosts publish their events to NATS. All three subscribe to topics they care
about (e.g. learning subsystem subscribes to `roko.task.completed` to update its
episode store).

**Status: Specified.** The Bus trait design is described in
[reference/04-bus/](../../reference/04-bus/README.md). NATS backend not yet implemented.

---

## Cloud Provider Deployments

For production SaaS-style deployments, the recommended architecture is:

```
                    ┌─────────────────────┐
                    │  Load Balancer (L4)  │
                    └──────────┬──────────┘
                               │
             ┌─────────────────┼─────────────────┐
             │                 │                 │
      [Roko Pod A]      [Roko Pod B]      [Roko Pod C]
         ↓                 ↓                 ↓
      [Local sccache]   [Local sccache]  [Local sccache]
             └─────────────────┼─────────────────┘
                               │
                 ┌─────────────┴──────────────┐
                 │       Shared Storage       │
                 │  (NFS / EFS / GCS Fuse)    │
                 │  substrate, episodes,      │
                 │  playbook                  │
                 └────────────────────────────┘
```

Each pod runs one `roko` process. Shared storage is mounted at the configured paths.
Each pod uses a local `sccache` instance for compilation caching (do not share `sccache`
across pods — cache coherence issues).

**Status: Specified.** This architecture has not been validated at scale.

---

## See Also

- [02-throughput-targets.md](02-throughput-targets.md) — when single-node throughput is insufficient
- [10-resource-limits.md](10-resource-limits.md) — caps to set per instance
- [operations/configuration/06-bus-config.md](../configuration/06-bus-config.md) — `[bus]` table for multi-host

## Open Questions

- Substrate write coordination (distributed locking) is the main open problem for horizontal scaling.
- Whether to use CRDT-based Engram stores (which are natively multi-writer) is under research consideration.
