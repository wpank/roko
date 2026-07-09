#!/usr/bin/env bash
# TUI Gap Implementation — Overnight-safe Codex agent runner.
#
# Designed to run unattended: each item is isolated via git commits,
# failures auto-revert, timeouts prevent hangs, health gates prevent cascades.
#
# Usage:
#   ./run-agents.sh                # Run all items on a fresh branch
#   ./run-agents.sh 0.1            # Run single item
#   ./run-agents.sh 0.1 0.2 1.1   # Run specific items
#   ./run-agents.sh --phase 0      # Run all Phase 0 items
#   ./run-agents.sh --dry-run      # Print prompts without running
#   ./run-agents.sh --continue     # Resume from last successful commit
#   ./run-agents.sh --worktree-dir /path  # Custom worktree location
#
# Prerequisites:
#   npm install -g @openai/codex
#   export OPENAI_API_KEY=sk-...
#
# Safety:
#   - Runs in an isolated git worktree (never touches your main working tree)
#   - Reuses the same worktree across runs to avoid disk bloat
#   - Shares CARGO_TARGET_DIR with main repo (no duplicate build artifacts)
#   - Commits after each successful item
#   - Reverts working tree on failure (git checkout .)
#   - 10-minute timeout per agent (configurable)
#   - Health gate (cargo check) before each item
#   - Skips items whose dependencies failed
#   - Never pushes, never touches main, never deletes worktrees

set -uo pipefail
# NOTE: no -e because we handle errors explicitly

ROOT="/Users/will/dev/nunchi/roko/roko"
LOGS_DIR="$ROOT/tmp/tui/logs"
RESULTS_FILE="$ROOT/tmp/tui/results.md"
DRY_RUN=false
CONTINUE=false
ITEMS_TO_RUN=()
PHASE_FILTER=""

# Codex config
CODEX_MODEL="o3"
CODEX_REASONING="high"
CODEX_SANDBOX="workspace-write"
ITEM_TIMEOUT=600       # 10 minutes per item
HEALTH_CHECK_TIMEOUT=120  # 2 minutes for cargo check

# Worktree config — all work runs in an isolated worktree
WORKTREE_BASE=""  # Set via --worktree-dir, defaults to $ROOT/.roko/worktrees
WORKTREE_NAME="tui-agents"

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)    DRY_RUN=true; shift ;;
    --continue)   CONTINUE=true; shift ;;
    --phase)      PHASE_FILTER="$2"; shift 2 ;;
    --model)      CODEX_MODEL="$2"; shift 2 ;;
    --reasoning)  CODEX_REASONING="$2"; shift 2 ;;
    --sandbox)    CODEX_SANDBOX="$2"; shift 2 ;;
    --timeout)    ITEM_TIMEOUT="$2"; shift 2 ;;
    --worktree-dir) WORKTREE_BASE="$2"; shift 2 ;;
    *)            ITEMS_TO_RUN+=("$1"); shift ;;
  esac
done

mkdir -p "$LOGS_DIR"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; DIM='\033[2m'; RESET='\033[0m'

log()     { echo -e "${CYAN}[$(date +%H:%M:%S)]${RESET} $*"; }
ok()      { echo -e "${GREEN}[PASS]${RESET} $*"; }
fail()    { echo -e "${RED}[FAIL]${RESET} $*"; }
warn()    { echo -e "${YELLOW}[WARN]${RESET} $*"; }
section() { echo -e "\n${BOLD}═══════════════════════════════════════════════════${RESET}"; }

# Track which phases have had failures (for dependency skipping)
declare -A PHASE_FAILED
declare -A ITEM_STATUS

###############################################################################
# Preflight checks
###############################################################################

cd "$ROOT" || { echo "Cannot cd to $ROOT"; exit 1; }

if ! command -v codex &>/dev/null; then
  echo -e "${RED}Error: codex CLI not found.${RESET}"
  echo "Install: npm install -g @openai/codex"
  exit 1
fi

if ! command -v cargo &>/dev/null; then
  echo -e "${RED}Error: cargo not found. Need Rust toolchain.${RESET}"
  exit 1
fi

# ── Worktree setup (never touches main working tree) ─────────────────────────
# All agent work happens in an isolated worktree. Your main checkout, including
# any uncommitted changes, is left completely untouched.
#
# Storage: reuses the same worktree path across runs and shares CARGO_TARGET_DIR
# with the main repo, so build artifacts aren't duplicated (~GB saved).
# Worktrees are NEVER deleted automatically — clean up manually if needed.

WORKTREE_BASE="${WORKTREE_BASE:-$ROOT/.roko/worktrees}"
WORK_DIR="$WORKTREE_BASE/$WORKTREE_NAME"
BRANCH="tui-fixes-$(date +%Y%m%d)"

# ── Cargo build config (minimize disk usage) ──────────────────────────────────
# Share build artifacts with main repo to avoid duplicating target/ (~GB saved)
export CARGO_TARGET_DIR="$ROOT/target"
# Disable incremental compilation — agents do one-off builds, incremental caches
# just waste disk space (can be hundreds of MB per crate)
export CARGO_INCREMENTAL=0
# Cap parallel rustc jobs to reduce peak disk and memory usage during builds
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

if $CONTINUE; then
  if [[ ! -d "$WORK_DIR" ]]; then
    fail "No worktree at $WORK_DIR to continue from."
    exit 1
  fi
  log "Resuming in existing worktree: $WORK_DIR"
  cd "$WORK_DIR"
  BRANCH="$(git branch --show-current 2>/dev/null || echo "$BRANCH")"
  log "On branch: $BRANCH"
else
  if [[ -d "$WORK_DIR" ]]; then
    # Reuse existing worktree — create a fresh branch from main repo's HEAD
    log "Reusing existing worktree: $WORK_DIR"
    cd "$WORK_DIR"
    MAIN_HEAD="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null)"
    if git rev-parse --verify "$BRANCH" &>/dev/null; then
      BRANCH="${BRANCH}-$(date +%H%M%S)"
    fi
    git checkout -b "$BRANCH" "$MAIN_HEAD" 2>/dev/null
  else
    # Create new worktree from main repo's HEAD
    mkdir -p "$(dirname "$WORK_DIR")"
    if git -C "$ROOT" rev-parse --verify "$BRANCH" &>/dev/null; then
      BRANCH="${BRANCH}-$(date +%H%M%S)"
    fi
    log "Creating worktree: $WORK_DIR (branch: $BRANCH)"
    git -C "$ROOT" worktree add -b "$BRANCH" "$WORK_DIR" HEAD
    cd "$WORK_DIR"
  fi
fi

log "Worktree: $WORK_DIR"
log "Target dir: $CARGO_TARGET_DIR (shared with main repo)"

# Guard against stray target/ in worktree (codex agents sometimes ignore env vars)
if [[ -d "$WORK_DIR/target" ]]; then
  target_size=$(du -sh "$WORK_DIR/target" 2>/dev/null | cut -f1)
  warn "Stray target/ found in worktree ($target_size) — removing to save disk"
  rm -rf "$WORK_DIR/target"
fi

###############################################################################
# Shared context for all agents
###############################################################################

SHARED_CONTEXT="
PROJECT CONTEXT:
- Workspace root: $WORK_DIR
- This is a Rust workspace with 18 crates, ~177K LOC
- TUI source: crates/roko-cli/src/tui/ (69 files, ~38K LOC)
- The TUI uses ratatui + crossterm
- Key files:
  - app.rs (2077L): Main app, event loop, draw, action dispatch
  - state.rs (1365L): TuiState with all mutable state
  - input.rs (774L): TuiAction enum, key dispatch
  - dashboard.rs (5669L): DashboardData disk loading, Theme
  - event.rs (75L): Event enum, EventHandler
  - scroll.rs (95L): ScrollAccel (currently unused)
  - views/: 7 view files (dashboard, plans, agents, git, logs, config, context)
  - widgets/: 28 widget files
  - modals/: 14 modal files
- Build: cargo check -p roko-cli
- Test: cargo test -p roko-cli

RULES:
- NEVER push, NEVER touch main branch
- NEVER modify files outside crates/roko-cli/ unless the task explicitly says to
- NEVER run 'cargo build --release' (wastes disk with optimized artifacts)
- NEVER run 'cargo doc' (generates large HTML output)
- After changes, run: cargo check -p roko-cli
- If check fails, fix errors before finishing
- Keep changes minimal and focused
- Do not add unnecessary dependencies
- Do not refactor beyond what the task requires
- CARGO_TARGET_DIR is set — do not create or reference a local target/ directory
"

###############################################################################
# Item definitions: ID -> (title, deps, prompt, verify)
###############################################################################

declare -A ITEMS

# Helper to define an item compactly
# item ID TITLE "dep1 dep2" PROMPT VERIFY
item() {
  local id="$1" title="$2" deps="$3" prompt="$4" verify="$5"
  ITEMS["${id}:title"]="$title"
  ITEMS["${id}:deps"]="$deps"
  ITEMS["${id}:prompt"]="$prompt"
  ITEMS["${id}:verify"]="$verify"
}

CARGO_VERIFY="cd $WORK_DIR && cargo check -p roko-cli 2>&1 | tail -5"
CARGO_TEST_VERIFY="cd $WORK_DIR && cargo check -p roko-cli && cargo test -p roko-cli 2>&1 | tail -10"

# ── Phase 0: Critical path ──────────────────────────────────────────────────

item "0.1" "Fix plans directory path" "" \
"$SHARED_CONTEXT
TASK: Fix the plans directory path mismatch.

PROBLEM: plans_dir() in crates/roko-cli/src/plan.rs:133-135 returns workdir.join(\".roko\").join(\"plans\")
but actual plans live at workdir.join(\"plans\") (P06-process-management/, P07-autofix-retry/, etc).

FIX: Change plans_dir() to prefer workdir.join(\"plans\") if it exists, fall back to .roko/plans/:
  pub fn plans_dir(workdir: &Path) -> PathBuf {
      let top = workdir.join(\"plans\");
      if top.is_dir() { return top; }
      workdir.join(\".roko\").join(\"plans\")
  }

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "0.2" "Fix episode file path alignment" "" \
"$SHARED_CONTEXT
TASK: Fix episode file path mismatch between TUI and orchestrator.

PROBLEM: dashboard.rs:38 has MEMORY_DIR=\".roko/memory\" and EPISODES_FILE=\"episodes.jsonl\", so TUI
reads .roko/memory/episodes.jsonl. But orchestrate.rs:4449 writes to .roko/episodes.jsonl.

FIX: Everywhere episodes_path is computed in dashboard.rs (lines 507, 620, 2836-2838, and more —
search for all MEMORY_DIR joined with EPISODES_FILE), add a fallback:
  let episodes_path = roko_dir.join(MEMORY_DIR).join(EPISODES_FILE);
  let episodes_path = if episodes_path.exists() { episodes_path }
                      else { roko_dir.join(EPISODES_FILE) };

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "0.3" "Add Mouse variant to Event enum" "" \
"$SHARED_CONTEXT
TASK: Wire mouse events through the event system.

PROBLEM: Event enum in crates/roko-cli/src/tui/event.rs has Key/Resize/Tick but no Mouse.
Mouse events hit '_ => continue' in EventHandler::next() and are silently dropped.

FIX:
1. event.rs: Add Mouse(crossterm::event::MouseEvent) to Event enum.
2. event.rs EventHandler::next(): Add match arm: crossterm::event::Event::Mouse(m) => return Ok(Event::Mouse(m))
3. app.rs main_loop() event match (~line 332): Add Event::Mouse(m) => { self.handle_mouse(m); }

handle_mouse() already exists at app.rs:~1145.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "0.4" "Fix mouse hit-test 80x24 hardcode" "0.3" \
"$SHARED_CONTEXT
TASK: Use actual terminal size for mouse hit-test instead of hardcoded 80x24.

PROBLEM: app.rs:1093-1094 uses Rect::new(0,0,80,24) for hit zones. Wrong on any real terminal.

FIX:
1. Add terminal_size: (u16, u16) field to App struct.
2. Init from crossterm::terminal::size().unwrap_or((80, 24)) in App::new_with_page().
3. Update on Event::Resize(w, h) in main_loop.
4. Use in handle_mouse(): Rect::new(0, 0, self.terminal_size.0, self.terminal_size.1)

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 1: Data flow ──────────────────────────────────────────────────────

item "1.1" "Bridge gate_results into TuiState" "" \
"$SHARED_CONTEXT
TASK: Bridge gate results from DashboardData into TuiState.

PROBLEM: DashboardData loads gate_results from disk but update_from_snapshot() never writes them
to TuiState.gate_results. The types differ (summary vs output field name).

FIX: In state.rs update_from_snapshot() after the cost section (~line 832), convert and assign:
  self.gate_results = data.gate_results.iter().map(|g| GateResultEntry {
      gate: g.gate.clone(), plan_id: g.plan_id.clone(),
      passed: g.passed, output: g.summary.clone(),
  }).collect();
Read dashboard.rs to find the exact struct/field names — they may differ from this example.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.2" "Populate token_history and token_rate" "" \
"$SHARED_CONTEXT
TASK: Populate token_history and token_rate from efficiency events.

PROBLEM: token_history (HashMap<String, VecDeque<u64>>) and token_rate (f64) never populated.
Token sparkline always shows 'waiting for data...'.

FIX: In state.rs update_from_snapshot(), build token_history by grouping data.efficiency_events
by role, collecting (input_tokens + output_tokens) per event into VecDeque capped at 60.
Compute token_rate = total_tokens / elapsed_minutes from event timestamps.

Read AgentEfficiencyEvent struct in roko-learn for exact field names.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.5" "Populate git structured fields from bg thread" "" \
"$SHARED_CONTEXT
TASK: Extract git_branch_tree/git_commit_graph/git_worktree_list from GitViewData in bg thread.

PROBLEM: Background git thread populates tui_state.git_view_data but the structured fields
git_branch_tree (Vec<GitBranchNode>), git_commit_graph (Vec<GitCommitEntry>),
git_worktree_list (Vec<String>) are never extracted from it.

FIX: In app.rs drain_background_channels() git_rx handler (~line 1400-1430), after setting
git_view_data, extract the structured fields. May need type conversions. Also fix git_age:
the bg thread sends empty string for age (app.rs:305) — compute it from git log -1 --format=%cr.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.7" "Add cross-platform system metrics" "" \
"$SHARED_CONTEXT
TASK: Replace macOS-only sys metrics with sysinfo crate.

PROBLEM: collect_sys_metrics_bg() in app.rs:1633 only works on macOS (uses 'top -l 2').
Linux returns all zeros.

FIX:
1. Add sysinfo = \"0.32\" to crates/roko-cli/Cargo.toml
2. Replace the cfg-guarded function with cross-platform sysinfo:
   use sysinfo::System;
   let mut sys = System::new();
   loop {
       sys.refresh_cpu_usage(); sys.refresh_memory();
       tx.send(SysMetrics {
           cpu_pct: sys.global_cpu_usage(),
           mem_used_bytes: sys.used_memory(),
           mem_total_bytes: sys.total_memory(),
           ..Default::default()
       }).ok();
       std::thread::sleep(Duration::from_secs(3));
   }

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.8" "Fix plan status partial progress" "" \
"$SHARED_CONTEXT
TASK: Make plan progress show intermediate values instead of binary 0%/100%.

PROBLEM: state.rs:739 sets tasks_done = task_count if completed else 0. No partial progress.
state.rs:748 sets tasks_failed = 0 always.

FIX: In update_from_snapshot(), instead of binary completion check, count actual completed tasks
from data. If DashboardData has per-task status (check PlanSummary struct in dashboard.rs),
use that. Otherwise load task-tracker files to count completed/failed per plan.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 2: Input / interaction ─────────────────────────────────────────────

item "2.1" "Render inject and filter input lines" "" \
"$SHARED_CONTEXT
TASK: Show visible input bar when user is in inject ('i') or filter ('/') mode.

PROBLEM: Typing in inject/filter mode captures text but renders NOTHING. User types blind.
Also: filter_text never copied to filter field (what plan_tree reads).

FIX:
1. In app.rs draw() after status footer, before modals, render input bar:
   if matches!(self.tui_state.input_mode, InputMode::Inject | InputMode::Filter) {
       let input_area = Rect::new(content_area.x, content_area.bottom().saturating_sub(1),
                                   content_area.width, 1);
       frame.render_widget(Clear, input_area);
       let (label, buf) = match self.tui_state.input_mode {
           InputMode::Inject => (\"inject> \", &self.tui_state.message_input),
           InputMode::Filter => (\"filter> \", &self.tui_state.filter_text),
           _ => unreachable!(),
       };
       // Render label + text + cursor using Paragraph with Spans
   }

2. In dispatch_action AcceptFilter (~line 728-731), add:
   self.tui_state.filter = self.tui_state.filter_text.clone();

You need content_area Rect from the layout — extract it if not already in scope.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.3" "Fix PageUp/PageDown to scroll by page" "" \
"$SHARED_CONTEXT
TASK: PageUp/PageDown should scroll by ~20 lines, not 1.

PROBLEM: All scroll actions call scroll_focused(+/-1). PageUp = Up = 1 line.

FIX: Add ScrollPageUp/ScrollPageDown variants to TuiAction in input.rs. Map PageUp/PageDown
to these in per-tab handlers. In dispatch_action, handle with scroll_focused(-20)/scroll_focused(20).
Also: Home -> set scroll to 0. End -> set scroll to large value.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.5" "Fix plan_scroll desync" "" \
"$SHARED_CONTEXT
TASK: Fix desync between plan_scroll_offset (key handlers write) and plan_scroll (widget reads).

PROBLEM: state.rs has both plan_scroll_offset and plan_scroll. Key handlers modify offset but
plan_tree.rs reads plan_scroll. Never synced.

FIX: Remove plan_scroll. Change plan_tree.rs to read plan_scroll_offset. Update Default,
reset_scrolls(), and all references. grep -rn plan_scroll crates/roko-cli/src/tui/

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "2.7" "Add modal key intercepts" "" \
"$SHARED_CONTEXT
TASK: Add key intercepts for modals that let keys fall through.

PROBLEM: input.rs handle_key() only intercepts task_picker/task_detail/queue_overview.
show_help, show_wave_overview, show_plan_detail have NO intercept. 'q' while help is open
quits the app.

FIX: At top of handle_key() before existing modal checks (~line 293), add:
1. if modal.show_help: Esc/?/q -> ShowHelp (toggle). Others -> None.
2. if modal.show_wave_overview: Esc/w -> ShowWaveOverview. Up/Down -> scroll. Others -> None.
3. if modal.show_plan_detail: Esc/Enter -> ClosePlanDetail. Up/Down -> ScrollDetail. Others -> None.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.13" "Fix Ctrl-C to always quit" "" \
"$SHARED_CONTEXT
TASK: Ctrl-C must force-quit from any modal/input state.

PROBLEM: Modal intercepts run before global keys. Ctrl-C returns None in inject/filter/confirm.

FIX: At very top of handle_key() in input.rs, before ALL modal checks:
  if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
      return TuiAction::Quit;
  }

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 3: Modal fixes ────────────────────────────────────────────────────

item "3.1" "Create PlanDetail modal" "2.7" \
"$SHARED_CONTEXT
TASK: Make Enter on a plan show a detail modal.

PROBLEM: show_plan_detail toggles but nothing renders — no ModalState::PlanDetail variant exists.

FIX:
1. modals/mod.rs: Add PlanDetail { plan_idx: usize, scroll_offset: usize } to ModalState.
2. modals/mod.rs render_modal(): Add match arm rendering plan name, tasks, progress.
3. app.rs ShowPlanDetail dispatch: Create ModalState::PlanDetail instead of just toggling boolean.
4. Add Esc/Up/Down key handling for the modal.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "3.2" "Populate data modals with real data" "" \
"$SHARED_CONTEXT
TASK: Pass real data to WaveOverview, QueueOverview, TaskPicker modals.

PROBLEM: app.rs dispatch_action creates all three with Vec::new(). Always empty.

FIX: Pass tui_state.execution_waves to WaveOverview, current_task_checklist to TaskPicker.
Read ModalState variants in modals/mod.rs for expected types, then map from tui_state data.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 4: Dead code cleanup ──────────────────────────────────────────────

item "4.1" "Remove dead widget files" "" \
"$SHARED_CONTEXT
TASK: Delete 13 widget files with zero callers.

DELETE from crates/roko-cli/src/tui/widgets/:
agent_grid.rs, agent_output.rs, agent_pool.rs, command_output.rs, context_gauge.rs,
phase_bar.rs, phase_timeline.rs, plan_list.rs, scrollbar.rs, status_badge.rs,
tab_bar.rs, token_bar.rs, wave_bar.rs

Also remove their mod declarations from widgets/mod.rs and any pub use from tui/mod.rs.
If cargo check finds errors, something DOES reference them — investigate.

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

# ── Phase 6: Polish ─────────────────────────────────────────────────────────

item "6.5" "Add F7 to header F-key strip" "" \
"$SHARED_CONTEXT
TASK: Add F7 (Inspect) to header bar. Currently only F1-F6 shown.

FIX: In widgets/header_bar.rs find the F-key strip array and add (\"F7\", \"inspect\").

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "6.6" "Add tracing/logging to TUI" "" \
"$SHARED_CONTEXT
TASK: Add file logging so errors are visible.

PROBLEM: Zero logging anywhere. 20+ .ok() silently swallow errors.

FIX:
1. In app.rs App::run(), set up tracing subscriber to .roko/tui.log.
2. Add tracing + tracing-subscriber deps to Cargo.toml if needed.
3. Replace 5-10 critical .ok() with .inspect_err(|e| tracing::warn!(...)).ok():
   thread spawns (app.rs:267,286,319), signal writes (app.rs:687-694,764-770).

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 1 continued: Data flow ─────────────────────────────────────────────

item "1.3" "Populate PlanEntry nested tasks" "" \
"$SHARED_CONTEXT
TASK: Populate PlanEntry.tasks Vec so expanded plans show nested task rows.

PROBLEM: In state.rs update_from_snapshot(), PlanEntry.tasks is always Vec::new() (empty).
When a plan is expanded in the plan_tree widget, no tasks are shown.

FIX: In update_from_snapshot() where PlanEntry structs are built from data.plans, also load
the plan's task definitions. DashboardData has methods or fields for per-plan tasks.

1. Search dashboard.rs for how tasks are loaded per plan. Look for TasksFile, TaskDef,
   task_parser, or any method that returns tasks for a plan ID/directory.
2. For each plan, populate the tasks Vec with TaskEntry { id, name, status, agent_id }.
3. Map task status from the task tracker or active_tasks data.
4. Also populate tasks_failed by counting tasks with status 'failed'/'error'.
5. Populate elapsed_secs from episode durations if available.

Read dashboard.rs to understand the data model. The plan has a dir field pointing to the
plan directory which contains tasks.toml.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.4" "Populate orchestrator_state and current_phase" "" \
"$SHARED_CONTEXT
TASK: Populate orchestrator_state, current_iteration, and current_phase from executor state.

PROBLEM: In state.rs, orchestrator_state is permanently 'idle', current_iteration is 0,
current_phase is empty. These are never set in update_from_snapshot().

FIX: DashboardData loads .roko/state/executor.json. In update_from_snapshot():
1. Check if data has executor state info. Look in dashboard.rs for how executor state JSON
   is parsed — search for 'executor' or 'state' fields on DashboardData.
2. Extract the orchestrator status (running/paused/idle/error).
3. Extract iteration count if available.
4. Extract current phase label from the most advanced active task.
5. Set self.orchestrator_state, self.current_iteration, self.current_phase.
6. If executor state is absent, keep defaults (idle/0/empty) — don't crash.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.6" "Unify notification systems" "" \
"$SHARED_CONTEXT
TASK: Remove the dead TuiState.notifications and use only App.notifications.

PROBLEM: Two separate Notification types exist:
- state.rs:138-151 defines Notification with level: NotificationLevel (NEVER populated)
- modals/notification.rs defines Notification with kind: NotificationKind (actually used by App)
TuiState.notifications is dead weight.

FIX:
1. Remove struct Notification and enum NotificationLevel from state.rs
2. Remove the notifications: Vec<Notification> field from TuiState
3. Remove its Default initialization and any methods referencing it
4. Fix any compilation errors from the removal
5. Make sure App.notifications (the working one in modals/) is unaffected
6. If any code references state.notifications, redirect to the App-level notifications

grep -rn 'TuiState.*notification\|state\.notification\|NotificationLevel' in the tui/ dir
to find all references.

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "1.9" "Populate parallel_agents field" "" \
"$SHARED_CONTEXT
TASK: Populate TuiState.parallel_agents from DashboardData.

PROBLEM: parallel_agents (Vec<ParallelAgentState>) in state.rs is never written. The
parallel_pool widget renders nothing.

FIX: In update_from_snapshot(), build parallel_agents from agents that are currently active:
  self.parallel_agents = self.agents.iter()
      .filter(|a| a.active)
      .map(|a| ParallelAgentState {
          agent_id: a.id.clone(),
          plan_id: a.current_plan.clone(),
          task_id: a.current_task.clone(),
          status: if a.active { \"running\".to_string() } else { \"idle\".to_string() },
          progress_pct: 0.0,
      })
      .collect();

Also check if the parallel_pool widget in views/dashboard_view.rs uses the state.parallel_agents
or builds its own. If it builds its own, that's OK — but state should also be populated for
any widget that reads it.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.10" "Populate cost_per_plan and cost_per_task" "" \
"$SHARED_CONTEXT
TASK: Populate cost_per_plan and cost_per_task from efficiency events.

PROBLEM: cost_per_plan (HashMap<String, f64>) and cost_per_task (HashMap<String, f64>) in
TuiState are empty. No cost breakdown is available.

FIX: In update_from_snapshot(), iterate data.efficiency_events and bucket costs:
  for event in &data.efficiency_events {
      // Read the actual field names from AgentEfficiencyEvent in roko-learn
      let cost = event.cost_usd; // or compute from tokens
      if let Some(plan) = &event.plan_id {
          *self.cost_per_plan.entry(plan.clone()).or_default() += cost;
      }
      if let Some(task) = &event.task_id {
          *self.cost_per_task.entry(task.clone()).or_default() += cost;
      }
  }

Read the AgentEfficiencyEvent struct in roko-learn/src/efficiency.rs to get exact field names.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "1.11" "Fix phase pipeline to use real phase data" "" \
"$SHARED_CONTEXT
TASK: Replace the synthetic phase pipeline heuristic with real data.

PROBLEM: build_phase_pipeline() in state.rs:925-971 uses a position-based midpoint heuristic
to assign done/active/pending status to phases. This is completely fake.

FIX: Map actual task statuses to canonical phases. The 9 canonical phases are:
preflight, strategist, implementer, compile-gate, test-gate, reviewing, critic-review,
verdict, committing.

In build_phase_pipeline():
1. Look at active_tasks from DashboardData
2. For each task, determine which canonical phase it maps to (use task status, kind, or
   the episode_to_phase_name() helper that already exists)
3. Mark a phase as Done if all tasks for that phase completed
4. Mark as Active if any task for that phase is running
5. Mark as Failed if any task for that phase failed
6. Mark as Pending otherwise
7. Compute pct from the ratio of done/total tasks for that phase

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 2 continued: Input / interaction ───────────────────────────────────

item "2.2" "Wire filter_active into plan_tree rendering" "" \
"$SHARED_CONTEXT
TASK: Make the filter actually filter the plan tree when filter_active is true.

PROBLEM: filter_active and filter (the filter text) exist in TuiState but plan_tree.rs
may not use filter_active to actually hide non-matching plans.

FIX: In widgets/plan_tree.rs, find where plans are iterated for rendering. When
state.filter_active is true and state.filter is non-empty, filter the plan list to only
show plans whose name contains the filter text (case-insensitive).

1. Read plan_tree.rs to find the rendering loop
2. Add a filter step: if state.filter_active, skip plans where
   !plan.name.to_lowercase().contains(&state.filter.to_lowercase())
3. Update the plan count display to show filtered/total (e.g., '3/10 filtered')
4. Make sure selected_plan_idx is clamped to the filtered list

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.4" "Wire ScrollAccel into scroll handlers" "" \
"$SHARED_CONTEXT
TASK: Use the existing ScrollAccel for held-key scroll acceleration.

PROBLEM: scroll.rs has a complete ScrollAccel struct (1x->2x->4x->8x within 300ms) that is
exported but never instantiated. App has no ScrollAccel field. All scrolling is 1 line/key.

FIX:
1. Add scroll_accel: super::scroll::ScrollAccel field to App struct
2. Initialize it with ScrollAccel::new() in App::new_with_page()
3. In dispatch_action, for scroll actions (ScrollFocusedUp/Down, ScrollAgentUp/Down,
   ScrollLogUp/Down, ScrollDiffUp/Down), call:
     let delta = self.scroll_accel.push(direction);
   where direction is -1 or 1, and use the returned delta instead of hardcoded 1.
4. Read scroll.rs to understand the API — ScrollAccel::push() takes a direction and returns
   an accelerated delta based on timing.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.6" "Fix output_scroll vs agent_scroll desync" "" \
"$SHARED_CONTEXT
TASK: Remove the dead output_scroll field and use agent_scroll everywhere.

PROBLEM: state.rs has both agent_scroll: Option<usize> (modified by key handlers) and
output_scroll: usize (read by some widgets). They represent the same concept but are never
synchronized.

FIX:
1. Remove output_scroll from TuiState
2. Remove its Default initialization and reset_scrolls() entry
3. Search for all reads of output_scroll in the tui/ directory
4. Replace with agent_scroll.unwrap_or(0) or the appropriate Option handling
5. grep -rn output_scroll crates/roko-cli/src/tui/ to find all references

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "2.8" "Make focus cycling tab-aware" "" \
"$SHARED_CONTEXT
TASK: Tab/BackTab should only cycle through focus zones that exist on the current tab.

PROBLEM: FocusZone::next()/prev() in input.rs always cycles through all 5 zones
(PlanTree, TaskProgress, AgentOutput, CommandOutput, RightPanel) regardless of which tab
is active. On Logs/Config/Inspect tabs, this cycles through invisible zones.

FIX: Modify FocusZone::next() and prev() to take the active Tab as a parameter:
  pub fn next(&self, tab: Tab) -> Self {
      match tab {
          Tab::Dashboard => // full 5-zone cycle
          Tab::Plans => // PlanTree <-> RightPanel
          Tab::Agents => // AgentOutput <-> RightPanel
          Tab::Git | Tab::Logs | Tab::Config | Tab::Inspect => *self, // single zone, no cycle
      }
  }

Update the callers in dispatch_action (FocusNext/FocusPrev) to pass self.tui_state.active_tab.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.9" "Add visible focus indicator" "" \
"$SHARED_CONTEXT
TASK: Show which panel has focus by changing its border color.

PROBLEM: Tab cycling changes the internal focus zone but most panels have no visible indicator.
Only plan_tree shows a subtle title text change.

FIX: In each view file (dashboard_view.rs, plans_view.rs, agents_view.rs), when rendering
panels, check if tui_state.focus matches the panel's zone. If focused, use an accent-colored
border instead of the default muted border.

Example pattern for each panel's Block:
  let border_style = if tui_state.focus == FocusZone::PlanTree {
      Style::default().fg(MoriTheme::ROSE)
  } else {
      Style::default().fg(MoriTheme::GHOST)
  };
  let block = Block::default().borders(Borders::ALL).border_style(border_style);

Apply this to: plan_tree panel, task_progress panel, agent_output panel, command_output panel,
and the right panel in each view.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.10" "Fix DrillIn/DrillOut to be tab-specific" "" \
"$SHARED_CONTEXT
TASK: Left/Right/h/l should do different things depending on the active tab.

PROBLEM: DrillIn/DrillOut in dispatch_action always toggles plans[selected].expanded
regardless of active tab. On Git tab, Left/Right should navigate git branches, not plans.

FIX: In dispatch_action for DrillIn/DrillOut, check self.tui_state.active_tab:
  TuiAction::DrillIn => match self.tui_state.active_tab {
      Tab::Dashboard | Tab::Plans => { /* expand selected plan — existing code */ }
      Tab::Git => { /* increment git_branch_cursor or expand git tree node */ }
      Tab::Inspect => { /* expand inspect node */ }
      _ => {} // no drill on other tabs
  }
Same for DrillOut but with collapse/decrement.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.11" "Fix Logs End/G to scroll log pane" "" \
"$SHARED_CONTEXT
TASK: End/G on the Logs tab should scroll to the bottom of the log, not the agent pane.

PROBLEM: input.rs handle_logs_key maps End/G to ScrollAgentEnd, which resets agent_scroll
(the agent output pane), not log_scroll (the log viewer).

FIX: In input.rs handle_logs_key, change the End/G mapping:
  KeyCode::End | KeyCode::Char('G') => TuiAction::ScrollLogEnd,

Add ScrollLogEnd variant to the TuiAction enum if it doesn't exist.

In dispatch_action, handle ScrollLogEnd:
  TuiAction::ScrollLogEnd => {
      self.tui_state.log_scroll = usize::MAX; // will be clamped during render
  }

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.12" "Add scroll upper-bound clamping" "" \
"$SHARED_CONTEXT
TASK: Prevent scrolling past the end of content.

PROBLEM: No scroll field has an upper bound. Users can scroll into empty space below content.

FIX: In the scroll_focused() method in app.rs, after adjusting the scroll offset, clamp it:
  // After incrementing, clamp to reasonable max
  // We don't know exact content height here, but we can cap at a large reasonable value
  // and let widgets do fine clamping during render.

Better approach: in each widget that reads scroll, clamp at render time:
  let scroll = state.plan_scroll_offset.min(total_lines.saturating_sub(visible_height));

Check plan_tree.rs, task_progress.rs, agents_view.rs render_output_body, logs_view.rs —
add .min(max_scroll) wherever a scroll offset is read.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.14" "Add missing plan operation key bindings" "" \
"$SHARED_CONTEXT
TASK: Add key bindings for plan operations that have ConfirmAction variants but no keys.

PROBLEM: ConfirmAction enum has DiagnosePlan, RepairPlanPreserve, RepairPlanClean, GitReconcile,
MergePlan, MergeBatchToMain, MergeAllDone, IngestTask — but none have key bindings.

FIX: In input.rs handle_plans_key, add:
  KeyCode::Char('d') => TuiAction::RequestConfirm(ConfirmAction::DiagnosePlan),
  KeyCode::Char('m') => TuiAction::RequestConfirm(ConfirmAction::MergePlan),
  KeyCode::Char('M') => TuiAction::RequestConfirm(ConfirmAction::MergeAllDone),

In handle_global_key or handle_dashboard_key, add:
  Ctrl-G => TuiAction::RequestConfirm(ConfirmAction::GitReconcile)

Make sure RequestConfirm sets up the confirm dialog properly in dispatch_action. It should
set input_mode to Confirm, store the action in pending_confirm, and create the modal.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "2.15" "Add visual pause indicator" "" \
"$SHARED_CONTEXT
TASK: Make TogglePause show a visible PAUSED indicator.

PROBLEM: Pressing 'p' toggles pipeline_run_state between 'paused' and 'running' but no widget
reads this field. Nothing changes visually.

FIX: In widgets/header_bar.rs or widgets/status_bar.rs, check state.pipeline_run_state:
  if state.pipeline_run_state == \"paused\" {
      // Render a PAUSED badge in warning color
      spans.push(Span::styled(\" PAUSED \", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)));
  }

Put this in the header bar next to the heartbeat dot, or in the status bar. The user should
see a clear PAUSED label when they press 'p'.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 3 continued: Modal fixes ──────────────────────────────────────────

item "3.3" "Unify modal systems into one" "" \
"$SHARED_CONTEXT
TASK: Remove the triple modal tracking system and use a single source of truth.

PROBLEM: Three parallel systems track modals:
1. active_modal: Option<ModalState> in App
2. show_plan_detail, show_help, show_wave_overview, etc. booleans in TuiState
3. overlay: Option<OverlayState> in App (legacy)

These can go out of sync. Some modals are tracked by both 1 and 2 simultaneously.

FIX: Pick active_modal (Option<ModalState>) as THE source of truth.
1. For each show_* boolean in TuiState, check if there's a corresponding ModalState variant.
   If not, create one (e.g., ModalState::Help, ModalState::None).
2. Replace all show_* = true with self.active_modal = Some(ModalState::Xxx).
3. Replace all show_* checks with matches!(self.active_modal, Some(ModalState::Xxx)).
4. Remove the show_* booleans from TuiState.
5. Remove the legacy overlay: Option<OverlayState> and handle_overlay_key.
6. Update ModalVisibility to read from active_modal.
7. Update has_modal() to check active_modal.is_some().
8. Update dismiss_all_modals() to set active_modal = None.

This is a large refactor — be thorough. Search for every show_plan_detail, show_help,
show_wave_overview, show_agent_pool_modal, show_queue_overview, show_task_detail,
show_task_picker in the entire tui/ directory.

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "3.4" "Fix TaskDetail modal rendering" "" \
"$SHARED_CONTEXT
TASK: Wire the task_detail modal to actually render.

PROBLEM: show_task_detail boolean is toggled and input.rs intercepts keys for it, but no
ModalState::TaskDetail variant exists. The modal is invisible — keys are consumed but nothing
renders.

FIX:
1. Add TaskDetail { task_idx: usize, scroll_offset: usize } variant to ModalState
2. In dispatch_action for ShowTaskDetail, create the variant
3. In render_modal(), add a match arm that renders task name, status, elapsed time,
   agent assignment, gate results for this task
4. Read the task data from tui_state.current_task_checklist[task_idx]

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 4 continued: Dead code cleanup ─────────────────────────────────────

item "4.2" "Remove dead help.rs modal file" "" \
"$SHARED_CONTEXT
TASK: Delete modals/help.rs which has a broken import and is never called.

PROBLEM: modals/help.rs uses crate::tui::mori_theme::MoriTheme — a module path that doesn't
exist. It's never called from app.rs (which uses its own render_help_overlay).

FIX:
1. Delete crates/roko-cli/src/tui/modals/help.rs
2. Remove 'mod help;' from modals/mod.rs
3. Remove any pub use of help from modals/mod.rs or tui/mod.rs
4. Verify that app.rs render_help_overlay and help_lines() (the working help) are unaffected

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "4.3" "Remove duplicate TuiState fields" "" \
"$SHARED_CONTEXT
TASK: Remove dead/duplicate fields from TuiState.

These fields are confirmed dead:
- selected_plan (duplicate of selected_plan_idx — only selected_plan_idx is used by dispatch)
- cumulative_cost_usd (duplicate of cost_dollars — same value)
- log_messages (Vec<LogEntry> — logs_view builds its own from DashboardData)
- plan_summary_scroll (never modified by any key handler)
- task_detail_scroll (never modified by any key handler)
- config_scroll_offset (never modified by any key handler)

For the LogEntry struct (state.rs:154-160), also remove it if nothing uses it after
removing log_messages.

FIX:
1. Remove each field from the TuiState struct
2. Remove from Default impl
3. Remove from reset_scrolls() if present
4. Fix compilation errors — grep for each field name in tui/ directory

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "4.4" "Remove legacy rendering path" "" \
"$SHARED_CONTEXT
TASK: Remove the 2656-line legacy render_dashboard function from widgets/mod.rs.

PROBLEM: widgets/mod.rs contains a massive render_dashboard function that is only reachable
via draw_legacy() which is never triggered. All tab rendering goes through views/mod.rs.

FIX:
1. In widgets/mod.rs, find and remove the render_dashboard function (it's ~2656 lines)
2. Remove any helper functions only used by render_dashboard
3. Keep the module declarations for the live widgets
4. If the function is the only thing in the file besides mod declarations, the file
   should become much smaller

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 5: Widget & view completion ────────────────────────────────────────

item "5.1" "Add ANSI color parsing to agent output" "" \
"$SHARED_CONTEXT
TASK: Parse ANSI escape sequences in agent output and render with correct colors.

PROBLEM: Agent output is rendered as plain text in agents_view.rs and dashboard_view.rs.
ANSI color codes (e.g., \\x1b[31m for red) appear as raw text or are stripped.

FIX: Create a simple ANSI parser function in a new file or in agents_view.rs:
  fn parse_ansi_line(line: &str) -> Vec<Span> {
      // Split on \\x1b[ sequences
      // Map ANSI codes to ratatui Style:
      //   30-37 = foreground colors (black, red, green, yellow, blue, magenta, cyan, white)
      //   40-47 = background colors
      //   1 = bold, 0 = reset
      // Return Vec<Span> with appropriate styles
  }

Use this in render_output_body() when building Lines from output text.
A simple regex-based approach works: split on '\\x1b\\[([0-9;]*)m' and apply styles.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.2" "Add auto-tail scroll pinning to agent output" "" \
"$SHARED_CONTEXT
TASK: Agent output should auto-scroll to the latest line unless the user scrolled up.

PROBLEM: agent_scroll is Option<usize> where None = auto-tail and Some(n) = pinned.
But the rendering in agents_view.rs may not implement this correctly.

FIX:
1. In agents_view.rs render_output_body(), check state.agent_scroll:
   - If None: scroll to the end (set offset to max(0, total_lines - visible_height))
   - If Some(n): use n as the scroll offset
2. When new output arrives (in update_from_snapshot), if agent_scroll is None, it stays None
   (auto-tail continues). If Some, it stays pinned.
3. ScrollAgentUp should set agent_scroll to Some(current - 1) — pinning it.
4. ScrollAgentEnd should set agent_scroll to None — resuming auto-tail.
5. Add a visual indicator: '[TAIL]' in the output panel title when auto-tailing,
   '[PINNED line N]' when manually scrolled.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.3" "Complete Plans tab right panel" "" \
"$SHARED_CONTEXT
TASK: Add a right-side detail panel to the Plans tab (F2).

PROBLEM: Plans tab is a single-column flat list. Mori has two columns: wave/plan list on
the left and plan detail on the right.

FIX: In views/plans_view.rs:
1. Split the layout into 35%/65% left/right columns
2. Left column: the existing plan tree with wave grouping
3. Right column: detail for the selected plan:
   - Plan name and status at top
   - Task list with status icons
   - Gate results for this plan
   - Timing breakdown (if available)
   - Progress bar
4. Read the selected plan from tui_state.plans[tui_state.selected_plan_idx]
5. Show task detail from the plan's tasks Vec (populated by item 1.3)

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.4" "Complete Dashboard MCP sub-tab" "" \
"$SHARED_CONTEXT
TASK: Make the MCP sub-tab (index 5) on the Dashboard show real MCP data.

PROBLEM: The MCP sub-tab shows placeholder text like 'input tokens: 0' with no real
MCP server health or configuration data.

FIX: In views/dashboard_view.rs, find the MCP sub-tab renderer. Show:
1. MCP server config from roko.toml (agent.mcp_config path)
2. If the MCP config file exists, parse and display:
   - Server names and their command/args
   - Server count
3. Show efficiency summary (already available from data.efficiency):
   - Total input/output tokens
   - Total cost
   - Model usage breakdown
4. Show cascade router model stats if available from data.cascade_router

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.5" "Complete Dashboard Processes sub-tab" "" \
"$SHARED_CONTEXT
TASK: Make the Processes sub-tab (index 6) on the Dashboard show process data.

PROBLEM: The Processes sub-tab shows 'no tracked processes' with no real data.

FIX: In views/dashboard_view.rs, find the Processes sub-tab renderer. Show:
1. Active agents from tui_state.agents (already populated)
2. For each active agent: id, role, model, current task, token counts, status
3. Process table with columns: PID/ID | Role | Model | Task | Tokens | Status
4. If no agents active, show the existing empty state message
5. Summary footer: 'N active / M total agents, X tokens, \$Y cost'

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.6" "Add diff panel with real git diffs" "" \
"$SHARED_CONTEXT
TASK: Show actual git diffs in the Dashboard Diff sub-tab.

PROBLEM: diff_panel shows agent output text filtered for +/-/@@/diff lines, not real diffs.
Often empty.

FIX: In views/dashboard_view.rs, for the Diff sub-tab:
1. Instead of filtering agent output, run git diff to get real diff content
2. Add a helper function that runs: git diff HEAD (for unstaged changes) or
   git diff --cached (for staged changes)
3. Store the diff output and pass it to diff_panel widget
4. Add basic syntax highlighting: + lines in green, - lines in red, @@ in cyan,
   diff headers in bold
5. Cache the diff output and refresh it when data refreshes (not every frame)

Running git subprocess on every frame would be too expensive, so compute it in the
background data thread or cache it.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.7" "Add log level filtering" "" \
"$SHARED_CONTEXT
TASK: Add interactive log level filtering to the Logs tab (F5).

PROBLEM: Logs view renders all entries with level-based coloring but has no way to filter
by level interactively.

FIX: In views/logs_view.rs:
1. Add a LogFilter struct or field to TuiState for active log levels:
   pub log_filter_levels: HashSet<String> or similar
2. Add key bindings in input.rs handle_logs_key:
   '1' = toggle INFO, '2' = toggle WARN, '3' = toggle ERROR, '4' = toggle DEBUG
   'a' = show all levels
3. When rendering log entries, skip entries whose level is not in the active set
4. Show the active filter in the status bar: '[INF] [WRN] [ERR]' with active ones highlighted
5. Default: show all levels

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "5.8" "Add config live reload" "" \
"$SHARED_CONTEXT
TASK: Make config changes take effect immediately after save without restarting TUI.

PROBLEM: Config view saves to roko.toml but the App doesn't re-read the config. Changes
require restarting the TUI.

FIX: In dispatch_action for ConfigSave (after writing to roko.toml):
1. Re-read the config from roko.toml
2. Update relevant App fields (model settings, MCP config path, etc.)
3. Refresh the DashboardData snapshot to pick up new config
4. Push a notification: 'Config reloaded'

The config is loaded via roko_core::config - check how it's parsed and what fields
the App uses from it.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 6 continued: Rendering / polish ────────────────────────────────────

item "6.1" "Enable PostFX pipeline via config toggle" "" \
"$SHARED_CONTEXT
TASK: Make PostFX visual effects toggleable via keyboard and config.

PROBLEM: EffectsConfig::default() sets all fields to false. No config key or keyboard toggle.
The entire postfx_pipeline.rs is unreachable.

FIX:
1. Add a keyboard toggle: Ctrl-E toggles self.fx_config.screen_postfx
2. In config_meta.rs, add 'tui.effects.screen_postfx' boolean field so it can be saved
3. When screen_postfx is true, the existing postfx_pipeline::apply_pipeline() in draw()
   should activate (it's already conditionally called — just needs the flag to be true)
4. Also enable dim_overlay for modals (already working) and modal_glow if screen_postfx on

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "6.2" "Unify theme systems" "" \
"$SHARED_CONTEXT
TASK: Consolidate Theme (dashboard.rs) and MoriTheme (rosedust.rs) into one system.

PROBLEM: Theme struct in dashboard.rs is used by modals and app.rs. MoriTheme in rosedust.rs
is used by widgets. Same colors but different API surfaces.

FIX: The simplest approach — make Theme delegate to MoriTheme constants:
1. In dashboard.rs Theme::dark(), use MoriTheme constants for the colors:
   foreground: Color::Rgb(MoriTheme::TEXT.0, MoriTheme::TEXT.1, MoriTheme::TEXT.2),
   accent: MoriTheme::ROSE_COLOR, etc.
2. Or: add methods to MoriTheme that return Style objects matching what Theme provides
3. The goal is that changing a color in one place changes it everywhere
4. Don't break existing code — both types can coexist if they reference the same values

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "6.3" "Fix dual Atmosphere instances" "" \
"$SHARED_CONTEXT
TASK: Remove the duplicate Atmosphere and use a single instance.

PROBLEM: Both app.atmosphere and tui_state.atmosphere exist. Both are ticked in the event
loop. Widgets read from tui_state.atmosphere. PostFX reads from app.atmosphere.

FIX:
1. Remove the atmosphere field from App struct
2. Use only tui_state.atmosphere everywhere
3. In draw(), pass &self.tui_state.atmosphere where the PostFX pipeline needs it
4. In main_loop tick handling, only tick tui_state.atmosphere (remove app.atmosphere.tick())
5. grep -rn 'self.atmosphere' crates/roko-cli/src/tui/app.rs to find all references to
   the App-level atmosphere and replace with self.tui_state.atmosphere

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "6.4" "Add message throttle to event loop" "" \
"$SHARED_CONTEXT
TASK: Limit messages processed per tick to prevent render starvation.

PROBLEM: drain_background_channels() processes ALL queued messages with no limit. Rapid
orchestrator output can starve rendering.

FIX: In drain_background_channels() in app.rs, add a counter:
  const MAX_MESSAGES_PER_DRAIN: usize = 20;
  let mut count = 0;
  while let Ok(data) = data_rx.try_recv() {
      // process...
      count += 1;
      if count >= MAX_MESSAGES_PER_DRAIN { break; }
  }

Apply the same pattern to sys_rx and git_rx channels. Remaining messages will be processed
on the next tick (~16ms later).

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "6.7" "Replace stringly-typed status with enums" "" \
"$SHARED_CONTEXT
TASK: Replace raw string status comparisons with proper enums.

PROBLEM: 90+ sites compare status via string literals like 'running', 'active', 'done',
'completed', 'passed', 'failed', 'error', 'pending', 'queued'. No compile-time checking.
Multiple strings mean the same thing ('done' = 'completed' = 'passed').

FIX:
1. In state.rs, define enums:
   pub enum AgentStatus { Active, Idle, Done, Failed }
   pub enum TaskStatus { Pending, Active, Done, Failed, Blocked }
   pub enum PlanPhase { Pending, Active, Done, Failed }

2. Implement From<&str> for each enum to parse existing string statuses:
   impl From<&str> for TaskStatus {
       fn from(s: &str) -> Self {
           match s {
               \"done\" | \"completed\" | \"passed\" => Self::Done,
               \"running\" | \"active\" | \"executing\" => Self::Active,
               \"failed\" | \"error\" => Self::Failed,
               \"blocked\" => Self::Blocked,
               _ => Self::Pending,
           }
       }
   }

3. Change the status fields on AgentRow, TaskRow, PlanEntry from String to the enum types
4. Update update_from_snapshot() to use .into() or From::from() when building these structs
5. Update widgets/views that compare status strings to use enum matches instead

This is a large change — do it incrementally. Start with TaskStatus (most impactful),
then PlanPhase, then AgentStatus.

VERIFY: cargo check -p roko-cli && cargo test -p roko-cli" \
"$CARGO_TEST_VERIFY"

item "6.8" "Replace hardcoded 200K context limit" "" \
"$SHARED_CONTEXT
TASK: Use real model context limits instead of hardcoded 200,000.

PROBLEM: context_limit is hardcoded to 200,000 in 5 places: state.rs:771, agents_view.rs:246,
dashboard_view.rs:218-222, and config_meta.rs:219.

FIX:
1. In state.rs update_from_snapshot(), look up the model's context limit from config:
   - If agent has a model field, map it to a known context limit
   - Common limits: claude-haiku=200K, claude-sonnet=200K, claude-opus=200K,
     gpt-4o=128K, gemini-pro=1M
2. Add a helper function: fn model_context_limit(model: &str) -> u64 that returns the
   appropriate limit based on model name substring matching
3. Replace all 200_000 hardcodes with calls to this function or the agent's stored limit
4. Default to 200_000 only when model is unknown/empty

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

item "6.9" "Add SmoothedValues for metric display" "" \
"$SHARED_CONTEXT
TASK: Add EMA smoothing to metrics so they don't jump between frames.

PROBLEM: System metrics (CPU, memory, token rate) jump between raw values each refresh.
Mori uses SmoothedValues with EMA for fluid animation.

FIX: Add a simple EMA helper:
  struct SmoothedValue { current: f64, alpha: f64 }
  impl SmoothedValue {
      fn new(alpha: f64) -> Self { Self { current: 0.0, alpha } }
      fn update(&mut self, sample: f64) -> f64 {
          self.current = self.alpha * sample + (1.0 - self.alpha) * self.current;
          self.current
      }
  }

Add SmoothedValue fields to TuiState for: cpu_pct, token_rate, cost_rate.
In update_from_snapshot() and drain_background_channels(), call .update() instead of
direct assignment.

VERIFY: cargo check -p roko-cli" \
"$CARGO_VERIFY"

# ── Phase 7: Critical bug fixes from audit ───────────────────────────────────

item "7.1" "Call build_token_history from update_from_snapshot" "" \
"$SHARED_CONTEXT
TASK: Wire the existing build_token_history() and compute_token_rate() into update_from_snapshot().

Use subagents to search the codebase and make changes in parallel where possible.

CONTEXT:
- File: crates/roko-cli/src/tui/state.rs
- build_token_history() exists at line ~1612. It takes &[AgentEfficiencyEvent] and returns HashMap<String, VecDeque<u64>>.
- compute_token_rate() exists at line ~1630. It takes the same and returns f64.
- update_from_snapshot() at line ~911 NEVER calls either function.
- TuiState.token_history (line ~685) stays empty forever.
- The token_sparkline widget in widgets/token_sparkline.rs reads state.token_history and shows 'waiting for data...' when empty.

WHAT TO DO:
1. Read state.rs to find update_from_snapshot() and locate where cost/token fields are set (look for self.token_total, self.cost_dollars).
2. After those lines, add:
     self.token_history = build_token_history(&data.efficiency_events);
     if self.token_rate == 0.0 {
         self.token_rate = compute_token_rate(&data.efficiency_events);
     }
3. Read build_token_history() and compute_token_rate() to verify the parameter type matches data.efficiency_events.
4. If there's a type mismatch, adapt the call (e.g., pass a slice reference).

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -n 'build_token_history\|compute_token_rate' crates/roko-cli/src/tui/state.rs should show the functions being CALLED (not just defined)." \
"$CARGO_VERIFY"

item "7.2" "Fix plans_dir in workspace_paths.rs" "" \
"$SHARED_CONTEXT
TASK: Ensure ALL plans_dir() functions check ./plans/ before .roko/plans/.

Use subagents to search all definitions and callers in parallel.

CONTEXT:
- There are potentially TWO plans_dir functions in the codebase:
  1. crates/roko-cli/src/plan.rs:133 — this one was fixed to check ./plans/ first
  2. crates/roko-cli/src/workspace_paths.rs — may still return only .roko/plans/
- The TUI's DashboardData in dashboard.rs calls one of these to find plans.
- If it calls the wrong one, the dashboard shows zero plans.

WHAT TO DO:
1. Run: grep -rn 'fn plans_dir\|plans_dir(' crates/roko-cli/src/ — find ALL definitions and callers.
2. Check workspace_paths.rs — if it defines plans_dir(), does it check ./plans/ first?
3. Check dashboard.rs load_plan_summaries() — which plans_dir() does it import/call?
4. Fix any version that returns only .roko/plans/ to also check ./plans/:
     pub fn plans_dir(workdir: &Path) -> PathBuf {
         let top = workdir.join(\"plans\");
         if top.is_dir() { return top; }
         workdir.join(\".roko\").join(\"plans\")
     }
5. Better yet: if there are two definitions, delete one and have the other re-export or call the canonical version in plan.rs.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  # Only ONE plans_dir function should exist, and it should check ./plans/ first:
  grep -rn 'fn plans_dir' crates/roko-cli/src/ | wc -l  # should be 1
  grep -A5 'fn plans_dir' crates/roko-cli/src/plan.rs | grep 'join(\"plans\")' # should show top-level check" \
"$CARGO_VERIFY"

item "7.3" "Fix WaveNext/WavePrev to use execution_waves.len" "" \
"$SHARED_CONTEXT
TASK: Fix wave navigation to wrap at correct boundary.

CONTEXT:
- File: crates/roko-cli/src/tui/app.rs
- Line ~1175-1182: WaveNext and WavePrev handlers compute max from plans.len(), not execution_waves.len().
- This means wave index wraps at the wrong boundary.

WHAT TO DO:
1. Open app.rs and find the TuiAction::WaveNext and TuiAction::WavePrev handlers.
2. In BOTH handlers, change:
     let max = self.tui_state.plans.len().max(1);
   to:
     let max = self.tui_state.execution_waves.len().max(1);

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  grep 'execution_waves.len()' crates/roko-cli/src/tui/app.rs | grep -c max  # should be >= 2" \
"$CARGO_VERIFY"

item "7.4" "Preserve wave expanded state across refreshes" "" \
"$SHARED_CONTEXT
TASK: Stop waves from reopening every 500ms when data refreshes.

CONTEXT:
- File: crates/roko-cli/src/tui/state.rs
- build_execution_waves() at line ~1391 hardcodes expanded: true (lines ~1408, ~1436).
- update_from_snapshot() calls build_execution_waves() every refresh cycle.
- User collapses a wave → 500ms later it reopens because expanded is reset to true.
- Plan expanded state IS preserved (there's already a prev_expanded HashMap pattern for plans).

WHAT TO DO:
1. In update_from_snapshot(), BEFORE the line that calls build_execution_waves(), save wave state:
     let prev_wave_expanded: std::collections::HashMap<usize, bool> = self.execution_waves
         .iter().map(|w| (w.index, w.expanded)).collect();
2. After build_execution_waves() returns and sets self.execution_waves, restore:
     for wave in &mut self.execution_waves {
         if let Some(&exp) = prev_wave_expanded.get(&wave.index) {
             wave.expanded = exp;
         }
     }
3. Find where build_execution_waves is called in update_from_snapshot and add the save/restore around it.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -c 'prev_wave_expanded' crates/roko-cli/src/tui/state.rs  # should be >= 2" \
"$CARGO_VERIFY"

item "7.5" "Remove duplicate modal intercept checks in input.rs" "" \
"$SHARED_CONTEXT
TASK: Remove unreachable duplicate modal checks.

CONTEXT:
- File: crates/roko-cli/src/tui/input.rs
- In handle_key(), modal checks happen TWICE:
  - First set at lines ~362-368: show_help (362), show_wave_overview (365), show_plan_detail (368)
  - Second set at lines ~380-386: show_wave_overview (380), show_plan_detail (383), show_help (386)
- The second set is UNREACHABLE dead code because the first set returns early.

WHAT TO DO:
1. Open input.rs and find handle_key().
2. Find the SECOND occurrence of these checks (around lines 380-386).
3. Delete those 3 duplicate if-blocks entirely.
4. Keep the FIRST occurrence (around lines 362-368) intact.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  grep -c 'show_help' crates/roko-cli/src/tui/input.rs  # should decrease by at least 1
  # And no duplicate handler function calls" \
"$CARGO_VERIFY"

item "7.6" "Fix Logs tab PageUp/PageDown to use page scroll" "" \
"$SHARED_CONTEXT
TASK: Make PageUp/PageDown in Logs tab scroll by a page instead of one line.

CONTEXT:
- File: crates/roko-cli/src/tui/input.rs
- In handle_logs_key(), PageUp/PageDown currently map to ScrollLogUp/ScrollLogDown (single-line).
- All OTHER tabs correctly use ScrollPageUp/ScrollPageDown for page-sized scrolling.
- ScrollPageUp/ScrollPageDown are already defined as TuiAction variants and handled in dispatch_action with a 20-line delta.

WHAT TO DO:
1. In input.rs, find handle_logs_key().
2. Find the PageUp and PageDown key mappings.
3. Change ScrollLogUp to ScrollPageUp and ScrollLogDown to ScrollPageDown.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  grep -A1 'PageUp' crates/roko-cli/src/tui/input.rs | grep -c ScrollPageUp  # should be >= 1 for logs" \
"$CARGO_VERIFY"

item "7.7" "Fix efficiency event timestamp in logs view" "" \
"$SHARED_CONTEXT
TASK: Fix efficiency events appearing at wrong position in unified log.

Use subagents to read both the logs_view and the AgentEfficiencyEvent struct in parallel.

CONTEXT:
- File: crates/roko-cli/src/tui/views/logs_view.rs
- Efficiency events are added to the unified BTreeMap log using event.wall_time_ms as timestamp.
- But wall_time_ms is a DURATION (e.g., 5000 = 5 seconds), NOT a Unix timestamp.
- This causes efficiency events to sort near position zero (the beginning of time).
- The AgentEfficiencyEvent struct is in crates/roko-learn/src/efficiency.rs.

WHAT TO DO:
1. Read crates/roko-learn/src/efficiency.rs to find the AgentEfficiencyEvent struct.
2. Look for a proper timestamp field: created_at_ms, timestamp_ms, started_at_ms, or similar.
3. In logs_view.rs, find where efficiency events are inserted into the BTreeMap.
4. Replace wall_time_ms with the correct timestamp field.
5. If no proper timestamp exists on the struct, use chrono::Utc::now().timestamp_millis() as a fallback, or derive from other available data.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -n 'wall_time_ms' crates/roko-cli/src/tui/views/logs_view.rs | wc -l  # should be 0 (replaced)" \
"$CARGO_VERIFY"

item "7.8" "Fix TaskEntry.status to use TaskStatus enum" "" \
"$SHARED_CONTEXT
TASK: Change TaskEntry.status from String to the TaskStatus enum.

Use subagents to find all references and update them in parallel.

CONTEXT:
- File: crates/roko-cli/src/tui/state.rs, line ~373-378
- TaskEntry has: pub status: String — the ONLY remaining String-typed status field.
- All other status fields use typed enums: AgentRow.status = AgentStatus, TaskRow.status = TaskStatus, PlanEntry.status = PlanPhase.
- TaskStatus enum is already defined in state.rs with From<&str> and Display impls.

WHAT TO DO:
1. In state.rs TaskEntry struct (~line 373), change:
     pub status: String,   →   pub status: TaskStatus,
2. In update_from_snapshot() where TaskEntry is constructed (~line 968), change:
     status: task.status.clone(),   →   status: TaskStatus::from(task.status.as_str()),
3. Search for ALL reads of TaskEntry.status:
     grep -rn 'task\.status\|entry\.status' crates/roko-cli/src/tui/views/ crates/roko-cli/src/tui/modals/ crates/roko-cli/src/tui/widgets/
4. Update each to use enum pattern matching instead of string comparison:
     // Before: if task.status == \"done\" { ... }
     // After:  if task.status.is_done() { ... }
     // Or:     match task.status { TaskStatus::Done => ..., }
5. For Display formatting (e.g., format!(\"{}\", task.status)), TaskStatus already implements Display.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  grep 'pub status: String' crates/roko-cli/src/tui/state.rs | wc -l  # should be 0 (all converted)" \
"$CARGO_VERIFY"

item "7.9" "Fix SwitchTab focus reset per tab" "" \
"$SHARED_CONTEXT
TASK: Set appropriate default focus zone per tab instead of always PlanTree.

CONTEXT:
- File: crates/roko-cli/src/tui/app.rs, line ~740
- SwitchTab handler always sets: self.tui_state.focus = FocusZone::PlanTree
- On Logs/Config/Inspect tabs, PlanTree doesn't exist as a panel.
- FocusZone enum has: PlanTree, TaskProgress, AgentOutput, CommandOutput, RightPanel

WHAT TO DO:
1. In app.rs, find TuiAction::SwitchTab handler (~line 740).
2. Replace the hardcoded PlanTree with per-tab defaults:
     self.tui_state.focus = match tab {
         Tab::Dashboard | Tab::Plans => FocusZone::PlanTree,
         Tab::Agents => FocusZone::AgentOutput,
         Tab::Git | Tab::Logs | Tab::Config | Tab::Inspect => FocusZone::RightPanel,
     };

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -A5 'SwitchTab' crates/roko-cli/src/tui/app.rs | grep 'match tab'" \
"$CARGO_VERIFY"

item "7.10" "Remove dead agents_by_id HashMap and AgentState struct" "" \
"$SHARED_CONTEXT
TASK: Remove unused data structures from TuiState.

Use subagents to grep for all references before removing.

CONTEXT:
- File: crates/roko-cli/src/tui/state.rs
- agents_by_id: HashMap<String, AgentState> at line ~573 — populated in update_from_snapshot() but NEVER read by any widget or view.
- AgentState struct at line ~254 — only used by agents_by_id. Dead code.
- Only Vec<AgentRow> (the agents field) is read by widgets.

WHAT TO DO:
1. First verify no code reads agents_by_id:
     grep -rn 'agents_by_id' crates/roko-cli/src/tui/ --include='*.rs' | grep -v 'state.rs'
   If any matches, investigate before removing.
2. Remove agents_by_id field from TuiState struct
3. Remove its initialization in Default impl (~line 740: agents_by_id: HashMap::new())
4. Remove all code in update_from_snapshot() that populates it
5. Remove the AgentState struct definition (~line 254-275)
6. Remove 'use' imports of AgentState if orphaned

VERIFY:
  cargo check -p roko-cli && cargo test -p roko-cli
ACCEPTANCE:
  ! grep -q 'agents_by_id' crates/roko-cli/src/tui/state.rs  # field gone
  ! grep -q 'struct AgentState' crates/roko-cli/src/tui/state.rs  # struct gone" \
"$CARGO_TEST_VERIFY"

item "7.11" "Remove dead token_burn_history and TokenBurnEntry" "" \
"$SHARED_CONTEXT
TASK: Remove unused token tracking structures.

CONTEXT:
- File: crates/roko-cli/src/tui/state.rs
- token_burn_history: HashMap<String, Vec<TokenBurnEntry>> at line ~683 — NEVER populated.
- TokenBurnEntry struct at line ~403 — only used by token_burn_history. Dead code.
- token_history (different field, VecDeque-based) is the live one used by sparkline.

WHAT TO DO:
1. Verify nothing reads token_burn_history:
     grep -rn 'token_burn_history\|TokenBurnEntry' crates/roko-cli/src/tui/
2. Remove token_burn_history field from TuiState
3. Remove its Default initialization (~line 795)
4. Remove the TokenBurnEntry struct (~line 403)

VERIFY:
  cargo check -p roko-cli && cargo test -p roko-cli
ACCEPTANCE:
  ! grep -q 'token_burn_history' crates/roko-cli/src/tui/state.rs
  ! grep -q 'TokenBurnEntry' crates/roko-cli/src/tui/state.rs" \
"$CARGO_TEST_VERIFY"

item "7.12" "Stop DashboardScaffold rebuild on every data refresh" "" \
"$SHARED_CONTEXT
TASK: Add change detection to avoid rebuilding scaffold every 500ms.

CONTEXT:
- File: crates/roko-cli/src/tui/app.rs
- drain_background_channels() calls DashboardScaffold::new_in() at line ~1913 on EVERY data_rx receive.
- This runs every 500ms and reads many files from disk unnecessarily.
- Also at line ~1718 in refresh_snapshot().

WHAT TO DO:
1. Add a generation counter to DashboardData. In dashboard.rs, add a field:
     pub generation: u64
   Increment it whenever any file stamp changes during load_best_effort().
2. In app.rs, track the last-seen generation:
     let mut last_data_gen: u64 = 0;
3. In drain_background_channels(), only rebuild scaffold when generation changes:
     if new_data.generation != last_data_gen {
         last_data_gen = new_data.generation;
         self.scaffold = DashboardScaffold::new_in(&self.workdir);
     }
     self.data = new_data;
     self.tui_state.update_from_snapshot(&self.data);

Alternative simpler approach: just remove the scaffold rebuild from drain_background_channels
entirely and only rebuild in refresh_snapshot() (triggered by Ctrl-R). The scaffold is only
used for the legacy text-mode path anyway.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: The scaffold rebuild should not run unconditionally on every data receive." \
"$CARGO_VERIFY"

# ── Phase 8: Remaining gap fixes ─────────────────────────────────────────────

item "8.1" "Remove dead CollapseExpand and ConfigStartEdit variants" "" \
"$SHARED_CONTEXT
TASK: Remove two unreachable TuiAction variants.

CONTEXT:
- File: crates/roko-cli/src/tui/input.rs — TuiAction enum
- CollapseExpand at line ~307: identical to ExpandCollapse, no key binding maps to it
- ConfigStartEdit at line ~279: no key binding, ConfigToggle handles free-form fields
- File: crates/roko-cli/src/tui/app.rs — dispatch_action match arms for both

WHAT TO DO:
1. In input.rs, remove CollapseExpand and ConfigStartEdit from the TuiAction enum.
2. In app.rs dispatch_action, remove their match arms.
3. Verify no code references them: grep -rn 'CollapseExpand\|ConfigStartEdit' crates/roko-cli/src/tui/

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  ! grep -q 'CollapseExpand' crates/roko-cli/src/tui/input.rs
  ! grep -q 'ConfigStartEdit' crates/roko-cli/src/tui/input.rs" \
"$CARGO_VERIFY"

item "8.3" "Fix task picker navigation and agent tab bounds" "" \
"$SHARED_CONTEXT
TASK: Fix two related navigation bugs.

Use subagents to fix both in parallel.

CONTEXT — Task picker (input.rs):
- handle_task_picker_key() at line ~482 maps Up/Down to SelectPlanUp/SelectPlanDown.
- This navigates the PLAN list, not the task picker's own task list.
- The ModalState::TaskPicker has a selected_index field that should be modified.

CONTEXT — Agent tab bounds (app.rs):
- SwitchAgentTab handler: when idx is a direct number (1-7), it sets selected_agent_tab directly without bounds checking.
- Pressing 7 with 2 agents sets index to 6 (out of bounds).

WHAT TO DO for task picker:
1. In input.rs handle_task_picker_key, change Up/Down mappings:
     KeyCode::Up | KeyCode::Char('k') => TuiAction::TaskPickerUp,
     KeyCode::Down | KeyCode::Char('j') => TuiAction::TaskPickerDown,
2. Add TaskPickerUp and TaskPickerDown to TuiAction enum.
3. In app.rs dispatch_action, handle them by modifying the selected_index inside the ModalState:
     TuiAction::TaskPickerUp => {
         if let Some(ModalState::TaskPicker { ref mut selected_index, .. }) = self.active_modal {
             *selected_index = selected_index.saturating_sub(1);
         }
     }
   (Similar for TaskPickerDown with +1 and max clamping)

WHAT TO DO for agent tab bounds:
1. In app.rs SwitchAgentTab handler, add clamping for direct indices:
     let max_idx = self.tui_state.agents.len().saturating_sub(1);
     self.tui_state.selected_agent_tab = idx.min(max_idx);

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  grep -q 'TaskPickerUp\|TaskPickerDown' crates/roko-cli/src/tui/input.rs
  grep 'selected_agent_tab.*min' crates/roko-cli/src/tui/app.rs" \
"$CARGO_VERIFY"

item "8.5" "Move DismissNotification to global keys and add quit confirm" "" \
"$SHARED_CONTEXT
TASK: Two quick input improvements.

Use subagents to fix both in parallel.

CONTEXT — DismissNotification:
- In input.rs, 'n' -> DismissNotification is ONLY in handle_dashboard_key() at line ~627.
- Should work on all tabs.

CONTEXT — Quit confirmation:
- TuiAction::Quit in app.rs (~line 733) sets running = false immediately.
- ModalState::Quit variant and render_quit() already exist but are never used.
- Accidental 'q' kills the TUI with no warning.

WHAT TO DO for DismissNotification:
1. In input.rs handle_global_key(), add:
     KeyCode::Char('n') => TuiAction::DismissNotification,
2. Remove the 'n' mapping from handle_dashboard_key() to avoid duplication.

WHAT TO DO for quit confirm:
1. In app.rs dispatch_action for TuiAction::Quit:
   - Keep existing logic: if modals are open, dismiss them first.
   - Change the else branch: instead of self.running = false, show the quit modal:
       self.active_modal = Some(ModalState::Quit);
   - Add a new action TuiAction::QuitConfirmed that actually sets self.running = false.
   - In the quit modal key handler (or ConfirmYes when pending_confirm is quit), call QuitConfirmed.
2. Ctrl-C should still force-quit immediately (bypass the confirmation).

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: The quit modal should be shown on 'q', and only confirmed on 'y'/Enter." \
"$CARGO_VERIFY"

item "8.7" "Add visible focus indicator borders to panels" "" \
"$SHARED_CONTEXT
TASK: Change border color of the focused panel so users can see which panel has keyboard focus.

Use subagents to modify all three view files in parallel.

CONTEXT:
- Tab/BackTab cycling changes tui_state.focus (FocusZone enum) but most panels show no visual change.
- FocusZone variants: PlanTree, TaskProgress, AgentOutput, CommandOutput, RightPanel.
- The rosedust theme in widgets/rosedust.rs has constants like ROSE, GHOST, BONE_DIM, etc.

WHAT TO DO:
1. Read widgets/rosedust.rs to find color constants (ROSE for accent, GHOST for dim).
2. In views/dashboard_view.rs, for EACH bordered panel:
   - Check if tui_state.focus matches this panel's zone
   - If focused: use border_style(Style::default().fg(Color::Rgb(185,120,148)))  // ROSE accent
   - If not focused: use border_style(Style::default().fg(Color::Rgb(60,50,60)))  // ghost/dim
   - Apply to: left panel (PlanTree zone), task progress area, right panel, sub-panels
3. In views/plans_view.rs:
   - Left panel (PlanTree) and right panel (RightPanel) — same pattern
4. In views/agents_view.rs:
   - Agent output (AgentOutput) and right panel (RightPanel) — same pattern

The Block widget is created with Block::default().borders(Borders::ALL).border_style(style).
Find where each panel's Block is created and add the focus-aware style.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -c 'focus.*=.*FocusZone\|FocusZone.*focus' crates/roko-cli/src/tui/views/*.rs  # should be >= 4" \
"$CARGO_VERIFY"

item "8.8" "Deduplicate git subprocess calls between threads" "" \
"$SHARED_CONTEXT
TASK: Remove redundant git diff invocation from dashboard data loading.

CONTEXT:
- dashboard.rs load_best_effort() runs git diff as a subprocess (~line 585).
- app.rs spawns a separate git background thread that also collects git data.
- Both run simultaneously every few seconds, causing redundant git subprocess calls.

WHAT TO DO:
1. In dashboard.rs, find where git diff is loaded during load_best_effort().
   Look for: run_dashboard_git_diff, git diff, Command::new(\"git\").
2. Remove or skip the git diff call from load_best_effort().
3. Leave the git_diff field on DashboardData but initialize it to String::new().
4. In app.rs drain_background_channels(), when git data arrives from the background thread,
   copy the diff content into self.data.git_diff (so views can still read it from data).
5. The git background thread should remain the sole source of git data.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: Only ONE background thread should run git commands (the git thread in app.rs)." \
"$CARGO_VERIFY"

item "8.9" "Add notification auto-expiry" "" \
"$SHARED_CONTEXT
TASK: Automatically dismiss notifications after 5 seconds.

CONTEXT:
- App.notifications: Vec<modals::Notification> — accumulates without cleanup.
- modals/notification.rs defines the Notification struct — check for a timestamp field.
- The tick handler in app.rs main_loop (Event::Tick branch) is where periodic cleanup should go.

WHAT TO DO:
1. Read modals/notification.rs to find the Notification struct and its timestamp field name.
   It likely has created_at_ms: i64 or similar.
2. In app.rs, in the Event::Tick handler (where atmosphere.tick() is called), add:
     let now_ms = std::time::SystemTime::now()
         .duration_since(std::time::UNIX_EPOCH)
         .map(|d| d.as_millis() as i64)
         .unwrap_or(0);
     self.notifications.retain(|n| now_ms - n.created_at_ms < 5000);
3. Adjust the field name to match whatever the Notification struct actually uses.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -q 'retain.*notification\|notifications.*retain' crates/roko-cli/src/tui/app.rs" \
"$CARGO_VERIFY"

item "8.10" "Add 256-color and 24-bit ANSI support" "" \
"$SHARED_CONTEXT
TASK: Extend the ANSI color parser for 256-color and RGB color codes.

CONTEXT:
- File: crates/roko-cli/src/tui/ansi.rs
- Currently handles 16-color SGR codes (30-37, 40-47, 90-97, 100-107).
- Claude CLI outputs 256-color codes like \\x1b[38;5;208m (orange foreground).
- These are currently silently ignored.
- ratatui supports Color::Indexed(u8) for 256-color and Color::Rgb(r,g,b) for 24-bit.

WHAT TO DO:
1. Read ansi.rs to understand how SGR codes are currently parsed. Find where individual
   numeric codes are matched (the match on code values like 0, 1, 22, 30-37, etc).
2. The tricky part: 256-color uses MULTIPLE semicolon-separated params: 38;5;N.
   Instead of processing codes one-at-a-time, detect multi-code sequences:

   When iterating through the codes slice, check for sequences:
   - If code == 38 and next code == 5: consume the THIRD code as Color::Indexed(n)
   - If code == 48 and next code == 5: same for background
   - If code == 38 and next code == 2: consume next THREE codes as Color::Rgb(r,g,b)
   - If code == 48 and next code == 2: same for background

3. Use an index-based loop instead of for-each so you can skip consumed codes:
     let mut i = 0;
     while i < codes.len() {
         match codes[i] {
             38 if codes.get(i+1) == Some(&5) => {
                 if let Some(&n) = codes.get(i+2) {
                     style = style.fg(Color::Indexed(n as u8));
                     i += 3; continue;
                 }
             }
             // ... etc
         }
         i += 1;
     }

4. Add tests for 256-color and RGB sequences.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -q 'Color::Indexed\|Color::Rgb' crates/roko-cli/src/tui/ansi.rs" \
"$CARGO_VERIFY"

item "8.11" "Add scroll clamping in all scrollable widgets" "" \
"$SHARED_CONTEXT
TASK: Prevent scrolling past the end of content.

Use subagents to fix all widgets in parallel.

CONTEXT:
- Scroll positions can exceed content length, showing empty space.
- Each widget reads a scroll offset from TuiState and renders a slice of content.
- The fix is to clamp at render time: scroll.min(total.saturating_sub(visible)).

FILES TO FIX:
1. widgets/plan_tree.rs — reads state.plan_scroll_offset. Already has clamping — verify.
2. widgets/task_progress.rs — reads state.task_scroll. Add: let scroll = state.task_scroll.min(items.len().saturating_sub(visible_height));
3. views/agents_view.rs render_output_body — reads agent_scroll. Add: scroll.min(lines.len().saturating_sub(visible_height));
4. views/logs_view.rs — reads log_scroll or view_state.scroll. Add clamping.
5. views/config_view.rs — reads config_cursor. Add: cursor.min(items.len().saturating_sub(1));

For each, find where the scroll offset is used to slice content, and add .min(max_scroll) before the slice.

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: All scroll offsets should be clamped before use in rendering." \
"$CARGO_VERIFY"

item "8.12" "Fix DrillIn/DrillOut for Git tab" "" \
"$SHARED_CONTEXT
TASK: Make Left/Right navigate git branches on the Git tab.

CONTEXT:
- File: crates/roko-cli/src/tui/app.rs
- DrillIn/DrillOut dispatch currently toggles plans[selected].expanded for ALL tabs.
- On Git tab, it should move git_branch_cursor instead.
- TuiState has git_branch_cursor: usize and git_branch_tree: Vec<GitBranchNode>.

WHAT TO DO:
1. In app.rs, find TuiAction::DrillIn and TuiAction::DrillOut handlers.
2. Wrap in a match on active_tab:
     TuiAction::DrillIn => match self.tui_state.active_tab {
         Tab::Dashboard | Tab::Plans => {
             // existing plan expand code
         }
         Tab::Git => {
             let max = self.tui_state.git_branch_tree.len().saturating_sub(1);
             self.tui_state.git_branch_cursor = (self.tui_state.git_branch_cursor + 1).min(max);
         }
         _ => {}
     },
     TuiAction::DrillOut => match self.tui_state.active_tab {
         Tab::Dashboard | Tab::Plans => {
             // existing plan collapse code
         }
         Tab::Git => {
             self.tui_state.git_branch_cursor = self.tui_state.git_branch_cursor.saturating_sub(1);
         }
         _ => {}
     },

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE: grep -A10 'DrillIn' crates/roko-cli/src/tui/app.rs | grep 'Tab::Git'" \
"$CARGO_VERIFY"

item "8.13" "Remove remaining dead TuiState fields" "" \
"$SHARED_CONTEXT
TASK: Remove 4 dead fields from TuiState.

Use subagents to verify each field has zero readers before removing.

FIELDS TO REMOVE:
1. plan_detail_content: String (line ~651) — never populated, plan detail modal reads PlanEntry directly
2. plan_summary_content: String (line ~655) — never populated
3. parallel_run: bool (line ~661) — never set to true
4. plan_detail_tab: usize — may or may not be used, verify first

WHAT TO DO for each field:
1. grep -rn 'FIELD_NAME' crates/roko-cli/src/tui/ --include='*.rs' to find all references
2. If only found in struct definition and Default impl → safe to remove
3. If found in a view or widget → it IS used, keep it
4. Remove the field, its Default initialization, and any dead assignment code

VERIFY:
  cargo check -p roko-cli && cargo test -p roko-cli
ACCEPTANCE: The removed fields should not appear in TuiState struct definition." \
"$CARGO_TEST_VERIFY"

item "8.14" "Convert pipeline_run_state from String to bool" "" \
"$SHARED_CONTEXT
TASK: Replace stringly-typed pause state with a boolean.

CONTEXT:
- TuiState.pipeline_run_state: String at line ~659 — toggles between \"paused\" and \"running\".
- Read by status_bar.rs for PAUSED indicator.
- TogglePause handler in app.rs compares and sets strings.

WHAT TO DO:
1. In state.rs, replace: pub pipeline_run_state: String → pub is_paused: bool
2. Default to false instead of \"idle\"
3. In app.rs TogglePause handler, change to: self.tui_state.is_paused = !self.tui_state.is_paused
4. In widgets/status_bar.rs, change the check from string comparison to: if state.is_paused { ... }
5. grep -rn 'pipeline_run_state' to find and update ALL references

VERIFY:
  cargo check -p roko-cli && cargo test -p roko-cli
ACCEPTANCE:
  ! grep -q 'pipeline_run_state' crates/roko-cli/src/tui/state.rs
  grep -q 'is_paused' crates/roko-cli/src/tui/state.rs" \
"$CARGO_TEST_VERIFY"

item "8.15" "Deduplicate truncate_middle and fix AcceptFilter" "" \
"$SHARED_CONTEXT
TASK: Two quick cleanup items.

CONTEXT — truncate_middle:
- Identical function defined in BOTH widgets/plan_tree.rs and views/agents_view.rs.
- Should be in one shared location.

CONTEXT — AcceptFilter double-assignment:
- In app.rs dispatch_action for AcceptFilter (~line 1063), the line
  self.tui_state.filter = self.tui_state.filter_text.clone();
  appears TWICE consecutively. Delete the second occurrence.

WHAT TO DO:
1. Create a utility function in a shared location. Options:
   - Add to widgets/mod.rs as pub(crate) fn truncate_middle(...)
   - Or create a new file crates/roko-cli/src/tui/util.rs with pub(crate) fn
2. Import from both plan_tree.rs and agents_view.rs
3. Delete the local definitions in both files
4. In app.rs, find AcceptFilter handler and remove the duplicate line

VERIFY:
  cargo check -p roko-cli
ACCEPTANCE:
  grep -rn 'fn truncate_middle' crates/roko-cli/src/tui/ | wc -l  # should be 1 (one definition)" \
"$CARGO_VERIFY"

item "8.16" "Add config live reload and fix stale test" "" \
"$SHARED_CONTEXT
TASK: Two items: config reload after save + fix broken test.

CONTEXT — Config reload:
- ConfigSave in app.rs writes to roko.toml but App doesn't re-read config.
- DashboardData::load_best_effort reads roko.toml, so a data refresh picks up changes.

CONTEXT — Stale test:
- state.rs line ~2076 has: assert_eq!(state.plan_scroll, 0)
- plan_scroll was removed, should be plan_scroll_offset.

WHAT TO DO for config reload:
1. In app.rs dispatch_action ConfigSave handler, after the successful write, add:
     self.data = DashboardData::load_best_effort(&self.workdir);
     self.tui_state.update_from_snapshot(&self.data);
2. Change the notification message to 'Config saved and reloaded'

WHAT TO DO for stale test:
1. In state.rs, find the test with assert_eq!(state.plan_scroll, 0)
2. Change to: assert_eq!(state.plan_scroll_offset, 0)
3. Search for any other references to removed fields in tests:
     grep -n 'plan_scroll\b\|output_scroll\|log_messages\|agents_by_id' crates/roko-cli/src/tui/state.rs

VERIFY:
  cargo check -p roko-cli && cargo test -p roko-cli
ACCEPTANCE: cargo test -p roko-cli should pass with zero errors." \
"$CARGO_TEST_VERIFY"

###############################################################################
# Runner infrastructure
###############################################################################

get_all_ids() {
  for key in "${!ITEMS[@]}"; do
    if [[ "$key" == *":title" ]]; then
      echo "${key%%:title}"
    fi
  done | sort -t. -k1,1n -k2,2n
}

should_run() {
  local id="$1" phase="${1%%.*}"
  if [[ ${#ITEMS_TO_RUN[@]} -gt 0 ]]; then
    for item in "${ITEMS_TO_RUN[@]}"; do
      [[ "$item" == "$id" ]] && return 0
    done
    return 1
  fi
  if [[ -n "$PHASE_FILTER" ]]; then
    [[ "$phase" == "$PHASE_FILTER" ]] && return 0
    return 1
  fi
  return 0
}

# Check if all deps for an item have passed
deps_met() {
  local id="$1"
  local deps="${ITEMS["${id}:deps"]:-}"
  [[ -z "$deps" ]] && return 0
  for dep in $deps; do
    if [[ "${ITEM_STATUS[$dep]:-}" != "PASS" ]]; then
      return 1
    fi
  done
  return 0
}

# Health gate: verify codebase compiles before starting an item
health_check() {
  log "Health gate: cargo check..."
  if timeout "$HEALTH_CHECK_TIMEOUT" cargo check -p roko-cli 2>&1 | tail -3; then
    return 0
  else
    return 1
  fi
}

run_item() {
  local id="$1"
  local title="${ITEMS["${id}:title"]}"
  local prompt="${ITEMS["${id}:prompt"]}"
  local verify="${ITEMS["${id}:verify"]}"
  local logfile="$LOGS_DIR/${id}.log"
  local prompt_file="$LOGS_DIR/${id}.prompt.txt"
  local phase="${id%%.*}"

  section
  log "${BOLD}Item ${id}: ${title}${RESET}"
  log "Phase: ${phase} | Timeout: ${ITEM_TIMEOUT}s | Log: ${logfile}"

  if $DRY_RUN; then
    echo -e "${DIM}--- PROMPT (first 20 lines) ---${RESET}"
    echo "$prompt" | head -20
    echo -e "${DIM}...${RESET}"
    ITEM_STATUS[$id]="SKIP"
    return 0
  fi

  # Check dependencies
  if ! deps_met "$id"; then
    warn "SKIPPED: dependency not met"
    local deps="${ITEMS["${id}:deps"]}"
    for dep in $deps; do
      [[ "${ITEM_STATUS[$dep]:-}" != "PASS" ]] && warn "  -> $dep: ${ITEM_STATUS[$dep]:-NOT_RUN}"
    done
    ITEM_STATUS[$id]="SKIP_DEP"
    echo "| ${id} | ${title} | SKIP (dep) | -- |" >> "$RESULTS_FILE"
    return 0
  fi

  # Health gate
  if ! health_check; then
    fail "Health gate failed — codebase doesn't compile. Reverting."
    git checkout -- . 2>/dev/null || true
    if ! health_check; then
      fail "CRITICAL: Cannot recover compilable state. Stopping."
      ITEM_STATUS[$id]="FAIL_HEALTH"
      echo "| ${id} | ${title} | FAIL (health) | -- |" >> "$RESULTS_FILE"
      return 1
    fi
  fi

  local start_time=$(date +%s)

  # Write prompt to file
  echo "$prompt" > "$prompt_file"

  # Run codex with timeout
  log "Spawning codex (model=$CODEX_MODEL, reasoning=$CODEX_REASONING)..."
  local agent_exit=0
  timeout "$ITEM_TIMEOUT" \
    codex exec \
      --model "$CODEX_MODEL" \
      --full-auto \
      -c "model_reasoning_effort=$CODEX_REASONING" \
      --sandbox "$CODEX_SANDBOX" \
      --cd "$WORK_DIR" \
      "$(cat "$prompt_file")" \
    2>&1 | tee "$logfile" || agent_exit=$?

  local end_time=$(date +%s)
  local elapsed=$(( end_time - start_time ))

  if [[ $agent_exit -eq 124 ]]; then
    fail "TIMEOUT after ${ITEM_TIMEOUT}s"
    git checkout -- . 2>/dev/null || true
    ITEM_STATUS[$id]="TIMEOUT"
    PHASE_FAILED[$phase]=1
    echo "| ${id} | ${title} | TIMEOUT | ${elapsed}s |" >> "$RESULTS_FILE"
    return 0
  fi

  # Verification
  log "Verifying (cargo check)..."
  local verify_output verify_exit
  verify_output=$(timeout "$HEALTH_CHECK_TIMEOUT" bash -c "$verify" 2>&1) || verify_exit=$?
  verify_exit=${verify_exit:-$?}

  if [[ $verify_exit -eq 0 ]]; then
    ok "Item ${id}: ${title} (${elapsed}s)"

    # Commit the changes
    git add -A 2>/dev/null
    if git diff --cached --quiet 2>/dev/null; then
      warn "No files changed — agent may not have made edits"
      ITEM_STATUS[$id]="PASS_NOOP"
      echo "| ${id} | ${title} | PASS (no changes) | ${elapsed}s |" >> "$RESULTS_FILE"
    else
      git commit -m "$(cat <<EOF
tui: ${title} [${id}]

Automated fix by codex agent (model=${CODEX_MODEL}, reasoning=${CODEX_REASONING}).
Item ${id} from tmp/tui/CHECKLIST.md.
EOF
      )" 2>/dev/null
      log "Committed: $(git log --oneline -1)"
      ITEM_STATUS[$id]="PASS"
      echo "| ${id} | ${title} | PASS | ${elapsed}s |" >> "$RESULTS_FILE"
    fi
  else
    fail "Item ${id}: verification failed (${elapsed}s)"
    echo -e "${DIM}$(echo "$verify_output" | tail -10)${RESET}"

    # Revert tracked files to last good state (never deletes untracked files)
    log "Reverting changes..."
    git checkout -- . 2>/dev/null || true

    ITEM_STATUS[$id]="FAIL"
    PHASE_FAILED[$phase]=1
    echo "| ${id} | ${title} | FAIL | ${elapsed}s |" >> "$RESULTS_FILE"
  fi

  # Clean up stray target/ if codex agent created one in the worktree
  if [[ -d "$WORK_DIR/target" ]]; then
    warn "Agent created stray target/ in worktree — removing"
    rm -rf "$WORK_DIR/target"
  fi

  echo ""
}

###############################################################################
# Main
###############################################################################

cat > "$RESULTS_FILE" <<HEADER
# TUI Implementation Results

**Date**: $(date +%Y-%m-%d)
**Branch**: ${BRANCH}
**Model**: ${CODEX_MODEL} (reasoning: ${CODEX_REASONING})

| ID | Title | Status | Time |
|----|-------|--------|------|
HEADER

section
log "${BOLD}TUI Gap Implementation Runner${RESET}"
log "Branch: ${BRANCH}"
log "Model: ${CODEX_MODEL} | Reasoning: ${CODEX_REASONING} | Sandbox: ${CODEX_SANDBOX}"
log "Timeout: ${ITEM_TIMEOUT}s per item"
log "Logs: ${LOGS_DIR}/"
log "Results: ${RESULTS_FILE}"

total=0
for id in $(get_all_ids); do
  if should_run "$id"; then
    total=$((total + 1))
    run_item "$id"
  fi
done

# Summary
section
log "${BOLD}RESULTS${RESET}"
echo ""

pass_count=$(grep -c "| PASS |" "$RESULTS_FILE" 2>/dev/null || echo 0)
noop_count=$(grep -c "no changes" "$RESULTS_FILE" 2>/dev/null || echo 0)
fail_count=$(grep -c "| FAIL" "$RESULTS_FILE" 2>/dev/null || echo 0)
skip_count=$(grep -c "| SKIP" "$RESULTS_FILE" 2>/dev/null || echo 0)
timeout_count=$(grep -c "| TIMEOUT" "$RESULTS_FILE" 2>/dev/null || echo 0)

echo -e "  ${GREEN}Passed:  ${pass_count}${RESET}"
echo -e "  ${GREEN}No-op:   ${noop_count}${RESET}"
echo -e "  ${RED}Failed:  ${fail_count}${RESET}"
echo -e "  ${YELLOW}Skipped: ${skip_count}${RESET}"
echo -e "  ${RED}Timeout: ${timeout_count}${RESET}"
echo -e "  Total:   ${total}"
echo ""
log "Worktree: ${WORK_DIR} (kept — not auto-deleted)"
log "Branch: ${BRANCH} ($(git log --oneline "$BRANCH" --not HEAD~${total} 2>/dev/null | wc -l | tr -d ' ') commits)"
log "Results: ${RESULTS_FILE}"
log "Logs: ${LOGS_DIR}/"

# Disk usage report
worktree_size=$(du -sh "$WORK_DIR" 2>/dev/null | cut -f1)
target_size=$(du -sh "$CARGO_TARGET_DIR" 2>/dev/null | cut -f1)
logs_size=$(du -sh "$LOGS_DIR" 2>/dev/null | cut -f1)
log "Disk: worktree=${worktree_size:-?}, shared target=${target_size:-?}, logs=${logs_size:-?}"

# macOS notification
if command -v osascript &>/dev/null; then
  osascript -e "display notification \"${pass_count} passed, ${fail_count} failed, ${skip_count} skipped\" with title \"TUI Agent Runner Complete\"" 2>/dev/null || true
fi

echo ""
log "Done."
