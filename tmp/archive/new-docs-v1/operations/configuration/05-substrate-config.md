# Substrate Configuration

> The `[substrate]` table selects the storage backend for Engram persistence and
> controls data directory location, garbage collection, and disk usage limits.

**Status**: Shipping (jsonl, memory backends) / Specified (sqlite, lancedb backends)
**Crate**: `roko-fs`, `roko-core`
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md)
**Used by**: [reference/03-substrate/](../../reference/03-substrate/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

For most operators, the default JSONL substrate is the right choice. It is append-only,
human-readable, and requires no external dependencies. Point `data_dir` at a persistent
volume on a server.

```toml
[substrate]
backend  = "jsonl"
data_dir = ".roko/substrate"
```

---

## Backends

### `jsonl` (Shipping — default)

Append-only JSONL files, one per Engram kind. Implemented in `roko-fs` as `FileSubstrate`.

- No external dependencies.
- Human-readable and grep-able.
- Efficient for write-heavy workloads (append-only → O(1) writes).
- Slower for range queries (full file scan) — acceptable for small-to-medium stores.
- Garbage collected via the `gc_interval_hours` schedule.

**Use when**: default single-process deployment, laptop, CI.

```toml
[substrate]
backend  = "jsonl"
data_dir = ".roko/substrate"
```

### `memory` (Shipping)

In-memory store. No persistence — all Engrams are lost when the process exits.

- Zero disk I/O.
- Ideal for unit tests and ephemeral evaluation runs.
- Not for production.

```toml
[substrate]
backend = "memory"
```

### `sqlite` (Specified — planned)

SQLite-backed store with indexed queries. Faster range queries than JSONL at the cost
of a slightly heavier write path.

**Status: Specified.** Not yet implemented. Set `backend = "sqlite"` today and Roko
will emit a validation error with a clear message.

### `lancedb` (Specified — planned)

LanceDB-backed store with native HDC vector search integration. Enables sub-millisecond
HDC fingerprint similarity queries across the full Engram store.

**Status: Specified.** Not yet implemented.

---

## Data Directory Layout

When `backend = "jsonl"`, the `data_dir` contains:

```
.roko/substrate/
  engrams.jsonl          ← all persisted Engrams (append-only)
  episodes.jsonl         ← learning episodes (written by roko-learn)
  playbook-rules.jsonl   ← promoted patterns (written by roko-learn)
  gc.lock                ← GC lock file (present during GC run)
  gc.log                 ← GC run history
```

Each line in `engrams.jsonl` is a JSON object representing one Engram. The file is
never rewritten in place; GC creates a new compacted file and atomically renames it.

---

## Garbage Collection

The substrate GC removes Engrams that meet all of the following conditions:

1. All seven decay axis values are below the configured floor (default: 0.001).
2. No active provenance chain references the Engram.
3. The Engram's `created_at` timestamp is older than the `gc_min_age_hours` floor
   (default: 72h, not yet configurable in the schema).

**GC schedule:**

```toml
[substrate]
gc_interval_hours = 24  # run GC every 24 hours
```

GC runs as a background task while the runtime is active. It does not block agent
execution. If Roko is not running, GC does not run; it will run at the next startup.

**Triggering GC manually:**

```bash
roko substrate gc --dry-run   # see what would be removed
roko substrate gc              # run GC now
```

---

## Disk Usage Cap

```toml
[substrate]
max_size_gb = 10.0
```

When the substrate directory exceeds this size:

1. GC is triggered immediately (emergency GC).
2. Cold-tier Engrams (score < 0.01 on all axes) have their decay accelerated.
3. If still over the cap after GC, a `SubstrateDiskPressure` warning Pulse is emitted.
4. The runtime does **not** hard-fail; it continues running.

Set `max_size_gb = 0.0` to disable the cap entirely.

---

## Server Deployment

On a team server, point `data_dir` at a persistent volume and ensure the directory is
owned by the user running Roko:

```toml
[substrate]
backend             = "jsonl"
data_dir            = "/var/roko/substrate"
gc_interval_hours   = 12
max_size_gb         = 100.0
```

```bash
# One-time setup
sudo mkdir -p /var/roko/substrate
sudo chown roko-user:roko-user /var/roko/substrate
```

---

## Two Full Examples

**Laptop (defaults, small disk budget):**

```toml
[substrate]
backend           = "jsonl"
data_dir          = ".roko/substrate"
gc_interval_hours = 24
max_size_gb       = 5.0
```

**Server (persistent volume, larger budget, more frequent GC):**

```toml
[substrate]
backend           = "jsonl"
data_dir          = "/mnt/roko-data/substrate"
gc_interval_hours = 6
max_size_gb       = 200.0
```

---

## See Also

- [reference/03-substrate/](../../reference/03-substrate/README.md) — Substrate trait internals
- [reference/01-engram/](../../reference/01-engram/README.md) — what Engrams contain
- [operations/performance/03-memory-model.md](../performance/03-memory-model.md) — allocation patterns for the JSONL backend

## Open Questions

- Per-kind disk quotas (e.g. `episodes` capped at 2 GB independent of total) are not yet configurable.
- The GC `min_age_hours` floor is not yet exposed in `roko.toml`.
- Substrate replication (for high-availability deployments) is not yet specified.
