# 62 — Relay Topic Namespace Migration (colon to dot)

**Priority**: P2 — protocol hygiene; wire-format consistency before any external consumers exist
**Size**: M (2-3 days)
**Crates**: `crates/roko-core/`, `crates/roko-serve/`, `crates/roko-chain/`, `crates/roko-neuro/`, `crates/roko-runtime/`, `crates/roko-cli/`; **Apps**: `apps/agent-relay/`
**Depends on**: None

---

## Background

The relay bus routes messages between agents using hierarchical topic strings. A topic like `chain:31337` means "chain events for chain ID 31337"; `feed:meta:relay` means "relay metadata feed." Agents subscribe to topics by string value; the topic is the only routing discriminant.

The relay protocol uses colon (`:`) as the segment separator throughout the codebase today. An earlier protocol review decided that dot (`.`) separators should be used instead because: (1) dots match the conventions of NATS, RabbitMQ, and Kafka (widely-used messaging systems that operators integrate with); (2) dots are URL-safe, making them usable directly in REST paths like `GET /relay/topics/chain.31337/messages` without percent-encoding; (3) dots enable natural wildcard segment matching in the future (`chain.*` matching `chain.31337`, `feed.>` matching all feed subtopics), whereas `chain:*` has no natural segment boundary.

The relay protocol is explicitly not version-1 frozen, making this the right time to land the change. No external consumers of roko's relay exist yet. The migration requires changing approximately 90 call sites: the canonical `RoomPattern` type in `roko-core`, inline `format!` strings across several crates, match arms in `roko-serve`, and test string literals across `apps/agent-relay` and `crates/roko-serve`.

The `WireEnvelope::validate()` function in `crates/roko-core/src/wire_protocol.rs` accepts dots in room names (it only rejects whitespace and control characters), so the format change does not require a validator update.

## Current State

1. `RoomPattern` in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/wire_protocol.rs:122-162` is the canonical source of topic names. All eight constructors produce colon-separated strings:
   ```rust
   pub fn agent(id: &str) -> String          { format!("agent:{id}") }       // line 125
   pub fn agent_heartbeat(id: &str) -> String { format!("agent:{id}:heartbeat") } // line 130
   pub fn agent_output(id: &str) -> String   { format!("agent:{id}:output") }  // line 135
   pub fn plan(id: &str) -> String           { format!("plan:{id}") }         // line 140
   pub fn group(id: &str) -> String          { format!("group:{id}") }        // line 145
   pub fn chain(chain_id: u64) -> String     { format!("chain:{chain_id}") }  // line 150
   pub const fn system() -> &'static str     { "system" }                     // line 154
   pub const fn learning() -> &'static str   { "learning" }                   // line 159
   ```
   Unit tests at lines 355-362 assert the colon format: `assert_eq!(RoomPattern::chain(8453), "chain:8453")`.

2. Inline `format!` strings that bypass `RoomPattern` (all verified in source):
   - `apps/agent-relay/src/chain_watcher.rs:40` — `format!("chain:{}", config.chain_id)` (doc comments at lines 6 and 32 also use `chain:{chain_id}`)
   - `crates/roko-chain/src/chain_profile.rs:122` — `format!("chain:{}", self.chain_id)` (doc comments at lines 36, 120)
   - `crates/roko-core/src/feed_bus_bridge.rs:60` — `format!("feed:{}:data", pulse.feed_id)`
   - `crates/roko-core/src/feeds/derived.rs:111` — `format!("feed:{}:data", self.id)`; line 131 — `.strip_prefix("feed:")`
   - `crates/roko-core/src/exoskeleton.rs:163` — `"feed:blocks".to_owned()`
   - `crates/roko-serve/src/lib.rs:2486-2550` — match arms: `"chain:block"`, `"chain:tx"`, `"chain:log"`, `"chain:event"`, `"chain:reorg"`
   - `crates/roko-serve/src/feed_agents/onchain.rs:37,90,117,170,197,237,264,322,349,397` — `"feed:chain:block-space"`, `"feed:chain:tps"`, `"feed:chain:fee-burn"`, `"feed:chain:health"`, `"feed:chain:contracts"` (~10 occurrences)
   - `crates/roko-serve/src/feed_agents/chain_watcher.rs:29,66` — `"feed:chain:blocks"` (~2 occurrences)
   - `crates/roko-serve/src/feed_agents/monitors.rs:30,64,93,129,158,196` — `"feed:meta:agents"`, `"feed:meta:relay"`, `"feed:meta:heartbeat"` (~6 occurrences)
   - `crates/roko-neuro/src/lifecycle.rs:989` — `format!("agent:{agent}")`
   - `crates/roko-serve/src/routes/middleware.rs:832,859` — `format!("agent:{}", agent.agent_id)`
   - `crates/roko-runtime/src/builtin_lenses_performance.rs:683` — `format!("agent:{}", wildcard(name))`
   - `crates/roko-runtime/src/builtin_lenses_health.rs:746` — `format!("agent:{agent}")`
   - `crates/roko-serve/src/routes/auth.rs:1275` — `format!("agent:{}", credential.agent_id)`
   - `crates/roko-serve/src/routes/agents.rs:909` — `format!("agent:{agent_id}")` (in a `session_id` field, not a relay topic — verify before changing)
   - `crates/roko-cli/src/knowledge_helpers.rs:303,839` — `format!("agent:{}", transition.agent_id)`

3. Test string literals using colon format:
   - `crates/roko-core/src/wire_protocol.rs:355-362` — 6 `assert_eq!` calls
   - `apps/agent-relay/src/bus.rs:1166-1321` — `"isfr:rates"`, `"chain:31337"`, `"room:a"`, `"agent:a"` (~25 occurrences)
   - `apps/agent-relay/src/protocol.rs:442-467` — `"agent:a"`, `"agent:b"`, `"room:a"` (~5 occurrences)
   - `apps/agent-relay/tests/integration.rs:331-512` — `"isfr:rates"`, `"test:topic"`, `"replay:topic"`, `"room:resume"`, `"room:first"`, `"room:second"` (~15 occurrences)
   - `crates/roko-serve/src/subscription_relay.rs:1426-2090` — `"feed:prices"`, `"feed:news"`, `"feed:*"`, `"feed:{index}"` (~35 occurrences)
   - `crates/roko-serve/src/lib.rs:3066-3070` — `"feed:prices"`, `"feed:*"`, `"feed:price?"`

4. `is_exact_relay_room` at `crates/roko-serve/src/lib.rs:2919` validates relay room names using `WireEnvelope::validate()`, which accepts dots (rejects only whitespace and control characters). No validator change is needed.

## Implementation Plan

Work through the changes in order. Each step should compile and pass tests before moving to the next.

### Step 1: Migrate `RoomPattern` constructors

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/wire_protocol.rs`

Change lines 125-161:
```rust
// Before:
pub fn agent(id: &str) -> String          { format!("agent:{id}") }
pub fn agent_heartbeat(id: &str) -> String { format!("agent:{id}:heartbeat") }
pub fn agent_output(id: &str) -> String   { format!("agent:{id}:output") }
pub fn plan(id: &str) -> String           { format!("plan:{id}") }
pub fn group(id: &str) -> String          { format!("group:{id}") }
pub fn chain(chain_id: u64) -> String     { format!("chain:{chain_id}") }

// After:
pub fn agent(id: &str) -> String          { format!("agent.{id}") }
pub fn agent_heartbeat(id: &str) -> String { format!("agent.{id}.heartbeat") }
pub fn agent_output(id: &str) -> String   { format!("agent.{id}.output") }
pub fn plan(id: &str) -> String           { format!("plan.{id}") }
pub fn group(id: &str) -> String          { format!("group.{id}") }
pub fn chain(chain_id: u64) -> String     { format!("chain.{chain_id}") }
```

Update unit tests at lines 355-360:
```rust
assert_eq!(RoomPattern::agent("a"), "agent.a");
assert_eq!(RoomPattern::agent_heartbeat("a"), "agent.a.heartbeat");
assert_eq!(RoomPattern::agent_output("a"), "agent.a.output");
assert_eq!(RoomPattern::plan("p"), "plan.p");
assert_eq!(RoomPattern::group("g"), "group.g");
assert_eq!(RoomPattern::chain(8453), "chain.8453");
```

Lines 361-362 (`"system"` and `"learning"`) are single-segment names with no separator; leave them unchanged.

Also update the test string literal at line 422 (`"room:a"` in a test fixture if it's a relay topic) — check context and update if it's meant to be a relay room name.

Run `cargo test -p roko-core` after this step to confirm all `roko-core` tests pass.

### Step 2: Migrate inline `format!` strings in production code

Replace all colon-separated topic `format!` strings in non-test code with dot-separated equivalents.

**`apps/agent-relay/src/chain_watcher.rs`** (lines 6, 25, 32, 40):
```rust
// Line 40:
let topic = format!("chain.{}", config.chain_id);
// Update doc comments at lines 6 and 32 to reference chain.{chain_id}
```

**`crates/roko-chain/src/chain_profile.rs`** (lines 36, 120, 122):
```rust
// Line 122:
format!("chain.{}", self.chain_id)
// Update doc comments at lines 36 and 120
```

**`crates/roko-core/src/feed_bus_bridge.rs`** (line 60):
```rust
// Before:
Topic::new(format!("feed:{}:data", pulse.feed_id))
// After:
Topic::new(format!("feed.{}.data", pulse.feed_id))
```

**`crates/roko-core/src/feeds/derived.rs`** (lines 111, 131):
```rust
// Line 111:
format!("feed.{}.data", self.id)
// Line 131 (strip_prefix):
.strip_prefix("feed.")
// If the suffix ":data" is also matched, change to ".data"
```

**`crates/roko-core/src/exoskeleton.rs`** (line 163):
```rust
// Before:
purpose: "feed:blocks".to_owned()
// After:
purpose: "feed.blocks".to_owned()
```

**`crates/roko-serve/src/lib.rs`** match arms (lines 2486-2550):
```rust
"chain.block" => { ... }
"chain.tx"    => { ... }
"chain.log"   => { ... }
"chain.event" => { ... }
"chain.reorg" => { ... }
```

**`crates/roko-serve/src/feed_agents/onchain.rs`** (~10 occurrences):
Change `"feed:chain:block-space"` → `"feed.chain.block-space"`, etc. Confirm every occurrence.

**`crates/roko-serve/src/feed_agents/chain_watcher.rs`** (~2 occurrences):
Change `"feed:chain:blocks"` → `"feed.chain.blocks"`.

**`crates/roko-serve/src/feed_agents/monitors.rs`** (~6 occurrences):
Change `"feed:meta:agents"` → `"feed.meta.agents"`, `"feed:meta:relay"` → `"feed.meta.relay"`, `"feed:meta:heartbeat"` → `"feed.meta.heartbeat"`.

**`crates/roko-neuro/src/lifecycle.rs`** (line 989):
Change `format!("agent:{agent}")` → `format!("agent.{agent}")`.

**`crates/roko-serve/src/routes/middleware.rs`** (lines 832, 859):
Change `format!("agent:{}", agent.agent_id)` → `format!("agent.{}", agent.agent_id)`. Verify these are relay topic strings (the context is session_id assignment for relay subscriptions).

**`crates/roko-runtime/src/builtin_lenses_performance.rs`** (line 683):
Change `format!("agent:{}", wildcard(name))` → `format!("agent.{}", wildcard(name))`.

**`crates/roko-runtime/src/builtin_lenses_health.rs`** (line 746):
Change `format!("agent:{agent}")` → `format!("agent.{agent}")`.

**`crates/roko-serve/src/routes/auth.rs`** (line 1275):
Change `format!("agent:{}", credential.agent_id)` → `format!("agent.{}", credential.agent_id)`. Verify this is a relay topic (not an auth scope string).

**`crates/roko-serve/src/routes/agents.rs`** (line 909):
Verify: `session_id: format!("agent:{agent_id}")` — check whether this is a relay room topic or a session identifier. If it's used as a relay topic, change to `"agent.{agent_id}"`. If it's a session identifier (not a relay routing key), leave it unchanged and note the exception.

**`crates/roko-cli/src/knowledge_helpers.rs`** (lines 303, 839):
Change `format!("agent:{}", transition.agent_id)` → `format!("agent.{}", transition.agent_id)`. Verify these are relay topics.

Run `cargo build --workspace` after completing all production changes. Fix any missed occurrences that cause compile errors.

### Step 3: Migrate test string literals

Update all hardcoded colon-format topic strings in test code:

**`apps/agent-relay/src/bus.rs`** (lines 1166-1321, ~25 occurrences):
- `"isfr:rates"` → `"isfr.rates"`
- `"chain:31337"` → `"chain.31337"`
- `"room:a"`, `"room:b"`, `"room:c"` → `"room.a"`, `"room.b"`, `"room.c"`
- `"agent:a"`, `"agent:b"` → `"agent.a"`, `"agent.b"`

**`apps/agent-relay/src/protocol.rs`** (lines 442-467, ~5 occurrences):
- `"agent:a"` → `"agent.a"`, `"agent:b"` → `"agent.b"`, `"room:a"` → `"room.a"`

**`apps/agent-relay/tests/integration.rs`** (lines 331-512, ~15 occurrences):
- `"isfr:rates"` → `"isfr.rates"`
- `"test:topic"` → `"test.topic"`
- `"replay:topic"` → `"replay.topic"`
- `"room:resume"` → `"room.resume"`, `"room:first"` → `"room.first"`, `"room:second"` → `"room.second"`
- All test assertions that check `"subscribed:isfr:rates"` → `"subscribed:isfr.rates"` (the event name format may embed the topic)

**`crates/roko-serve/src/subscription_relay.rs`** (lines 1426-2090, ~35 occurrences):
- `"feed:prices"` → `"feed.prices"`
- `"feed:news"` → `"feed.news"`
- `"feed:*"` → `"feed.*"` (wildcard still uses dot separator)
- `"feed:{index}"` pattern (if it's a format string, change the colon)

**`crates/roko-serve/src/lib.rs`** (lines 3066-3070):
- `"feed:prices"` → `"feed.prices"`
- `"feed:*"` → `"feed.*"`
- `"feed:price?"` → `"feed.price?"`

Run `cargo test --workspace` to confirm all tests pass.

### Step 4: Verify no remaining colon-format topic strings

Run a workspace-wide search to confirm no colon-separated relay topic strings remain:

```bash
grep -rn '"[a-z][a-z]*:[a-z{]' crates/ apps/ --include='*.rs' \
  | grep -v target/ \
  | grep -v '"http\|"ws:\|"wss:\|"https:\|0x\|://\|":"\|permission\|agent:write\|agent:capability\|plan:write\|admin\|read\|:write\|:read' \
  | grep -v '// '
```

Review each remaining hit. Non-topic colons (auth scope strings like `"agent:write"`, URL prefixes like `"http:"`, etc.) are expected and should be excluded. The goal is zero relay topic strings using colons.

### Step 5: Build and test

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## Acceptance Criteria

1. `RoomPattern::agent("a")` returns `"agent.a"` (not `"agent:a"`).
2. `RoomPattern::agent_heartbeat("a")` returns `"agent.a.heartbeat"`.
3. `RoomPattern::chain(31337)` returns `"chain.31337"`.
4. `apps/agent-relay/src/chain_watcher.rs` publishes on `"chain.{chain_id}"`.
5. `crates/roko-core/src/feed_bus_bridge.rs` routes to `"feed.{id}.data"`.
6. `crates/roko-core/src/feeds/derived.rs` constructs and parses `"feed.{id}.data"` topics.
7. `publish_chain_watcher_payload` in `crates/roko-serve/src/lib.rs` matches on `"chain.block"`, `"chain.tx"`, `"chain.log"`, `"chain.event"`, `"chain.reorg"`.
8. All feed agent descriptors in `crates/roko-serve/src/feed_agents/` use dot separators (`"feed.chain.blocks"`, `"feed.meta.relay"`, etc.).
9. No colon-separated relay topic string literals remain in `apps/agent-relay/`, `crates/roko-core/`, `crates/roko-serve/`, `crates/roko-chain/`, `crates/roko-neuro/`, `crates/roko-runtime/`, `crates/roko-cli/` (excluding non-topic colons in auth scopes, URLs, etc.).
10. All unit tests in `crates/roko-core/src/wire_protocol.rs` are updated and pass.
11. All integration tests in `apps/agent-relay/tests/integration.rs` pass.
12. `cargo test --workspace` passes.
13. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Out of Scope

- **Wildcard subscription support** (`chain.*`, `feed.>`) — this migration only changes the separator to make wildcards possible later.
- **`resume_after` / `last_seq` semantics** — separate protocol enhancement.
- **Multi-topic batch subscribe** — separate protocol enhancement.
- **Chain watcher `eth_subscribe` migration** (replacing 2s polling with WebSocket subscription) — separate performance improvement.
- **Relay auth** (agent passport verification for shared relays) — separate feature.
- **`ts` field in outbound topic_message** — already stored internally, not yet serialized; separate wire-format fix.
- **Topic GC, backpressure policies, metrics** — separate operational concerns.

## Verification Checklist

- [ ] Change `RoomPattern` constructors; run `cargo test -p roko-core` — passes
- [ ] Update inline `format!` strings in all production files (Step 2); run `cargo build --workspace` — clean
- [ ] Update test string literals in all test files (Step 3); run `cargo test --workspace` — passes
- [ ] Run the grep search from Step 4 and confirm zero remaining topic colons
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` — clean
- [ ] Manually confirm: start `roko serve`, observe feed agent topic names in logs match dot format
- [ ] Manually confirm: start `apps/agent-relay`, connect a test agent, subscribe to `chain.31337`, confirm chain watcher publishes on the dotted topic

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-core/src/wire_protocol.rs` | Change `RoomPattern` constructors and unit tests to dot format |
| `apps/agent-relay/src/chain_watcher.rs` | Change `format!("chain:{}", ...)` to `"chain.{}"` and update doc comments |
| `apps/agent-relay/src/bus.rs` | Update test topic strings (~25 occurrences) |
| `apps/agent-relay/src/protocol.rs` | Update test topic strings (~5 occurrences) |
| `apps/agent-relay/tests/integration.rs` | Update test topic strings (~15 occurrences) |
| `crates/roko-chain/src/chain_profile.rs` | Change `format!("chain:{}", ...)` and doc comments |
| `crates/roko-core/src/feed_bus_bridge.rs` | Change `"feed:{}:data"` → `"feed.{}.data"` |
| `crates/roko-core/src/feeds/derived.rs` | Change feed topic construction and `strip_prefix` |
| `crates/roko-core/src/exoskeleton.rs` | Change `"feed:blocks"` → `"feed.blocks"` |
| `crates/roko-serve/src/lib.rs` | Change match arms and test assertions |
| `crates/roko-serve/src/feed_agents/onchain.rs` | Change all `"feed:chain:*"` topic strings (~10) |
| `crates/roko-serve/src/feed_agents/chain_watcher.rs` | Change `"feed:chain:blocks"` strings (~2) |
| `crates/roko-serve/src/feed_agents/monitors.rs` | Change `"feed:meta:*"` strings (~6) |
| `crates/roko-serve/src/subscription_relay.rs` | Update test topic strings (~35) |
| `crates/roko-serve/src/routes/middleware.rs` | Change relay topic `format!` strings at lines 832, 859 |
| `crates/roko-serve/src/routes/auth.rs` | Change relay topic `format!` at line 1275 (verify it's a relay topic) |
| `crates/roko-serve/src/routes/agents.rs` | Change or verify `format!` at line 909 |
| `crates/roko-neuro/src/lifecycle.rs` | Change `format!("agent:{agent}")` at line 989 |
| `crates/roko-runtime/src/builtin_lenses_performance.rs` | Change topic format at line 683 |
| `crates/roko-runtime/src/builtin_lenses_health.rs` | Change topic format at line 746 |
| `crates/roko-cli/src/knowledge_helpers.rs` | Change topic format at lines 303, 839 |
