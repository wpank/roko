# Depth: Legacy Tab Mapping

> Parent: [22-SURFACES](../../22-SURFACES.md) SS5

This file documents the compatibility mapping between the existing TUI tabs
(F1-F10) and the seven v2 named-surface identities introduced by E37.

---

## 1. The V2Surface Enum

Defined in `crates/roko-cli/src/tui/tabs.rs`:

```rust
pub enum V2Surface {
    Workbench,
    Canvas,
    Flows,
    Inbox,
    Knowledge,
    System,
    Agents,
}
```

These seven identities are the bridge between the legacy tab system and the five
named surfaces. The mapping is not 1:1 because:

- Some legacy tabs contribute multiple surface identities (F1 carries both
  Workbench and Inbox)
- Some surface identities add navigation concepts beyond the five core surfaces
  (Flows and System are additional compatibility identities)
- Five legacy tabs carry no v2 surface identity at all (F4, F5, F8, F9 and
  F10)

---

## 2. Tab Enum and Function Keys

The `Tab` enum defines ten values mapped to function keys:

```rust
pub enum Tab {
    Dashboard,    // F1
    Plans,        // F2
    Agents,       // F3
    Git,          // F4
    Logs,         // F5
    Config,       // F6
    Inspect,      // F7
    Marketplace,  // F8
    Learning,     // F9
    Providers,    // F10
}
```

Each tab has:
- `fkey()`: the `KeyCode` that activates it
- `from_key()`: reverse lookup from key to tab
- `label()`: human-readable name (e.g., "Dashboard")
- `label_with_key()`: name with key hint (e.g., "F1 Dashboard")
- `v2_surfaces()`: the compatibility mapping

---

## 3. The Mapping Table

Implemented in `Tab::v2_surfaces()`:

```rust
pub fn v2_surfaces(self) -> Vec<V2Surface> {
    match self {
        Self::Dashboard => vec![V2Surface::Workbench, V2Surface::Inbox],
        Self::Plans     => vec![V2Surface::Canvas, V2Surface::Flows],
        Self::Agents    => vec![V2Surface::Agents],
        Self::Config    => vec![V2Surface::System],
        Self::Inspect   => vec![V2Surface::Knowledge],
        Self::Git | Self::Logs | Self::Marketplace
        | Self::Learning | Self::Providers => vec![],
    }
}
```

### Detailed mapping rationale

| Tab | Surfaces | Rationale |
|---|---|---|
| **F1 Dashboard** | Workbench, Inbox | The dashboard is the primary task management and notification surface. Workbench provides the plan/agent overview; Inbox contributes attention badges. |
| **F2 Plans** | Canvas, Flows | Plans are the closest existing surface to Graph authoring (Canvas) and execution monitoring (Flows). |
| **F3 Agents** | Agents | The agent list/detail view is the compatibility shell for the Stigmergy Minimap. |
| **F4 Git** | *(none)* | Git branch/commit views are rendering-target-specific with no named-surface counterpart. |
| **F5 Logs** | *(none)* | Log viewing is rendering-target-specific. |
| **F6 Config** | System | Config editing maps to the System surface, which includes autonomy controls. |
| **F7 Inspect** | Knowledge | The signal DAG inspector is the closest match to the Knowledge surface. |
| **F8 Marketplace** | *(none)* | Marketplace browsing has no named-surface counterpart. |
| **F9 Learning** | *(none)* | Learning analytics (cascade router, experiments) are rendering-target-specific. |
| **F10 Providers** | *(none)* | Provider health is rendering-target-specific. |

---

## 4. What the Mapping Does Not Do

The compatibility mapping is purely metadata. It does **not**:

1. **Change tab rendering**: F1 Dashboard still renders its existing dashboard view,
   not the full Workbench+Inbox named-surface layout.
2. **Re-key tabs**: F4 remains Git, not Inbox. F3 remains Agents, not Flows.
3. **Enable named-surface features**: The Autonomy Slider is not rendered in F6 Config.
   The Stigmergy Minimap is not rendered in F3 Agents.
4. **Add new tabs**: No new tabs are created for surfaces that do not map to existing
   tabs (like a dedicated Inbox tab).

---

## 5. Target Layout vs. Current State

The named-surface design envisions a different tab assignment:

| Target Tab | Target Surface | Current Tab | Current Surface Identity |
|---|---|---|---|
| F1 Workbench | Workbench + Inbox badges | F1 Dashboard | Workbench, Inbox (identity only) |
| F2 Canvas | Graph library, state-graph | F2 Plans | Canvas, Flows (identity only) |
| F3 Flows | Flow inspector | F3 Agents | Agents |
| F4 Inbox | Full Inbox surface | F4 Git | *(none)* |
| F5 Knowledge | Knowledge browser | F5 Logs | *(none)* |
| F6 System | Space + Autonomy Slider | F6 Config | System (identity only) |
| F7 Agents | Fleet + Minimap | F7 Inspect | Knowledge |

The gap between current and target is the primary product residual for TUI rendering.

---

## 6. Surface Inventory

The `crates/roko-cli/src/surface_inventory.rs` module provides a programmatic
inventory of all operator-visible entry points across CLI commands, TUI tabs,
TUI subviews, and modals. Each entry carries:

```rust
pub struct SurfaceEntry {
    pub name: String,              // e.g., "F1 Dashboard", "plan run"
    pub kind: SurfaceKind,         // CliCommand | TuiTab | TuiSubView | Modal
    pub status: SurfaceStatus,     // Wired | Partial | Stub | Missing
    pub backend_dependency: String, // e.g., "roko-agent lifecycle"
    pub notes: String,             // free-form status description
}
```

The inventory is accessible via `roko status --surfaces` and serves as the
single source of truth for what operator surfaces exist and their wiring state.
