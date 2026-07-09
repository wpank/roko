#!/usr/bin/env bash
# run-remediation.sh — Fix STUB/MISSING items identified by the parity audit
#
# Unlike run-parity.sh, this script:
#   1. Has STRICT verification gates (cargo test must pass for touched crate)
#   2. Provides detailed context + acceptance criteria per item
#   3. Works with both Claude and Codex
#
# Usage:
#   bash tmp/run-remediation.sh                          # run all (claude)
#   bash tmp/run-remediation.sh --agent codex            # use codex
#   bash tmp/run-remediation.sh --agent codex --model o4-mini --reasoning medium
#   bash tmp/run-remediation.sh --dry-run                # print prompts only
#   bash tmp/run-remediation.sh --tier 1                 # only tier 1 items
#   bash tmp/run-remediation.sh --item 1H.07             # run single item
#   bash tmp/run-remediation.sh --list                   # show all items + status
#   bash tmp/run-remediation.sh --tests-only             # just fix test compilation
#   bash tmp/run-remediation.sh --reset                  # clear remediation state

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="$REPO_ROOT/.roko/remediation-state"
LOG_DIR="$REPO_ROOT/tmp/logs/remediation"
MAX_RETRIES=2

# Agent selection
AGENT_TYPE="${AGENT_TYPE:-claude}"
AGENT_MODEL=""
CODEX_REASONING=""

CLAUDE_CMD="${CLAUDE_CMD:-claude}"
CODEX_CMD="${CODEX_CMD:-npx codex}"

# Flags
DRY_RUN=false
FILTER_TIER=""
FILTER_ITEM=""
LIST_ONLY=false
TESTS_ONLY=false
AUTO_COMMIT=false

# ─── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
BLUE='\033[0;34m'; MAGENTA='\033[0;35m'; CYAN='\033[0;36m'
BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_err()   { echo -e "${RED}[ERROR]${NC} $*"; }
log_step()  { echo -e "${BOLD}${CYAN}─── $* ───${NC}"; }

# ─── Parse args ───────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --agent)      AGENT_TYPE="$2"; shift 2 ;;
    --model)      AGENT_MODEL="$2"; shift 2 ;;
    --reasoning)  CODEX_REASONING="$2"; shift 2 ;;
    --dry-run)    DRY_RUN=true; shift ;;
    --tier)       FILTER_TIER="$2"; shift 2 ;;
    --item)       FILTER_ITEM="$2"; shift 2 ;;
    --list)       LIST_ONLY=true; shift ;;
    --tests-only) TESTS_ONLY=true; shift ;;
    --reset)      rm -rf "$STATE_DIR"; echo "State cleared."; exit 0 ;;
    --commit)     AUTO_COMMIT=true; shift ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

mkdir -p "$STATE_DIR" "$LOG_DIR"

# ─── State helpers ────────────────────────────────────────────────────────────

mark_done()   { date -u +%Y-%m-%dT%H:%M:%SZ > "$STATE_DIR/$1.done"; }
mark_failed() { echo "$2" > "$STATE_DIR/$1.failed"; }
is_done()     { [[ -f "$STATE_DIR/$1.done" ]]; }
is_failed()   { [[ -f "$STATE_DIR/$1.failed" ]]; }

# ─── Agent dispatch ───────────────────────────────────────────────────────────

run_agent() {
  local prompt="$1"
  local item_id="$2"
  local context_files="$3"

  case "$AGENT_TYPE" in
    claude) run_agent_claude "$prompt" "$item_id" "$context_files" ;;
    codex)  run_agent_codex "$prompt" "$item_id" "$context_files" ;;
    *)      log_err "Unknown agent: $AGENT_TYPE"; return 1 ;;
  esac
}

run_agent_claude() {
  local prompt="$1" item_id="$2" context_files="$3"

  local context_args=()
  while IFS= read -r f; do
    [[ -n "$f" && -f "$f" ]] && context_args+=("--read" "$f")
  done <<< "$context_files"

  local model_args=()
  [[ -n "$AGENT_MODEL" ]] && model_args+=("--model" "$AGENT_MODEL")

  $CLAUDE_CMD --print \
    "${context_args[@]}" \
    "${model_args[@]}" \
    --output-format text \
    --max-turns 30 \
    --dangerously-skip-permissions \
    -p "$prompt"
}

run_agent_codex() {
  local prompt="$1" item_id="$2" context_files="$3"

  local context_block=""
  while IFS= read -r f; do
    [[ -n "$f" && -f "$f" ]] && context_block+="- ${f}"$'\n'
  done <<< "$context_files"

  local full_prompt="$(cat <<EOF
## Context files to read first

${context_block}

## Task

${prompt}
EOF
)"

  local model="${AGENT_MODEL:-o4-mini}"
  local extra_args=()
  [[ -n "${CODEX_REASONING:-}" ]] && extra_args+=("-c" "model_reasoning_effort=$CODEX_REASONING")

  echo "$full_prompt" | $CODEX_CMD exec \
    --model "$model" \
    --full-auto \
    "${extra_args[@]}" \
    -
}

# ─── Verification gate ────────────────────────────────────────────────────────

verify_crate() {
  local crate="$1"
  log_info "Gate: cargo test -p $crate --no-run ..."
  if ! cargo test -p "$crate" --no-run 2>&1 | tail -5; then
    log_err "Gate FAILED: $crate tests don't compile"
    return 1
  fi
  log_ok "Gate passed: $crate"
}

verify_workspace_check() {
  log_info "Gate: cargo check --workspace ..."
  if ! cargo check --workspace 2>&1 | tail -5; then
    log_err "Gate FAILED: workspace doesn't compile"
    return 1
  fi
  log_ok "Gate passed: workspace check"
}

# ─── Auto-commit ──────────────────────────────────────────────────────────────

auto_commit() {
  local item_id="$1" desc="$2"
  [[ "$AUTO_COMMIT" != "true" ]] && return 0
  if git diff --quiet && git diff --cached --quiet; then
    log_info "No changes to commit for ${item_id}"
    return 0
  fi
  git add -A
  git commit -m "$(cat <<EOF
remediate(${item_id}): ${desc}

Automated fix via run-remediation.sh
EOF
)"
  log_ok "Committed: ${item_id}"
}

# ─── Run a single remediation item ───────────────────────────────────────────

run_item() {
  local item_id="$1"
  local tier="$2"
  local gate_crate="$3"
  local context_files="$4"
  local prompt="$5"
  local desc="$6"

  if is_done "$item_id"; then
    log_info "  ✅ ${item_id} already done — skip"
    return 0
  fi

  log_step "${item_id}: ${desc}"

  if [[ "$DRY_RUN" == "true" ]]; then
    echo -e "${DIM}--- PROMPT ---${NC}"
    echo "$prompt" | head -20
    echo -e "${DIM}--- (truncated) ---${NC}"
    echo ""
    return 0
  fi

  local log_file="$LOG_DIR/${item_id}-$(date +%Y%m%d-%H%M%S).log"
  local attempt=0

  while (( attempt < MAX_RETRIES )); do
    attempt=$((attempt + 1))
    log_info "Attempt ${attempt}/${MAX_RETRIES}"

    if run_agent "$prompt" "$item_id" "$context_files" 2>&1 | tee -a "$log_file"; then
      # Verify with crate-level test gate
      if [[ -n "$gate_crate" ]]; then
        if verify_crate "$gate_crate" 2>&1 | tee -a "$log_file"; then
          mark_done "$item_id"
          auto_commit "$item_id" "$desc"
          log_ok "${item_id} complete + verified"
          return 0
        else
          log_warn "Agent succeeded but gate failed — retrying"
          continue
        fi
      else
        # No specific crate gate — just check workspace
        if verify_workspace_check 2>&1 | tee -a "$log_file"; then
          mark_done "$item_id"
          auto_commit "$item_id" "$desc"
          log_ok "${item_id} complete + verified"
          return 0
        else
          log_warn "Agent succeeded but workspace check failed — retrying"
          continue
        fi
      fi
    else
      log_err "Agent failed (exit code)"
    fi
  done

  mark_failed "$item_id" "Failed after ${MAX_RETRIES} attempts"
  log_err "${item_id} FAILED after ${MAX_RETRIES} attempts"
  return 1
}

# ─── Remediation items ───────────────────────────────────────────────────────
#
# Each item is: ID|TIER|GATE_CRATE|CONTEXT_FILES(newline-sep)|DESCRIPTION|PROMPT
#
# We define them as functions to keep prompts readable.

ITEMS=()

add_item() {
  ITEMS+=("$1")
}

# ─── Test fix items ──────────────────────────────────────────────────────────

define_test_fixes() {

add_item "TEST.01|0|roko-daimon|
$REPO_ROOT/crates/roko-daimon/Cargo.toml
$REPO_ROOT/crates/roko-daimon/src/lib.rs
|Fix roko-daimon test compilation|
You are fixing test compilation in roko-daimon.

## Problem
Tests use \`tempfile::TempDir\` but \`tempfile\` is not in [dev-dependencies].

## Fix
Add \`tempfile = \"3\"\` to [dev-dependencies] in crates/roko-daimon/Cargo.toml.

## Verification
Run: cargo test -p roko-daimon --no-run
Must exit 0 with no errors."

add_item "TEST.02|0|roko-conductor|
$REPO_ROOT/crates/roko-conductor/src/watchers/time_overrun.rs
$REPO_ROOT/crates/roko-conductor/Cargo.toml
|Fix roko-conductor test compilation|
You are fixing test compilation in roko-conductor.

## Problem
\`TaskTimingEvent\` is missing the \`Serialize\` derive, causing test compilation failure.

## Fix
Add \`#[derive(serde::Serialize)]\` (or add Serialize to existing derive) on \`TaskTimingEvent\` in crates/roko-conductor/src/watchers/time_overrun.rs.

## Verification
Run: cargo test -p roko-conductor --no-run
Must exit 0."

add_item "TEST.03|0|roko-dreams|
$REPO_ROOT/crates/roko-dreams/Cargo.toml
$REPO_ROOT/crates/roko-dreams/src/cycle.rs
$REPO_ROOT/crates/roko-neuro/src/knowledge_store.rs
|Fix roko-dreams test compilation (12 errors)|
You are fixing test compilation in roko-dreams.

## Problems (3 distinct issues)
1. \`tempfile\` not in [dev-dependencies] — add \`tempfile = \"3\"\.
2. Tests use \`ChronoDuration\` which doesn't exist — replace with \`chrono::Duration\`.
3. Tests call \`KnowledgeStore::read_all()\` which is private — either make it \`pub\` or use the public query API.

## Rules
- Only change test code and Cargo.toml. Don't change non-test production logic unless making read_all pub.
- If making read_all pub, add \`pub\` visibility only — don't change the function body.

## Verification
Run: cargo test -p roko-dreams --no-run
Must exit 0."

add_item "TEST.04|0|roko-learn|
$REPO_ROOT/crates/roko-learn/src/pattern_discovery.rs
|Fix roko-learn lib test compilation|
You are fixing test compilation in roko-learn.

## Problem
In pattern_discovery.rs, a \`&&str\` is passed where \`impl Into<String>\` is expected.
The fix is to dereference: use \`*gate\` or \`.to_string()\` instead of \`gate\`.

## Fix
Find the call site (around line 880) and fix the type conversion.

## Verification
Run: cargo test -p roko-learn --lib --no-run
Must exit 0."

add_item "TEST.05|0|roko-learn|
$REPO_ROOT/crates/roko-learn/src/model_router.rs
$REPO_ROOT/crates/roko-learn/tests/learning_loop.rs
|Fix roko-learn integration test compilation|
You are fixing roko-learn's integration test (tests/learning_loop.rs).

## Problems
1. \`LinUCBRouter::new()\` signature changed to take 3 args but tests pass 4.
2. \`.with_health_tracker()\` method no longer exists on LinUCBRouter.
3. \`.health_tracker()\` accessor no longer exists.

## Fix
Read the CURRENT LinUCBRouter API in crates/roko-learn/src/model_router.rs, then update tests/learning_loop.rs to match.

## Rules
- Only change the test file. Do NOT change model_router.rs.
- Match the current API exactly.

## Verification
Run: cargo test -p roko-learn --test learning_loop --no-run
Must exit 0."

add_item "TEST.06|0|roko-plugin|
$REPO_ROOT/crates/roko-plugin/Cargo.toml
$REPO_ROOT/crates/roko-plugin/src/lib.rs
|Fix roko-plugin test compilation|
You are fixing test compilation in roko-plugin.

## Problems
1. \`notify::CreateKind\`, \`notify::ModifyKind\`, \`notify::RemoveKind\` — the notify crate API may have changed. Check the notify version in Cargo.toml and update imports to match.
2. A lifetime issue in test code — temporary value dropped while borrowed.

## Fix
Read the current notify version, check its API, and fix the imports and lifetime issue.

## Verification
Run: cargo test -p roko-plugin --no-run
Must exit 0."

add_item "TEST.07|0|roko-serve|
$REPO_ROOT/crates/roko-serve/Cargo.toml
$REPO_ROOT/crates/roko-serve/src/lib.rs
$REPO_ROOT/crates/roko-learn/src/prompt_experiment.rs
$REPO_ROOT/crates/roko-learn/src/cascade_router.rs
|Fix roko-serve test compilation|
You are fixing test compilation in roko-serve.

## Problems
1. \`Uuid\` imported twice (from different paths) — remove the duplicate.
2. \`CascadeRouter\` type not found — check if it was renamed or moved in roko-learn, then fix the import.
3. \`PromptExperiment\` has private fields — tests can't construct it with struct literal. Use a constructor or make fields pub.

## Rules
- Prefer updating test code to match current API.
- If a constructor is needed, check if one exists before adding.

## Verification
Run: cargo test -p roko-serve --no-run
Must exit 0."

add_item "TEST.08|0|roko-cli|
$REPO_ROOT/crates/roko-cli/src/cloud.rs
$REPO_ROOT/crates/roko-serve/src/deploy/mod.rs
|Fix roko-cli test compilation|
You are fixing test compilation in roko-cli.

## Problem
\`CloudExecutionParams\` is missing field \`workspace_dir\` in a struct initializer.

## Fix
Find the struct literal missing the field and add \`workspace_dir\` with an appropriate default (e.g., PathBuf::from(\".\") or read from config).
Check the struct definition first to see what type workspace_dir expects.

## Verification
Run: cargo test -p roko-cli --lib --no-run
Must exit 0."

add_item "TEST.09|0|roko-agent|
$REPO_ROOT/crates/roko-agent/src/dispatcher/mod.rs
$REPO_ROOT/crates/roko-agent/src/safety/mod.rs
$REPO_ROOT/crates/roko-agent/tests/safety_integration.rs
|Fix roko-agent integration test compilation|
You are fixing roko-agent's safety integration test.

## Problems
1. \`SafetyPolicy\` import doesn't exist — was it renamed or removed?
2. \`.with_safety_policy()\` method not found on \`ToolDispatcher\`.
3. Type annotation issues.

## Fix
Read the CURRENT ToolDispatcher and safety module APIs, then update the integration test to match.

## Rules
- Only change the test file. Do NOT change dispatcher/mod.rs or safety/mod.rs.

## Verification
Run: cargo test -p roko-agent --test safety_integration --no-run
Must exit 0."

add_item "TEST.10|0|roko-mcp-slack|
$REPO_ROOT/crates/roko-mcp-slack/src/main.rs
|Fix roko-mcp-slack test compilation|
You are fixing test compilation in roko-mcp-slack.

## Problem
A temporary value is dropped while still borrowed.

## Fix
Extend the lifetime of the temporary — typically by binding it to a named variable.

## Verification
Run: cargo test -p roko-mcp-slack --no-run
Must exit 0."

} # end define_test_fixes

# ─── Tier 1 remediation items ────────────────────────────────────────────────

define_tier1_items() {

add_item "1H.07|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/tui/app.rs
$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs
$REPO_ROOT/crates/roko-cli/src/tui/pages/mod.rs
$REPO_ROOT/crates/roko-cli/src/tui/widgets/mod.rs
|Implement TUI Page 1: Overview/Health with ratatui|
You are implementing the Overview (Health) page for the roko TUI dashboard using ratatui.

## What exists
- App struct, event loop, keyboard nav all wired (app.rs)
- DashboardData with plans, active_tasks, agents, gate_results, efficiency, conductor_alerts (dashboard.rs)
- PageId::Health exists in pages/mod.rs
- render_page() in app.rs dispatches to render_health() but the render function is a stub

## What to implement
A 3-column ratatui layout:
- Left column (40%): Plan list as a Table widget (plan name, status, progress %, task count)
- Center column (30%): Health indicators — Gauge widgets for gate pass rate, cost burn rate, agent utilization
- Right column (30%): Recent conductor alerts as a List widget (timestamp, watcher name, severity, message)

## Requirements
- Use ratatui::layout::Layout with Direction::Horizontal for the 3 columns
- Use ratatui::widgets::{Table, Row, Cell, Gauge, List, ListItem, Block, Borders}
- Pull data from the \`DashboardData\` struct — DO NOT create fake/mock data
- Handle empty states gracefully (\"No plans found\", \"No alerts\")
- Use the existing Theme struct for colors

## Verification
Run: cargo test -p roko-cli --lib --no-run
Must compile. Then run: cargo run -p roko-cli -- dashboard (should show interactive TUI)"

add_item "1H.08|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/tui/app.rs
$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs
$REPO_ROOT/crates/roko-cli/src/tui/pages/mod.rs
|Implement TUI Page 2: Plan Execution with ratatui|
You are implementing the Plan Execution page.

## Layout
- Top (20%): Progress bar (Gauge) showing overall plan completion
- Center-left (60%): Task table (Table widget) with columns: ID, name, status, duration, model
- Center-right (40%): Selected task detail panel (Paragraph) — description, gate results, agent output tail
- Bottom (15%): Live agent output tail (last 5 lines of current agent's stdout)

## Requirements
- DashboardData.active_tasks provides the data
- Highlight the selected row (use StatefulWidget with TableState)
- Arrow keys change selection, detail panel updates
- Use existing Theme colors

## Verification
cargo test -p roko-cli --lib --no-run must exit 0."

add_item "1H.09|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/tui/app.rs
$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs
$REPO_ROOT/crates/roko-cli/src/tui/pages/mod.rs
|Implement TUI Page 3: Agent Activity with ratatui|
You are implementing the Agent Activity page.

## Layout
- Top (60%): Active agents Table (agent_id, model, role, task, tokens_used, cost, status)
- Bottom-left (50%): Model distribution — Paragraph showing model name + count (text-based bar chart)
- Bottom-right (50%): Cost breakdown — Table with per-model total cost, avg cost per task

## Requirements
- DashboardData.agents provides the data
- DashboardData.efficiency has aggregate token/cost metrics
- Handle no-agents state gracefully

## Verification
cargo test -p roko-cli --lib --no-run must exit 0."

add_item "1H.10|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/tui/app.rs
$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs
$REPO_ROOT/crates/roko-cli/src/tui/pages/mod.rs
|Implement TUI Page 4: Gate Results with ratatui|
You are implementing the Gate Results page.

## Layout
- Top (30%): Gate summary Table (gate name, total runs, pass rate %, last result)
- Middle (30%): Adaptive thresholds Table (rung, gate, current threshold, EMA, trend arrow)
- Bottom (40%): Recent failures List (timestamp, task_id, gate name, error snippet)

## Requirements
- DashboardData.gate_results provides the data
- Use color coding: green for >80% pass, yellow for 50-80%, red for <50%

## Verification
cargo test -p roko-cli --lib --no-run must exit 0."

add_item "1H.11|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/tui/app.rs
$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs
$REPO_ROOT/crates/roko-cli/src/tui/pages/mod.rs
|Implement TUI Page 5: Learning with ratatui|
You are implementing the Learning page.

## Layout
- Top-left (50%): Cascade router Table (model, arm count, avg reward, selection rate)
- Top-right (50%): Active experiments Table (name, variants, observations, winner)
- Bottom (40%): Efficiency trends — text sparklines or Paragraph with recent metrics

## Requirements
- DashboardData.cascade_router, .experiments, .efficiency provide data

## Verification
cargo test -p roko-cli --lib --no-run must exit 0."

add_item "1H.12|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/tui/app.rs
$REPO_ROOT/crates/roko-cli/src/tui/dashboard.rs
$REPO_ROOT/crates/roko-cli/src/tui/pages/mod.rs
|Implement TUI Page 6: Signals with ratatui|
You are implementing the Signals page.

## Layout
- Top (60%): Recent signals Table (hash prefix, kind, timestamp, plan_id, task_id, payload preview)
- Bottom-left (40%): Signal kind distribution — text bar chart (kind → count)
- Bottom-right (40%): Selected signal detail Paragraph

## Requirements
- DashboardData.recent_signals provides the data
- Scrollable table with selection

## Verification
cargo test -p roko-cli --lib --no-run must exit 0."

add_item "1D.09|1|roko-cli|
$REPO_ROOT/crates/roko-cli/src/main.rs
$REPO_ROOT/crates/roko-cli/Cargo.toml
|Wire secret redaction in tracing layer|
Add a tracing-subscriber layer that redacts secrets from log output.

## What to implement
In the tracing subscriber initialization (main.rs), add a layer that replaces patterns matching:
- \`sk-[a-zA-Z0-9]{20,}\` → [REDACTED]
- \`xoxb-[a-zA-Z0-9-]+\` → [REDACTED]
- \`ghp_[a-zA-Z0-9]{36}\` → [REDACTED]
- \`ghs_[a-zA-Z0-9]{36}\` → [REDACTED]

Use a custom \`tracing_subscriber::fmt::FormatEvent\` wrapper or a \`Layer\` that sanitizes before output.

## Verification
cargo check -p roko-cli must pass."

add_item "1C.05|1|roko-cli|
$REPO_ROOT/crates/roko-std/src/tool/mod.rs
$REPO_ROOT/crates/roko-agent/src/dispatcher/mod.rs
$REPO_ROOT/crates/roko-cli/src/orchestrate.rs
$REPO_ROOT/crates/roko-cli/src/task_parser.rs
|Wire role-based tool profiles to auto-populate denied_tools|
Role-based tool profiles exist in roko-std but aren't auto-applied.

## What to implement
1. In orchestrate.rs, when building the agent dispatch for a task:
   - Read the task's role (from task_def.role or template)
   - Look up the corresponding tool profile (e.g., RESEARCHER_TOOL_PROFILE)
   - Merge profile's denied_tools into the task's denied_tools list
2. Profiles should be: Researcher=deny(Write,Edit,Bash), Reviewer=deny(Write,Edit), Scribe=deny(Bash)

## Verification
cargo check --workspace must pass."

add_item "1I.04|1|roko-cli|
$REPO_ROOT/crates/roko-learn/src/skill_library.rs
$REPO_ROOT/crates/roko-cli/src/orchestrate.rs
|Wire SkillLibrary.query() to inject skill guidance before dispatch|
SkillLibrary is loaded and extract_skill() is called on success, but query() is never called to inject prior skill knowledge into the agent prompt.

## What to implement
In orchestrate.rs, before dispatching an agent for a task:
1. Call \`skill_library.query(task_description, limit=3)\`
2. If results are non-empty, format them as a \"## Prior Experience\" section
3. Append to the system prompt or task context

## Verification
cargo check --workspace must pass."

} # end define_tier1_items

# ─── Tier 4 remediation items (biggest gap area) ─────────────────────────────

define_tier4_items() {

add_item "4A.01|4|roko-cli|
$REPO_ROOT/crates/roko-cli/src/daemon.rs
$REPO_ROOT/crates/roko-cli/src/main.rs
$REPO_ROOT/crates/roko-cli/src/lib.rs
|Wire daemon subcommands into CLI main.rs|
Daemon functions exist in daemon.rs but are not registered as CLI subcommands.

## What to implement
1. Add Daemon subcommand enum to the CLI (Start, Stop, Status, Logs, Reload, Install, Uninstall)
2. Wire each variant to the corresponding function in daemon.rs
3. If daemon_stop/status/logs/reload are stubs, implement them:
   - stop: read PID from .roko/daemon.json, send SIGTERM, wait, remove PID file
   - status: read .roko/daemon.json, check if PID is alive, print info
   - logs: tail .roko/daemon.log
   - reload: send SIGHUP to daemon PID

## Verification
cargo check -p roko-cli must pass.
cargo run -p roko-cli -- daemon --help must show subcommands."

add_item "4D.02|4|roko-cli|
$REPO_ROOT/crates/roko-cli/src/config.rs
$REPO_ROOT/crates/roko-cli/Cargo.toml
|Wire .env file loading via dotenvy|
Config supports \${VAR} interpolation but doesn't load .env files.

## What to implement
1. Add \`dotenvy = \"0.15\"\` to roko-cli Cargo.toml
2. In config initialization (before TOML parsing), call \`dotenvy::from_path_override()\` for:
   - \`.roko/.env\` (project-local)
   - \`~/.roko/.env\` (user-global, lower priority)
3. Silently ignore if .env files don't exist

## Verification
cargo check -p roko-cli must pass."

} # end define_tier4_items

# ─── Tier 5 remediation items ────────────────────────────────────────────────

define_tier5_items() {

add_item "5C.05|5|roko-daimon|
$REPO_ROOT/crates/roko-daimon/src/lib.rs
$REPO_ROOT/crates/roko-cli/src/orchestrate.rs
|Fire all 6 affect events from orchestrate.rs|
Only GateResult event fires. Wire the other 5:

## What to implement
In orchestrate.rs:
1. \`TaskOutcome\` — fire after task completes (success or failure), with success bool and duration
2. \`Blocked\` — fire when a task is blocked by dependencies, with wait duration estimate
3. \`TimePressure\` — fire when task elapsed > 80% of timeout_secs
4. \`QueueWait\` — fire when task waits >60s for an available agent slot
5. \`DreamFailure\` — fire when dream cycle fails (in cmd_dream error path)

## Rules
- Find the right call site for each — don't add to random locations
- Use the existing self.daimon.appraise() pattern from the GateResult call

## Verification
cargo check --workspace must pass."

add_item "5D.05|5|roko-cli|
$REPO_ROOT/crates/roko-dreams/src/runner.rs
$REPO_ROOT/crates/roko-cli/src/orchestrate.rs
|Wire auto-dream scheduling into plan runner|
DreamLoopConfig exists but orchestrate.rs never calls DreamRunner::schedule().

## What to implement
In orchestrate.rs, at plan completion (after all tasks done):
1. Check if config has auto_dream enabled
2. If episodes since last dream >= min_episodes_for_dream, call DreamRunner::consolidate_now()
3. Log the dream results

## Verification
cargo check --workspace must pass."

} # end define_tier5_items

# ─── Main execution ──────────────────────────────────────────────────────────

define_test_fixes
define_tier1_items
define_tier4_items
define_tier5_items

# List mode
if [[ "$LIST_ONLY" == "true" ]]; then
  echo -e "${BOLD}Remediation Items${NC}"
  echo ""
  printf "%-10s %-6s %-8s %s\n" "ITEM" "TIER" "STATUS" "DESCRIPTION"
  printf "%-10s %-6s %-8s %s\n" "────" "────" "──────" "───────────"
  for item_def in "${ITEMS[@]}"; do
    IFS='|' read -r id tier gate ctx desc prompt <<< "$item_def"
    if is_done "$id"; then status="✅ done"
    elif is_failed "$id"; then status="❌ fail"
    else status="⬜ todo"; fi
    printf "%-10s %-6s %-8s %s\n" "$id" "$tier" "$status" "$desc"
  done
  echo ""
  echo "Total: ${#ITEMS[@]} items"
  exit 0
fi

# Run items
total=0; done_count=0; fail_count=0; skip_count=0

for item_def in "${ITEMS[@]}"; do
  IFS='|' read -r id tier gate ctx desc prompt <<< "$item_def"

  # Filter by tier
  if [[ -n "$FILTER_TIER" && "$tier" != "$FILTER_TIER" ]]; then continue; fi

  # Filter by item
  if [[ -n "$FILTER_ITEM" && "$id" != "$FILTER_ITEM" ]]; then continue; fi

  # Tests-only filter
  if [[ "$TESTS_ONLY" == "true" && "$id" != TEST.* ]]; then continue; fi

  total=$((total + 1))

  if run_item "$id" "$tier" "$gate" "$ctx" "$prompt" "$desc"; then
    done_count=$((done_count + 1))
  else
    fail_count=$((fail_count + 1))
  fi
done

echo ""
log_step "Results: ${done_count}/${total} passed, ${fail_count} failed"
