# M051 — StateHub Projection Types

## Objective
Define StateHub projection types: FlowSummary, AgentStatus, SpaceStatus, MemoryStats, RouteStats, VerifyStats, CostSummary. Each is a versioned struct with TTL. StateHub computes projections from Lens outputs and caches them. Projections are the typed data contract between the system internals and all surfaces (TUI, HTTP, WebSocket). Surfaces never read raw data -- they consume projections.

## Scope
- Crates: `roko-conductor`
- Files: `crates/roko-conductor/src/statehub.rs` (new), `crates/roko-conductor/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.7
- Spec ref: `tmp/unified/09-TELEMETRY.md` SS3 (StateHub)

## Steps
1. Check for existing StateHub or projection types:
   ```bash
   grep -rn 'StateHub\|statehub\|Projection\|FlowSummary\|AgentStatus' crates/roko-conductor/src/ --include='*.rs' | head -15
   grep -rn 'StateHub\|statehub' crates/roko-core/src/ --include='*.rs' | head -10
   ```

2. Define projection types in `crates/roko-conductor/src/statehub.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FlowSummary {
       pub run_id: String,
       pub graph_name: String,
       pub progress_pct: f64,
       pub cost_usd: f64,
       pub elapsed: Duration,
       pub status: String,
       pub active_nodes: Vec<String>,
       pub version: u64,
       pub updated_at: DateTime<Utc>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AgentStatus { /* id, name, state, vitality, slots, current_tasks */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct SpaceStatus { /* id, agents_active, flows_active, budget_remaining */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct MemoryStats { /* total_signals, by_tier, by_kind, decay_rate */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct RouteStats { /* model_distribution, cost_by_model, requests_by_model */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct VerifyStats { /* pass_rate, threshold_drift, by_rung */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CostSummary { /* total_usd, by_provider, by_agent, budget_remaining */ }
   ```

3. Define the StateHub struct:
   ```rust
   pub struct StateHub {
       projections: DashMap<String, CachedProjection>,
       ttl: Duration,
   }

   struct CachedProjection {
       data: serde_json::Value,
       version: u64,
       computed_at: Instant,
       ttl: Duration,
   }
   ```

4. Implement StateHub methods:
   - `get<T: Projection>(key: &str) -> Option<T>` -- return cached projection if fresh
   - `update<T: Projection>(key: &str, projection: T)` -- update cache, increment version
   - `subscribe(key: &str) -> Receiver<ProjectionUpdate>` -- for live push to surfaces
   - `all_projections() -> Vec<(String, serde_json::Value)>` -- dump all for HTTP endpoints

5. Define a `Projection` trait:
   ```rust
   pub trait Projection: Serialize + DeserializeOwned + Clone + Send + Sync {
       fn projection_key() -> &'static str;
       fn ttl() -> Duration;
   }
   ```

6. Implement `Projection` for each projection type.

7. Write tests:
   - StateHub produces all 7 projection types
   - Cached projection expires after TTL
   - Update increments version
   - Subscribe receives updates

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- statehub
```

## What NOT to do
- Do NOT wire StateHub into TUI/HTTP yet -- that is M052
- Do NOT make StateHub depend on specific Lens implementations -- it consumes generic projection data
- Do NOT add database backing -- in-memory DashMap with TTL is sufficient
- Do NOT add complex aggregation logic -- projections are pre-computed by Lenses and stored as-is
