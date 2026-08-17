# M110 — Pheromone Pulse Types on Bus

## Objective
Define pheromone Pulse types that encode indirect coordination signals (success markers, difficulty warnings, resource claims, progress breadcrumbs) as first-class Pulse variants on the Bus. This eliminates the need for separate pheromone infrastructure by using the existing Bus publish/subscribe and demurrage-driven evaporation. Pheromones become Pulses with `Kind::Pheromone` and pheromone-specific metadata in tags.

**Note**: A `PheromoneKind` enum already exists in `roko-orchestrator/src/coordination.rs` with high-level categories (Threat, Opportunity, Wisdom, Alpha, Pattern, Anomaly, Consensus, Custom). This batch defines **Bus-level pheromone payload types** in `roko-core` that are more granular and designed for stigmergic routing, complementing the orchestrator-level enum.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/pulse.rs` (Pulse struct), `crates/roko-core/src/kind.rs` (Kind enum -- already has `Kind::Pheromone`), new file `crates/roko-core/src/pheromone.rs`
- Phase ref: depth doc 11-memory/11-stigmergy-as-bus.md
- Depth doc: `tmp/unified-depth/11-memory/11-stigmergy-as-bus.md`

## Steps
1. Discover existing Pulse, Kind, and pheromone types:
   ```bash
   grep -n 'pub struct Pulse' crates/roko-core/src/pulse.rs | head -5
   grep -A 10 'pub struct Pulse {' crates/roko-core/src/pulse.rs
   grep -n 'Pheromone' crates/roko-core/src/kind.rs | head -5
   grep -n 'pub struct Topic' crates/roko-core/src/pulse.rs | head -3
   grep -rn 'pub enum PheromoneKind' crates/ --include='*.rs' | head -5
   ```

2. **Existing Pulse struct** (in `crates/roko-core/src/pulse.rs`):
   ```rust
   pub struct Pulse {
       pub seq: u64,
       pub topic: Topic,                     // Topic(String) newtype
       pub kind: Kind,                       // from kind.rs, has Kind::Pheromone
       pub body: Body,                       // payload
       pub created_at_ms: i64,
       pub tags: BTreeMap<String, String>,    // metadata
   }
   ```

3. **Existing PheromoneKind** in orchestrator (in `crates/roko-orchestrator/src/coordination.rs`):
   ```rust
   pub enum PheromoneKind { Threat, Opportunity, Wisdom, Alpha, Pattern, Anomaly, Consensus, Custom(String) }
   ```
   This is high-level. The new types below are more granular Bus payloads.

4. Create `crates/roko-core/src/pheromone.rs`:
   ```rust
   use crate::{Body, Kind, Topic, Pulse};
   use serde::{Deserialize, Serialize};

   /// Pheromone payload types for stigmergic coordination on the Bus.
   /// Each variant is serialized into a Pulse body with Kind::Pheromone.
   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
   #[serde(tag = "pheromone_type")]
   pub enum PheromonePulse {
       /// Success marker: "this approach worked here"
       Success {
           task_type: String,
           approach: String,
           confidence: f64,
       },
       /// Difficulty warning: "this area is hard, proceed with caution"
       Difficulty {
           area: String,
           severity: f64,
           failure_count: usize,
       },
       /// Resource claim: "I am working on this, others should avoid"
       Claim {
           resource: String,
           agent_id: String,
           ttl_seconds: u64,
       },
       /// Progress breadcrumb: "I passed through here"
       Breadcrumb {
           location: String,
           direction: String,
           progress: f64,
       },
       /// Attraction: "something interesting here"
       Attraction {
           target: String,
           intensity: f64,
       },
       /// Repulsion: "avoid this area"
       Repulsion {
           target: String,
           intensity: f64,
           reason: String,
       },
   }
   ```

5. Add helper constructors for creating Pulses:
   ```rust
   /// Default pheromone evaporation time (Bus ring buffer TTL)
   pub const PHEROMONE_DEFAULT_TTL_SECS: u64 = 3600; // 1 hour
   /// Topic prefix for all pheromone Pulses
   pub const PHEROMONE_TOPIC_PREFIX: &str = "pheromone/";

   impl PheromonePulse {
       /// Create a Pulse for the Bus from this pheromone.
       pub fn to_pulse(&self, seq: u64, source_agent: &str) -> Result<Pulse> {
           let topic = Topic::new(format!("{}{}", PHEROMONE_TOPIC_PREFIX, self.variant_name()));
           let body = Body::from_json(self)?;   // Note: Body::from_json, not Body::json
           let mut pulse = Pulse::new(seq, topic, Kind::Pheromone, body);
           pulse.tags.insert("source_agent".into(), source_agent.into());
           Ok(pulse)
       }

       /// Parse a pheromone from a Bus Pulse.
       pub fn from_pulse(pulse: &Pulse) -> Option<Self> {
           if pulse.kind != Kind::Pheromone { return None; }
           pulse.body.as_json::<Self>().ok()
       }

       /// Variant name for topic construction.
       pub fn variant_name(&self) -> &'static str {
           match self {
               Self::Success { .. } => "success",
               Self::Difficulty { .. } => "difficulty",
               Self::Claim { .. } => "claim",
               Self::Breadcrumb { .. } => "breadcrumb",
               Self::Attraction { .. } => "attraction",
               Self::Repulsion { .. } => "repulsion",
           }
       }
   }
   ```

6. Register in `crates/roko-core/src/lib.rs`:
   ```rust
   pub mod pheromone;
   pub use pheromone::PheromonePulse;
   ```

7. Write tests:
   - Each PheromonePulse variant round-trips through serde JSON
   - `to_pulse` / `from_pulse` round-trips correctly
   - Topic strings are correctly prefixed with `pheromone/`
   - `from_pulse` returns `None` for non-Pheromone kind pulses

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- pheromone
```

## What NOT to do
- Do NOT modify the existing `PheromoneKind` in `roko-orchestrator/src/coordination.rs` -- that serves a different purpose
- Do NOT implement the Bus subscription or eviction logic -- that already exists
- Do NOT create a separate pheromone store -- pheromones live on the Bus only
- Do NOT implement the stigmergic Route Cell -- that is M111
- Do NOT add roko-runtime as a dependency of roko-core -- pheromone types are data-only
