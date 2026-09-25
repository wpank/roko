# Depth: Surface Events

> Parent: [22-SURFACES](../../22-SURFACES.md) SS4

This file documents the `SurfaceEvent` command model, its transport paths through
the system, and the separation between user-emitted commands and engine-emitted
runtime events.

---

## 1. Design: Commands vs. Events

Roko distinguishes two event families:

| Family | Type | Direction | Authorization |
|---|---|---|---|
| **RuntimeEvent** | Engine output | System -> Surface | Read-only; surfaces consume |
| **SurfaceEvent** | User command | Surface -> System | Must be authorized before effect |

`SurfaceEvent` values are kept in a separate enum from `RuntimeEvent` so that any
transport layer (HTTP, TUI broadcast, WebSocket) can authorize commands before
translating them into system effects. A surface never directly mutates system state;
it emits a `SurfaceEvent` that the runtime interprets.

---

## 2. The SurfaceEvent Enum

Defined in `crates/roko-core/src/runtime_event.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum SurfaceEvent {
    TaskAssign {
        graph: String,
        inputs: serde_json::Value,
        budget: Option<f64>,
        deadline: Option<DateTime<Utc>>,
    },
    SlotFill {
        agent_id: String,
        slot_index: usize,
        cell_ref: String,
    },
    MacroAdjust {
        run_id: String,
        macro_name: String,
        new_value: serde_json::Value,
    },
    FlowCancel { run_id: String },
    FlowPause { run_id: String },
    FlowResume { run_id: String },
    HumanRespond {
        run_id: String,
        cell_id: String,
        response: serde_json::Value,
    },
}
```

### Variant semantics

| Variant | Emitting Surface | System Effect |
|---|---|---|
| `TaskAssign` | Workbench | Start a new Graph execution with the given inputs and budget |
| `SlotFill` | Workbench | Assign a Cell to an agent's Slot for execution |
| `MacroAdjust` | Workbench, Canvas | Change a Macro parameter on a live Flow |
| `FlowCancel` | Workbench | Terminate a running Flow |
| `FlowPause` | Workbench | Pause a running Flow |
| `FlowResume` | Workbench | Resume a paused Flow |
| `HumanRespond` | Workbench, Inbox | Answer a Cell's human-input prompt |

---

## 3. HTTP Transport

### POST /api/surface-events

The HTTP ingress is in `crates/roko-serve/src/routes/run.rs`:

```rust
async fn handle_surface_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<SurfaceEvent>,
) -> Result<Json<Value>, ApiError> {
    match &event {
        SurfaceEvent::FlowCancel { run_id } | SurfaceEvent::FlowPause { run_id } => {
            // Log the action and return acknowledgment
        }
        SurfaceEvent::HumanRespond { run_id, cell_id, .. } => {
            // Log the response and return acknowledgment
        }
        _ => {
            // Log and acknowledge other events
        }
    }
}
```

**Current status**: The HTTP handler logs events and returns acknowledgments but does
not execute them as system commands. This is an explicit product residual -- the
transport is wired but the command consumer that would translate events into engine
actions is not yet connected.

### Request format

```json
POST /api/surface-events
Content-Type: application/json

{
  "kind": "flow_cancel",
  "data": {
    "run_id": "wf_01HGZK7B..."
  }
}
```

The serde tag format (`"kind"` + `"data"`) means the variant name is snake_case
(e.g., `"flow_cancel"`, `"human_respond"`, `"task_assign"`).

---

## 4. TUI Transport

The ratatui application uses a `tokio::sync::broadcast` channel for SurfaceEvent
distribution, defined in `crates/roko-cli/src/tui/app/mod.rs`:

```rust
// Field on the TUI App struct
surface_event_tx: tokio::sync::broadcast::Sender<SurfaceEvent>,

// Emit from a TUI action
fn emit_surface_event(&self, event: SurfaceEvent) {
    let _ = self.surface_event_tx.send(event);
}

// Subscribe from a runner or consumer
pub fn subscribe_surface_events(&self) -> broadcast::Receiver<SurfaceEvent> {
    self.surface_event_tx.subscribe()
}
```

### TUI actions that emit SurfaceEvents

The TUI approval dialog (`crates/roko-cli/src/tui/app/actions.rs`) emits
`HumanRespond` events when the user approves, rejects, or defers an inbox item:

```rust
// Approve action
self.emit_surface_event(SurfaceEvent::HumanRespond {
    run_id: run_id.clone(),
    cell_id: cell_id.clone(),
    response: json!({ "action": "approve" }),
});

// Reject action
self.emit_surface_event(SurfaceEvent::HumanRespond {
    run_id: run_id.clone(),
    cell_id: cell_id.clone(),
    response: json!({ "action": "reject", "reason": reason }),
});

// Defer action
self.emit_surface_event(SurfaceEvent::HumanRespond {
    run_id: run_id.clone(),
    cell_id: cell_id.clone(),
    response: json!({ "action": "defer", "until": defer_until }),
});
```

---

## 5. DashboardEvent (Engine -> Surface)

In the opposite direction, the engine emits `DashboardEvent` values that surfaces
consume. These are defined in `crates/roko-core/src/dashboard_snapshot.rs` and
include 39 production variants covering plan lifecycle, task lifecycle, agent
lifecycle, gate results, efficiency telemetry, inbox operations, and system events.

Key variants relevant to surface rendering:

| DashboardEvent Variant | Surfaces Affected |
|---|---|
| `PlanStarted` / `PlanCompleted` | Workbench, Canvas |
| `TaskStarted` / `TaskCompleted` / `TaskPhaseChanged` | Workbench, Canvas |
| `AgentSpawned` / `AgentCompleted` | Workbench, Minimap, Autonomy |
| `GateResult` | Workbench, Canvas |
| `EfficiencyEvent` | Workbench, Inbox, Minimap |
| `InboxItemReceived` | Inbox |
| `InboxApprove` / `InboxReject` / `InboxDefer` / `InboxDismiss` | Inbox |
| `CFactorTrendUpdated` | Minimap, Autonomy |
| `ProjectionUpdated` | Routed to surfaces based on projection ID matching |

The `projection_accepts_event()` function in `projection_contract.rs` implements
the event-to-surface routing logic, ensuring each SSE stream only receives events
relevant to its surface.

---

## 6. Inbox Events (Specialized Surface Events)

Inbox operations have dedicated DashboardEvent variants rather than flowing through
SurfaceEvent. This is because inbox state changes are persisted in the
`DashboardSnapshot` and must be replayed on restart:

| DashboardEvent | Effect on DashboardSnapshot |
|---|---|
| `InboxItemReceived` | Inserts item into `inbox_items` map |
| `InboxApprove` | Removes item, records resolution |
| `InboxReject` | Removes item, records rejection with reason |
| `InboxDefer` | Updates item's `defer_until` timestamp |
| `InboxDismiss` | Removes item from the map |

---

## 7. Autonomy Events

Autonomy level changes are modeled in the AutonomyConfig struct rather than as
distinct events. When the autonomy store at `.roko/state/autonomy.json` is updated,
the next projection read picks up the new state. No streaming event is emitted for
autonomy changes -- the 5-second `max_age_secs` on the autonomy projection ensures
consumers see updates within that window.

The `AutonomyLevelChange`, `CapabilityGrant`, `CapabilityRevoke`, and `BulkAutonomySet`
event types described in the surface contract (SS3.5) are design targets. The current
implementation stores autonomy state as a flat JSON file, and the event model is not
yet connected to a live mutation path.
