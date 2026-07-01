# C5: Five discrete TUI bug fixes

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**TUI source:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/`
**Framework:** ratatui + crossterm + tokio
**Theme:** ROSEDUST (rose=#AA7088, bone=#C8B890, bg=#060608)
**Architecture:**
- `App` (app.rs) owns `TuiState` + `DashboardData`, runs 60fps loop
- `TuiState` (state.rs) holds all display state, populated by two paths:
  - StateHub: `drain_snapshot_channel()` -> `update_from_dashboard_snapshot()`
  - File poll: `tick_snapshot()` -> `DashboardData::tick()` -> `update_from_snapshot()`
- Views (views/*.rs) receive `(&DashboardData, &TuiState, &ViewState, &Theme)` -- zero I/O
- Widgets (widgets/*.rs) are reusable ratatui components
- Navigation: F-keys switch tabs, 1-9 switch sub-views, j/k navigate, Enter expands

**Tab enum** at `crates/roko-cli/src/tui/tabs.rs`:
- Current: Dashboard(F1), Plans(F2), Agents(F3), Git(F4), Logs(F5), Config(F6), Inspect(F7)

**SubView enum** at `crates/roko-cli/src/tui/views/mod.rs`:
- F6: ConfigEditor, ProviderHealth, ModelComparison
- F7: EngramDag, EpisodeReplay, KnowledgeBrowse

**Key ViewState fields** (in state.rs):
- `sub_tab: usize` -- which sub-view within a tab
- `selected_primary: usize`, `selected_secondary: usize` -- list selection indices
- `scroll_offset: usize` -- for scrollable content

### Pre-commit (MANDATORY)
```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Goal

Fix five discrete bugs. Each fix is independent -- they can be done in any order. Do all
five in a single commit.

**Audit update (2026-04-22):** git parser hardening and failure-specific Dashboard hints are now implemented and covered. This file stays active only for the dashboard unified-log cache follow-up: either wire a generation-based cache if the O(N) per-frame merge still exists, or document the proof that the current dashboard view no longer rebuilds a unified log inline.

- [ ] Resolve the remaining unified-log cache question by implementation or proof.

## Dependency

None. All fixes target existing code.

---

## Fix 1: plan_tree.rs vfy column -- wire gate verdict

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/plan_tree.rs`

### Before

The plan tree has a `vfy` column (3-char width, `COL_VERIFY`) that renders gate
verification status for each plan. It currently shows a static placeholder or empty
string because it does not look up gate verdicts from `data.gate_results_page`.

The render function receives `&TuiState` but not `&DashboardData`. Gate results live
in `data.gate_results_page.gate_rows`, which is not accessible from the widget.

### Fix

The plan tree widget receives `&TuiState`. Add a field to `TuiState` that holds
per-plan gate verdicts, populated from `DashboardData` during `update_from_snapshot()`.

**Step 1: Add field to TuiState.**

Read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`. Find the
`TuiState` struct definition. Add:

```rust
/// Per-plan gate verdict for the plan tree vfy column.
/// Key: plan ID. Value: "P" (passed), "F" (failed), or "" (no data).
pub gate_verdicts: HashMap<String, String>,
```

Initialize as `HashMap::new()` in the constructor.

**Step 2: Populate gate_verdicts from DashboardData.**

Find `update_from_snapshot()` (or `from_dashboard_data()`) in state.rs. Add:

```rust
// Populate gate verdicts from gate_results
self.gate_verdicts.clear();
for result in &data.gate_results {
    if !result.plan_id.is_empty() {
        let verdict = if result.passed { "P".to_string() } else { "F".to_string() };
        // Keep the latest verdict for each plan
        self.gate_verdicts.insert(result.plan_id.clone(), verdict);
    }
}
```

Check `GateResultSummary` in `dashboard.rs` for exact field names (`plan_id`, `passed`).
Adapt the field names if they differ.

**Step 3: Use gate_verdicts in plan_tree.rs.**

Find where plan rows are rendered (in `render_plan_row()`, `render_flat_plans()`, or
`render_wave_tree()`). Locate the vfy column cell. Replace the placeholder:

```rust
// BEFORE: placeholder or empty
let vfy_text = "";

// AFTER: look up from state
let vfy_text = state.gate_verdicts.get(&plan.id).map(|s| s.as_str()).unwrap_or("");
let vfy_style = match vfy_text {
    "P" => Style::default().fg(Theme::SAGE),
    "F" => Style::default().fg(Theme::EMBER).add_modifier(Modifier::BOLD),
    _   => Style::default().fg(Theme::TEXT_GHOST),
};
```

### After

The vfy column shows:
- `P` in green (SAGE) when the plan last passed all gate rungs
- `F` in bold red (EMBER) when the plan last failed a gate rung
- ` ` (empty) in ghost color when no gate data exists for this plan

### Verification

```bash
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Fix 2: dashboard.rs O(N) log rebuild -- add generation-based caching

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/dashboard_view.rs`

### Before

The dashboard view rebuilds a unified log (merging `event_log`, `recent_signals`, and
potentially `efficiency_events` into a single sorted timeline) on every frame. At 60fps
with 1000+ events, this creates visible lag because sorting is O(N log N) per frame.

The merge happens inline in the render function, which cannot be cached -- views are
called 60 times per second and are not allowed to mutate state.

### Fix

Add a cached unified log to `TuiState` that only rebuilds when `data.generation` changes.
Call the rebuild in the tick loop (before rendering), not in the view.

**Step 1: Add cache fields to TuiState.**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`, add:

```rust
/// Cached unified log lines (merged + sorted events, signals, episodes).
pub cached_unified_log: Vec<String>,
/// Generation value when the unified log was last built.
pub cached_unified_log_generation: u64,
```

Initialize both in the constructor:
```rust
cached_unified_log: Vec::new(),
cached_unified_log_generation: 0,
```

**Step 2: Add the rebuild method.**

Add a method to `TuiState`:

```rust
/// Rebuild the unified log only if `data.generation` has changed.
///
/// Call this from the tick loop in app.rs, never from a render function.
pub fn rebuild_unified_log_if_stale(&mut self, data: &DashboardData) {
    if data.generation == self.cached_unified_log_generation
        && !self.cached_unified_log.is_empty()
    {
        return; // cache is fresh
    }
    self.cached_unified_log_generation = data.generation;

    // Merge event_log + recent_signals into one sorted timeline.
    let mut entries: Vec<(u64, String)> =
        Vec::with_capacity(data.event_log.len() + data.recent_signals.len());

    for event in &data.event_log {
        entries.push((
            event.timestamp_ms,
            format!(
                "[{}] {} {} {}",
                event.event_type, event.plan_id, event.task_id, event.message,
            ),
        ));
    }

    for signal in &data.recent_signals {
        entries.push((
            signal.timestamp_ms,
            format!(
                "[signal] {} {} {:.2}",
                signal.kind, signal.hash_short, signal.confidence,
            ),
        ));
    }

    // Sort by timestamp ascending; latest events appear at the bottom.
    entries.sort_by_key(|(ts, _)| *ts);
    self.cached_unified_log = entries.into_iter().map(|(_, line)| line).collect();
}
```

**Step 3: Call it from the tick loop in app.rs.**

Search `app.rs` for where `tui_state.update_from_snapshot()` or
`tui_state.from_dashboard_data()` is called. Add the rebuild call immediately after:

```rust
// BEFORE:
tui_state.update_from_snapshot(&data);

// AFTER:
tui_state.update_from_snapshot(&data);
tui_state.rebuild_unified_log_if_stale(&data);
```

**Step 4: Use the cache in dashboard_view.rs.**

Find the location that merges the log inline. Replace it:

```rust
// BEFORE: inline O(N log N) merge on every frame
let mut entries: Vec<(u64, String)> = Vec::new();
for event in &data.event_log { ... }
for signal in &data.recent_signals { ... }
entries.sort_by_key(|(ts, _)| *ts);
let log_lines: Vec<String> = entries.into_iter().map(|(_, l)| l).collect();

// AFTER: read from cache (O(1))
let log_lines = &tui_state.cached_unified_log;
```

### After

The unified log is built once per `DashboardData` generation tick (roughly once per
second from file polling), not 60 times per second. The render function only slices
into the cached Vec.

### Verification

```bash
cargo test --workspace
```

---

## Fix 3: wave_progress.rs collapse toggle

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs`

### Before

The wave progress widget in the plan tree has collapsible wave groups. The `wave.expanded`
field controls visibility. The toggle is supposed to work with h/l/Enter when the wave
header is focused in the Plans tab, but the input handler does not dispatch
`TuiAction::ExpandCollapse` for those keys in that context.

### Investigation

Read `input.rs`. Search for `ExpandCollapse`. Find where it is dispatched and check
whether the guard condition includes the Plans tab + wave focus.

Read `state.rs` to find `execution_waves` and confirm the `Wave` struct has a mutable
`expanded: bool` field.

### Fix

**Step 1: Add key dispatch in input.rs.**

Find the Normal mode key handling block for `Tab::Plans`. Add or fix the expand/collapse
dispatch. The guard requires both the correct tab and the correct focus zone:

```rust
// BEFORE: ExpandCollapse not dispatched for h/l/Enter in Plans tab

// AFTER: dispatch on h / l / Enter when in PlanTree focus zone
KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Enter
    if current_tab == Tab::Plans
        && matches!(focus, FocusZone::PlanTree) =>
{
    Some(TuiAction::ExpandCollapse)
}
```

**Step 2: Ensure the action handler toggles the correct wave.**

Find where `TuiAction::ExpandCollapse` is handled in the action processor (often in
`app.rs` or a `handle_action()` function). The handler must toggle
`state.execution_waves[current_wave].expanded`:

```rust
// BEFORE: ExpandCollapse handler did not toggle wave.expanded
TuiAction::ExpandCollapse => { /* nothing */ }

// AFTER:
TuiAction::ExpandCollapse => {
    let wave_idx = state.current_wave();
    if let Some(wave) = state.execution_waves.get_mut(wave_idx) {
        wave.expanded = !wave.expanded;
    }
}
```

Confirm that `state.current_wave()` returns the index of the wave that is currently
selected in the plan tree. If no such method exists, use `state.selected_primary` as
the index (check the actual field name in `TuiState`).

### After

Pressing h, l, or Enter on a wave header in the Plans tab toggles that wave's collapsed
state. Collapsed waves hide their task rows; only the wave summary header is shown.

### Verification

```bash
cargo test --workspace
```

---

## Fix 4: git_view.rs parser -- NUL-separated git log format

**File:** git log command and parser in the TUI git view

### Before

The git log parser splits on spaces or pipe characters to extract fields (hash, subject,
author, age). This breaks when a commit subject contains those characters, causing fields
to be misaligned or merged.

Example broken output with a subject like "fix: handle a|b edge case":
```
# Parsed as: hash="abc1234", subject="fix:", author="handle", age="a|b"
# Correct:   hash="abc1234", subject="fix: handle a|b edge case", author="Will", age="2h ago"
```

### Investigation

Search the TUI source for the git log command:

```bash
grep -rn 'git.*log\|Command.*git' crates/roko-cli/src/tui/ --include='*.rs' | grep -v target/
```

Find:
1. The `Command::new("git")` call and its `--format` string
2. The parser that creates `CommitEntry` structs from the output lines

### Fix

**Step 1: Change the git log format string to NUL-separated fields.**

```rust
// BEFORE: space/pipe separated -- breaks on subjects with those chars
"--format=%h %s %an %cr"

// AFTER: NUL-separated -- safe for any subject text
"--format=%H%x00%h%x00%an%x00%cr%x00%s"
```

This produces: `full_hash\0short_hash\0author\0age\0subject` per line.
The `%x00` format directive emits a literal NUL byte. Git outputs one record per
commit separated by newlines, with NUL as the intra-record field delimiter.

**Step 2: Update the parser to split on `'\0'`.**

```rust
// BEFORE: space/pipe split
let parts: Vec<&str> = line.split('|').collect();
// or:
let parts: Vec<&str> = line.splitn(5, ' ').collect();

// AFTER: NUL split
let parts: Vec<&str> = line.splitn(6, '\0').collect();
if parts.len() >= 5 {
    commits.push(CommitEntry {
        hash_short: parts[1].to_string(),
        author:     parts[2].to_string(),
        age:        parts[3].to_string(),
        subject:    parts[4].to_string(),
        // graph_prefix not available from --format alone; set to empty string
        // unless --graph is in use, in which case parse from parts[5] if present
        graph_prefix: String::new(),
    });
}
```

Note: if `--graph` is used, git may prepend a graph column before the format output.
Check whether the existing code uses `--graph`. If it does, the graph decoration appears
as a prefix on each line before the first `\0`; capture it in `parts[0]` or strip it
before splitting.

### Unit test

Add a test that verifies the parser handles subjects containing spaces, pipes, and
hash characters:

```rust
#[test]
fn git_log_parser_handles_special_chars_in_subject() {
    // Simulate git log --format=%H%x00%h%x00%an%x00%cr%x00%s output
    let line = "abc123def456\x00abc123\x00Will Smith\x002 hours ago\x00fix: handle a|b and c#d edge case";
    let parts: Vec<&str> = line.splitn(6, '\0').collect();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[1], "abc123");
    assert_eq!(parts[2], "Will Smith");
    assert_eq!(parts[3], "2 hours ago");
    assert_eq!(parts[4], "fix: handle a|b and c#d edge case");
}
```

Place this test in the same file as the parser.

### After

Commit subjects containing `|`, ` `, `-`, `#`, or any other delimiter character are
parsed correctly. The full subject text is preserved as-is.

### Verification

```bash
cargo test --workspace
# Manually verify the format works:
git log -5 --format='%H%x00%h%x00%an%x00%cr%x00%s'
```

---

## Fix 5: status_bar.rs keybind hints -- differentiate based on has_failures

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/status_bar.rs`

### Before

The Dashboard tab keybind hints are identical in both branches of the `has_failures`
check. Both branches show the same navigation hints, giving the operator no visual cue
that recovery actions are available when there are failures.

```rust
// BEFORE: both branches identical -- operator cannot tell recovery keys exist
Tab::Dashboard => {
    if has_failures {
        "\u{2191}\u{2193}:nav  a/o/d/e/g:sub-tab  Tab:panel  ?:help"
    } else {
        "\u{2191}\u{2193}:nav  a/o/d/e/g:sub-tab  Tab:panel  ?:help"
    }
}
```

### Fix

Replace the Dashboard match arm with differentiated hints. When `has_failures` is true,
show `R:retry` and `D:diag` hints. When false, show the normal navigation hints only.

```rust
// AFTER: different hints when failures exist
Tab::Dashboard => {
    if has_failures {
        "\u{2191}\u{2193}:nav  a/o/d/e/g:sub  R:retry  D:diag  Tab:panel  ?:help"
    } else {
        "\u{2191}\u{2193}:nav  a/o/d/e/g:sub-tab  Tab:panel  ?:help"
    }
}
```

The failure hint string uses `sub` (shortened) instead of `sub-tab` to fit within the
width budget. The `R:retry` and `D:diag` labels match the `TuiAction::RetryFailed` and
`TuiAction::DiagnoseFailure` actions that are already defined.

### After

When at least one plan has a failed gate result:
- Status bar shows: `↑↓:nav  a/o/d/e/g:sub  R:retry  D:diag  Tab:panel  ?:help`

When no failures exist:
- Status bar shows: `↑↓:nav  a/o/d/e/g:sub-tab  Tab:panel  ?:help`

### Verification

```bash
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Final verification

After all five fixes:

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

All five fixes are independent and do not touch the same functions. They can be applied
in any order and reviewed in separate hunks.

## Acceptance criteria

- [ ] Fix 1: `TuiState.gate_verdicts: HashMap<String, String>` populated from
      `data.gate_results`; plan tree vfy column renders P/F/empty with correct color
- [ ] Fix 2: `TuiState.cached_unified_log` rebuilt only when `data.generation` changes;
      dashboard_view reads `&tui_state.cached_unified_log` instead of merging inline
- [ ] Fix 3: h/l/Enter on wave header in Plans + PlanTree focus zone dispatches
      `TuiAction::ExpandCollapse`; handler toggles `wave.expanded`
- [ ] Fix 4: git log command uses `--format=%H%x00%h%x00%an%x00%cr%x00%s`; parser
      splits on `'\0'`; unit test verifies subjects with `|` and `#` parse correctly
- [ ] Fix 5: Dashboard tab hints show `R:retry D:diag` when `has_failures` is true;
      plain navigation hints when false
- [ ] All five fixes are independent and do not conflict
- [ ] `cargo clippy` clean, `cargo +nightly fmt` clean
- [ ] All existing tests pass
