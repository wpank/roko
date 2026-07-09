# M111 — Stigmergic Route Cell

## Objective
Implement a Route Cell that uses pheromone Pulses from the Bus to influence routing decisions. When routing tasks or knowledge queries, the stigmergic router checks for pheromone signals (success markers, difficulty warnings, claims) and adjusts routing scores accordingly. This enables indirect coordination where agents influence each other's behavior through environmental traces rather than direct messages.

## Scope
- Crates: `roko-learn`, `roko-core`
- Files: `crates/roko-core/src/pheromone.rs` (from M110, defines `PheromonePulse`), new file `crates/roko-learn/src/stigmergic_router.rs`
- Phase ref: depth doc 11-memory/11-stigmergy-as-bus.md
- Depth doc: `tmp/unified-depth/11-memory/11-stigmergy-as-bus.md`

## Steps
1. Discover existing routing code and pheromone types:
   ```bash
   grep -rn 'pub struct CascadeRouter' crates/roko-learn/src/ --include='*.rs' | head -5
   grep -rn 'pub trait.*Route' crates/roko-core/src/ --include='*.rs' | head -5
   grep -n 'pub enum PheromonePulse\|pub enum PheromoneKind' crates/roko-core/src/pheromone.rs 2>/dev/null || echo "M110 not yet applied"
   grep -n 'pub enum PheromoneKind' crates/roko-orchestrator/src/coordination.rs | head -3
   grep 'roko-core' crates/roko-learn/Cargo.toml | head -3
   ```
   **Note**: M110 creates `PheromonePulse` in `roko-core/src/pheromone.rs` (not `PheromoneKind` -- that name is already taken by `roko-orchestrator/src/coordination.rs`).

2. Create `crates/roko-learn/src/stigmergic_router.rs`:
   ```rust
   /// A routing modifier that reads pheromone Pulses from the Bus
   /// and adjusts routing scores for task/model selection.
   pub struct StigmergicRouter {
       /// Weight for success pheromone influence (0.0-1.0)
       pub success_weight: f64,
       /// Weight for difficulty pheromone influence (0.0-1.0)
       pub difficulty_weight: f64,
       /// Weight for claim pheromone (avoid claimed resources)
       pub claim_weight: f64,
       /// Decay factor for pheromone age (older pheromones have less influence)
       pub age_decay_rate: f64,
   }
   ```

3. Implement pheromone collection:
   ```rust
   impl StigmergicRouter {
       /// Collect active pheromones relevant to a routing decision.
       /// Input: `PheromonePulse` values from M110 (or raw `Pulse` data decoded via `PheromonePulse::from_pulse`).
       pub fn collect_pheromones(
           &self,
           context: &RoutingContext,
           pheromones: &[PheromonePulse],
       ) -> PheromoneSnapshot {
           // Filter pheromones by relevance to the routing context
           // Apply age decay
           // Group by kind
       }
   }

   /// Snapshot of pheromone state relevant to a routing decision.
   #[derive(Debug, Clone)]
   pub struct PheromoneSnapshot {
       pub success_signals: Vec<(String, f64)>,  // (approach, decayed_confidence)
       pub difficulty_signals: Vec<(String, f64)>,  // (area, decayed_severity)
       pub claims: Vec<(String, String)>,  // (resource, agent_id)
       pub attractions: Vec<(String, f64)>,  // (target, intensity)
       pub repulsions: Vec<(String, f64)>,  // (target, intensity)
   }
   ```

4. Implement score adjustment:
   ```rust
   impl StigmergicRouter {
       /// Adjust routing scores based on pheromone signals.
       /// Returns a map of route_id -> score_adjustment.
       pub fn adjust_scores(
           &self,
           snapshot: &PheromoneSnapshot,
           candidates: &[RouteCandidate],
       ) -> Vec<(String, f64)> {
           candidates.iter().map(|c| {
               let mut adjustment = 0.0;
               // Boost routes with matching success pheromones
               // Penalize routes in difficult areas
               // Block routes to claimed resources
               // Attract toward interesting targets
               // Repel from dead ends
               (c.id.clone(), adjustment)
           }).collect()
       }
   }

   /// A candidate route to be scored.
   #[derive(Debug, Clone)]
   pub struct RouteCandidate {
       pub id: String,
       pub base_score: f64,
       pub tags: Vec<String>,
   }
   ```

5. Register in `crates/roko-learn/src/lib.rs`:
   ```rust
   pub mod stigmergic_router;
   ```

6. Write tests:
   - Success pheromone boosts matching route scores
   - Difficulty pheromone reduces route scores
   - Claimed resources are penalized
   - Old pheromones have less influence (age decay)
   - No pheromones results in zero adjustment
   - Repulsion signals correctly penalize routes

## Verification
```bash
cargo check -p roko-learn -p roko-core
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- stigmergic_router
```

## What NOT to do
- Do NOT modify the existing CascadeRouter -- this is a separate routing modifier
- Do NOT implement Bus subscription logic -- accept pheromones as input
- Do NOT implement pheromone deposit (publishing to Bus) -- that happens at the call site
- Do NOT add async to the router -- keep it synchronous
- Do NOT implement the graduation bridge (Bus -> Store for durable pheromones) -- that is separate
