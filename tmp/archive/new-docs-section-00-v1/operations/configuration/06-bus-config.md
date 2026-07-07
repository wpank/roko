# Bus Configuration

> The `[bus]` table configures the event transport fabric. Today the shipped transport
> is the in-process `EventBus<E>` from `roko-runtime`, which needs no configuration.
> This table is reserved for when external bus backends (NATS, Redis) are promoted
> to Shipping.

**Status**: Specified (target-state; `[bus]` table not required today)
**Crate**: `roko-runtime` (EventBus<E> — no config needed today)
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md)
**Used by**: [reference/04-bus/](../../reference/04-bus/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

If you are running a single-process Roko instance (the common case), you do not need a
`[bus]` table. The `EventBus<E>` in `roko-runtime` is wired automatically and requires
no configuration.

The `[bus]` table only becomes relevant when you are running multiple Roko processes that
need to exchange events — for example, a Roko orchestrator and a Roko learning daemon
on separate machines.

---

## Today vs. Planned

**Today (Shipping):** Event transport is handled by `EventBus<E>` in `roko-runtime`. This
is an in-process broadcast channel. All agents, gates, and learning subsystems subscribe
to it within one process. No external message broker is needed.

**Target state (Specified):** The `Bus` trait abstraction will allow plugging in external
transport backends. When that lands, the `[bus]` table will configure which backend to
use and how to connect to it.

> Shipping today: `EventBus<E>` (in-process, no config needed)
> Target state: `Bus` trait with NATS / Redis backends

---

## `[bus]` Keys (Target-State Reference)

These keys are defined in the schema but generate a validation warning if used today,
because the `Bus` abstraction is not yet wired:

### `bus.backend`

```
Type:    String
Default: "internal"
Range:   "internal" | "nats" | "redis" (planned)
Env var: ROKO_BUS_BACKEND
Notes:   "internal" uses the in-process EventBus<E>. No external service required.
         "nats" and "redis" are target-state and will connect to an external broker.
```

### `bus.url`

```
Type:    String (connection URL)
Default: "" (not used for "internal" backend)
Range:   A valid URL for the selected backend.
Env var: ROKO_BUS_URL
Example: url = "nats://localhost:4222"
Notes:   Connection URL for external bus backends. Not used when bus.backend = "internal".
```

### `bus.subject_prefix`

```
Type:    String
Default: "roko"
Range:   Any valid NATS subject prefix (alphanumeric, dots, hyphens).
Env var: ROKO_BUS_SUBJECT_PREFIX
Example: subject_prefix = "roko.prod"
Notes:   Namespace prefix for all NATS subjects. Useful when multiple Roko environments
         share the same NATS server (e.g. "roko.dev" and "roko.prod").
```

---

## When You Will Need This

The `[bus]` table becomes necessary in two scenarios:

1. **Multi-process learning**: You want to run `roko-learn` as a separate daemon process
   that reads episodes from a shared substrate and emits playbook updates, without both
   processes being in the same binary.

2. **Multi-host orchestration**: You are running Roko orchestrators on several machines
   (one per large project) and want them to share Pulses (e.g. health events, learning
   signals) via a central broker.

Neither scenario is Shipping today. If you are running a single-node deployment, ignore
this table.

---

## Example (Target-State, not yet functional)

```toml
[bus]
backend        = "nats"
url            = "nats://nats.internal:4222"
subject_prefix = "roko.prod"
```

---

## See Also

- [reference/04-bus/](../../reference/04-bus/README.md) — Bus trait design and target-state semantics
- [reference/02-pulse/](../../reference/02-pulse/README.md) — the ephemeral event type that flows over the Bus
- [operations/performance/09-scaling-patterns.md](../performance/09-scaling-patterns.md) — Bus scaling in multi-host deployments

## Open Questions

- The exact NATS subject schema (which events go to which subjects) is not yet specified.
- Whether the Bus abstraction will support durable queues (replay on reconnect) is an open design question.
- Redis Pub/Sub vs Redis Streams for the Redis backend is not yet decided.
