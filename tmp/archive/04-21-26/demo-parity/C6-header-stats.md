# C6: Add network stats to header bar

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

### Pre-commit (MANDATORY)
```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Goal

Add an "agents online" counter and an "ISFR" (instant success-failure ratio) indicator
to the header bar. The new section appears between the system metrics section and the
active agent spinner section.

**Audit update (2026-04-22):** the header renders agent count and an ISFR-like interval from efficiency timestamps, but the file's acceptance criteria also require file fallback behavior and different `ISFR` display semantics that are not implemented.

- [ ] Reconcile the ISFR definition/display in this PRD with the implementation, then add the missing fallback file count and uppercase `ISFR`/em-dash rendering behavior if that remains the intended contract.

## Dependency

None. All changes target existing files.

## Steps

### Step 1: Read existing code

Read these files:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/dashboard.rs`

In state.rs, search for `agents_online` -- if the field already exists, skip Step 2.
In dashboard.rs, confirm the field names on `AgentSummary` (`status` field) and
`GateResultSummary` (`passed` field).

### Step 2: Add `agents_online` and `isfr` fields to TuiState

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs`

Add two fields to the `TuiState` struct:

```rust
/// Number of agents currently online (active or idle, not done or failed).
pub agents_online: usize,
/// Instant success-failure ratio from the last N gate results.
/// `None` when no gate results exist.
pub isfr: Option<f64>,
```

Initialize both in the constructor:
```rust
agents_online: 0,
isfr: None,
```

### Step 3: Populate agents_online and isfr from DashboardData

In `TuiState`, find `update_from_snapshot()` or `from_dashboard_data()`. Add:

```rust
// Agents online: count agents that are not in a terminal state
self.agents_online = data
    .agents
    .iter()
    .filter(|a| a.status != "done" && a.status != "failed" && a.status != "completed")
    .count();

// ISFR: gate pass rate over the most-recent gate results
if data.gate_results.is_empty() {
    self.isfr = None;
} else {
    let passed = data.gate_results.iter().filter(|g| g.passed).count();
    self.isfr = Some(passed as f64 / data.gate_results.len() as f64);
}
```

Check the actual field names in `AgentSummary` and `GateResultSummary` by reading
`dashboard.rs`. Adapt as needed.

### Step 4: Optional background HTTP poll (fire-and-forget)

This step is optional. Skip it if `reqwest` is not already a dependency of `roko-cli`.

Check:
```bash
grep 'reqwest' crates/roko-cli/Cargo.toml
```

If `reqwest` IS available, add a non-blocking poller to `TuiState`. The poller spawns a
tokio task with a timeout, sends the result back on a channel, and updates
`tui_state.agents_online` in the tick loop. The file-based count from Step 3 is the
fallback if the HTTP request fails or times out.

```rust
/// Attempt to fetch live agent count from the control plane.
///
/// Non-blocking: spawns a tokio task. If the request fails or times out,
/// the DashboardData-derived count from Step 3 remains unchanged.
/// Caller must wire up the `rx` end in the app tick loop.
pub fn spawn_network_stats_poll(
    tx: tokio::sync::mpsc::Sender<usize>,
) {
    tokio::spawn(async move {
        // Hard timeout: 500ms. The TUI must not block a render frame.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            async {
                reqwest::get("http://localhost:6677/api/network/stats")
                    .await?
                    .json::<serde_json::Value>()
                    .await
            },
        )
        .await;

        if let Ok(Ok(body)) = result {
            let count = body["agents_online"].as_u64().unwrap_or(0) as usize;
            // Ignore send errors (receiver may have been dropped)
            let _ = tx.send(count).await;
        }
        // On timeout or HTTP error: do nothing -- DashboardData count is used
    });
}
```

Wire the receiver in `app.rs`:
```rust
// In the tick loop, after update_from_snapshot:
while let Ok(count) = network_stats_rx.try_recv() {
    tui_state.agents_online = count;
}
```

If `reqwest` is NOT available, skip this step entirely. The `data.agents` count from
Step 3 is sufficient.

### Step 5: Lazy fallback file count (alternative to HTTP poll)

If neither `reqwest` is available nor `roko serve` is running, count active agents
from files under `.roko/jobs/`. This function is only called during `update_from_snapshot()`
when `data.agents` is empty -- not on every tick.

```rust
/// Count active jobs from `.roko/jobs/*.json` files.
///
/// This is a lazy fallback: only called when the agent list in DashboardData
/// is empty. It performs synchronous file I/O, so it must NOT be called on
/// every render tick (only on snapshot ticks, ~1Hz).
fn count_online_from_files(root: &std::path::Path) -> usize {
    let jobs_dir = root.join(".roko").join("jobs");
    let Ok(entries) = std::fs::read_dir(&jobs_dir) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .filter_map(|e| {
            let content = std::fs::read_to_string(e.path()).ok()?;
            let v: serde_json::Value = serde_json::from_str(&content).ok()?;
            let status = v["status"].as_str().unwrap_or("");
            // Count as online if not in a terminal state
            if matches!(status, "done" | "completed" | "failed" | "cancelled") {
                None
            } else {
                Some(())
            }
        })
        .count()
}
```

Call it in `update_from_snapshot()`:
```rust
// Use DashboardData agent count; fall back to file count only if empty
if data.agents.is_empty() {
    self.agents_online = count_online_from_files(data.root());
} else {
    self.agents_online = data
        .agents
        .iter()
        .filter(|a| a.status != "done" && a.status != "failed" && a.status != "completed")
        .count();
}
```

### Step 6: Render in header_bar.rs

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs`

Find the separator between section 6 (system metrics) and section 7 (active agent
spinner). The pattern to find:

```rust
    spans.push(sep());

    // -- 7. Active agent spinner with role label ---------------
```

Insert a new section 6b BEFORE that block:

```rust
    // -- 6b. Network stats: agents online + ISFR ---------------
    {
        let online = state.agents_online;
        let online_style = if online > 0 {
            Style::default().fg(Theme::SAGE).bg(Theme::BG_SECONDARY)
        } else {
            Style::default()
                .fg(Theme::TEXT_GHOST)
                .bg(Theme::BG_SECONDARY)
        };
        spans.push(Span::styled(
            format!("  {online}ag"),
            online_style,
        ));

        // ISFR display: use em-dash when no data, not "--"
        let isfr_text = match state.isfr {
            Some(ratio) => format!("{:.0}%", ratio * 100.0),
            None => "\u{2014}".to_string(), // em-dash: —
        };
        let isfr_style = match state.isfr {
            Some(r) if r >= 0.8 => {
                Style::default().fg(Theme::SAGE).bg(Theme::BG_SECONDARY)
            }
            Some(r) if r >= 0.5 => {
                Style::default().fg(Theme::WARNING).bg(Theme::BG_SECONDARY)
            }
            Some(_) => Style::default().fg(Theme::EMBER).bg(Theme::BG_SECONDARY),
            None => Style::default().fg(Theme::TEXT_GHOST).bg(Theme::BG_SECONDARY),
        };
        spans.push(Span::styled(
            format!(" ISFR:{isfr_text}"),
            isfr_style,
        ));
    }

    spans.push(sep());

    // -- 7. Active agent spinner with role label ---------------
```

Note: `\u{2014}` is the em-dash character `—`. Using `"--"` is incorrect; it is two
hyphens, not the typographic em-dash. The em-dash is the conventional "no value"
indicator in the ROSEDUST theme (used elsewhere for missing numeric values).

### Step 7: Update or verify the header bar test

**File:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/header_bar.rs`

The existing test creates `TuiState::from_dashboard_data(&data)` and renders the header.
Since the new fields default to `0` and `None`, the test passes without changes. Verify:

```bash
cargo test -p roko-cli -- header_bar
```

If the test fails because `agents_online` or `isfr` are missing from `Default` or
`from_dashboard_data()`, add them to both. They must both have zero-value defaults so
the header renders `0ag ISFR:\u{2014}` when no data exists.

Add an explicit test for the ISFR display:

```rust
#[test]
fn header_bar_isfr_shows_em_dash_when_no_data() {
    let data = DashboardData::default();
    let mut state = TuiState::from_dashboard_data(&data);
    state.isfr = None;

    let area = Rect::new(0, 0, 120, 1);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    // Render header bar -- should not panic
    // The buffer contains the rendered text; inspect it if needed:
    let backend = ratatui::backend::TestBackend::new(120, 1);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|frame| {
        crate::tui::widgets::header_bar::render(frame, area, &data, &state, &Theme::default());
    }).unwrap();
    // If the render completed without panic, the em-dash path is exercised.
}

#[test]
fn header_bar_isfr_shows_percentage_when_data_exists() {
    let data = DashboardData::default();
    let mut state = TuiState::from_dashboard_data(&data);
    state.isfr = Some(0.75); // 75%

    // Render without panic is the minimum bar.
    // The test verifies the Some(0.75) path does not cause a format error.
    let backend = ratatui::backend::TestBackend::new(120, 1);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let area = ratatui::layout::Rect::new(0, 0, 120, 1);
    terminal.draw(|frame| {
        crate::tui::widgets::header_bar::render(frame, area, &data, &state, &Theme::default());
    }).unwrap();
}
```

Adapt the import paths and function signatures to match the actual code.

### Step 8: Verify

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Check that:
1. Header bar shows `0ag ISFR:\u{2014}` when no agents are running and no gate data
2. ISFR uses `—` (em-dash, U+2014), not `--` (two hyphens)
3. When agents are active, shows the count in green (SAGE)
4. ISFR shows gate success percentage: green >=80%, amber >=50%, red <50%
5. Compact mode (width < 120) still renders without overflow or panic

## ISFR display reference

| State           | Text     | Color          |
|-----------------|----------|----------------|
| No gate data    | `—`      | TEXT_GHOST     |
| rate >= 80%     | `80%`    | SAGE (green)   |
| rate >= 50%     | `65%`    | WARNING (amber)|
| rate < 50%      | `33%`    | EMBER (red)    |

The `—` must be the Unicode em-dash (`\u{2014}`), not two ASCII hyphens (`--`). Other
numeric-display widgets in the TUI already use `\u{2014}` for "no value" and the header
should be consistent.

## Agents online display reference

| State           | Text   | Color      |
|-----------------|--------|------------|
| 0 agents active | `0ag`  | TEXT_GHOST |
| N agents active | `Nag`  | SAGE       |

## Acceptance criteria

- [ ] `TuiState` has `agents_online: usize` (default 0) and `isfr: Option<f64>` (default None)
- [ ] `agents_online` populated from `data.agents` active count in `update_from_snapshot()`
- [ ] When `data.agents` is empty, `count_online_from_files()` is used as fallback
- [ ] `isfr` computed from `data.gate_results` pass rate
- [ ] Header bar section 6b renders `Nag` with SAGE (>0) or TEXT_GHOST (0) style
- [ ] Header bar section 6b renders `ISFR:XX%` with SAGE/WARNING/EMBER by ratio
- [ ] `isfr == None` displays `ISFR:\u{2014}` (em-dash), not `ISFR:--`
- [ ] HTTP polling (if wired): 500ms timeout, fire-and-forget, does not block render
- [ ] New section appears between system metrics (section 6) and agent spinner (section 7)
- [ ] Existing header bar tests pass without modification
- [ ] `cargo clippy` clean, `cargo +nightly fmt` clean
- [ ] All existing tests pass
