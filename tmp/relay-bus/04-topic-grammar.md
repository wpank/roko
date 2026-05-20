# Topic Grammar Decision

## Decision: Use dots (`.`)

Topics use dot-separated hierarchical namespaces. Migrate from the current colon-separated format.

## Why Dots

### Industry convention

The three messaging systems with first-class hierarchical topic support and wildcards all use dots:

| System | Delimiter | Wildcards |
|---|---|---|
| NATS | `.` | `*` (one segment), `>` (rest) |
| RabbitMQ | `.` | `*` (one segment), `#` (zero+) |
| Kafka | `.` (convention) | None |

MQTT and Solace use `/` (path-style). Redis uses `:` by convention but has no structural semantics — its glob matching operates on raw strings, not segments.

### URL safety

Topics appear in REST endpoints like `GET /relay/topics/{topic}/messages`. Dots are unremarkable in URLs. Colons are technically legal (RFC 3986) but visually confusing and sometimes mishandled by URL parsers, proxy rules, and logging tools.

```
# Dots — clean
GET /relay/topics/isfr.rates/messages

# Colons — ambiguous
GET /relay/topics/isfr:rates/messages

# Slashes — collides with route structure
GET /relay/topics/isfr/rates/messages
```

### Wildcard readiness

When we add wildcard subscriptions (planned for v2), dots give us natural segment boundaries:

```
chain.*         → matches chain.31337, chain.1
isfr.>          → matches isfr.rates, isfr.epochs, isfr.symphony.123
job.*           → matches job.posted, job.awarded
```

With colons, wildcards would need custom parsing or fall back to glob matching (Redis-style), which is less expressive.

### Internal consistency

The codebase already uses dots in most places:
- Internal bus topics (Pulse): `isfr.rates`, `isfr.epochs`
- Tool names: `chain.balance`, `chain.transfer`
- Event log: `task.assigned`, `agent.spawned`

The relay is the outlier using colons. The `ISFRFeed.map_topic()` function literally converts colons to dots (`"isfr:rates" => "isfr.rates"`). Unifying on dots eliminates this translation layer.

## Migration

### Before (colons)

```
chain:{chain_id}
isfr:rates
isfr:epochs
feed:meta:relay
feed:{domain}:{name}
```

### After (dots)

```
chain.{chain_id}
isfr.rates
isfr.epochs
feed.meta.relay
feed.{domain}.{name}
```

### Files to change

1. `apps/agent-relay/src/chain_watcher.rs` — topic string in publish call
2. `apps/agent-relay/src/lib.rs` — any hardcoded topic references
3. `crates/roko-core/src/isfr_feed.rs` — remove the colon-to-dot mapping in `map_topic()`
4. `crates/roko-agent-server/src/features/relay_subscriber.rs` — topic strings in subscribe calls
5. `crates/roko-serve/src/lib.rs` — topic strings in ISFR relay bridge setup
6. Any test files referencing topic strings

### Compatibility

This is a breaking change to the relay wire protocol. Since the relay is not yet publicly frozen and has no external consumers, this is the right time to make the change. The protocol is explicitly "not v1 frozen" per the assessment doc.

## Topic Namespace

```
# Chain events (from chain watcher)
chain.{chain_id}                  Blocks, contract events
chain.{chain_id}.block            Block-only events (future, with wildcards)
chain.{chain_id}.event            Contract events only (future)

# ISFR
isfr.rates                        Composite rate updates
isfr.epochs                       Epoch transitions
isfr.symphony.{job_id}            Per-job ISFR coordination

# Jobs / marketplace
job.posted                        New job announcements
job.{job_id}.status               Per-job status updates
job.{job_id}.bid                  Per-job bid events

# Agent presence
agent.presence                    Online/offline/heartbeat
agent.{id}.status                 Per-agent detailed status

# Feeds (data streams with metadata)
feed.{domain}.{name}              Named data feeds
feed.meta.relay                   Relay internal stats feed

# Workspaces
workspace.{id}                    Workspace-scoped events

# System
system.relay                      Relay health events
```

## Wildcard Syntax (Future)

When wildcard subscriptions are added, use NATS conventions:

- `*` matches exactly one segment: `chain.*` matches `chain.31337` but not `chain.31337.block`
- `>` matches one or more segments (terminal only): `chain.>` matches `chain.31337`, `chain.31337.block`, `chain.31337.event`
