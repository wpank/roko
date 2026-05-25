# M054 — Workbench Tab in TUI

## Objective
Implement the Workbench surface as a TUI tab in `roko dashboard`. The Workbench is the primary task delegation surface: it shows active Flows, agent slots, Graph topology, pending human input, and recent completions. Users can assign tasks, fill slots, adjust macros, and cancel/pause/resume Flows. This replaces the blank-chat pattern with structured task management.

## Scope
- Crates: `roko-cli`
- Files: `crates/roko-cli/src/tui/` (new or modify existing tab files)
- Phase ref: `tmp/unified-migration/03-PHASE-2-ENGINE.md` SS2.8
- Spec ref: `tmp/unified/16-SURFACES.md` SS3 (Workbench)

## Steps
1. Read the current TUI structure:
   ```bash
   ls crates/roko-cli/src/tui/
   grep -rn 'Tab\|tab\|F1\|F2\|F3' crates/roko-cli/src/tui/ --include='*.rs' | head -20
   ```

2. Read an existing tab implementation for patterns:
   ```bash
   head -80 crates/roko-cli/src/tui/agents_tab.rs 2>/dev/null || head -80 crates/roko-cli/src/tui/plan_tab.rs 2>/dev/null
   ```

3. Create or modify the Workbench tab to display:
   - **Active Flows panel**: table of running Flows with columns: name, progress %, cost, duration, status, active nodes
   - **Agent Slots panel**: grid of agents with their slot states (free/running/paused)
   - **Pending Human Input panel**: list of flows waiting for human response with urgency indicators
   - **Recent Completions panel**: last N completed flows with Verify verdicts

4. Consume StateHub projections: `FlowSummary`, `AgentStatus` for data.

5. Implement keybindings for Workbench actions:
   - `Enter` on a Flow: show detail view
   - `c` on a Flow: cancel
   - `p` on a Flow: pause/resume toggle
   - `r` on a pending human input: respond
   - `n`: new task assignment dialog

6. Wire the Workbench tab into the TUI tab bar (add as a new F-key tab or replace an existing one).

7. Write tests:
   - Workbench tab renders without panic with mock data
   - Keybinding dispatch calls correct action handlers
   - Empty state (no flows, no agents) renders gracefully

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo test -p roko-cli -- tui::workbench
# Manual: cargo run -p roko-cli -- dashboard  (then navigate to Workbench tab)
```

## What NOT to do
- Do NOT break existing TUI tabs -- add Workbench alongside them
- Do NOT implement the actual task assignment logic in the TUI -- emit SurfaceEvents that the system handles
- Do NOT add real LLM or network calls from the TUI -- TUI is a pure rendering layer
- Do NOT redesign the entire TUI layout -- add one tab at a time
