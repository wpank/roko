# Resource Limits

> The configurable and hard-coded limits on memory, disk, and rate that prevent a Roko
> instance from consuming unbounded resources.

**Status**: Shipping (disk cap, rate limits) / Built (memory cap enforcement)
**Crate**: `roko-orchestrator`, `roko-fs`, `roko-runtime`
**Depends on**: [03-memory-model.md](03-memory-model.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Set `substrate.max_size_gb` to cap disk usage. Use `--concurrency` to cap CPU/memory
usage. API rate limits are handled by key rotation — set `ANTHROPIC_API_KEY_2` through
`_10` for high-concurrency deployments.

---

## Disk Limits

### Substrate Disk Cap

```toml
[substrate]
max_size_gb = 10.0
```

When the substrate directory exceeds `max_size_gb`:

1. Emergency GC triggers immediately.
2. Cold-tier Engrams (all score axes < 0.01) have their decay accelerated (decay
   rate multiplied by 10×) for the duration of the GC pass.
3. A `SubstrateDiskPressure` warning Pulse is emitted.
4. If still over the cap after GC, the warning is repeated every 60 minutes.
5. **The runtime does not hard-fail.** New Engrams continue to be written.

**Hard failure behaviour**: not implemented (the cap is a soft cap). If you need a
hard cap (fail rather than overflow), monitor the `substrate_size_mb` metric and
take action in your alerting pipeline.

**Recommended caps by deployment:**

| Deployment | Recommended `max_size_gb` |
|------------|--------------------------|
| Laptop (personal) | 2–5 GB |
| Developer workstation | 10–20 GB |
| Server (team) | 100–500 GB |
| Production cluster | Per-node; set equal to available persistent volume size |

### Log Rotation

Roko's log output (`ROKO_LOG`) goes to stderr by default. For persistent logging,
redirect to a file and use `logrotate`:

```bash
roko plan run plans/ 2>> /var/log/roko/roko.log &

# /etc/logrotate.d/roko
/var/log/roko/roko.log {
    daily
    rotate 7
    compress
    missingok
    notifempty
    postrotate
        kill -HUP $(cat /var/run/roko.pid) 2>/dev/null || true
    endscript
}
```

Log files are not capped by Roko itself — use `logrotate` or a log aggregator.

---

## Memory Limits

### Configuring Concurrency (Primary Knob)

The primary way to limit Roko's memory footprint is to limit agent concurrency:

```bash
roko plan run plans/ --concurrency 4
```

Each agent subprocess uses ~100–200 MB RSS. At concurrency 4: ~400–800 MB agent
memory + ~300 MB orchestrator = ~700 MB–1.1 GB total.

### HDC Index Memory Cap (Planned)

A configurable maximum HDC index size (entries before LRU eviction) is planned but
not yet implemented. Today the HDC index grows unbounded until the process exits or
GC removes entries from the substrate.

**Workaround**: run `roko substrate gc` periodically to remove old Engrams from the
substrate, which reduces the HDC index size on the next startup.

### OS-Level Memory Limits

For production deployments, enforce a hard memory limit at the container or cgroup
level:

```yaml
# Kubernetes example
resources:
  limits:
    memory: "4Gi"
  requests:
    memory: "1Gi"
```

When OOM-killed, Roko writes a state snapshot to `.roko/state/executor.json` (if
the OOM kill is graceful — i.e. SIGTERM before SIGKILL). The next run can resume
from the snapshot:

```bash
roko plan run plans/ --resume .roko/state/executor.json
```

If SIGKILL is sent before the snapshot is written, the run cannot be resumed and
must restart from the last committed plan.

---

## Rate Limits

### LLM API Rate Limits

Anthropic rate limits are applied per API key, per model. When a rate limit is hit
(HTTP 429), Roko:

1. Waits for the `Retry-After` header value (or 60 seconds if absent).
2. Rotates to the next available key (if `ANTHROPIC_API_KEY_2` through `_10` are set).
3. If all keys are rate-limited, queues the request and retries.
4. Logs the rate limit event as `warn` with the key index and wait duration.

**Key rotation setup:**

```bash
export ANTHROPIC_API_KEY=sk-ant-key1
export ANTHROPIC_API_KEY_2=sk-ant-key2
export ANTHROPIC_API_KEY_3=sk-ant-key3
```

### Tool Call Rate Limits

Some MCP servers (e.g. Brave Search, GitHub) have per-minute rate limits. Roko does
not enforce these internally — the MCP server handles rate limiting and returns an
error. Roko treats tool call errors as gate failures and applies normal retry logic.

### Substrate Write Rate

The JSONL substrate does not enforce a write rate limit. If the disk IOPS are
exhausted, writes will block. Monitor `substrate_append_ms` p99; if it exceeds 5 ms
regularly, move the substrate to an SSD.

---

## Summary: Resource Limit Matrix

| Resource | Configurable? | Where | Enforcement |
|----------|--------------|-------|-------------|
| Disk (substrate) | Yes | `substrate.max_size_gb` | Soft: trigger GC, emit warning |
| Memory (agent) | Indirect | `--concurrency` | No hard cap; OOM at OS level |
| Memory (HDC index) | No (planned) | — | None today; grows unbounded |
| CPU | Indirect | `--concurrency` | Bounded by process count |
| LLM API rate | Automatic | Key rotation | Retry with backoff |
| Log disk | No | OS `logrotate` | None in Roko |

---

## See Also

- [03-memory-model.md](03-memory-model.md) — memory breakdown per component
- [operations/configuration/05-substrate-config.md](../configuration/05-substrate-config.md) — disk cap configuration
- [operations/error-handling/06-cascade-failure.md](../error-handling/06-cascade-failure.md) — what happens under extreme resource pressure

## Open Questions

- `substrate.max_size_gb` enforcement is a soft cap; hard cap (refuse new writes when full) is under discussion.
- A configurable maximum HDC index size with disk-based overflow is planned.
- Roko does not yet have a built-in metrics endpoint for resource usage monitoring — operators must use OS-level tools today.
