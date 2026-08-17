# Backlog: DaimonState TUI Visualization (Affect Tab)

**Status**: Backlog
**Priority**: P2
**Size**: S (1-2 days)
**Origin**: `tmp/architecture-archive/21-tui-and-operations.md` (Section 1: DaimonState visualization)

---

## Problem Statement

`DaimonState` is loaded and used in the runner v2 event loop (`crates/roko-cli/src/runner/event_loop.rs`) on every task dispatch. The PAD (Pleasure/Arousal/Dominance) vector, `PadRegion` label, somatic marker history, and behavioral bias flags are all computed and influence cascade routing decisions. None of this internal state is visible to the operator.

The TUI has F1-F10 tabs and a file-watcher-driven state machine that responds to `DashboardEvent` variants. The `DashboardSnapshot` in `crates/roko-core/src/dashboard_snapshot.rs` receives events from the runner via the `StateHub`/`DashboardEvent` bus. The pipeline for getting runtime state into the TUI already exists and is proven — it is used for gate results, efficiency events, agent outputs, and cascade router snapshots.

What is missing is:

1. A `DashboardEvent::AffectUpdated` variant carrying the current `DaimonState` summary (PAD vector, region, somatic markers, active biases).
2. The runner emitting that event after each task turn where `DaimonState` is updated.
3. A TUI view (new file under `crates/roko-cli/src/tui/views/`) that renders the PAD gauges, region label, somatic marker histogram, and bias indicators.
4. A tab binding or sub-view slot that makes the view reachable.

The bardo predecessor had dedicated "Emotions" and "Vitality" terminal screens (`bardo/apps/bardo-terminal/src/screens/`) with exactly this layout; this item ports and adapts that design to roko's ratatui stack.

---

## Proposed Solution

### Step 1: `DashboardEvent::AffectUpdated`

Add a new variant to the `DashboardEvent` enum in `crates/roko-core/src/dashboard_snapshot.rs`:

```rust
AffectUpdated {
    /// PAD vector: pleasure, arousal, dominance in [-1.0, 1.0]
    pleasure: f64,
    arousal: f64,
    dominance: f64,
    /// Derived octant label (e.g., "Exuberant", "Anxious", "Docile")
    region_label: String,
    /// Last N somatic marker valences (positive=true, negative=false)
    recent_markers: Vec<(String, f64)>,
    /// Active behavioral bias names (e.g., "SeekSafety", "AvoidTrade")
    active_biases: Vec<String>,
},
```

The `DashboardSnapshot::apply_with_ts()` arm stores this into a new `affect: Option<AffectSnapshot>` field on the snapshot.

### Step 2: Runner emission

In `event_loop.rs`, after the `DaimonState` is updated following a task turn (the call site that already loads and uses `DaimonState`), emit `DashboardEvent::AffectUpdated` via the existing `tui_bridge` / `hub.publish()` path. The `DaimonState::effective_affect()` method returns the blended PAD vector; `DaimonState::active_biases()` (or equivalent) returns the bias set.

### Step 3: TUI view

New file: `crates/roko-cli/src/tui/views/affect_view.rs`

Layout (ratatui):

```
┌─ Affect ─────────────────────────────────────────────────┐
│ Pleasure  [-1 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ +1]  0.34│
│ Arousal   [-1 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ +1] -0.12│
│ Dominance [-1 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ +1]  0.67│
│                                                           │
│  Region: Exuberant                                        │
│                                                           │
│  Recent somatic markers:                                  │
│  ████░░░░░░ gate_pass (+0.22)                            │
│  ░░░░████░░ budget_spike (-0.31)                         │
│  ████████░░ task_complete (+0.18)                        │
│                                                           │
│  Active biases: [none]                                    │
└───────────────────────────────────────────────────────────┘
```

- PAD gauges use `ratatui::widgets::Gauge` with a symmetric axis: value 0.0 maps to the center, -1.0 to the left edge, +1.0 to the right.
- Region label uses the existing `PadVector` octant logic from `roko-daimon`.
- Somatic marker rows use braille sparklines (consistent with existing `learning_view.rs` style).
- Bias indicators are a `ratatui::widgets::List` showing active bias names; "none" when empty.

### Step 4: Tab binding

Add `AffectView` as a sub-view within the F1 Dashboard tab (as a switchable panel), or expose it as an F11 tab if the tab bar permits extension. The simpler path is a sub-panel in F1 (Dashboard) toggled by a key (e.g., `a`), consistent with how other Dashboard sub-views are toggled.

---

## Implementation Location

| Component | Path |
|---|---|
| New `DashboardEvent` variant | `crates/roko-core/src/dashboard_snapshot.rs` |
| `AffectSnapshot` on `DashboardSnapshot` | `crates/roko-core/src/dashboard_snapshot.rs` |
| Runner emission | `crates/roko-cli/src/runner/event_loop.rs` |
| TUI view | `crates/roko-cli/src/tui/views/affect_view.rs` (new file) |
| View registration | `crates/roko-cli/src/tui/views/mod.rs` |
| Tab/panel wiring | `crates/roko-cli/src/tui/tabs.rs` or `dashboard_view.rs` |

---

## Acceptance Criteria

1. After a task turn completes in the runner, a `DashboardEvent::AffectUpdated` is published with non-default PAD values; `cargo test` integration tests confirm the event is emitted (via a mock hub subscriber).

2. The TUI `affect_view` renders without panicking when `AffectSnapshot` is `None` (no data yet), displaying placeholder dashes.

3. PAD gauges reflect the correct values from `DaimonState::effective_affect()` — verified by injecting a known `DaimonState` into the test harness and asserting the rendered gauge fill levels.

4. Region label ("Exuberant", "Anxious", etc.) matches the octant computed from the same PAD vector by the existing `roko-daimon` octant mapping logic.

5. The view is reachable from the TUI without restarting `roko dashboard`; toggling the key or tab renders the affect panel inline.

6. No regressions in existing TUI tabs F1-F10; `cargo test --workspace` passes.

---

## References

- Source spec: `/Users/will/dev/nunchi/roko/roko/tmp/architecture-archive/21-tui-and-operations.md` (Section 1)
- DaimonState: `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs` (`DaimonState`, `PadVector`, `effective_affect()`)
- Dashboard events: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`
- Existing views for style reference: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/learning_view.rs`
- TUI tab wiring: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/tabs.rs`
- Runner event loop (emission site): `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
