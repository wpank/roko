# 04 — ERC-8004 Chain-Based Agent Discovery

> **Source plans**: `tmp/ux/01-agent-server-design.md` (registration flow,
> discovery table, filtering), `tmp/ux/03-auth-and-discovery.md` (the chain
> half), `tmp/ux/06-open-questions.md` Q4 + Q10 (network-only flow,
> ERC-8004 filtering).
>
> **Status as of 2026-05-01**:
> - The agent-server already publishes an `AgentCard` and (if configured)
>   submits `updateAgentCardUri(passportId, cardUri)` to `IdentityRegistry`
>   (`crates/roko-agent-server/src/registration.rs:177-191`).
> - Discovery in production currently runs through roko-serve's HTTP
>   registry: agents heartbeat via `POST /api/heartbeats` and the aggregator
>   fans out from a roko-serve-internal `DiscoveredAgent` list.
> - Chain-side discovery — enumerating ERC-8004 passports, filtering by
>   capability bitmask, fetching Agent Card JSON, and using its
>   `endpoints` map — is **not implemented**. The aggregator does not call
>   `IdentityRegistry::registeredCount()` or read `agentCardUri` for any
>   passport.
> - Filtering (bit 15 + `"roko"` domain tag) is **not implemented**.
>
> **Effort**: 5-8 days.
>
> **Risk**: Medium. New on-chain plumbing, network-only-mode UX
> implications, RPC cost considerations.

---

## What this plan accomplishes

Make agent discovery a chain primitive, not a roko-serve-internal registry.
After this plan:

- `IdentityRegistryReader` exists in `roko-chain` with `enumerate_passports`,
  `get_agent_card_uri`, `get_capability_mask` methods.
- `AgentCardFetcher` (small HTTP/IPFS/data-URI multi-resolver) hydrates the
  Agent Card JSON behind any `cardUri` returned from the registry.
- The aggregator's `known_agents()` becomes the *union* of (a) chain-derived
  agents and (b) roko-serve-managed agents. Conflicts (same agent in both)
  resolve to the chain entry.
- Capability bitmask **bit 15** is reserved as the "Roko-compatible" flag.
  Agents set it on registration. Discovery filters by it.
- The `"roko"` domain tag is required on every Agent Card published by
  `roko-agent-server`. Discovery confirms with this tag (defence-in-depth).
- A `/api/agents/discover-chain` debug endpoint dumps the raw chain list
  (helps QA distinguish chain-discovery bugs from roko-serve-registry bugs).
- A "network-only" dashboard mode (configured via env var) sources its
  agent list purely from chain discovery, without roko-serve. This unblocks
  open question #4 from `tmp/ux/06-open-questions.md`.
- Demo bootstrap pre-populates the mirage-rs fork with N Roko-tagged
  passports + Agent Card data-URIs. Demo runs out of the box without
  manual on-chain setup (resolves the sub-question in
  `06-open-questions.md` §10).

## Why this matters

The architecture vision in `00-architecture-overview.md` says "agent
server discovery is already solved by ERC-8004 — no new registration
mechanism needed." Today we still rely on a separate HTTP registry (roko-serve).
Closing this gap is what makes the system actually decentralised:
multiple operators can run roko-serve instances and discover each others'
agents purely on-chain.

It also resolves the network-only user flow (a user without roko-serve)
which today has no path at all.

---

## Required reading

```
contracts/src/IdentityRegistry.sol                       (8004 spec)
contracts/test/IdentityRegistry.t.sol
crates/roko-chain/src/identity_economy_identity.rs       (existing wallet-side ops)
crates/roko-chain/src/lib.rs
crates/roko-chain/src/marketplace.rs                     (template for a reader)
crates/roko-agent-server/src/registration.rs             (publishes agent cards)
crates/roko-serve/src/state.rs                           (DiscoveredAgent + AppState)
crates/roko-serve/src/routes/aggregator.rs::known_agents  (current discovery path)
tmp/ux/01-agent-server-design.md
tmp/ux/03-auth-and-discovery.md
tmp/ux/06-open-questions.md   §4 (network-only), §10 (8004 filtering)
```

`cast call <IdentityRegistry> 'registeredCount()(uint256)' --rpc-url $RPC`
plus `cast call <IdentityRegistry> 'getPassport(uint256)((address,uint256,string))' 0`
are the read primitives this plan wraps.

---

## Deliverables

### Chain reader

1. **`crates/roko-chain/src/identity_registry.rs`** — read-only counterpart
   to the existing wallet-side identity ops. API:

   ```rust
   pub struct IdentityRegistryReader { /* address + provider */ }

   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct PassportRecord {
       pub passport_id: U256,
       pub owner: Address,
       pub agent_card_uri: String,
       pub capability_mask: u64,
       pub registered_at: u64,
   }

   impl IdentityRegistryReader {
       pub fn new(address: Address, provider: Arc<dyn ChainProvider>) -> Self;
       pub async fn count(&self) -> Result<u64>;
       pub async fn passport(&self, id: U256) -> Result<PassportRecord>;
       pub async fn enumerate_passports(&self, filter: PassportFilter) -> Result<Vec<PassportRecord>>;
       pub async fn watch_registrations(&self) -> mpsc::Receiver<PassportRecord>;
   }

   #[derive(Debug, Default, Clone)]
   pub struct PassportFilter {
       pub require_capability_bits: Option<u64>,    // e.g. (1 << 15) for Roko-compatible
       pub forbid_capability_bits: Option<u64>,
       pub since_passport_id: Option<U256>,
       pub max_count: Option<usize>,
   }
   ```

   Use alloy `sol!` to bind the registry. Mirror the layout of
   `marketplace.rs`. Pagination: `enumerate_passports` walks
   `0..count()` in chunks of 50; if a `since` filter is given start there.

### Agent Card fetcher

2. **`crates/roko-chain/src/agent_card_fetcher.rs`** — fetches an Agent Card
   JSON given its URI. Three resolvers: HTTP, IPFS gateway,
   `data:application/json;base64,…`. Returns
   `roko_agent_server::AgentCard`. Cap fetch size at 64 KB; reject larger
   responses.

   ```rust
   pub struct AgentCardFetcher { /* http client + ipfs gateway URL */ }

   impl AgentCardFetcher {
       pub fn new() -> Self;
       pub fn with_ipfs_gateway(self, url: String) -> Self;
       pub async fn fetch(&self, uri: &str) -> Result<AgentCard>;
   }
   ```

   Validate the resulting JSON shape. Reject cards without an `endpoints`
   map; missing `endpoints.rest` is fine (some agents are MCP-only).

### Capability bitmask

3. **`crates/roko-core/src/capability_bits.rs`** — single source of truth
   for the 64-bit capability mask. Bit allocations:

   ```rust
   pub const BIT_MESSAGING: u64       = 1 << 0;
   pub const BIT_PREDICTIONS: u64     = 1 << 1;
   pub const BIT_RESEARCH: u64        = 1 << 2;
   pub const BIT_TASKS: u64           = 1 << 3;
   pub const BIT_DEFI_ANALYSIS: u64   = 1 << 4;
   pub const BIT_CODE_GEN: u64        = 1 << 5;
   // ... bits 6-14 reserved for future skill flags ...
   pub const BIT_ROKO_COMPATIBLE: u64 = 1 << 15;
   pub const BITS_RESERVED: u64       = 0xFFFF_FFFF_FFFF_0000;
   ```

   Provide `pub fn capability_string_to_bit(name: &str) -> Option<u64>`
   so the agent-server can populate the mask from the
   `capabilities: Vec<String>` field.

### Agent-server registration

4. **`crates/roko-agent-server/src/registration.rs`**:
   - Compute the capability mask from `state.capabilities` plus the
     `BIT_ROKO_COMPATIBLE` bit. Pass it to the `updateAgentCardUri` call
     (via a new selector, e.g. `updateAgentCard(uint256,string,uint64)`,
     which `IdentityRegistry.sol` must support — see Deliverable 5).
   - Always include `"roko"` in `card.domain_tags`. The
     `AgentRegistration::default()` already does this; assert it cannot
     be removed.

### Identity registry contract

5. **`contracts/src/IdentityRegistry.sol`**: add (or expose) a method
   `updateAgentCard(uint256 passportId, string cardUri, uint64 capabilityMask)`
   that updates both `agentCardUri` and `capabilityMask` in one tx.
   Maintain a `mapping(uint256 => uint64) public capabilityMask` storage
   slot. Emit `AgentCardUpdated(uint256 indexed passportId, string cardUri, uint64 capabilityMask)`.
   Include a Foundry test in `contracts/test/IdentityRegistry.t.sol`.

### Aggregator wiring

6. **`crates/roko-serve/src/routes/aggregator.rs`**:

   - Replace `known_agents()` with a function that merges chain-derived
     agents with roko-serve's local registry. Chain entries dominate
     (their `endpoints.rest` is canonical).
   - Cache key: `aggregator:agents:chain:v1`, TTL 30 s. Invalidate on the
     `AgentCardUpdated` event subscription.
   - On boot: subscribe to `AgentCardUpdated` events; on each event,
     refresh the affected entry within ~1 s.
   - New debug route `GET /api/agents/discover-chain` (bearer-protected)
     — returns the raw chain enumeration, no merging. Useful for
     diagnosing "why isn't this agent showing up?"

7. **`crates/roko-serve/src/state.rs::DiscoveredAgent`**: add a field
   `discovery_source: DiscoverySource { Chain { passport_id }, LocalRegistry, Both }`.

### Network-only mode

8. **Dashboard `light` mode** (cross-cutting; coordinate with
   nunchi-dashboard owner): a build flag `VITE_NETWORK_ONLY=true` makes the
   dashboard skip its `roko-serve` calls and source the agent list directly
   from chain (using a JS port of `enumerate_passports` over the existing
   wagmi provider). Leave the aggregator pheromone/knowledge views off in
   network-only mode (those need roko-serve until the on-chain pheromone
   contract lands).

   This is a parallel UX track; the dashboard team owns it. This plan
   provides the chain-side primitives they need.

### Demo bootstrap

9. **`crates/roko-demo/src/scenarios/bootstrap_passports.rs`** — runs
   inside `cargo demo init` and pre-populates 5 Roko passports against a
   freshly forked mirage-rs:

   - Five hardhat keys (well-known dev addresses).
   - Five base64-data-URI Agent Cards with distinct capabilities.
   - Capability mask = `BIT_ROKO_COMPATIBLE | BIT_MESSAGING | …`.
   - Tx hashes printed to `tmp/demo-bootstrap.json` for downstream tests.

10. **README**: add a "Network-only mode" section to `docs/v2/`. Document
    that the dashboard can run without roko-serve when `VITE_NETWORK_ONLY=true`,
    and that pheromone/knowledge views are limited until the matching
    contracts ship.

---

## Step-by-step

### Step 1 — Land contract changes (1 day)

1. Edit `contracts/src/IdentityRegistry.sol`. Add `mapping(uint256 => uint64) public capabilityMask;` and the `updateAgentCard` function.
2. Add tests to `contracts/test/IdentityRegistry.t.sol` for both the new
   write path and the old-style `updateAgentCardUri` (must remain).
3. `forge test`. `forge fmt`. `forge build`.
4. Commit and tag the contract version. Update the deployed addresses in
   `roko.toml.example` (or whatever the existing config carrier is).

Anti-pattern: don't repurpose an unrelated existing function (e.g.
`updateMetadata`) just to avoid adding a function. The semantic is clear,
the gas cost of a new function is negligible, and the tooling assumes
named functions.

### Step 2 — Build `roko-chain::IdentityRegistryReader` (1 day)

Mirror `marketplace.rs`. Use alloy's `sol!` to embed the ABI.
`enumerate_passports` walks ascending IDs in batches of 50.
`watch_registrations` uses `eth_subscribe` over WS; if the provider doesn't
support WS, fall back to a 30 s polling loop and log it.

Tests in `crates/roko-chain/tests/identity_registry_reader.rs` against an
anvil-forked chain with the new contract. Use the existing `crates/roko-chain/src/mock.rs`
shape for unit tests.

### Step 3 — Build `AgentCardFetcher` (half day)

Three resolvers. The `data:` resolver is trivial (decode base64); the IPFS
resolver wraps the gateway URL; the HTTP resolver is `reqwest::get` with
a 5 s timeout and a 64 KB cap.

Tests in `crates/roko-chain/tests/agent_card_fetcher.rs` cover all three
resolvers (use `axum` test server + ad-hoc routes for HTTP, a static
in-memory IPFS gateway stub).

### Step 4 — Capability bitmask in roko-core (half day)

Mostly constants + a small mapping function. Add a single doctest:

```rust
/// ```
/// use roko_core::capability_bits::{capability_string_to_bit, BIT_PREDICTIONS};
/// assert_eq!(capability_string_to_bit("predictions"), Some(BIT_PREDICTIONS));
/// ```
```

Wire into `roko_agent_server::registration` so the registration call sets
the right mask.

### Step 5 — Update agent-server registration (half day)

Two changes in `crates/roko-agent-server/src/registration.rs`:

1. `update_identity_registry` builds the new selector + calldata for
   `updateAgentCard(uint256,string,uint64)` (3 args; current code uses
   the 2-arg form). Keep the 2-arg path for back-compat with old contracts;
   choose based on a `use_v2: bool` field on `AgentRegistration`.

2. Construct the capability mask:

   ```rust
   let mut mask = roko_core::capability_bits::BIT_ROKO_COMPATIBLE;
   for cap in &state.capabilities {
       if let Some(bit) = roko_core::capability_bits::capability_string_to_bit(cap) {
           mask |= bit;
       }
   }
   ```

Add an integration test in `crates/roko-agent-server/tests/registration.rs`
asserting the published card always has `domain_tags` containing `"roko"`.

### Step 6 — Aggregator integration (1.5 days)

Replace `known_agents` with a merge function. Sketch:

```rust
async fn known_agents(state: &Arc<AppState>) -> Result<Vec<DiscoveredAgent>> {
    let cache_key = "aggregator:agents:merged:v1";
    if let Some(cached) = state.cached_value::<Vec<DiscoveredAgent>>(cache_key).await {
        return Ok(cached);
    }

    // 1) Chain-derived list (filtered by Roko bit + "roko" domain tag).
    let chain_agents = if let Some(reader) = state.identity_reader.as_ref() {
        let filter = PassportFilter {
            require_capability_bits: Some(BIT_ROKO_COMPATIBLE),
            ..Default::default()
        };
        let records = reader.enumerate_passports(filter).await?;
        let fetcher = state.agent_card_fetcher.clone();
        join_all(records.iter().map(|r| async {
            let card = fetcher.fetch(&r.agent_card_uri).await.ok()?;
            if !card.domain_tags.iter().any(|t| t == "roko") {
                return None;
            }
            Some(DiscoveredAgent::from_card(r.passport_id, card))
        }))
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    // 2) Local roko-serve registry (legacy until all agents are on-chain).
    let local_agents = state.process_supervisor.discovered_agents();

    // 3) Merge: chain wins on ID conflicts.
    let mut merged: HashMap<String, DiscoveredAgent> = local_agents
        .into_iter()
        .map(|a| (a.agent_id.clone(), a))
        .collect();
    for agent in chain_agents {
        merged.insert(agent.agent_id.clone(), agent);
    }
    let result: Vec<_> = merged.into_values().collect();
    state.cache_value(cache_key, &result, AGENT_LIST_TTL).await;
    Ok(result)
}
```

Tests in `crates/roko-serve/tests/aggregator_chain_discovery.rs` against
an anvil instance with the new contract. Cover: a chain agent only, a
local agent only, both with same id (chain wins), both with different ids.

### Step 7 — Demo bootstrap (1 day)

`crates/roko-demo/src/scenarios/bootstrap_passports.rs` runs inside the
mirage-rs fork on first init:

1. Picks 5 hardhat-default keys.
2. For each, builds an Agent Card JSON, base64-encodes it as a data URI.
3. Calls `IdentityRegistry::register(...)` then `updateAgentCard(...)`
   with appropriate capability masks.
4. Prints the resulting passport IDs to `tmp/demo-bootstrap.json`.

Wire into `roko demo init` (or the existing demo seed CLI). Document in
`tmp/ux/implementation-plans/04-erc8004-chain-discovery.md` (this file)
where the script is and how to re-run it.

### Step 8 — Network-only dashboard mode (1 day, dashboard side)

Mostly a dashboard-team task; this plan provides the prerequisites.
Outline:

1. `nunchi-dashboard/src/services/discovery-chain.ts` — a TS port of
   `enumerate_passports` using viem/wagmi.
2. Behind `VITE_NETWORK_ONLY=true`, replace the
   `${API_BASE}/api/agents` call with a chain enumeration.
3. Document that pheromone/knowledge tabs degrade in network-only mode
   (they need roko-serve until contracts catch up).

### Step 9 — Cleanup + docs (half day)

- Update `tmp/ux/03-auth-and-discovery.md` with a "Closed YYYY-MM-DD"
  header + link to merged PR.
- Update `tmp/ux/06-open-questions.md` items 4 + 10 with their resolution.
- CLAUDE.md: add an "ERC-8004 chain discovery" status row.
- `docs/v2/`: add `chain-discovery.md` covering the capability bitmask,
  the domain tag, the data URI fallback, and the network-only mode.

---

## Anti-patterns to avoid

- **Don't replace the roko-serve registry overnight.** During this
  rollout there will be agents only on chain, only on roko-serve, and
  some on both. The merge function is the contract.
- **Don't trust the capability bitmask alone.** Adversaries can set bit
  15 to false-flag as Roko-compatible. The `"roko"` domain tag in the
  Agent Card is a second filter; defence-in-depth.
- **Don't fetch all Agent Cards on every aggregator request.** Cache the
  hydrated list with TTL 30 s + event-driven invalidation. Cards can
  number in the thousands; un-cached fetch storms will crash a small
  IPFS gateway.
- **Don't use 64 bits as 14 + 50 reserved.** The reserved bits are real;
  treat them as opaque storage. Don't repurpose without a contract
  upgrade plan.
- **Don't extend the schema in a way that breaks the published Agent
  Cards on chain.** Cards already published are immutable from the
  registrar's perspective. Add new fields with `#[serde(default)]` on
  the deserializer and document the schema version in the card itself
  (`version: "1.1"` etc).
- **Don't poll `eth_getLogs` on every cache miss.** Subscribe via WS at
  startup. Polling is the fallback only.
- **Don't make the demo bootstrap idempotent through stateful checks** —
  e.g. "if already registered, skip". The demo runs against a forked
  state; re-running should always produce the same result. If a re-run
  causes "already registered", the bootstrap is broken.

## Done when

1. `cast call <IdentityRegistry> 'capabilityMask(uint256)(uint64)' 0 --rpc-url $RPC`
   returns a non-zero value with bit 15 set, after running an agent server
   wired to a wallet.
2. `curl http://localhost:6677/api/agents/discover-chain` (bearer-protected)
   returns the chain list separately from the merged `/api/agents`.
3. `curl http://localhost:6677/api/agents` includes the chain-discovered
   agents *plus* any roko-serve-managed agents, with no duplicates.
4. `roko demo init` produces 5 chain-registered Roko agents discoverable
   via the aggregator.
5. `cargo test -p roko-chain --test identity_registry_reader` passes.
6. `cargo test -p roko-serve --test aggregator_chain_discovery` passes.
7. The dashboard in network-only mode renders the agent list against
   chain only.
8. `tmp/ux/03-auth-and-discovery.md` and `tmp/ux/06-open-questions.md`
   updated.
