# Realtime Event Surface

> **TL;DR**: Build a single, consistent realtime interface that
> exposes the Bus, the Substrate, and StateHub projections to
> external consumers via three co-equal transports: WebSocket,
> Server-Sent Events, and (optionally) gRPC streaming. Every
> consumer — browser, mobile, Slack bot, dashboard, another
> Roko instance — uses the same vocabulary. This doc specifies
> the protocol, the auth story, the back-pressure semantics,
> and the patterns for common consumer shapes.

> **For first-time readers**: "Realtime" here means three co-equal
> transports carrying the same message vocabulary: WebSocket (full-duplex,
> browsers), SSE (server-to-client, survives proxies), gRPC (typed,
> server-to-server). Every external consumer picks one; the protocol,
> auth, back-pressure, and cursor-resumption are identical. Read 26
> (StateHub) first — this is 26's wire layer. Pair with 29 for the
> web UI that is the first-party consumer.

## 1. Why three transports

- **WebSocket**: full-duplex, low-latency, best for chat and
  interactive UIs. Required for multi-direction (send Pulses back).
- **Server-Sent Events (SSE)**: one-way, survives most proxies,
  trivially implementable in a browser, cheap on the server.
  Best for dashboards and lightweight feeds.
- **gRPC streaming**: typed, efficient, necessary for server-to-server
  and high-throughput cases. Optional because it's heavier.

Same data, three ways in. Consumers pick based on what fits their
stack.

## 2. The subscription protocol

A protocol-agnostic message vocabulary usable over any transport:

```json
{
  "type": "subscribe" | "unsubscribe" | "query" | "publish" |
          "state" | "delta" | "event" | "ack" | "error" | "ping" | "pong",
  "id": "req-12345",
  "payload": { ... }
}
```

### 2.1 Subscribe

```json
{
  "type": "subscribe",
  "id": "sub-abc",
  "payload": {
    "channel": "projection:cohort_health" | "topic:gate.*" | "engram-stream:lineage:0xabc...",
    "filter": { ... },
    "cursor": "optional-resume-cursor"
  }
}
```

Server responds with an initial `state` (for projections) or
`ack` (for raw topics), then an ongoing stream of `delta` or
`event` messages.

### 2.2 Publish (WebSocket only)

```json
{
  "type": "publish",
  "id": "pub-def",
  "payload": {
    "topic": "user.prompt",
    "body": { ... }
  }
}
```

Allows a chat UI to send user-originated Pulses. Subject to
auth, rate limits, topic allow-lists.

### 2.3 Query

```json
{
  "type": "query",
  "id": "q-ghi",
  "payload": {
    "target": "projection:cohort_health" | "engram:0xabc..." | "heuristic:uuid",
    "filter": { ... },
    "at_cursor": "optional"
  }
}
```

One-shot retrieval. Response is a single `state` or `event`.

## 3. Channel taxonomy

Channels are the unit of subscription. Five types:

| Prefix | Meaning | Example |
|---|---|---|
| `projection:` | Named StateHub projection | `projection:cohort_health` |
| `topic:` | Raw Bus topic (pattern allowed) | `topic:gate.failed.*` |
| `engram-stream:` | Live filtered Substrate writes | `engram-stream:role=researcher` |
| `agent:` | Per-agent activity feed | `agent:agt_042` |
| `session:` | Per-session conversation | `session:sess_xyz` |

A consumer can mix channel types in one connection. The client
library handles the fanout internally.

## 4. Back-pressure

Every subscription is back-pressured:

- Server buffers up to N messages per subscription.
- On overflow, either:
  - **Drop**: `AtMostOnce` mode — oldest dropped silently.
  - **Coalesce**: if the channel is a projection, collapse
    overlapping deltas.
  - **Disconnect**: `Exactly(Cursor)` mode — disconnect with an
    error; client reconnects with a cursor.

The consumer declares its preference on subscribe.

## 5. Auth and authorization

Connections authenticate once, subscribe many times. Auth
options:

- **API key** (header or query param) for machine consumers.
- **OIDC Bearer token** for user-scoped consumers.
- **Session cookie** for browser UIs.
- **Per-tenant scoped tokens** that limit which channels are
  accessible.

Authorization happens per-subscribe: the server checks whether
this identity can see this channel with this filter. Denials
come back as `error` messages, not dropped connections — so a
single bad subscription doesn't kill the link.

## 6. Cursors and resumption

Every message carries a cursor. Clients track the last cursor
they successfully handled. On reconnect:

```json
{
  "type": "subscribe",
  "payload": {
    "channel": "projection:gate_pipeline",
    "cursor": "0x04f1..."
  }
}
```

Server replays from cursor. If the cursor is too old (beyond
retention), server sends a `state` (full reload) and the
current cursor, letting the client catch up.

## 7. Presence and heartbeat

WebSocket sessions have `ping`/`pong` every 30s. SSE uses
comment-lines as heartbeat. gRPC has native keep-alive.

Presence: a `presence:*` channel surfaces who's connected. For
multi-user UIs this enables "3 others viewing this plan" chrome.
Not critical for v1; reserve the namespace.

## 8. Client libraries

Ship first-party client libraries in three languages:

- **TypeScript** (`@roko/client`): for web UIs. Includes typed
  projection shapes generated from the schema.
- **Python** (`roko-client`): for data scripts and bots.
- **Rust** (`roko-client-rs`): for server-to-server integration
  and native GUIs.

Each:

- Wraps the transport choice.
- Exposes typed `subscribe<T>(channel)` functions.
- Handles reconnect with cursor resumption automatically.
- Rate-limits publishes.
- Converts error messages to idiomatic exceptions.

Reuse the wire schema from `roko-statehub` (`26`) so types stay
in sync. Use a schema codegen (e.g., TypeShare) to avoid
hand-duplicated types.

## 9. GraphQL (maybe, carefully)

Some users will ask for GraphQL. It's a natural fit for the
query-plus-subscribe pattern. Two paths:

- **First-party**: a thin GraphQL gateway on top of the same
  subscription protocol. About a week of work for a read-only
  schema; more for mutations.
- **User-built**: publish the wire protocol; users wanting
  GraphQL build an adapter themselves.

Recommendation: skip first-party GraphQL for v1. It adds a second
query language and splits attention. If the ecosystem asks for
it, ship an adapter plugin.

## 10. Example consumer: a React UI

```ts
import { RokoClient } from "@roko/client";

const roko = new RokoClient({ url: "wss://roko.example.com/stream", token });

function CohortHealth() {
  const [state, setState] = useState<CohortHealthState | null>(null);
  useEffect(() => {
    const sub = roko.subscribe("projection:cohort_health", {}, (msg) => {
      if (msg.type === "state") setState(msg.payload);
      else if (msg.type === "delta") setState(s => applyDelta(s, msg.payload));
    });
    return () => sub.close();
  }, []);
  return state ? <Dashboard data={state} /> : <Spinner />;
}
```

Subscribe, render. No polling, no custom reconnect code, no
custom cursor management. The client library handles it.

## 11. Example consumer: a Slack bot

```python
from roko_client import RokoClient

roko = RokoClient(url="https://roko.example.com/stream", token=...)

def on_event(e):
    if e.topic == "gate.failed.unit":
        slack.post(f"Gate failed on {e.body['task']}: {e.body['reason']}")

roko.subscribe_topic("gate.failed.*").each(on_event)
```

Four lines. Existing Slack bot infrastructure handles the rest.
The Slack bot is a *subscriber*, not an *integration*; this
distinction is the point of the event surface.

## 12. Example: two Roko instances syncing

A deployment that wants to share state across two machines:

```rust
// instance B subscribes to instance A's heuristic stream
let client = RokoClient::connect("wss://a.example.com/stream", token).await?;
let mut sub = client.subscribe_engram_stream(Filter::kind("heuristic")).await?;
while let Some(h) = sub.next().await {
    my_substrate.put(h).await?;
}
```

Cross-instance replication built on the same primitives as the
web UI subscription. The distributed-Roko story (clustered
deployment from `24` §1.4) uses this plumbing.

## 13. Publishing back

WebSocket consumers can publish Pulses. Use cases:

- **Chat UI**: user prompt → `topic:user.prompt`.
- **Approval UI**: human approves a checkpoint → `topic:ack.approval`.
- **External trigger**: CI system says a test passed →
  `topic:external.gate_result`.

Publishing is rate-limited, topic-filtered by role, and audited.
This is the asymmetric but still-capable inbound channel that
makes external UIs first-class participants in the agent loop
rather than passive observers.

## 14. SSE nuances

SSE is great but has gotchas:

- **Many browsers limit 6 SSE per origin**. Don't fan out one
  SSE per subscription; multiplex.
- **Proxies buffer**: require `X-Accel-Buffering: no` and
  configuration.
- **Reconnect**: use the standard `Last-Event-ID` header so the
  browser's native reconnect carries the cursor.

The server library should handle all three; doc it once so users
don't rediscover them.

## 15. Observability of the surface

The realtime surface generates its own telemetry:

- `roko.realtime.connections` (gauge)
- `roko.realtime.subscriptions` (gauge, by channel kind)
- `roko.realtime.messages_per_second` (by direction)
- `roko.realtime.cursor_lag` (how far behind consumers are)

Exposed via the same Prometheus endpoint as the rest of the
system. Operators can tell at a glance if the surface is
healthy.

## 16. Security

Beyond authentication:

- **Input validation** on `publish`: topic allow-list, payload
  size cap, body schema check.
- **Rate limits** per connection per channel.
- **DDoS hygiene**: connection limit per IP, exponential backoff
  on auth failures.
- **No user-controlled filter functions** cross-process — only
  server-side declarative filters.
- **Secrets never in events**: the Bus infrastructure already
  tags secret-bearing fields; the realtime surface drops them
  on the way out.

## 17. What this enables end-to-end

A developer building a bespoke interface gets:

1. Realtime data without polling, without reinventing.
2. Typed client libraries so autocomplete works.
3. Reconnect and resumption without thinking about it.
4. A publishing path for user-initiated events.
5. Auth story consistent with the CLI and Web UI.
6. Observability out of the box.

The net effect: somebody can build a high-quality custom Roko UI
— for a specific domain, for their team's taste — in a weekend
rather than a month. That's a force multiplier for the plugin
ecosystem (`17`) and the domain-specific agent story (`25`).

## 18. Staging

1. **WebSocket + SSE server** on top of StateHub. Two weeks.
2. **TypeScript client** with codegen. One week.
3. **Python + Rust clients**. One week.
4. **Auth integration** with existing identity layer. One week.
5. **Back-pressure + cursor resumption hardening**. One week.
6. **Docs + examples**. One week.

Total: ~7 weeks for a production-quality realtime surface. After
this the external ecosystem has a stable contract to build on.

## 19. Wire format stability contract

Consumers build against the wire protocol; breaking it breaks
everyone downstream. Stability rules:

- **Additions**: new `type` values and new fields under `payload` are
  non-breaking. Clients must ignore unknown fields.
- **Renames**: forbidden after public release. Ship a new name
  alongside.
- **Removals**: forbidden without a major version bump *and* a
  deprecation period of at least one minor version.
- **Semantics changes**: forbidden. If a message's meaning
  changes, use a new `type`.
- **Cursor format**: opaque to clients. Server is free to change
  internal cursor encoding but must accept any cursor it previously
  emitted for the retention window.

The wire format lives in `roko-protocol` (new crate, split out of
`roko-serve`) with a frozen schema file. Tests validate that every
released version still parses a recorded corpus of messages from
prior versions. Breaking the corpus fails CI.

## 20. Reconnect behavior details

Concrete rules clients can rely on:

1. **Connection drop**: client auto-reconnects with exponential
   backoff (100 ms, 500 ms, 1 s, 2 s, 5 s, 10 s, 30 s cap).
2. **On reconnect**: client resubscribes with the last-known cursor
   per channel.
3. **Cursor too old**: server sends a fresh `state` for projection
   channels, or an `error` + current cursor for raw topic channels.
   Client's responsibility to refresh local state.
4. **Auth expiry during reconnect**: server responds `error: {code:
   "unauthenticated"}`; client re-obtains token and retries.
5. **Rate limit on reconnect**: per-IP cap on reconnects/minute.
   Client libraries implement this client-side to pre-empt server
   disconnects.

These rules go into client libraries (§8) so they're inherited for
free by most consumers.

## 21. Integration patterns

Common consumer shapes with recommended protocol choices:

| Consumer | Transport | Channels typically used |
|---|---|---|
| Web UI (React/Svelte) | WebSocket | `projection:*`, publish for user input |
| Mobile app (background) | SSE | `projection:cohort_health`, `projection:active_tasks` |
| Slack bot | WebSocket | `topic:gate.failed.*`, `topic:safety.approval.requested` |
| Grafana data source | SSE (parsed as JSONL) | `projection:bus_stats`, `projection:substrate_stats` |
| Audit log ingestion | gRPC | `topic:safety.*`, all `engram-stream:*` |
| Cross-Roko replication | gRPC | `engram-stream:*`, filter by kind |
| Browser extension | WebSocket | `session:*`, `projection:active_tasks` |

Each pattern has an example in `@roko/client` docs. The wire protocol
doesn't care; the ergonomics in the client libraries pick the right
transport per use case.

## 22. Cross-references

- Projections carried over this wire: `26-statehub-rearchitecture.md`.
- Bus behind projections: `03-bus-as-first-class.md`.
- Auth/safety rules on subscriptions: `32-safety-sandbox-provenance.md` §6.
- Web UI that consumes this protocol: `29-web-ui-architecture.md`.
- Plugin protocols for custom projections/channels:
  `17-plugin-extension-architecture.md` §11 (WASM host).
- Observability for the realtime surface itself:
  `33-observability-telemetry.md` §6.
