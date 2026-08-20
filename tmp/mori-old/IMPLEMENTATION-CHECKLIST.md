# Roko Self-Development Implementation Checklist

> **Purpose**: This document describes every concrete task needed to make Roko capable of
> developing itself while an operator (or Claude) monitors, diagnoses, and improves the
> system in real-time. Each item is a standalone checklist entry with full implementation
> details — written so someone with zero prior context can pick it up and execute.
>
> **Date**: 2026-08-19
>
> **Visual target**: The screenshots in `tmp/mori-old/screenshots/` show what the
> predecessor system (Mori) looked like. Roko's TUI and UX should match or exceed that
> level of polish, information density, and interactivity.

---

## Background

### What is Roko?

Roko is a Rust toolkit (~893K LOC, 35 crates) for building agents that build themselves.
The core loop: read PRDs → generate implementation plans → execute tasks via LLM agents →
validate with gates (compile, test, clippy, diff) → persist results → learn from outcomes.

The codebase lives at `/Users/will/dev/nunchi/roko/roko/`. The CLI entry point is
`crates/roko-cli/`. The HTTP control plane is `crates/roko-serve/` (~365 routes on :6677).

### What is Mori?

Mori was the predecessor system at `/Users/will/dev/uniswap/bardo/apps/mori/` (~108K LOC).
It had a polished TUI with 8 tabs, queue/wave/milestone execution, ~30 specialized agent
roles, operator recovery actions, LLM-generated failure reflections, express mode auto-fix,
and production-proven metrics (6,600 episodes, 92% routing accuracy).

### The Problem

Roko was rewritten from scratch to fix Mori's architecture (streaming, composability,
modularity). The rewrite succeeded architecturally (learned routing, 11 providers, safety
layers, 365 HTTP routes) but failed experientially (TUI is 4x larger but less functional,
no operator recovery actions, no queue/wave system, learning data fragmented across surfaces).

### The Goal

Make Roko develop itself while Claude monitors everything:

1. Claude runs `roko plan run` to execute development plans
2. Claude reads logs, endpoints, and TUI snapshots to monitor progress
3. Claude diagnoses failures and queues fixes through roko's own plan system
4. Claude captures and reviews TUI screenshots to assess visual quality
5. The whole loop feeds back — roko improves itself through its own execution

### Visual Reference

The directory `tmp/mori-old/screenshots/` contains 17 screenshots of Mori's TUI showing:
- **F1:dash** — Dashboard with plan list, agent panel, system metrics, token burn
- **F2:plans** — Plan detail with implementation/verification tasks, branch/worktree info
- **F3:agents** — Agent detail with model, provider, tokens, output
- **F4:git** — Branch tree, worktree list, commit graph
- **F5:logs** — Runtime log viewer with timestamps and categories
- **F6:cfg** — Configuration with backend defaults, per-role overrides, agent status
- **F7:inspect** — MCP runtime, AST index stats, tool/learning metrics
- **F8:queue** — Queue overview with milestones, queue order, completion status

See `tmp/mori-old/MORI-TUI-SCREENSHOTS.md` for detailed text descriptions of each.

### Detailed Analysis Documents

17 comparison documents in `tmp/mori-old/` provide deep analysis. Key ones:
- `01-MORI-TUI-ARCHITECTURE.md` — Mori's TUI: 10.3K LOC, ROSEDUST palette, VFX system
- `02-ROKO-TUI-ARCHITECTURE.md` — Roko's TUI: 44K LOC, app.rs god object
- `03-EXECUTION-MODEL-COMPARISON.md` — Queue/wave vs runner-v2
- `06-ROKO-E2E-WIRING-AUDIT.md` — Core CLI commands work end-to-end
- `11-CYBERNETIC-FEATURES-AUDIT.md` — 4 working, 8 partial
- `SYNTHESIS.md` — Master findings and priorities

---

## Phase 0: Claude Observability Infrastructure

> **Goal**: Before anything else, give Claude the ability to see what roko is doing.
> Without this, Claude can't diagnose problems, verify fixes, or assess TUI quality.
> Each item below should include continuous screenshot/snapshot generation so Claude
> can visually inspect the system at any time.

### 0.1: TUI Headless Snapshot Mode (Text + PNG)

- [ ] **Add `roko dashboard --snapshot <dir>` command**

  **What**: Render every TUI tab and sub-tab to both text and PNG files using ratatui's
  `TestBackend`, then exit. This is the foundation for all automated visual assessment.
  Claude reads text files for content analysis and PNG files for visual/layout analysis.

  **Existing infrastructure to build on**:
  - `TestBackend` is already used in unit tests (`app.rs:3583-3662`, `dashboard_view.rs`,
    `header_bar.rs`, `error_digest.rs`, `phase_compact.rs`)
  - `rendered_text()` helper already exists in test modules — extracts `Buffer` → text
  - `--text` flag already renders dashboard to plaintext via `render_dashboard_text()`
  - `Tab::ALL` const array provides all 10 tabs (F1-F10)
  - 16 page slugs exist via `parse_dashboard_page(slug)`

  **Where to implement**:
  - `crates/roko-cli/src/main.rs` — add `--snapshot` CLI flag to `Dashboard` variant (alongside
    existing `--text`, `--page`, `--list-pages`, `--high-contrast`, `--reduced-motion`)
  - `crates/roko-cli/src/tui/snapshot.rs` — new module for snapshot orchestration
  - `crates/roko-cli/src/tui/app.rs` — add `snapshot_all_tabs()` method

  **How it works**:
  1. Initialize ratatui with `TestBackend::new(width, height)` (default: 240×60, configurable
     via `--snapshot-width` and `--snapshot-height` flags)
  2. Initialize `TuiApp` in headless mode (load state from disk/serve, no crossterm)
  3. For each of the 10 tabs (F1-F10):
     a. Set the active tab
     b. Call `render()` to fill the `Buffer`
     c. Extract text via the existing `rendered_text()` pattern
     d. Write to `<dir>/f01-dashboard.txt`, `<dir>/f02-plans.txt`, etc.
     e. Convert buffer to PNG (see PNG rendering below)
     f. Write to `<dir>/f01-dashboard.png`, `<dir>/f02-plans.png`, etc.
  4. For tabs with sub-views (F1 has agents/output/diff/errors/git/mcp/procs/impl):
     a. Cycle through each sub-view
     b. Render and save as `<dir>/f01-dashboard-agents.{txt,png}`, etc.
  5. For the 16 legacy page slugs:
     a. Render each via `parse_dashboard_page(slug)` → `<dir>/page-health.{txt,png}`, etc.
  6. Write `<dir>/manifest.json` with:
     ```json
     {
       "timestamp": "2026-08-19T12:00:00Z",
       "roko_version": "0.1.0",
       "terminal_size": { "width": 240, "height": 60 },
       "tabs": [
         { "key": "f01", "name": "Dashboard", "files": { "txt": "f01-dashboard.txt", "png": "f01-dashboard.png" } },
         ...
       ],
       "sub_views": [...],
       "pages": [...]
     }
     ```

  **PNG rendering approach** (Rust-native, no external tool dependencies):
  1. Add `image` crate dependency to `roko-cli/Cargo.toml` (behind a `snapshot` feature flag)
  2. Embed a monospace font atlas (e.g., bundled Iosevka/JetBrains Mono bitmap or use
     `rusttype`/`fontdue` for runtime glyph rendering)
  3. For each cell in the `Buffer`:
     a. Map `ratatui::style::Color` → RGB pixel color
     b. Render the cell's symbol glyph at the cell's grid position
     c. Apply foreground/background colors, bold/italic/underline modifiers
  4. Write the pixel buffer as PNG via `image::ImageBuffer::save()`
  5. Target: ~14px cell width × 20px cell height → 240×60 terminal = 3360×1200px PNG

  **Acceptance**:
  - `cargo run -p roko-cli -- dashboard --snapshot /tmp/tui-snap` produces text + PNG files
    for all 10 tabs, all sub-views, and all 16 page slugs
  - Claude can `Read` any `.txt` file and understand TUI content
  - Claude can `Read` any `.png` file with vision and see the actual visual layout
  - `manifest.json` lists all captured files with metadata

  **Bonus — ANSI text mode**: Also write `<dir>/f01-dashboard.ansi` files that preserve
  ANSI color codes. These can be piped through `cat` to see colors in a terminal, or
  converted to HTML/PNG by external tools. This is cheap to produce since the `Buffer`
  already has style information.

### 0.2: Automated Full-TUI Screenshot Collection (`roko screenshot`)

- [ ] **Add `roko screenshot` top-level command**

  **What**: A dedicated command (not a dashboard sub-flag) that captures every visible
  surface of the TUI in one shot. This is the primary tool Claude/roko uses for visual
  self-assessment. It should be as simple as `roko screenshot` and produce a complete
  visual audit of every page.

  **Why a separate command**: `roko dashboard --snapshot` implies the dashboard is running.
  `roko screenshot` is explicit — it says "capture the visual state right now" without
  implying an interactive session. It's also easier for agents to discover and invoke.

  **Where to implement**:
  - `crates/roko-cli/src/screenshot.rs` — new module
  - Wire into `main.rs` as a top-level subcommand

  **CLI interface**:
  ```
  roko screenshot                           # capture all pages to .roko/screenshots/latest/
  roko screenshot --dir /tmp/snap           # capture to custom directory
  roko screenshot --tabs f1,f2,f7           # capture specific tabs only
  roko screenshot --pages health,learning   # capture specific page slugs only
  roko screenshot --format txt              # text only (faster, no image dep)
  roko screenshot --format png              # PNG only
  roko screenshot --format all              # text + PNG + ANSI (default)
  roko screenshot --size 240x60            # custom terminal size
  roko screenshot --compare <dir>           # compare to a previous snapshot (see 0.7)
  roko screenshot --label "before-palette-change"  # human-readable label in manifest
  ```

  **How it works**:
  1. Load current workspace state (same as dashboard startup but headless)
  2. If `roko serve` is running on :6677, fetch live state via HTTP
  3. If not, load from disk (`.roko/state/`, `.roko/episodes.jsonl`, etc.)
  4. Run the snapshot engine from 0.1 to render all surfaces
  5. Auto-create the output directory with timestamp:
     `.roko/screenshots/2026-08-19T12-00-00/` (default) or user-specified dir
  6. Symlink `.roko/screenshots/latest/` → most recent snapshot dir
  7. Print summary: "Captured 10 tabs, 8 sub-views, 16 pages → .roko/screenshots/latest/"

  **Integration with plan execution** (see 0.3 for continuous mode):
  - `roko plan run` can optionally call `roko screenshot` at key lifecycle points
  - `roko screenshot` can be invoked by Claude between plan steps to check progress

  **Acceptance**:
  - `roko screenshot` captures all surfaces in < 5 seconds (no real terminal needed)
  - Output is deterministic — same state produces same snapshots (modulo timestamps)
  - Claude can invoke it, read the manifest, then read specific files for assessment

### 0.3: Continuous Screenshot Collection During Execution

- [ ] **Add `--screenshots` flag to `roko plan run`**

  **What**: Automatically capture TUI snapshots at regular intervals and on significant
  events during plan execution. This creates a visual timeline that Claude can review
  to understand how the system evolved during a run.

  **Where to implement**:
  - `crates/roko-cli/src/runner/event_loop.rs` — add snapshot timer + event triggers
  - Reuse the snapshot engine from 0.1/0.2

  **CLI interface**:
  ```
  roko plan run plans/ --screenshots                      # default: every 60s + events
  roko plan run plans/ --screenshots --screenshot-interval 30   # every 30s
  roko plan run plans/ --screenshots --screenshot-dir /tmp/run  # custom output
  ```

  **Snapshot triggers**:
  | Trigger | When | Why |
  |---------|------|-----|
  | Startup | Before first task | Baseline state |
  | Interval | Every N seconds | Regular cadence |
  | Plan state change | pending→running→complete→failed | Track major transitions |
  | Gate result | Pass or failure | See gate impact on TUI |
  | Agent lifecycle | Spawn or death | Track agent panel changes |
  | Wave transition | Wave N complete, Wave N+1 starts | Track execution phases |
  | Error | Any error logged | Capture error state |
  | Completion | All plans done | Final state |

  **Output structure**:
  ```
  .roko/screenshots/run-2026-08-19T12-00-00/
    manifest.json                    # full timeline with event triggers
    000-startup/
      f01-dashboard.{txt,png}
      f02-plans.{txt,png}
      ...
    001-interval-60s/
      f01-dashboard.{txt,png}
      ...
    002-event-gate-pass-T1/
      f01-dashboard.{txt,png}
      f02-plans.{txt,png}           # most relevant for gate events
      ...
    003-event-gate-fail-T3/
      ...
    999-completion/
      ...
  ```

  **Manifest format**:
  ```json
  {
    "run_id": "run-2026-08-19T12-00-00",
    "started": "2026-08-19T12:00:00Z",
    "completed": "2026-08-19T13:30:00Z",
    "snapshots": [
      {
        "seq": 0,
        "trigger": "startup",
        "timestamp": "2026-08-19T12:00:00Z",
        "dir": "000-startup",
        "tabs_captured": 10,
        "event": null
      },
      {
        "seq": 2,
        "trigger": "event",
        "timestamp": "2026-08-19T12:05:30Z",
        "dir": "002-event-gate-pass-T1",
        "tabs_captured": 10,
        "event": { "type": "gate_pass", "plan": "fix-header", "task": "T1", "rung": 1 }
      }
    ]
  }
  ```

  **Smart capture**: To avoid excessive disk use, only capture tabs that are likely to
  have changed for each event type. Gate events → F1+F2+F10. Agent events → F1+F3.
  Interval → all tabs. Manifest records which tabs were captured per snapshot.

  **Acceptance**:
  - `roko plan run plans/ --screenshots` produces a timestamped screenshot directory
  - Claude can read `manifest.json` to find snapshots for specific events
  - After a run, Claude can review the visual timeline to assess TUI quality and correctness
  - Disk usage is bounded (default: keep last 5 runs, configurable)

### 0.4: JSON Output Mode for All CLI Commands

- [ ] **Add `--json` flag to `roko status`**

  **What**: Output machine-readable JSON instead of human-formatted text.

  **Where**: `crates/roko-cli/src/status.rs`

  **Output shape**:
  ```json
  {
    "signals": { "total": 1234, "by_type": { "completion": 500, "error": 12 } },
    "episodes": { "total": 200, "passed": 180, "failed": 20 },
    "plans": { "total": 30, "complete": 25, "running": 2, "failed": 3 },
    "agents": { "active": 2, "total_spawned": 50 },
    "costs": { "total_usd": 45.23, "by_provider": { "anthropic": 40.00, "openai": 5.23 } },
    "uptime_seconds": 3600
  }
  ```

- [ ] **Add `--json` flag to `roko doctor`**

  **Where**: `crates/roko-cli/src/doctor.rs`

  **Output shape**:
  ```json
  {
    "checks": [
      { "name": "config_valid", "status": "pass", "message": "roko.toml is valid" },
      { "name": "provider_keys", "status": "fail", "message": "ANTHROPIC_API_KEY not set" },
      { "name": "disk_space", "status": "warn", "message": "2.1 GB free (threshold: 5 GB)" }
    ],
    "summary": { "pass": 18, "warn": 1, "fail": 1 }
  }
  ```

- [ ] **Add `--json` flag to `roko learn all`**

  **Where**: `crates/roko-cli/src/learn.rs`

  **Output shape**: Structured dump of all learning state files (cascade router, gate
  thresholds, efficiency events, episodes, experiments).

- [ ] **Add `--json` flag to `roko plan list` and `roko plan show <id>`**

  **Where**: `crates/roko-cli/src/plan.rs`

  **Output shape**: Plan metadata, task list with status, gate results, agent assignments.

- [ ] **Add `--json` flag to `roko agent list`**

  **Where**: `crates/roko-cli/src/agent.rs`

  **Output shape**: Agent name, role, model, status, token usage, PID, plan assignment.

### 0.5: `roko diagnose <plan-id>` Command

- [ ] **Create the `diagnose` subcommand**

  **What**: A single command that gives Claude (or a human) everything needed to understand
  why a plan failed. This is the #1 missing debugging tool.

  **Where**: New file `crates/roko-cli/src/diagnose.rs`, wire into `main.rs`

  **What it outputs** (always JSON, this is for machines):
  ```json
  {
    "plan_id": "fix-tui-header",
    "status": "failed",
    "phase": "implementation",
    "iteration": 2,
    "failed_task": {
      "id": "T3",
      "name": "Update header bar layout",
      "agent_role": "implementer",
      "model": "claude-sonnet-4-6",
      "provider": "anthropic",
      "attempt": 3,
      "error_class": "compile_error",
      "error_summary": "cannot find type `HeaderBar` in module `tui`"
    },
    "gate_results": [
      { "rung": 1, "gate": "compile", "status": "fail", "errors": 3, "warnings": 12 },
      { "rung": 2, "gate": "test", "status": "skip", "reason": "blocked by compile failure" }
    ],
    "classified_errors": [
      {
        "file": "crates/roko-cli/src/tui/header.rs",
        "line": 45,
        "type": "TypeMismatch",
        "message": "expected `HeaderBar`, found `StatusBar`",
        "suggestion": "HeaderBar was renamed to StatusBar in commit abc123"
      }
    ],
    "git_state": {
      "branch": "roko/plan/fix-tui-header",
      "worktree": ".roko/worktrees/fix-tui-header",
      "dirty_files": 3,
      "commits_ahead": 2
    },
    "suggested_recovery": [
      "Retry with updated type name: StatusBar",
      "Check if header.rs was modified by another plan"
    ],
    "episode_ids": ["ep-001", "ep-002", "ep-003"],
    "total_cost_usd": 2.45
  }
  ```

  **How it works**:
  1. Read the state snapshot (`.roko/state/state-snapshot.json`)
  2. Find the plan by ID and extract its current state
  3. Read the latest gate output for the failed task
  4. Classify errors using `roko-gate`'s `GateFailureClassification`
  5. Read git state for the plan's branch/worktree
  6. Generate recovery suggestions based on error classification
  7. Output as JSON

  **Acceptance**: `cargo run -p roko-cli -- diagnose fix-tui-header` outputs structured
  JSON that Claude can parse to understand the failure and decide on recovery actions.

### 0.6: Structured Log File Output

- [ ] **Add `--log-file <path>` flag to `roko plan run`**

  **What**: Write structured JSONL logs to a file that Claude can tail during execution.

  **Where**: `crates/roko-cli/src/runner/event_loop.rs`

  **Log entry format**:
  ```json
  {"ts":"2026-08-19T12:00:00Z","level":"info","source":"executor","plan":"fix-header","task":"T1","msg":"Task started","agent":"implementer","model":"claude-sonnet-4-6"}
  {"ts":"2026-08-19T12:01:30Z","level":"info","source":"gate","plan":"fix-header","task":"T1","msg":"Gate passed","rung":1,"gate":"compile","duration_ms":4500}
  {"ts":"2026-08-19T12:01:35Z","level":"error","source":"gate","plan":"fix-header","task":"T1","msg":"Gate failed","rung":2,"gate":"test","errors":3,"classified":["test_failure:tui::test_header_renders"]}
  ```

  **How it works**:
  1. If `--log-file` is provided, create a `BufWriter<File>` at that path
  2. In the event loop's `tokio::select!` arms, write JSONL entries for:
     - Task start/complete/fail
     - Agent spawn/death
     - Gate pass/fail with classified errors
     - Snapshot persistence
     - Wave/queue progression
  3. Flush after each line so Claude can tail in real-time

  **Acceptance**: `cargo run -p roko-cli -- plan run plans/ --log-file /tmp/roko.jsonl`
  produces a tailable log file. Claude reads it with `tail -f /tmp/roko.jsonl` or
  periodic `Read` calls.

### 0.7: Screenshot Diffing and Assessment

- [ ] **Add `roko screenshot --compare <dir>` flag**

  **What**: Compare a new snapshot set against a previous one and report visual differences.
  This is how Claude verifies that a TUI change achieved its goal. Also supports comparison
  against mori reference screenshots for parity assessment.

  **Where**: `crates/roko-cli/src/screenshot.rs` (same module as 0.2)

  **How it works — text diff**:
  1. Load `manifest.json` from both snapshot directories
  2. For each matching tab/page file pair, compute a line-by-line diff
  3. Output a structured diff report:
     ```json
     {
       "compared": { "before": "/tmp/before", "after": "/tmp/after" },
       "tabs": [
         {
           "tab": "f01-dashboard",
           "changed": true,
           "lines_added": 3,
           "lines_removed": 2,
           "lines_modified": 5,
           "diff_preview": "- Wave 1/7  Queue: Sprint-1\n+ Wave 2/7  Queue: Sprint-1"
         },
         { "tab": "f02-plans", "changed": false }
       ],
       "summary": { "tabs_changed": 4, "tabs_unchanged": 6, "total_lines_changed": 23 }
     }
     ```

  **How it works — PNG diff** (when both dirs have PNGs):
  1. Load both PNGs for each tab
  2. Compute per-pixel absolute difference
  3. Generate a diff image (highlighting changed regions in red)
  4. Write to `<output>/diff-f01-dashboard.png`
  5. Report percentage of pixels changed per tab

  **Usage pattern for Claude**:
  ```bash
  # 1. Capture baseline
  roko screenshot --label "before-palette-change"

  # 2. Make code changes

  # 3. Capture after
  roko screenshot --label "after-palette-change"

  # 4. Compare
  roko screenshot --compare .roko/screenshots/before-palette-change

  # 5. Claude reads the diff report to verify the visual change
  ```

  **Mori reference comparison**:
  ```bash
  # Compare roko TUI against mori reference screenshots
  roko screenshot --compare tmp/mori-old/screenshots/ --reference-mode
  ```
  In `--reference-mode`, the comparison is visual-only (PNG diff) since mori screenshots
  don't have matching text files. Claude reads the diff PNGs and assesses parity.

  **Acceptance**: `roko screenshot --compare <prev-dir>` produces a JSON diff report +
  visual diff PNGs. Claude can read the report to assess what changed and whether it
  matches the intended design.

---

## Phase 1: Execution UX (Make It Work Like Mori)

> **Goal**: Add the queue/wave/milestone system that made Mori intuitive.
> This is the highest-impact work for making roko usable for self-development.

### 1.1: Queue Manifest

- [ ] **Implement `.roko/queue.toml` support**

  **What**: A TOML file where the operator defines the execution order, milestone groups,
  and per-run settings. This is how Mori organized work — without it, roko has no way to
  express "do these plans first, then those, in this order."

  **Mori's format** (from `03-EXECUTION-MODEL-COMPARISON.md`):
  ```toml
  [queue]
  name = "Self-Development Sprint 1"
  mode = "parallel"       # parallel | sequential | express
  max_agents = 10
  max_parallel_plans = 5

  [[milestones]]
  name = "TUI Foundations"
  plans = ["fix-tui-data-model", "port-rosedust-palette", "add-snapshot-mode"]
  tags = ["tui", "critical-path"]
  description = "Fix structural TUI issues before visual work"

  [[milestones]]
  name = "Execution UX"
  plans = ["add-queue-manifest", "add-wave-computation", "add-recovery-keybindings"]
  tags = ["execution", "ux"]
  description = "Port Mori's queue/wave system"

  [[milestones]]
  name = "Observability"
  plans = ["add-json-output", "add-diagnose-command", "add-log-file"]
  tags = ["observability"]
  depends_on = ["TUI Foundations"]

  [session]
  conductor_model = "claude-sonnet-4-6"
  express = true           # skip strategist, auto-fix on gate failure
  auto_merge = false
  ```

  **Where to implement**:
  - `crates/roko-core/src/config/` — add `QueueConfig` types
  - `crates/roko-cli/src/runner/` — add queue loading and milestone ordering
  - `crates/roko-cli/src/plan.rs` — wire queue into `roko plan run`

  **How it works**:
  1. At `roko plan run` startup, look for `.roko/queue.toml`
  2. Parse milestones and validate all referenced plans exist
  3. Build a milestone dependency graph
  4. Feed the ordered plan list into the existing runner-v2 event loop
  5. Report milestone-level progress in addition to plan-level progress

  **Acceptance**: `roko plan run --queue .roko/queue.toml` executes plans in milestone
  order. `roko status --json` shows milestone progress.

### 1.2: Wave Computation

- [ ] **Add Kahn's algorithm for plan-level parallelism waves**

  **What**: Compute which plans can run in parallel based on their dependencies.
  Group them into "waves" — Wave 0 has no dependencies, Wave 1 depends on Wave 0, etc.
  This is the core of Mori's execution model and the primary thing shown in the TUI.

  **Where to implement**:
  - `crates/roko-cli/src/runner/` — new `wave.rs` module
  - Uses `depends_on` from tasks.toml files and milestone dependencies from queue.toml

  **Algorithm**:
  1. Build a directed graph: plan → plans it depends on
  2. Topological sort via Kahn's algorithm
  3. Group plans into waves by their topological depth
  4. Within each wave, plans run in parallel (up to `max_parallel_plans`)
  5. Wave N starts only when all plans in Wave N-1 are complete

  **Output** (for TUI and JSON):
  ```
  Wave 0 (3 plans): fix-tui-data-model, add-json-output, add-log-file
  Wave 1 (2 plans): port-rosedust-palette, add-snapshot-mode
  Wave 2 (1 plan):  add-recovery-keybindings
  ```

  **Acceptance**: Wave computation runs at plan startup. `roko plan list --waves` shows
  the wave grouping. The TUI dashboard shows `Wave 0 (3/3)` in the header.

### 1.3: Express Mode

- [ ] **Add `--express` flag to `roko plan run`**

  **What**: Skip strategist/review agents and auto-fix on gate failure. Mori saved
  40-60% wall-clock time with this mode. Essential for batch development runs.

  **Where**: `crates/roko-cli/src/runner/event_loop.rs`

  **How it works**:
  1. When `--express` is set, skip strategist and review phases in the task pipeline
  2. On gate failure (compile/test/clippy):
     a. Extract error digest (top 10 errors, grouped by file)
     b. Run `cargo fix --allow-dirty` for fixable lint/format errors
     c. If cargo fix doesn't resolve it, dispatch an auto-fixer agent with the error digest
     d. Retry up to 3 times before marking task as failed
  3. On task failure after 3 auto-fix attempts, move to next task (fail-forward)

  **Acceptance**: `roko plan run plans/ --express` runs significantly faster than normal
  mode. Gate failures trigger auto-fix attempts visible in logs.

### 1.4: TUI Recovery Keybindings

- [ ] **Add 5 operator recovery actions to the TUI**

  **What**: Mori had `s` (retry), `z` (diagnose), `S` (repair), `R` (clean-slate repair),
  `c` (reverify) keybindings when a plan/task was selected. Roko's TUI is currently
  read-only — operators can only watch.

  **Where**: `crates/roko-cli/src/tui/app.rs` — key event handler

  **Keybindings**:
  | Key | Action | Implementation |
  |-----|--------|----------------|
  | `s` | Soft retry | Re-queue the selected failed task, preserving completed work |
  | `z` | Diagnose | Run `diagnose` logic and display results in a modal |
  | `S` | Repair | Re-queue with error context injected into the agent prompt |
  | `R` | Clean slate | Reset task state, delete worktree, start fresh |
  | `c` | Reverify | Re-run gates only (compile/test/clippy) without re-implementing |

  **How it works**:
  1. When the user presses a recovery key with a task selected:
  2. Send a `TuiAction` variant (add `RetryTask`, `DiagnoseTask`, `RepairTask`,
     `CleanSlateTask`, `ReverifyTask` to the 72-variant enum)
  3. The action handler sends a message to the runner event loop via a channel
  4. The runner re-queues or re-gates the task
  5. The TUI updates to show the task's new state

  **Context-sensitive hints**: Show recovery keybindings in the bottom status bar only
  when a failed task is selected, like Mori did.

  **Acceptance**: Select a failed task in the TUI, press `s` to retry. The task re-enters
  the execution pipeline. The status bar shows available recovery keys.

### 1.5: LLM Failure Reflections

- [ ] **Add LLM-generated failure analysis on gate failures**

  **What**: When a gate fails, fire a background call to a fast model (Haiku/Sonnet)
  asking "What failed? Why? What should the next attempt do differently?" Store the
  reflection and inject it into the retry prompt.

  **Where**:
  - `crates/roko-cli/src/runner/event_loop.rs` — gate failure handler
  - `crates/roko-learn/src/` — reflection storage

  **How it works**:
  1. On gate failure, extract the error digest (classified errors from `roko-gate`)
  2. Fire a background async task to call a fast model with:
     ```
     You are analyzing a gate failure for task "{task_name}" in plan "{plan_name}".
     The compile/test gate produced these errors:
     {error_digest}

     Analyze:
     1. What specifically failed?
     2. Why did it fail?
     3. What should the next implementation attempt do differently?
     4. Which files should it focus on?
     ```
  3. Store the reflection in `.roko/learn/reflections.jsonl`
  4. When the task is retried, inject the last 3 reflections into the system prompt
  5. Deduplicate by error line to avoid repeating the same reflection

  **Acceptance**: After a gate failure, a reflection appears in the learning state.
  When the task retries, the agent prompt includes "Previous attempts failed because..."

### 1.6: Preflight Checks at Plan-Run Startup

- [ ] **Run doctor subset before plan execution**

  **What**: Validate critical prerequisites before starting execution. Fail fast with
  clear messages instead of failing mid-execution.

  **Where**: `crates/roko-cli/src/runner/event_loop.rs` — startup sequence

  **Checks to run**:
  1. Config file exists and is valid (`roko.toml`)
  2. At least one LLM provider has valid credentials
  3. Sufficient disk space (> 2 GB free)
  4. Git repo is clean or in a known state
  5. Plans directory exists and contains valid tasks.toml files
  6. No stale lock files from crashed previous runs
  7. Rust toolchain is available (`rustc --version` succeeds)

  **How it works**:
  1. Run checks sequentially at startup
  2. Categorize results as PASS / WARN / FAIL
  3. If any FAIL: print the failures and exit with error code
  4. If any WARN: print warnings and continue
  5. Write preflight results to the log file (for Claude to read)

  **Acceptance**: `roko plan run plans/` with missing API keys prints
  "Preflight FAIL: ANTHROPIC_API_KEY not set" and exits before spawning any agents.

---

## Phase 2: TUI Quality (Make It Look Like Mori)

> **Goal**: Port Mori's visual design system and fix structural TUI issues.
> Use `roko screenshot` (Phase 0) to verify each change visually.
>
> **Process**: For every item in this phase:
> 1. `roko screenshot --label "before-<change-name>"`
> 2. Make the code change
> 3. `roko screenshot --label "after-<change-name>"`
> 4. `roko screenshot --compare .roko/screenshots/before-<change-name>`
> 5. Claude reads the diff report + PNGs to verify the visual change
> 6. Claude compares to mori reference: `roko screenshot --compare tmp/mori-old/screenshots/ --reference-mode`

### 2.1: Unify TUI Data Models

- [ ] **Merge `DashboardData` and `TuiState` into one model**

  **What**: The TUI currently has two parallel data models (`DashboardData` for the
  connected mode and `TuiState` for the standalone mode) bridged by a conversion
  function. This causes bugs where one model updates but the other doesn't.

  **Where**: `crates/roko-cli/src/tui/state.rs` (5,290 LOC), `app.rs` (4,576 LOC)

  **How to fix**:
  1. Identify all fields in both models
  2. Create a single `TuiModel` that serves both modes
  3. In standalone mode, populate from disk reads
  4. In connected mode, populate from `DashboardSnapshot` events
  5. Remove the bridge function
  6. Update all rendering code to read from `TuiModel`

  **Acceptance**: Only one data model exists. The TUI shows the same data in both
  standalone and connected modes.

### 2.2: Resolve Two Parallel Page Systems

- [ ] **Pick one page system and remove the other**

  **What**: Two page systems overlap: `PageId`/`PageScaffold` (text mode) and
  `Tab`/`SubView` (ratatui). This creates confusion about which system owns rendering.

  **Where**: `crates/roko-cli/src/tui/`

  **Recommendation**: Keep `Tab`/`SubView` (the ratatui system) since that's the
  actual rendering path. Remove or archive `PageId`/`PageScaffold`.

### 2.3: Port ROSEDUST Color Palette

- [ ] **Replace roko's color scheme with Mori's ROSEDUST palette**

  **What**: Mori used warm rose-tinted greys with true black backgrounds, only 3
  semantic status colors, and no pure white. This gave it a distinctive, professional look.

  **Mori's palette** (from `01-MORI-TUI-ARCHITECTURE.md`):
  - Background: true black
  - Base text: warm rose-tinted greys
  - SAGE green: success/complete
  - WARNING amber: warnings/in-progress
  - EMBER red-orange: errors/failures
  - BONE (#D7C69E): highest-priority emphasis (used sparingly)
  - DREAM (#7873A5): cool contrast (indigo)
  - No pure white anywhere
  - HSV-based gradient lookup tables for smooth color transitions

  **Where**: `crates/roko-cli/src/tui/` — likely a `colors.rs` or `theme.rs` file

  **Acceptance**: TUI snapshots show the ROSEDUST palette. Compare to mori screenshots.

### 2.4: Port Information-Dense Header Bar

- [ ] **Rebuild the header bar to match Mori's density**

  **What**: Mori's header was a single row packed with: pulsing heartbeat dot, app name,
  wave progress, queue info, 15-char gradient progress bar, ETA, elapsed time, cost,
  token counts, MCP status, compact system metrics, and F-key tab indicators.

  **Roko's current header**: Unknown — check current implementation and compare.

  **Target layout** (from screenshots):
  ```
  ● roko  Wave 1/7  Queue: Sprint-1  [████████░░]  83%  ETA:2m44s  13m51s  MCP:0  C:41% M:37G N:↑8M D:R616K
    F1:dash F2:plans F3:agents F4:git F5:logs F6:cfg F7:inspect F8:queue
  ```

  **Components to implement**:
  1. Pulsing dot (green=running, yellow=paused, red=error)
  2. Wave progress: `Wave N/M`
  3. Queue name from queue.toml
  4. 15-character gradient progress bar (using ROSEDUST colors)
  5. ETA calculation (based on average task duration × remaining tasks)
  6. MCP call count
  7. Compact system metrics: CPU%, MEM, NET, DSK
  8. F-key tab strip with active tab highlighted

  **Acceptance**: Header bar matches mori screenshots. Compare side-by-side.

### 2.5: Port Plan Tree Widget

- [ ] **Rebuild the plan tree to show Wave→Plan hierarchy**

  **What**: Mori's plan tree was the most complex widget (1,078 lines). It showed
  collapsible waves with plans grouped inside, progress bars, health indicators,
  iteration counts, and wave blocker chains.

  **Target layout** (from screenshots):
  ```
  Wave 0 (29/35)  ██░░  v20
    ✓ 01-workspace-scaffold    11/11  ████  i3
    ✓ 04-terminal-scaffold     10/10  ████  i3
    ► 94-m1-integration gate    7/7   ████  i3  13m
  Wave 1 (3/3)
    ✓ plan-a                    5/5   ████  i2
  ```

  **Where**: `crates/roko-cli/src/tui/widgets/` — plan list widget

  **Acceptance**: Plan tree shows waves, plan names, task counts, progress bars,
  iteration counts, and elapsed time. Matches mori's F1 left panel.

### 2.6: Port Error Digest Widget

- [ ] **Add an error digest panel to the dashboard**

  **What**: Mori aggregated errors from four sources (gate output, pipeline errors,
  preflight warnings, runtime issues) into one panel. The panel border turned red
  when errors existed.

  **Where**: `crates/roko-cli/src/tui/widgets/` — new `error_digest.rs`

  **Acceptance**: Errors from all sources appear in one panel. Border turns red on errors.

### 2.7: Add Adaptive Frame Rate

- [ ] **Drop TUI frame rate when agents are busy**

  **What**: Mori ran at 60fps when idle and dropped to ~20fps when agents were active
  and the user wasn't interacting. This reduced CPU usage during long runs.

  **Where**: `crates/roko-cli/src/tui/app.rs` — event loop tick interval

  **How**: Track last user input time and agent activity. If agents are active and
  no user input for 5 seconds, switch to 50ms tick (20fps). Resume 16ms (60fps) on
  any key/mouse event.

### 2.8: Add Exponential Smoothing to Metrics

- [ ] **Smooth all numeric displays to prevent visual jumps**

  **What**: Mori applied exponential smoothing (alpha ~0.12 per frame) to all metric
  displays (CPU%, token counts, progress percentages). This prevented jarring visual
  jumps when values changed suddenly.

  **Where**: `crates/roko-cli/src/tui/` — wherever metrics are rendered

  **Formula**: `displayed = displayed * (1 - alpha) + actual * alpha` each frame.

### 2.9: Context-Sensitive Keybind Hints

- [ ] **Change status bar hints based on active tab and selection**

  **What**: Mori showed different keybind hints depending on which tab was active AND
  what was selected within that tab. The F2:plans tab showed retry/diagnose/repair only
  when a failed plan was selected.

  **Where**: `crates/roko-cli/src/tui/` — status bar rendering

### 2.10: Content-Aware Tab Badges

- [ ] **Show counts on inactive tab labels**

  **What**: Mori showed `e:Errors(3)` or `a:Agents(2)` on inactive sub-tabs so
  operators could see important counts without switching tabs.

  **Where**: `crates/roko-cli/src/tui/` — tab bar rendering

---

## Phase 3: Observability (See Everything Like Mori's F7)

> **Goal**: Build a single-pane view where Claude can see all learning/runtime metrics.

### 3.1: Single-Pane Inspect View

- [ ] **Build an F7:inspect equivalent in roko's TUI**

  **What**: Mori's F7 showed MCP runtime, AST index stats, and all learning metrics
  in one dense view. Roko's equivalent data is fragmented across CLI commands, HTTP
  routes, and partially-rendered TUI tabs.

  **Target layout** (from mori screenshot):
  ```
  ┌─MCP Runtime─────────────┐ ┌─AST Index──────────┐ ┌─Tool / Learning──────────┐
  │ codex:on claude:on       │ │ files 6.1k         │ │ episodes 6.6k ok / 0 fail│
  │ task route T1            │ │ symbols 153.6k     │ │ playbook 98 total        │
  │ routing 92%              │ │ resolved 285.3k    │ │ routing 1.6k/1.7k (92%)  │
  │                          │ │ routing 92% ████   │ │ model claude-opus 100%   │
  │                          │ │                    │ │   129s avg $1.43/run     │
  └──────────────────────────┘ └────────────────────┘ └──────────────────────────┘
  ```

  **Data sources** (all exist in roko already):
  - Episodes: `.roko/episodes.jsonl`
  - Playbook: `.roko/learn/playbook.json`
  - Routing: `.roko/learn/cascade-router.json`
  - Gate thresholds: `.roko/learn/gate-thresholds.json`
  - Efficiency: `.roko/learn/efficiency.jsonl`
  - MCP: tool call counts from runtime
  - Index: from `roko-index` if running

  **Acceptance**: F7 tab shows all learning metrics in one view. Claude can snapshot it
  and assess the system's learning state.

### 3.2: Per-Model/Provider Cost Stats

- [ ] **Add cost tracking display to TUI**

  **What**: Mori's F7 showed per-model and per-provider statistics including pass rate,
  average duration, retry rate, tokens per run, and cost per run.

  **Data source**: `.roko/learn/cascade-router.json` already contains this data.

  **Where**: Include in the F7 inspect view.

### 3.3: Full Prompt Text Logging

- [ ] **Store complete prompt text per invocation**

  **What**: Mori stored the full prompt text for every agent invocation. Roko stores
  section metadata but not the raw text. Full text is essential for debugging prompt
  issues.

  **Where**: `crates/roko-cli/src/runner/event_loop.rs` — after prompt assembly

  **How**: Write the assembled prompt to `.roko/learn/prompt-logs/{episode-id}.txt`.
  Include a configurable retention policy (e.g., keep last 100).

---

## Phase 4: Self-Development Loop

> **Goal**: Wire everything together so Claude can run `roko plan run` on
> development plans and monitor/fix the system through its own tools.

### 4.1: CLI Plan Control (Non-TUI)

- [ ] **Add `roko plan retry <plan-id> [task-id]`**

  **What**: Retry a failed plan or specific task from the CLI, without needing the TUI.
  Essential for Claude to act on failures.

- [ ] **Add `roko plan pause` and `roko plan resume`**

  **What**: Pause/resume execution from the CLI. Currently only possible via TUI `p` key.

- [ ] **Add `roko plan cancel <plan-id>`**

  **What**: Cancel a running plan from the CLI.

### 4.2: Backlog-to-Plan Pipeline

- [ ] **Create a `roko backlog` subcommand**

  **What**: Convert markdown backlog items (like those in `tmp/backlog/`) into PRD ideas
  that feed the plan generation pipeline.

  **How**:
  1. `roko backlog import tmp/backlog/100-cli-error-message-quality.md`
  2. Parses the markdown, extracts title, description, acceptance criteria
  3. Creates a PRD idea via the existing `roko prd idea` mechanism
  4. Optionally triggers `roko prd draft` and `roko prd plan` to generate the full plan

  **This closes the loop**: Claude reads analysis docs → creates backlog items →
  imports them into roko → roko generates plans → roko executes plans → Claude monitors.

### 4.3: Automated Visual Assessment Loop

- [ ] **Wire `roko screenshot` into the self-development feedback loop**

  **What**: An automated workflow where Claude/roko captures screenshots, assesses visual
  quality against mori references, identifies gaps, generates fix plans, executes them,
  and verifies the result — all through roko's own tooling.

  **The Loop** (each step is a real CLI command):
  ```bash
  # 1. Capture current state
  roko screenshot --label "baseline"

  # 2. Claude reads the manifest + specific tabs
  # (Read .roko/screenshots/baseline/manifest.json)
  # (Read .roko/screenshots/baseline/f01-dashboard.png — vision)
  # (Read .roko/screenshots/baseline/f01-dashboard.txt — content)

  # 3. Claude compares to mori reference screenshots
  roko screenshot --compare tmp/mori-old/screenshots/ --reference-mode

  # 4. Claude identifies gaps → creates a plan
  roko prd idea "Port ROSEDUST palette: current TUI uses default colors"
  roko prd draft new "port-rosedust-palette"
  roko prd plan port-rosedust-palette

  # 5. Execute the plan with continuous screenshots
  roko plan run plans/port-rosedust-palette/ --express --screenshots

  # 6. After execution, capture + compare
  roko screenshot --label "after-palette-fix"
  roko screenshot --compare .roko/screenshots/baseline

  # 7. Claude reads the diff report to verify improvement
  # (Read .roko/screenshots/after-palette-fix/diff-report.json)

  # 8. If not satisfied → repeat from step 4 with refined plan
  ```

  **Integration with plan execution** (automated, no manual steps):
  - `roko plan run --screenshots` captures at startup, events, and completion (Phase 0.3)
  - After each plan completes, Claude reads the final screenshot set
  - The `--screenshots` output directory is recorded in the state snapshot for later review
  - A post-run summary includes a screenshot assessment score based on mori parity metrics

  **Mori reference screenshots** (`tmp/mori-old/screenshots/`):
  These 17 PNGs serve as the visual ground truth. Claude uses them to assess:
  - Information density (how much data per screen area)
  - Color palette adherence (ROSEDUST warm rose-tinted greys)
  - Layout structure (header bar, plan tree, error digest, agent panels)
  - Interactivity cues (keybind hints, active tab indicators, selection state)
  - Text descriptions in `MORI-TUI-SCREENSHOTS.md` provide acceptance criteria

  **Key**: The `roko screenshot` command makes visual inspection a **non-interactive,
  deterministic, automatable** operation. No real terminal is needed. Claude can run it
  from any context (CLI, plan execution, HTTP endpoint) and get a complete visual audit.

### 4.4: Endpoint-Based Monitoring

- [ ] **Document the HTTP monitoring workflow for Claude**

  **What**: While `roko serve` is running (port 6677), Claude can monitor everything
  via HTTP endpoints:

  ```bash
  # Health check
  curl localhost:6677/api/health

  # Plan status
  curl localhost:6677/api/plans

  # Active agents
  curl localhost:6677/api/agents

  # Learning state
  curl localhost:6677/api/learning/episodes
  curl localhost:6677/api/learning/routing
  curl localhost:6677/api/learning/playbook

  # Real-time events (SSE)
  curl -N localhost:6677/api/events

  # Gate results
  curl localhost:6677/api/gates/results

  # System metrics
  curl localhost:6677/api/metrics
  ```

  All ~365 routes serve real data. See `tmp/mori-old/16-ROKO-HTTP-ROUTES-AUDIT.md`
  for the complete route catalog.

---

## Phase 5: Cleanup

> **Goal**: Pay down tech debt and remove unused code.

### 5.1: Decompose God Objects

- [ ] **Split `event_loop.rs` (~23K lines)**

  Extract into: queue manager, agent dispatcher, gate runner, state persister,
  learning recorder, experiment manager, error handler.

- [ ] **Split `app.rs` (4,576 lines)**

  Extract into: key handler, action dispatcher, I/O coordinator, snapshot manager.

- [ ] **Split `state.rs` (5,290 lines)**

  Extract into: plan state, agent state, learning state, system metrics state.

- [ ] **Split `dashboard.rs` (7,445 lines)**

  Extract into: header widget, plan panel, agent panel, metrics panel, sub-tab manager.

### 5.2: Decide on Partial Cybernetic Features

For each PARTIAL feature (from `11-CYBERNETIC-FEATURES-AUDIT.md`):

- [ ] **CorticalState (2,717 LOC)** — Wire into runner dispatch or remove
- [ ] **Inference Gateway** — Route runner-v2 through it or document as HTTP-only
- [ ] **Agent Groups coordination** — Implement coordination modes or remove enum variants
- [ ] **Cross-Cut Functors** — Wire into main dispatch or keep gate-failure-only

### 5.3: Remove Unused Code

- [ ] **Archive advanced HDC math** (TDA, tropical algebra, sheaf Laplacian) — exists but unused
- [ ] **Remove duplicate page system** — after 2.2 is done

---

## Completion Criteria

All items above are complete when:

1. **Claude can run `roko plan run` and monitor everything** — via JSON CLI output,
   structured log files, HTTP endpoints, and TUI snapshots
2. **Claude can diagnose failures** — via `roko diagnose`, error classification,
   and LLM reflections
3. **Claude can fix problems** — via `roko plan retry`, backlog import, and
   plan generation
4. **Claude can verify visual quality** — by comparing TUI snapshots to mori
   screenshots in `tmp/mori-old/screenshots/`
5. **The TUI looks and feels like Mori** — ROSEDUST palette, information-dense header,
   wave/plan tree, error digest, recovery keybindings
6. **Everything works end-to-end** — no "built but not wired" features
7. **The self-development loop runs autonomously** — roko develops itself through
   its own plan execution system while Claude observes and steers
