# M053 — Five Named Surface Protocol Contracts

## Objective
Define the protocol contracts for all 5 named surfaces: Workbench (task delegation), Agent Inbox (ambient notification), Generative Canvas (visual Graph editor), Stigmergy Minimap (coordination overview), and Autonomy Slider (progressive trust). Each surface contract declares: projections consumed (from StateHub), events emitted (user actions), and invariants (behavioral guarantees). These contracts are the formal interface between system and user -- any rendering target (TUI, web, CLI) implements the same contracts.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/surfaces.rs` (new), `crates/roko-core/src/lib.rs`
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.8
- Spec ref: `tmp/unified/16-SURFACES.md` SS2-6

## Steps
1. Check for any existing surface-related code:
   ```bash
   grep -rn 'Surface\|Workbench\|Inbox\|Canvas\|Minimap\|AutonomySlider' crates/roko-core/src/ --include='*.rs' | head -10
   grep -rn 'Surface\|surface' crates/roko-cli/src/tui/ --include='*.rs' | head -10
   ```

2. Define the surface event types in `crates/roko-core/src/surfaces.rs`:
   ```rust
   /// Events that surfaces can emit back to the system.
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum SurfaceEvent {
       // Workbench events
       TaskAssign { graph: String, inputs: Value, budget: Option<f64> },
       SlotFill { agent_id: String, slot_index: usize, cell_ref: String },
       MacroAdjust { run_id: String, macro_name: String, new_value: Value },
       FlowCancel { run_id: String },
       FlowPause { run_id: String },
       FlowResume { run_id: String },
       HumanRespond { run_id: String, node_id: String, response: String },

       // Inbox events
       NotificationRead { notification_id: String },
       NotificationAct { notification_id: String, action: String },
       NotificationArchive { notification_id: String },

       // Autonomy events
       AutonomyLevelChange { capability: String, new_level: u8 },

       // Canvas events
       GraphEdit { graph_id: String, edit: GraphEditOp },
   }
   ```

3. Define the surface contract trait:
   ```rust
   pub trait SurfaceContract {
       /// Name of this surface.
       fn name(&self) -> &str;
       /// Which StateHub projections this surface consumes.
       fn projections_consumed(&self) -> &[&str];
       /// Which events this surface can emit.
       fn events_emitted(&self) -> &[&str];
       /// Invariants (human-readable descriptions).
       fn invariants(&self) -> &[&str];
   }
   ```

4. Implement contracts for each surface:
   - **Workbench**: consumes FlowSummary, AgentStatus, GraphSummary; emits TaskAssign, SlotFill, MacroAdjust, FlowCancel/Pause/Resume, HumanRespond
   - **Agent Inbox**: consumes notifications (Critical, Urgent, Notice); emits NotificationRead, NotificationAct, NotificationArchive
   - **Generative Canvas**: consumes GraphSummary; emits GraphEdit
   - **Stigmergy Minimap**: consumes AgentStatus, FlowSummary, PheromoneData
   - **Autonomy Slider**: consumes AutonomyConfig; emits AutonomyLevelChange

5. Define notification types for Agent Inbox:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Notification {
       pub id: String,
       pub urgency: Urgency,
       pub title: String,
       pub body: String,
       pub source: String,
       pub created_at: DateTime<Utc>,
       pub state: NotificationState,
   }

   pub enum Urgency { Critical, Urgent, Notice }
   pub enum NotificationState { Created, Read, Acted, Archived }
   ```

6. Export all types from lib.rs.

7. Write tests:
   - All 5 surface contracts compile and implement SurfaceContract
   - Each surface declares at least one projection and one event
   - SurfaceEvent serializes and deserializes correctly

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- surfaces
```

## What NOT to do
- Do NOT implement rendering here -- contracts define the interface, not the UI
- Do NOT add TUI widgets -- those are M054/M055/M056
- Do NOT add HTTP handlers -- those are separate integration tasks
- Do NOT couple contracts to specific TUI frameworks (ratatui)
