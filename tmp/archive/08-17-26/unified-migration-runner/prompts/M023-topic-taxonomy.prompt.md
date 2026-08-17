# M023 — Define topic taxonomy constants

## Objective
Define the canonical set of Bus topic strings as constants or a module of constants in roko-core. The unified spec (`tmp/unified/01-SIGNAL.md` §3.2) specifies hierarchical topic namespaces for orchestration, prediction, outcome, calibration, extension, agent, knowledge, and dream events. Currently topics are ad-hoc strings scattered across crates.

## Scope
- Crates: `roko-core`
- Files:
  - Create or populate `crates/roko-core/src/topics.rs` (new if not present, or add to `pulse.rs`)
  - `crates/roko-core/src/lib.rs` (export)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.2
- Spec ref: `tmp/unified/01-SIGNAL.md` §3.2 (Topic Taxonomy)

## Steps
1. Check if a topics module already exists:
   ```bash
   grep -rn 'topics\|TOPIC_' crates/roko-core/src/ --include='*.rs' | head -20
   ls crates/roko-core/src/topics.rs 2>/dev/null
   ```

2. Inventory existing ad-hoc topic strings:
   ```bash
   grep -rn 'Topic::new(' crates/ --include='*.rs' | grep -v target/ | head -30
   ```

3. Create `crates/roko-core/src/topics.rs` with the canonical topic hierarchy:
   ```rust
   //! Canonical topic taxonomy for Bus [`Pulse`]s.
   //!
   //! Topics follow a dotted hierarchy. Constants defined here are the
   //! official names — prefer these over ad-hoc strings.
   //!
   //! See: tmp/unified/01-SIGNAL.md §3.2

   // ─── Orchestration ──────────────────────────────────────────────────
   pub const FLOW_STARTED: &str = "orchestration.flow.started";
   pub const FLOW_COMPLETED: &str = "orchestration.flow.completed";
   pub const FLOW_FAILED: &str = "orchestration.flow.failed";
   pub const NODE_STARTED: &str = "orchestration.node.started";
   pub const NODE_COMPLETED: &str = "orchestration.node.completed";
   pub const NODE_FAILED: &str = "orchestration.node.failed";
   pub const NODE_RETRYING: &str = "orchestration.node.retrying";

   // ─── Prediction ─────────────────────────────────────────────────────
   pub const PREDICTION_SCORE: &str = "prediction.score";
   pub const PREDICTION_ROUTE: &str = "prediction.route";
   pub const PREDICTION_COMPOSE: &str = "prediction.compose";

   // ─── Outcome ────────────────────────────────────────────────────────
   pub const OUTCOME_SCORE: &str = "outcome.score";
   pub const OUTCOME_ROUTE: &str = "outcome.route";
   pub const OUTCOME_COMPOSE: &str = "outcome.compose";
   pub const OUTCOME_VERIFY: &str = "outcome.verify";

   // ─── Calibration ────────────────────────────────────────────────────
   pub const CALIBRATION_UPDATED: &str = "calibration.updated";
   pub const CALIBRATION_HEURISTIC: &str = "calibration.heuristic";

   // ─── Agent ──────────────────────────────────────────────────────────
   pub const AGENT_STARTED: &str = "agent.lifecycle.started";
   pub const AGENT_STOPPED: &str = "agent.lifecycle.stopped";
   pub const AGENT_HEARTBEAT: &str = "agent.heartbeat";
   pub const AGENT_ERROR: &str = "agent.error";

   // ─── Extension ──────────────────────────────────────────────────────
   pub const EXTENSION_LOADED: &str = "extension.loaded";
   pub const EXTENSION_UNLOADED: &str = "extension.unloaded";
   pub const EXTENSION_HOOK_FIRED: &str = "extension.hook.fired";

   // ─── Knowledge ──────────────────────────────────────────────────────
   pub const KNOWLEDGE_STORED: &str = "knowledge.stored";
   pub const KNOWLEDGE_RETRIEVED: &str = "knowledge.retrieved";
   pub const KNOWLEDGE_DECAYED: &str = "knowledge.decayed";
   pub const KNOWLEDGE_PROMOTED: &str = "knowledge.tier.promoted";

   // ─── Dream ──────────────────────────────────────────────────────────
   pub const DREAM_CYCLE_STARTED: &str = "dream.cycle.started";
   pub const DREAM_CYCLE_COMPLETED: &str = "dream.cycle.completed";
   pub const DREAM_INSIGHT: &str = "dream.insight.generated";

   // ─── Trigger ────────────────────────────────────────────────────────
   pub const TRIGGER_FIRED: &str = "trigger.fired";
   pub const TRIGGER_ARMED: &str = "trigger.armed";
   pub const TRIGGER_DISARMED: &str = "trigger.disarmed";
   ```

4. Add `pub mod topics;` to `crates/roko-core/src/lib.rs`.

5. Add a test that verifies all topic constants follow the dotted-hierarchy convention (no leading/trailing dots, at least one dot separator).

6. Optionally, add a helper to construct parameterized topics:
   ```rust
   /// Build a parameterized topic: `topics::prediction_score("my-cell-id")`
   /// produces `"prediction.score.my-cell-id"`.
   pub fn prediction_score(cell_id: &str) -> String {
       format!("{PREDICTION_SCORE}.{cell_id}")
   }
   ```

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- topics
# Confirm topics are exported:
grep 'topics' crates/roko-core/src/lib.rs
```

## What NOT to do
- Do NOT replace existing ad-hoc Topic::new() calls across other crates yet — that's a follow-up migration
- Do NOT use an enum for topics — string constants are more extensible and match the spec
- Do NOT add runtime validation that topics must be from this list — the taxonomy is advisory
- Do NOT make this module depend on any other crate — pure constants only
