# 02 — Aggregator Knowledge + Pheromone Backends

> **Source plan**: `tmp/ux/04-dashboard-migration.md` ("knowledge endpoints
> return compatibility envelopes but not chain-backed data yet"; "pheromone
> compatibility routes are not yet implemented on roko-serve").
>
> **Status as of 2026-05-01**:
> - Knowledge endpoints are wired in `crates/roko-serve/src/routes/aggregator.rs`
>   to read from a *local* `roko_neuro::knowledge_store::KnowledgeStore`. They
>   are not pulling from chain (`InsightBoard.sol`), and the per-kind counts
>   on `/knowledge/kinds` are hard-coded to 0.
> - Pheromone routes (`/api/pheromones/*`) are entirely **missing** from the
>   aggregator router (grep confirms 1 occurrence of `pheromone` in 1500+
>   lines, in `knowledge_kinds` only).
>
> **Effort**: 4-6 days.
>
> **Risk**: Medium. Adding routes is additive; the risk is in mis-matching
> the response shape against Sam's nunchi-dashboard expectations and breaking
> existing UI panels.

---

## What this plan accomplishes

Close the two remaining gaps in the aggregator's mirage-compatible surface
so that nunchi-dashboard can switch its base URL (track `03`) without
losing the InsightBoard and pheromone views.

After this plan:

- `/api/knowledge/entries`, `/api/knowledge/edges`, `/api/knowledge/search`,
  `/api/knowledge/kinds` are backed by `InsightBoard.sol` events + a local
  `roko-neuro` cache layer with explicit cache invalidation.
- `/api/pheromones`, `/api/pheromones/{kind}`, `/api/pheromones/topology`,
  `/api/pheromones/active`, `/api/pheromones/decay`, `/api/pheromones/decay/{kind}`
  exist, return the same shapes the old `apps/mirage-rs/src/http_api/pheromone.rs`
  did, and source data from chain events plus an in-memory decay simulator.
- The `/knowledge/kinds` counts reflect actual on-chain InsightBoard counts.
- A regression test asserts the aggregator response matches the legacy mirage
  shape for both surfaces.

## Why this matters

These two surfaces are the only blockers preventing track `03` (the
nunchi-dashboard URL swap) from being a one-line change. Without them, the
dashboard's Knowledge and Stigmergy tabs would go blank after the swap.
Operationally this also unblocks track `01` (mirage cleanup) — once the
dashboard works end-to-end against roko-serve, mirage's HTTP surface can
be retired.

---

## Required reading

```
# Source-of-truth on the legacy shape (about to be deleted)
apps/mirage-rs/src/http_api/knowledge.rs
apps/mirage-rs/src/http_api/pheromone.rs
apps/mirage-rs/src/chain/                     (InsightEntry state machine, pheromone field)

# Current target surface
crates/roko-serve/src/routes/aggregator.rs
crates/roko-serve/src/state.rs                (AppState, cache helpers)

# Chain side
contracts/src/InsightBoard.sol
contracts/test/InsightBoard.t.sol
crates/roko-chain/src/lib.rs                  (currently NO InsightBoard reader)
crates/roko-chain/src/marketplace.rs          (template for adding a contract reader)

# Local store side
crates/roko-neuro/src/knowledge_store.rs

# Dashboard consumers
nunchi-dashboard/src/services/mirage-api.ts    (the response shapes we must preserve)
nunchi-dashboard/src/services/mirage-knowledge.ts
nunchi-dashboard/src/pages/dashboard/KnowledgeEntries.tsx
nunchi-dashboard/src/pages/dashboard/KnowledgeGraph.tsx
demo/demo-app/src/pages/dashboard/KnowledgeEntries.tsx  (smoke test target)
```

If `nunchi-dashboard/` is not on disk, clone it; tracks `02` and `03` are
joined at the hip. Path: `/Users/will/dev/nunchi/nunchi-dashboard/`.

---

## Deliverables

### Knowledge surface (chain-backed)

1. **New module**: `crates/roko-chain/src/insight_board.rs` containing
   `InsightBoardReader` — an async reader for `InsightPosted` and
   `InsightConfirmed` events, plus `getInsight(id)` calls. Mirror the shape
   of `crates/roko-chain/src/marketplace.rs`.

2. **Aggregator changes** (`crates/roko-serve/src/routes/aggregator.rs`):
   - Replace the `KnowledgeStore::for_layout(...)` reads with
     `state.chain_reader.list_insights(...)` (new field on `AppState`).
   - Update `list_knowledge_kinds` to query the actual count from chain.
   - Cache layer: invalidate the `aggregator:knowledge:*` keys when an
     `InsightPosted` or `InsightConfirmed` event arrives via the existing
     chain event subscription (or a fresh poll if no subscription exists).
     TTL stays at 30 s on the read path.

3. **Backwards compatibility shim**: when the chain reader is absent
   (e.g. unit tests, agents running without chain access), fall back to
   the existing `KnowledgeStore`. Use a `KnowledgeSource` enum:

   ```rust
   pub enum KnowledgeSource {
       Chain(Arc<InsightBoardReader>),
       Local(roko_neuro::knowledge_store::KnowledgeStore),
       Hybrid {
           chain: Arc<InsightBoardReader>,
           local_cache: roko_neuro::knowledge_store::KnowledgeStore,
       },
   }
   ```

   `Hybrid` is the production default — chain is source of truth, local
   cache absorbs read load.

### Pheromone surface (new)

4. **New aggregator routes**:

   ```text
   GET  /api/pheromones                  — list all pheromones with intensity, decay, kind
   GET  /api/pheromones/{kind}           — list pheromones of a single kind
   GET  /api/pheromones/topology         — graph view: agents × pheromone kinds
   GET  /api/pheromones/active           — only non-decayed entries (intensity > 0)
   POST /api/pheromones/decay            — manually trigger decay tick (debug; gated by Bearer auth)
   GET  /api/pheromones/decay/{kind}     — decay schedule for a kind
   ```

   Response shapes must match `apps/mirage-rs/src/http_api/pheromone.rs` byte
   for byte for the GET routes. Diff the JSON in a regression test.

5. **Source of pheromone data**: the original mirage-rs implementation kept
   pheromones as in-memory state with a decay tick. The on-chain plan was
   "Pheromone contract"; that contract does **not** exist yet (verified —
   no `Pheromone*.sol` under `contracts/src/`). For this plan, do *not*
   wait for the contract:

   - **Phase 2a (this plan)**: Implement pheromone state inside roko-serve
     as `roko_serve::pheromone::PheromoneField` — a parking_lot::RwLock-protected
     map `BTreeMap<(AgentId, Kind), PheromoneSample>` with a 60 s decay tick.
     Persist to `.roko/pheromones.jsonl` for restart recovery (single-process
     resilience; not multi-process).
   - **Phase 2b (deferred, see `04-erc8004-chain-discovery.md`)**: when the
     pheromone contract lands, swap the storage backend behind a trait
     (`PheromoneSource`) without changing the route handlers.

6. **Decay model**: take it directly from
   `apps/mirage-rs/src/chain/pheromone.rs::Pheromone::decay`. Half-life
   per kind matches the values in `aggregator.rs::list_knowledge_kinds`
   (threat 1h, opportunity 6h, wisdom 24h). Use `f64` intensity, exponential
   decay `i' = i * exp(-ln(2) * dt / half_life)`.

7. **Tests**:
   - `crates/roko-serve/tests/pheromone_compat.rs` — boots the aggregator,
     posts a pheromone via internal API, asserts the GET response shape
     matches a captured fixture from mirage-rs.
   - `crates/roko-serve/tests/knowledge_compat.rs` — same idea against an
     anvil-backed InsightBoard contract instance.

### Cache invalidation glue

8. **Chain event subscription** in roko-serve startup:
   - On boot, if a chain reader is available, spawn a background task that
     subscribes to `InsightPosted` / `InsightConfirmed` topics on the
     configured `InsightBoard` address.
   - On each event, call `state.invalidate_cache_prefix("aggregator:knowledge:")`.
   - This is additive to the existing 30 s TTL cache. The TTL is the
     bound on staleness when subscription is unavailable.

9. **Telemetry**: emit `tracing::info!` with subscription status on boot
   (configured / fallback to TTL only / fallback to local store). The
   dashboard shouldn't crash if chain RPC is briefly unavailable.

---

## Step-by-step

### Step 1 — Capture the legacy shape (half day)

1. Run mirage-rs with `--features chain` and a small fixture:

   ```bash
   cargo run -p mirage-rs --features chain,binary -- \
     --upstream <RPC> --enable-knowledge --enable-stigmergy &
   sleep 3
   curl -s http://localhost:8545/api/knowledge/entries > tmp/ux/implementation-plans/fixtures/knowledge-entries.json
   curl -s http://localhost:8545/api/knowledge/kinds   > tmp/ux/implementation-plans/fixtures/knowledge-kinds.json
   curl -s http://localhost:8545/api/pheromones       > tmp/ux/implementation-plans/fixtures/pheromones.json
   curl -s http://localhost:8545/api/pheromones/topology > tmp/ux/implementation-plans/fixtures/pheromones-topology.json
   ```

2. Commit the fixtures. Mark them as `golden` in the test files; never
   regenerate without intent.

3. Read each fixture. Note exact field names, casing, units (Wei vs ETH,
   seconds vs ms), null-vs-empty conventions. The dashboard depends on
   these.

### Step 2 — `InsightBoardReader` in roko-chain (1 day)

Mirror `marketplace.rs`. Sketch:

```rust
// crates/roko-chain/src/insight_board.rs
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use crate::client::ChainClientBase;

sol! {
    interface IInsightBoard {
        struct Insight {
            address poster;
            bytes32 contentHash;
            string  uri;
            uint64  postedAt;
            uint64  pheromone;
        }
        function getInsight(uint256 id) external view returns (Insight memory);
        function nextInsightId() external view returns (uint256);
        event InsightPosted(uint256 indexed id, address indexed poster, bytes32 contentHash, string uri);
        event InsightConfirmed(uint256 indexed id, address indexed confirmer, uint64 pheromone);
    }
}

pub struct InsightBoardReader { /* address + alloy provider */ }

impl InsightBoardReader {
    pub async fn list_insights(&self) -> Result<Vec<InsightView>>;
    pub async fn get_insight(&self, id: U256) -> Result<InsightView>;
    pub async fn count(&self) -> Result<u64>;
    pub fn subscribe_events(self: Arc<Self>) -> mpsc::Receiver<InsightEvent>;
}
```

`InsightView` is a JSON-friendly shape that matches the legacy fixture from
Step 1. Conversion from on-chain types happens in this crate, not in the
aggregator.

Add it to `roko_chain::lib.rs` exports.

### Step 3 — Wire the reader into AppState (half day)

`crates/roko-serve/src/state.rs`:

```rust
pub struct AppState {
    // ... existing fields ...
    pub knowledge_source: KnowledgeSource,
    pub pheromone_field: Arc<PheromoneField>,
}
```

Constructor (`AppState::new`) reads `roko.toml`'s `[chain]` section. If a
chain RPC + InsightBoard address are present, build `Hybrid`. Otherwise
`Local`. The `Local` variant is the test/dev fallback.

### Step 4 — Replace the knowledge handlers (half day)

In `crates/roko-serve/src/routes/aggregator.rs`:

- `list_knowledge_entries`: dispatch on `state.knowledge_source`.
- `list_knowledge_edges`: same.
- `search_knowledge`: same.
- `list_knowledge_kinds`: count by querying chain `nextInsightId`-style
  counters or summing over `KnowledgeStore` for `Local`.

Keep the cache key prefix `aggregator:knowledge:`. After this step the
fixtures from Step 1 should match against the new endpoint output (run
`scripts/diff-knowledge-fixtures.sh` — write it inline, ~10 lines using
`jq`).

### Step 5 — Pheromone field module (1 day)

Create `crates/roko-serve/src/pheromone.rs`:

```rust
use std::time::{Duration, Instant};
use std::sync::Arc;

use parking_lot::RwLock;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum PheromoneKind { Threat, Opportunity, Wisdom }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PheromoneSample {
    pub agent_id: String,
    pub kind: PheromoneKind,
    pub intensity: f64,
    pub last_update: chrono::DateTime<chrono::Utc>,
}

pub struct PheromoneField {
    inner: RwLock<std::collections::BTreeMap<(String, PheromoneKind), PheromoneSample>>,
    persist_path: std::path::PathBuf,
}

impl PheromoneField {
    pub fn load_or_new(path: impl Into<std::path::PathBuf>) -> Self;
    pub fn list_all(&self) -> Vec<PheromoneSample>;
    pub fn list_by_kind(&self, kind: PheromoneKind) -> Vec<PheromoneSample>;
    pub fn list_active(&self) -> Vec<PheromoneSample>; // intensity > epsilon
    pub fn topology(&self) -> serde_json::Value;       // matches mirage shape
    pub fn deposit(&self, sample: PheromoneSample);
    pub fn decay_tick(&self);
    pub fn half_life(kind: PheromoneKind) -> Duration;
}

pub fn spawn_decay_loop(field: Arc<PheromoneField>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop { tick.tick().await; field.decay_tick(); }
    })
}
```

Persist `(agent_id, kind, intensity, last_update)` records as JSONL to
`.roko/pheromones.jsonl`. On boot, replay the file and apply elapsed-time
decay since `last_update`. Cap the file at 50 MB; if exceeded, write a
`.compacted` snapshot replacing tail-only entries.

### Step 6 — Pheromone routes (half day)

In `aggregator.rs::routes()`:

```rust
.route("/pheromones", get(list_pheromones))
.route("/pheromones/{kind}", get(list_pheromones_by_kind))
.route("/pheromones/topology", get(pheromone_topology))
.route("/pheromones/active", get(list_active_pheromones))
.route("/pheromones/decay", post(trigger_decay))      // bearer-protected
.route("/pheromones/decay/{kind}", get(decay_schedule))
```

Each handler fetches from `state.pheromone_field` and serialises to match
the captured fixture from Step 1. Reject requests with unknown `kind`
values (return 400 with the legacy error shape, e.g.
`{ "error": "unknown pheromone kind" }`).

### Step 7 — Cache invalidation on chain events (half day)

In `crates/roko-serve/src/lib.rs::serve(...)` startup, if
`state.knowledge_source` is `Chain` or `Hybrid`:

```rust
let reader = state.chain_insight_reader();
let cache = state.cache_handle();
tokio::spawn(async move {
    let mut events = reader.subscribe_events();
    while let Some(_event) = events.recv().await {
        cache.invalidate_prefix("aggregator:knowledge:").await;
    }
});
```

If `subscribe_events` errors (no WS endpoint), log once and rely on TTL.

### Step 8 — Tests (1 day)

1. `crates/roko-serve/tests/pheromone_compat.rs`:

   ```rust
   #[tokio::test]
   async fn pheromone_routes_match_legacy_shape() {
       let (state, _tmp) = test_state_with_pheromones().await;
       state.pheromone_field.deposit(PheromoneSample {
           agent_id: "agent-1".into(), kind: PheromoneKind::Threat,
           intensity: 0.8, last_update: chrono::Utc::now(),
       });
       let app = build_router(state);
       let body = call_get(&app, "/api/pheromones").await;
       let golden: serde_json::Value =
           serde_json::from_str(include_str!("../../tmp/ux/implementation-plans/fixtures/pheromones.json")).unwrap();
       assert_same_keys(&body, &golden);
   }
   ```

   `assert_same_keys` is a structural diff (keys + types match, values may
   differ). Don't compare timestamps directly.

2. `crates/roko-serve/tests/knowledge_compat.rs` — same shape, but boots
   anvil with `forge script Deploy.s.sol`, posts an insight via cast, and
   asserts the aggregator surfaces it within a 35 s window (TTL + slack).

3. Update `crates/roko-serve/tests/api_integration.rs` to include the new
   routes in its happy-path sweep.

### Step 9 — OpenAPI + status update (2 hrs)

- Add the new routes to `crates/roko-serve/src/openapi.rs` under tags
  `knowledge` (already exists) and a new `pheromones` tag.
- Update `crates/roko-serve/src/openapi.rs` route count test
  (`api_integration.rs`).
- Update CLAUDE.md "Status table" to reflect that `/api/pheromones` and
  `/api/knowledge` are chain-backed.

### Step 10 — Coordinate with track `03` (30 min)

Before merging, post in the dashboard channel: "the aggregator now
implements `/api/pheromones/*` and chain-backed `/api/knowledge/*`. Track
`03` (URL swap) is unblocked."

---

## Anti-patterns to avoid

- **Don't reach into mirage-rs from roko-serve to read pheromone state.**
  mirage-rs is being deleted; importing from it freezes the deletion.
- **Don't introduce a new HTTP client to talk to mirage's chain
  subsystem.** Talk to the chain RPC directly via `roko-chain`. This is
  the whole point of the extraction.
- **Don't make `/pheromones/decay` a public POST.** It's a debug helper
  and must be bearer-token protected (via the existing
  `routes/middleware.rs` auth layer). Without it, anyone on the network
  can wipe pheromones.
- **Don't skip the regression fixtures.** The compat layer's whole job is
  shape preservation. A test that "the route returns 200" misses 90 % of
  the bugs that would hit Sam's dashboard.
- **Don't unify knowledge and pheromones into one struct just because the
  contract bundles them.** Pheromones live on roko-serve in this phase;
  knowledge lives on chain. Mixing the storage layers couples
  the rollout.
- **Don't use a 1 s decay tick.** That's a 60× CPU multiplier for ~no UX
  benefit. 60 s matches the user-visible UX, persists to disk reasonably,
  and matches the mirage-rs cadence.
- **Don't forget the `serde(rename_all = "camelCase")` attribute on new
  serializable types** if the legacy shape uses camelCase. Compare against
  the captured fixture.

## Done when

1. `curl http://localhost:6677/api/pheromones` returns the same shape as
   the captured fixture (Step 1).
2. `curl http://localhost:6677/api/knowledge/entries` returns the same
   shape and includes any insight posted to InsightBoard within the last
   30 s.
3. `curl http://localhost:6677/api/knowledge/kinds` returns non-zero
   counts that match `cast call <InsightBoard> 'nextInsightId()' --rpc-url $RPC`.
4. `cargo test -p roko-serve --test pheromone_compat` passes.
5. `cargo test -p roko-serve --test knowledge_compat` passes (with
   `--ignored` if the anvil step is heavy; gate on `CI` env var).
6. `nunchi-dashboard/src/pages/dashboard/KnowledgeEntries.tsx` and
   `KnowledgeGraph.tsx` render in the demo when pointed at roko-serve.
7. `tmp/ux/04-dashboard-migration.md` "Still placeholder or deferred" list
   no longer mentions pheromone or knowledge.
8. CLAUDE.md aggregator row updated.
