# 10 — Daimon TUI View

**Priority**: P2 — closes a visibility gap: the affect state that influences every cascade routing decision is computed per-task but never surfaced to the operator
**Size**: S (1-2 days)
**Crates**: `crates/roko-core/` (new DashboardEvent variant + snapshot field), `crates/roko-cli/` (runner emission + TUI view + tab wiring)
**Depends on**: None

---

## Background

Roko runs an affect engine ("Daimon") on every task dispatch. The engine maintains a PAD (Pleasure/Arousal/Dominance) vector in the three-layer ALMA temporal model, a set of somatic markers that bias cascade routing based on past strategy-region performance, and a `BehavioralState` classification (`Engaged`, `Struggling`, `Coasting`, `Exploring`, `Focused`, `Resting`). These values influence which model tier is selected, how many turns the agent is given, and what effort level is requested. They are recorded per-task but never shown to the operator during a live run.

Roko's TUI (launched with `roko dashboard`) has ten tabs (F1-F10). Each tab has sub-views selectable with number keys. The F1 Dashboard tab currently has three sub-views: Health (1), Mesh (2), Cost (3). The tab system in `crates/roko-cli/src/tui/views/mod.rs` uses a `SubView` enum and `render_tab_content()` dispatch function. Adding a new sub-view to an existing tab requires: adding a `SubView` variant, registering it in `SubView::for_tab()`, and adding a render call in `dashboard_view.rs`.

The data pipeline for getting live state into the TUI already exists and is proven. The runner emits `DashboardEvent` variants through `TuiBridge` (in `crates/roko-cli/src/runner/tui_bridge.rs`). `StateHub` applies events to `DashboardSnapshot`. The snapshot is read by the TUI's app loop and merged into `TuiState`. Adding affect state to this pipeline requires: a new event variant, a new snapshot field, a new `TuiState` field, and a TUI view to render it.

## Current State

1. `DashboardEvent` enum is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` at line 77. It has ~35 variants. There is no `AffectUpdated` variant. Confirmed by `grep DashboardEvent::AffectUpdated` returning no matches.

2. `DashboardSnapshot` struct is at line 915. It has fields for plans, tasks, agents, gates, diagnoses, efficiency, learning, etc. It has no `affect` field and no `AffectSnapshot` type.

3. `DashboardSnapshot::apply_with_ts()` is at line 1078. Every `DashboardEvent` variant has an arm here. A new variant needs a new arm.

4. `DaimonState` is defined at line 2286 of `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs`. Its `state` field is `AffectState` (line 2288), which has `pad: PadVector` (line 359) and `behavioral_state: BehavioralState` (line 364) and `confidence: f64` (line 361).

5. `AffectState::effective_affect()` is at line 331 of `roko-daimon/src/lib.rs`. It returns a weighted blend: `0.5 * emotion + 0.3 * mood + 0.2 * temperament` for each PAD dimension, clamped to `[-1.0, 1.0]`.

6. The runner event loop at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` has a `daimon_task_hook()` function at line 7548 that calls `daimon.query()` and extracts `affect.pad.pleasure`, `affect.pad.arousal`, `affect.pad.dominance`, `affect.confidence`, and `affect.behavioral_state`. This struct (`DaimonTaskHook`) is assembled at line 7577. The hook is built before agent dispatch (the call site is `daimon_task_hook(config, task_def, attempt_num)` used throughout the dispatch logic).

7. `BehavioralState` is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/affect.rs` at line 15. Variants: `Engaged`, `Struggling`, `Coasting`, `Exploring`, `Focused`, `Resting`. The original spec mentioned "Exuberant/Anxious/Docile" octant labels — these are from the bardo predecessor but do not exist in the current codebase. The actual label to display is the `BehavioralState` variant name.

8. `SomaticSignal` returned by `daimon.query_somatic()` is referenced at line 7567 in event_loop.rs. Its `valence` and `intensity` fields are logged at lines 7570-7575 when `signal.should_emit_event()` is true.

9. `TuiState` is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` at line 1007. It has fields for plans, agents, gates, efficiency, cascade router, etc., but no affect state field.

10. The F1 Dashboard tab in `crates/roko-cli/src/tui/views/mod.rs` registers three sub-views at line 136-139: `DashboardHealth`, `MeshStatus`, `CostOverview`. The dashboard sub-views are rendered by key-letter codes `a`, `o`, `d`, `e`, `g`, `m`, `L`, `P` in `dashboard_view.rs`'s `SUB_TAB_LABELS` at line 35.

11. `TuiBridge` is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/tui_bridge.rs`. It has a `sender: StateHubSender` and a set of typed `publish` methods. Adding `affect_updated()` follows the same pattern as existing methods like `efficiency_event()` (line 141) and `diagnosis()` (line 248).

12. The view files follow the pattern in `learning_view.rs`: a `render()` function with a `match view_state.active_sub_view(Tab::X)` dispatch, plus named sub-functions for each sub-view.

## Implementation Plan

### Step 1: Add `AffectSnapshot` type and `DashboardSnapshot` field to `dashboard_snapshot.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs`, add a new struct before the `DashboardSnapshot` definition (before line 915):

```rust
/// Point-in-time snapshot of the Daimon affect state.
/// Populated by `DashboardEvent::AffectUpdated`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AffectSnapshot {
    /// PAD pleasure dimension [-1.0, 1.0].
    pub pleasure: f64,
    /// PAD arousal dimension [-1.0, 1.0].
    pub arousal: f64,
    /// PAD dominance dimension [-1.0, 1.0].
    pub dominance: f64,
    /// Behavioral state label (e.g. "Engaged", "Struggling").
    pub behavioral_state: String,
    /// Motivational confidence [0.0, 1.0].
    pub confidence: f64,
    /// Recent somatic marker valences: (label, valence) pairs, newest first.
    pub recent_markers: Vec<(String, f64)>,
    /// Active behavioral bias names (e.g. "Struggling", "Resting").
    pub active_biases: Vec<String>,
    /// Timestamp when this snapshot was recorded (unix ms).
    pub ts: u64,
}
```

Add a field to `DashboardSnapshot` struct (after `cfactor_trend`, around line 938):

```rust
/// Latest Daimon affect state from the runner.
#[serde(default)]
pub affect: Option<AffectSnapshot>,
```

### Step 2: Add `DashboardEvent::AffectUpdated` variant

Add to the `DashboardEvent` enum (after the last variant before the closing brace):

```rust
/// Daimon affect state updated after a task turn.
AffectUpdated {
    /// PAD pleasure dimension [-1.0, 1.0].
    pleasure: f64,
    /// PAD arousal dimension [-1.0, 1.0].
    arousal: f64,
    /// PAD dominance dimension [-1.0, 1.0].
    dominance: f64,
    /// Behavioral state name.
    behavioral_state: String,
    /// Motivational confidence [0.0, 1.0].
    confidence: f64,
    /// Recent somatic marker valences: (label, valence).
    #[serde(default)]
    recent_markers: Vec<(String, f64)>,
    /// Active behavioral bias names.
    #[serde(default)]
    active_biases: Vec<String>,
},
```

Add an arm in `DashboardSnapshot::apply_with_ts()` (inside the `match event` block at line 1079):

```rust
DashboardEvent::AffectUpdated {
    pleasure,
    arousal,
    dominance,
    behavioral_state,
    confidence,
    recent_markers,
    active_biases,
} => {
    self.affect = Some(AffectSnapshot {
        pleasure: *pleasure,
        arousal: *arousal,
        dominance: *dominance,
        behavioral_state: behavioral_state.clone(),
        confidence: *confidence,
        recent_markers: recent_markers.clone(),
        active_biases: active_biases.clone(),
        ts,
    });
}
```

### Step 3: Add `affect_updated()` to `TuiBridge`

Add to `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/tui_bridge.rs` (after the `diagnosis()` method at line 248):

```rust
/// Daimon affect state was updated after a task turn.
pub fn affect_updated(
    &self,
    pleasure: f64,
    arousal: f64,
    dominance: f64,
    behavioral_state: &str,
    confidence: f64,
    recent_markers: Vec<(String, f64)>,
    active_biases: Vec<String>,
) {
    self.sender.publish(DashboardEvent::AffectUpdated {
        pleasure,
        arousal,
        dominance,
        behavioral_state: behavioral_state.to_string(),
        confidence,
        recent_markers,
        active_biases,
    });
}
```

### Step 4: Emit the event from `event_loop.rs`

The `daimon_task_hook()` function at line 7548 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` already extracts `pleasure`, `arousal`, `dominance`, `behavioral_state`, and `confidence` into a `DaimonTaskHook` struct. After the hook is built and used for dispatch, emit the dashboard event via `tui_bridge`.

Locate all call sites of `daimon_task_hook()` in event_loop.rs (search for `daimon_task_hook(`) and after the call, add:

```rust
let hook = daimon_task_hook(config, task_def, attempt_num);
// ... existing use of hook ...

// Emit affect state to TUI.
if let Some(ref h) = hook {
    tui.affect_updated(
        h.pleasure,
        h.arousal,
        h.dominance,
        &format!("{:?}", h.behavioral_state),
        h.affect_confidence,
        Vec::new(),    // somatic markers: populate from daimon.somatic_landscape if needed
        Vec::new(),    // biases: can be derived from behavioral_state if needed
    );
}
```

The `recent_markers` and `active_biases` fields can be left empty in the initial implementation — the core PAD gauges and behavioral state label are the high-value items. Populating somatic markers requires calling `with_daimon_state()` again to read `daimon.somatic_landscape`, which can be added as a follow-up.

### Step 5: Add `affect` to `TuiState` in `state.rs`

Add to the `TuiState` struct in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` (after the `cascade_router` field around line 1229):

```rust
/// Latest Daimon affect state from the runner.
pub affect: Option<roko_core::dashboard_snapshot::AffectSnapshot>,
```

Add to `TuiState::default()` or `Default` impl (around line 1420):

```rust
affect: None,
```

In the method that syncs `DashboardSnapshot` into `TuiState` (look for where `cascade_router_json` is synced at line 2370):

```rust
self.affect = snap.affect.clone();
```

### Step 6: Add `AffectView` sub-view to `mod.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs`:

1. Add a new `SubView` variant inside the `// ── Region 1: Dashboard (F1) ──` block (after `CostOverview` at line 58):

```rust
/// Daimon affect state panel.
AffectView,
```

2. Update `SubView::for_tab()` for `Tab::Dashboard` (line 136):

```rust
Tab::Dashboard => &[
    SubView::DashboardHealth,
    SubView::MeshStatus,
    SubView::CostOverview,
    SubView::AffectView,   // sub-view 4, key "4"
],
```

3. Add a label arm in `SubView::label()`:

```rust
Self::AffectView => "Affect",
```

4. Add a `mod affect_view;` declaration alongside the other mod declarations (around line 27):

```rust
pub mod affect_view;
```

5. Add a dispatch arm in `render_tab_content()` for `Tab::Dashboard`. Currently it delegates entirely to `dashboard_view::render(...)`. Update `dashboard_view.rs` to handle `SubView::AffectView` internally, or add a special case in `render_tab_content`:

The simpler path: update `dashboard_view.rs`'s `render()` function to check `view_state.active_sub_view(Tab::Dashboard)` and delegate to `affect_view::render()` when it returns `SubView::AffectView`.

### Step 7: Create `affect_view.rs`

Create `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/affect_view.rs`:

```rust
//! Daimon affect state view for the F1 Dashboard tab (sub-view 4).
//!
//! Displays PAD gauges, behavioral state label, and somatic marker list.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};
use roko_core::dashboard_snapshot::AffectSnapshot;

use super::ViewState;
use crate::tui::dashboard::{DashboardData, Theme};
use crate::tui::state::TuiState;

/// Render the affect state panel.
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    _data: &DashboardData,
    tui_state: &TuiState,
    _view_state: &ViewState,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Daimon Affect ")
        .border_style(Style::default().fg(theme.border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(ref affect) = tui_state.affect else {
        let placeholder = Paragraph::new("No affect data yet.\nWaiting for first task turn...")
            .style(Style::default().fg(theme.dim));
        frame.render_widget(placeholder, inner);
        return;
    };

    // Layout: 3 gauges + state label + markers + biases
    let chunks = Layout::vertical([
        Constraint::Length(2), // pleasure gauge
        Constraint::Length(2), // arousal gauge
        Constraint::Length(2), // dominance gauge
        Constraint::Length(2), // state + confidence row
        Constraint::Min(0),    // markers + biases
    ])
    .split(inner);

    // PAD gauges. Ratatui Gauge accepts 0..=100, so map [-1, 1] → [0, 100].
    let pad_ratio = |v: f64| ((v + 1.0) / 2.0).clamp(0.0, 1.0);

    let gauge_color = |v: f64| {
        if v > 0.2 {
            Color::Green
        } else if v < -0.2 {
            Color::Red
        } else {
            Color::Yellow
        }
    };

    let pleasure_gauge = Gauge::default()
        .block(Block::default().title(format!(" Pleasure  {:+.2}", affect.pleasure)))
        .gauge_style(Style::default().fg(gauge_color(affect.pleasure)))
        .ratio(pad_ratio(affect.pleasure));
    frame.render_widget(pleasure_gauge, chunks[0]);

    let arousal_gauge = Gauge::default()
        .block(Block::default().title(format!(" Arousal   {:+.2}", affect.arousal)))
        .gauge_style(Style::default().fg(gauge_color(affect.arousal)))
        .ratio(pad_ratio(affect.arousal));
    frame.render_widget(arousal_gauge, chunks[1]);

    let dominance_gauge = Gauge::default()
        .block(Block::default().title(format!(" Dominance {:+.2}", affect.dominance)))
        .gauge_style(Style::default().fg(gauge_color(affect.dominance)))
        .ratio(pad_ratio(affect.dominance));
    frame.render_widget(dominance_gauge, chunks[2]);

    // State + confidence row
    let state_color = match affect.behavioral_state.as_str() {
        "Coasting" | "Focused" => Color::Green,
        "Struggling" => Color::Red,
        "Exploring" => Color::Cyan,
        "Resting" => Color::DarkGray,
        _ => Color::White,
    };
    let state_line = Paragraph::new(Line::from(vec![
        Span::styled("State: ", Style::default().fg(theme.label)),
        Span::styled(
            &affect.behavioral_state,
            Style::default()
                .fg(state_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   Confidence: {:.0}%", affect.confidence * 100.0),
            Style::default().fg(theme.dim),
        ),
    ]));
    frame.render_widget(state_line, chunks[3]);

    // Markers and biases
    let marker_chunks = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[4]);

    // Somatic markers
    let marker_items: Vec<ListItem> = if affect.recent_markers.is_empty() {
        vec![ListItem::new(Span::styled(
            "(no markers)",
            Style::default().fg(theme.dim),
        ))]
    } else {
        affect
            .recent_markers
            .iter()
            .take(8)
            .map(|(label, valence)| {
                let color = if *valence > 0.0 { Color::Green } else { Color::Red };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:+.2} ", valence),
                        Style::default().fg(color),
                    ),
                    Span::raw(label),
                ]))
            })
            .collect()
    };

    let markers_block = Block::default()
        .borders(Borders::ALL)
        .title(" Recent Markers ")
        .border_style(Style::default().fg(theme.border));
    let markers_list = List::new(marker_items).block(markers_block);
    frame.render_widget(markers_list, marker_chunks[0]);

    // Active biases
    let bias_items: Vec<ListItem> = if affect.active_biases.is_empty() {
        vec![ListItem::new(Span::styled(
            "(none)",
            Style::default().fg(theme.dim),
        ))]
    } else {
        affect
            .active_biases
            .iter()
            .map(|b| ListItem::new(Span::styled(b, Style::default().fg(Color::Cyan))))
            .collect()
    };

    let biases_block = Block::default()
        .borders(Borders::ALL)
        .title(" Active Biases ")
        .border_style(Style::default().fg(theme.border));
    let biases_list = List::new(bias_items).block(biases_block);
    frame.render_widget(biases_list, marker_chunks[1]);
}
```

### Step 8: Wire `AffectView` dispatch in `dashboard_view.rs`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/dashboard_view.rs`, update the `render()` function (currently at line 51) to check for the `AffectView` sub-view before falling through to the main dashboard layout:

```rust
pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &DashboardData,
    tui_state: &TuiState,
    view_state: &ViewState,
    theme: &Theme,
) {
    // Sub-view 4: Affect panel
    if view_state.active_sub_view(Tab::Dashboard) == super::SubView::AffectView {
        super::affect_view::render(frame, area, data, tui_state, view_state, theme);
        return;
    }

    // Default: standard dashboard layout
    // ... existing layout code unchanged ...
}
```

Add the import for `Tab` at the top of `dashboard_view.rs` if not already present (it is imported via `use crate::tui::tabs::Tab;`; check the existing imports).

## Acceptance Criteria

1. After one task turn completes in the runner, `DashboardEvent::AffectUpdated` is published with PAD values matching `DaimonTaskHook.pleasure`, `.arousal`, `.dominance`. Verifiable via a mock `StateHubSender` in a unit test.

2. `DashboardSnapshot::apply_with_ts()` correctly stores the event into `self.affect: Some(AffectSnapshot { ... })`. After applying a second event, the stored values reflect the second event (no accumulation — replace, not append).

3. `TuiState.affect` is `None` when no `AffectUpdated` events have been received; it is `Some(...)` after one event arrives.

4. The `affect_view::render()` function renders a placeholder string ("No affect data yet") when `tui_state.affect` is `None`; it renders three gauge widgets and a state label when `Some`.

5. Key `4` in the F1 Dashboard tab selects the `AffectView` sub-view, and the screen shows the PAD gauges without panicking.

6. PAD gauges are symmetric: a value of `0.0` maps to the center of the gauge (50% fill), `-1.0` maps to 0% fill, `+1.0` maps to 100% fill.

7. The behavioral state label uses the correct color coding: green for `Coasting`/`Focused`, red for `Struggling`, cyan for `Exploring`, dark gray for `Resting`, white for `Engaged` and any other variant.

8. `cargo test --workspace` passes with no regressions; the affected crates compile clean: `cargo clippy -p roko-core -p roko-cli -- -D warnings`.

## Verification Checklist

- [ ] `cargo build -p roko-core` passes after adding `AffectSnapshot`, `AffectUpdated` variant, and `apply_with_ts` arm
- [ ] `cargo build -p roko-cli` passes after adding `affect_view.rs`, updating `mod.rs`, `dashboard_view.rs`, `tui_bridge.rs`, `state.rs`, and `event_loop.rs`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` produces no new warnings
- [ ] `cargo +nightly fmt --all` produces no diff
- [ ] Run `cargo run -p roko-cli -- dashboard` in a workspace with an active plan; press `F1` then `4`; confirm the Affect panel appears with PAD gauges
- [ ] Let a task complete; confirm the PAD values update (they should change from the neutral 0.0 default as the daimon appraises task outcomes)
- [ ] Confirm that pressing `1`, `2`, `3` switches back to Health, Mesh, Cost sub-views without panicking
- [ ] Run `cargo test -p roko-core -- affect` to confirm the new `apply_with_ts` arm is exercised by existing tests (or add one)
- [ ] Confirm `roko serve` still compiles and the `/api/events` SSE stream includes `AffectUpdated` events when a plan runs

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` | Add `AffectSnapshot` struct; add `affect: Option<AffectSnapshot>` field to `DashboardSnapshot`; add `DashboardEvent::AffectUpdated` variant; add arm in `apply_with_ts()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/tui_bridge.rs` | Add `affect_updated()` method |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | After `daimon_task_hook()` call sites, emit `tui.affect_updated(...)` with extracted PAD + behavioral state |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` | Add `affect: Option<AffectSnapshot>` field to `TuiState`; set to `None` in default; sync from snapshot |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/mod.rs` | Add `SubView::AffectView` variant; register in `for_tab(Tab::Dashboard)`; add `"Affect"` label; add `pub mod affect_view;` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/dashboard_view.rs` | Add early-return check for `SubView::AffectView` that delegates to `affect_view::render()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/affect_view.rs` (new file) | Full view implementation: PAD gauges, state label, somatic markers list, active biases list |
